use std::fs;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use actix_multipart::Multipart;
use actix_web::http::StatusCode;
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use aptos_sdk::{rest_client::Client, types::LocalAccount};
use chain_listener::events_listener::run_listener;
use proxirun_sdk::constants::{CONTRACT_ADDRESS, CONTRACT_MODULE, MODULE_IDENTIFIER};
use proxirun_sdk::contract_interact::commit;
use proxirun_sdk::orchestrator::{ImageGenerationSettings, VoiceGenerationSettings};
use proxirun_sdk::{
    contract_interact::finalize_auction,
    events::ContractEvent,
    orchestrator::{TaskDefinition, TaskPayload, TextGenerationPayload, TextGenerationSettings},
};

use dotenv::dotenv;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};
use tokio::io::AsyncWriteExt;
use tokio::time::{sleep_until, Instant};
use tokio_stream::StreamExt;


const INDEXER_URL: &'static str = "https://grpc.testnet.aptoslabs.com";
const TESTNET_NODE: &'static str = "https://fullnode.testnet.aptoslabs.com";

const DELTA_TIME: u64 = 500000; // 500 ms

#[derive(sqlx::FromRow)]
struct RequestDataDb {
    pub request_id: i64,
    pub task_type: String,
    pub data: String,
    pub model: String,
    pub requester: String
}

#[derive(Clone)]
pub struct AppState {
    pub wallet: Arc<LocalAccount>,
    pub rest_client: Arc<Client>, // Mutex for concurrent access
    pub db_pool: Pool<Postgres>
}

#[get("/request-details/{id}")]
async fn request_details(id: web::Path<u64>, app_state: web::Data<AppState>) -> impl Responder {
    let data = sqlx::query_as::<_, RequestDataDb>("SELECT * from payloads where request_id=$1;")
        .bind(*id as i64)
        .fetch_one(&app_state.db_pool)
        .await
        .unwrap();

        match data.task_type.as_str() {
            "Text Generation" => {
                return Ok::<HttpResponse, actix_web::Error>(
                    HttpResponse::Ok().json(
                        &TaskDefinition::TextGeneration(TextGenerationSettings {
                            model: data.model
                        })
                    )
                );
            }
            "Image Generation" => {
                return Ok::<HttpResponse, actix_web::Error>(
                    HttpResponse::Ok().json(
                        &TaskDefinition::ImageGeneration(ImageGenerationSettings {
                            model: data.model
                        })
                    )
                );
            }
            "Voice Generation" => {
                return Ok::<HttpResponse, actix_web::Error>(
                    HttpResponse::Ok().json(
                        &TaskDefinition::VoiceGeneration(VoiceGenerationSettings {
                            model: data.model
                        })
                    )
                );
            },
            _ => { 
                // invalid task type, shouldn't happen
                return Ok::<HttpResponse, actix_web::Error>(
                    HttpResponse::new(StatusCode::EXPECTATION_FAILED)
                );
            }
        }
}

#[get("/request-payload/{id}")]
async fn request_payload(id: web::Path<u64>, app_state: web::Data<AppState>) -> impl Responder {
    let data = sqlx::query_as::<_, RequestDataDb>("SELECT * from payloads where request_id = $1;")
        .bind(*id as i64)
        .fetch_one(&app_state.db_pool)
        .await
        .unwrap();

    match data.task_type.as_str() {
        "Text Generation" => {
            return Ok::<HttpResponse, actix_web::Error>(
                HttpResponse::Ok().json(
                    &TaskPayload::TextGeneration(
                        serde_json::from_str(
                            &data.data
                        ).unwrap()

                    )
                )
            );
        }
        "Image Generation" => {
            return Ok::<HttpResponse, actix_web::Error>(
                HttpResponse::Ok().json(
                    &TaskPayload::VoiceGeneration(
                        serde_json::from_str(
                            &data.data
                        ).unwrap()

                    )
                )
            );
        }
        "Voice Generation" => {
            return Ok::<HttpResponse, actix_web::Error>(
                HttpResponse::Ok().json(
                    &TaskPayload::VoiceGeneration(
                        serde_json::from_str(
                            &data.data
                        ).unwrap()

                    )
                )
            );
        },
        _ => { 
            // invalid task type, shouldn't happen
            return Ok::<HttpResponse, actix_web::Error>(
                HttpResponse::new(StatusCode::EXPECTATION_FAILED)
            );
        }
    }

    /*
    return serde_json::to_string(&TaskPayload::TextGeneration(TextGenerationPayload {
        system_prompt: "You are a helpful assistant".into(),
        user_prompt: "What is the best kind of pasta? I usually only eat spaghettis".into(),
    }))
    .unwrap();
    */
}

#[post("/submit-text/{id}")]
async fn submit_text(
    id: web::Path<u64>,
    payload: String,
    app_state: web::Data<AppState>,
) -> impl Responder {
    // save on db
    sqlx::query("INSERT into text_completions (request_id, content) values ($1, $2) ;")
        .bind(*id as i64)
        .bind(payload)
        .execute(&app_state.db_pool)
        .await
        .unwrap();

    // update on smart contract
    commit(*id, &app_state.wallet, &app_state.rest_client)
        .await
        .unwrap();

    println!("Request {}: Received commit", *id);

    Ok::<HttpResponse, actix_web::Error>(
        HttpResponse::Ok().body("Submission saved successfully"),
    )
}

#[post("/submit-image/{id}")]
async fn submit_image(
    id: web::Path<u64>,
    mut payload: Multipart,
    app_state: web::Data<AppState>, // Access shared state
) -> impl Responder {
    let file_path = format!("./uploads/{}.jpg", id);
    let mut file = tokio::fs::File::create(file_path).await?;

    while let Some(field) = payload.next().await {
        let mut field = match field {
            Ok(field) => field,
            Err(e) => return Err(actix_web::error::ErrorBadRequest(e.to_string())),
        };

        if field.name() == Some("file") {
            // Write the file content to the file
            while let Some(chunk) = field.next().await {
                let chunk = match chunk {
                    Ok(chunk) => chunk,
                    Err(e) => return Err(actix_web::error::ErrorBadRequest(e.to_string())),
                };

                let _ = file.write_all(&chunk).await?;
            }
        }
    }

    // update on smart contract
    commit(*id, &app_state.wallet, &app_state.rest_client)
        .await
        .unwrap();

    println!("Request {}: Received commit", *id);

    Ok::<HttpResponse, actix_web::Error>(
        HttpResponse::Ok().body("Submission saved successfully"),
    )
}


#[post("/submit-voice/{id}")]
async fn submit_voice(
    id: web::Path<u64>,
    mut payload: Multipart,
    app_state: web::Data<AppState>, // Access shared state
) -> impl Responder {
    let file_path = format!("./uploads/{}.wav", id);
    let mut file = tokio::fs::File::create(file_path).await?;

    while let Some(field) = payload.next().await {
        let mut field = match field {
            Ok(field) => field,
            Err(e) => return Err(actix_web::error::ErrorBadRequest(e.to_string())),
        };

        if field.name() == Some("audio") {
            // Write the file content to the file
            while let Some(chunk) = field.next().await {
                let chunk = match chunk {
                    Ok(chunk) => chunk,
                    Err(e) => return Err(actix_web::error::ErrorBadRequest(e.to_string())),
                };

                let _ = file.write_all(&chunk).await?;
            }
        }
    }

    // update on smart contract
    commit(*id, &app_state.wallet, &app_state.rest_client)
        .await
        .unwrap();

    println!("Request {}: Received commit", *id);

    Ok::<HttpResponse, actix_web::Error>(
        HttpResponse::Ok().body("Submission saved successfully"),
    )
}


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    let auth_token = std::env::var("INDEXER_AUTH_KEY").expect("INDEXER_AUTH_KEY must be set.");
    let admin_priv_key =
        std::env::var("ADMIN_PRIVATE_KEY").expect("ADMIN_PRIVATE_KEY must be set.");

    let orchestrator_url = std::env::var("ORCHESTRATOR_URL").expect("ORCHESTRATOR_URL must be set.");
    let orchestrator_port = std::env::var("ORCHESTRATOR_PORT").expect("ORCHESTRATOR_PORT must be set.");
    let db_url = std::env::var("DB_URL").expect("DB_URL must be set.");

    // make sure that the uploads folder exists
    fs::create_dir_all("./uploads")?;

    // connect to db 
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .unwrap();


    let account = Arc::new(LocalAccount::from_private_key(&admin_priv_key, 0).unwrap());
    let rest_client = Arc::new(Client::new(TESTNET_NODE.parse().unwrap()));

    let res = rest_client.get_account(account.address()).await.unwrap();
    account.set_sequence_number(res.inner().sequence_number);

    let (sender_events, mut receiver_events) =
        tokio::sync::mpsc::unbounded_channel::<ContractEvent>();

    tokio::spawn(async move {
        run_listener(
            &auth_token,
            INDEXER_URL,
            CONTRACT_MODULE.to_owned(),
            sender_events,
        )
        .await
        .unwrap();
    });

    let temp_account = account.clone();
    let temp_rest_client = rest_client.clone();
    tokio::spawn(async move {
        while let Some(e) = receiver_events.recv().await {
            match e {
                ContractEvent::OnNewWorkRequest(new_work_request) => {
                    println!("Request {}: Scheduling auction finalization", new_work_request.request_id);
                    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
                    let now_as_instant = Instant::now();
                    let target_time = now_as_instant
                        + Duration::from_micros(new_work_request.time_limit + DELTA_TIME)
                        - now;

                    let task_account = temp_account.clone();
                    let task_client = temp_rest_client.clone();
                    tokio::spawn(async move {
                        // Calculate the target time as an Instant
                        sleep_until(target_time).await;

                        println!("Request {}: Sending finalization", new_work_request.request_id);
                        // notify the blockchain client to finalize the auction
                        finalize_auction(new_work_request.request_id, &task_account, &task_client)
                            .await
                            .unwrap();
                        println!("Request {}: Auction finalized", new_work_request.request_id);
                    });
                }
                _ => (),
            }
        }
    });

    let app_state = web::Data::new(AppState {
        wallet: account.clone(),
        rest_client: rest_client.clone(),
        db_pool: pool
    });

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .service(request_details)
            .service(request_payload)
            .service(submit_text)
            .service(submit_image)
            .service(submit_voice)
    })
    .bind(("127.0.0.1", orchestrator_port.parse().unwrap()))?
    .run()
    .await
}

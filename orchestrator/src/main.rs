use std::fs;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use actix_files::NamedFile;
use actix_multipart::Multipart;
use actix_web::http::StatusCode;
use actix_web::{get, post, web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use aptos_sdk::rest_client::Transaction;
use aptos_sdk::{rest_client::Client, types::LocalAccount};
use chain_listener::events_listener::run_listener;
use proxirun_sdk::constants::CONTRACT_MODULE;
use proxirun_sdk::contract_interact::commit;
use proxirun_sdk::orchestrator::{ImageGenerationSettings, VoiceGenerationSettings};
use proxirun_sdk::{
    contract_interact::finalize_auction,
    events::ContractEvent,
    orchestrator::{TaskDefinition, TaskPayload, TextGenerationPayload, TextGenerationSettings},
};

use actix_cors::Cors;
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};
use tokio::io::AsyncWriteExt;
use tokio::time::{sleep, sleep_until, Instant};
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
    pub requester: String,
}

#[derive(sqlx::FromRow, Serialize)]
struct TextCompletionDb {
    pub request_id: i64,
    pub content: String,
}

#[derive(Clone)]
pub struct AppState {
    pub wallet: Arc<LocalAccount>,
    pub rest_client: Arc<Client>, // Mutex for concurrent access
    pub db_pool: Pool<Postgres>,
}

#[get("/request-details/{id}")]
async fn request_details(id: web::Path<u64>, app_state: web::Data<AppState>) -> impl Responder {
    let mut data = None;
    let mut try_id: usize = 0;
    let max_retries: usize = 5;
    let mut delay = Duration::from_millis(100); // Initial delay

    while data.is_none() && try_id < max_retries {
        data =
            match sqlx::query_as::<_, RequestDataDb>("SELECT * from payloads where request_id=$1;")
                .bind(*id as i64)
                .fetch_one(&app_state.db_pool)
                .await
            {
                Ok(val) => Some(val),
                Err(_) => {
                    try_id += 1;
                    if try_id < max_retries {
                        sleep(delay).await;
                        delay = delay * 2;
                    }
                    None
                }
            }
    }

    let data = match data {
        Some(d) => d,
        None => {
            return Ok::<HttpResponse, actix_web::Error>(HttpResponse::new(StatusCode::NOT_FOUND))
        }
    };

    match data.task_type.as_str() {
        "Text Generation" => {
            return Ok::<HttpResponse, actix_web::Error>(HttpResponse::Ok().json(
                &TaskDefinition::TextGeneration(TextGenerationSettings { model: data.model }),
            ));
        }
        "Image Generation" => {
            return Ok::<HttpResponse, actix_web::Error>(HttpResponse::Ok().json(
                &TaskDefinition::ImageGeneration(ImageGenerationSettings { model: data.model }),
            ));
        }
        "Voice Generation" => {
            return Ok::<HttpResponse, actix_web::Error>(HttpResponse::Ok().json(
                &TaskDefinition::VoiceGeneration(VoiceGenerationSettings { model: data.model }),
            ));
        }
        _ => {
            // invalid task type, shouldn't happen
            return Ok::<HttpResponse, actix_web::Error>(HttpResponse::new(
                StatusCode::EXPECTATION_FAILED,
            ));
        }
    }
}

#[get("/request-payload/{id}")]
async fn request_payload(id: web::Path<u64>, app_state: web::Data<AppState>) -> impl Responder {
    let mut data = None;
    let mut try_id: usize = 0;
    let max_retries: usize = 5;
    let mut delay = Duration::from_millis(100); // Initial delay

    while data.is_none() && try_id < max_retries {
        data = match sqlx::query_as::<_, RequestDataDb>(
            "SELECT * from payloads where request_id = $1;",
        )
        .bind(*id as i64)
        .fetch_one(&app_state.db_pool)
        .await
        {
            Ok(val) => Some(val),
            Err(_) => {
                try_id += 1;
                if try_id < max_retries {
                    sleep(delay).await;
                    delay = delay * 2;
                }
                None
            }
        }
    }

    let data = match data {
        Some(d) => d,
        None => {
            return Ok::<HttpResponse, actix_web::Error>(HttpResponse::new(StatusCode::NOT_FOUND))
        }
    };

    match data.task_type.as_str() {
        "Text Generation" => {
            return Ok::<HttpResponse, actix_web::Error>(HttpResponse::Ok().json(
                &TaskPayload::TextGeneration(serde_json::from_str(&data.data).unwrap()),
            ));
        }
        "Image Generation" => {
            return Ok::<HttpResponse, actix_web::Error>(HttpResponse::Ok().json(
                &TaskPayload::ImageGeneration(serde_json::from_str(&data.data).unwrap()),
            ));
        }
        "Voice Generation" => {
            return Ok::<HttpResponse, actix_web::Error>(HttpResponse::Ok().json(
                &TaskPayload::VoiceGeneration(serde_json::from_str(&data.data).unwrap()),
            ));
        }
        _ => {
            // invalid task type, shouldn't happen
            return Ok::<HttpResponse, actix_web::Error>(HttpResponse::new(
                StatusCode::EXPECTATION_FAILED,
            ));
        }
    }
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

    Ok::<HttpResponse, actix_web::Error>(HttpResponse::Ok().body("Submission saved successfully"))
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

    Ok::<HttpResponse, actix_web::Error>(HttpResponse::Ok().body("Submission saved successfully"))
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

    Ok::<HttpResponse, actix_web::Error>(HttpResponse::Ok().body("Submission saved successfully"))
}

#[get("/output/{id}")]
async fn get_output(
    id: web::Path<u64>,
    app_state: web::Data<AppState>,
    req: HttpRequest,
) -> impl Responder {
    let mut data = None;
    let mut try_id: usize = 0;
    let max_retries: usize = 3;
    let mut delay = Duration::from_millis(100); // Initial delay

    while data.is_none() && try_id < max_retries {
        data =
            match sqlx::query_as::<_, RequestDataDb>("SELECT * from payloads where request_id=$1;")
                .bind(*id as i64)
                .fetch_one(&app_state.db_pool)
                .await
            {
                Ok(val) => Some(val),
                Err(_) => {
                    try_id += 1;
                    if try_id < max_retries {
                        sleep(delay).await;
                        delay = delay * 2;
                    }
                    None
                }
            }
    }

    let data = match data {
        Some(d) => d,
        None => {
            return Ok::<HttpResponse, actix_web::Error>(HttpResponse::new(StatusCode::NOT_FOUND))
        }
    };

    match data.task_type.as_str() {
        "Text Generation" => {
            // query from db
            match sqlx::query_as::<_, TextCompletionDb>(
                "SELECT * from text_completions where request_id=$1;",
            )
            .bind(*id as i64)
            .fetch_one(&app_state.db_pool)
            .await
            {
                Ok(val) => {
                    return Ok::<HttpResponse, actix_web::Error>(
                        HttpResponse::Ok()
                            .content_type("application/json")
                            .json(val),
                    );
                }
                Err(_) => {
                    return Ok::<HttpResponse, actix_web::Error>(HttpResponse::new(
                        StatusCode::NOT_FOUND,
                    ));
                }
            }
        }
        "Image Generation" => {
            // send file
            let file_path = format!("./uploads/{}.jpg", *id);
            return Ok(NamedFile::open(file_path).unwrap().into_response(&req));
        }
        "Voice Generation" => {
            // send file
            let file_path = format!("./uploads/{}.wav", *id);
            return Ok(NamedFile::open(file_path).unwrap().into_response(&req));
        }
        _ => {
            println!("Unexpected");
            // invalid task type, shouldn't happen
            return Ok::<HttpResponse, actix_web::Error>(HttpResponse::new(
                StatusCode::EXPECTATION_FAILED,
            ));
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    let auth_token = std::env::var("INDEXER_AUTH_KEY").expect("INDEXER_AUTH_KEY must be set.");
    let admin_priv_key =
        std::env::var("ADMIN_PRIVATE_KEY").expect("ADMIN_PRIVATE_KEY must be set.");

    let orchestrator_url =
        std::env::var("ORCHESTRATOR_URL").expect("ORCHESTRATOR_URL must be set.");
    let orchestrator_port =
        std::env::var("ORCHESTRATOR_PORT").expect("ORCHESTRATOR_PORT must be set.");
    let db_url = std::env::var("DB_URL").expect("DB_URL must be set.");

    println!("Starting orchestrator on port: {}", orchestrator_port);

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
            if let ContractEvent::OnNewWorkRequest(new_work_request) = e {
                println!(
                    "Request {}: Scheduling auction finalization",
                    new_work_request.request_id
                );
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

                    println!(
                        "Request {}: Sending finalization",
                        new_work_request.request_id
                    );

                    let mut finalization_successful = false;
                    let max_try = 5;
                    let mut curr_try = 0;
                    while !finalization_successful && curr_try < max_try {
                        let finalization_res = match finalize_auction(
                            new_work_request.request_id,
                            &task_account,
                            &task_client,
                        )
                        .await
                        {
                            Ok(tx) => tx,
                            Err(_e) => {
                                // tx failed, possibly due to invalid sequence number
                                let res = task_client.get_account(task_account.address()).await.unwrap();
                                task_account.set_sequence_number(res.inner().sequence_number);
                                curr_try += 1;
                                continue;
                            }
                        };

                        // wait for tx to be processed
                        let temp = finalization_res; //.unwrap();
                        let inner = temp.inner();
                        match task_client
                            .wait_for_transaction_by_hash(
                                inner.hash.into(),
                                inner.request.expiration_timestamp_secs.into(),
                                None,
                                None,
                            )
                            .await
                        {
                            Ok(v) => {
                                let v = v.inner();
                                match v {
                                    Transaction::UserTransaction(user_tx) => {
                                        if user_tx.info.success {
                                            finalization_successful = true;
                                        } else {
                                            curr_try += 1;
                                        }
                                    }
                                    _ => {
                                        curr_try += 1;
                                    }
                                }
                            }
                            Err(_) => {
                                curr_try += 1;
                            }
                        }
                    }

                    if finalization_successful {
                        println!("Request {}: Auction finalized", new_work_request.request_id);
                    } else {
                        println!("Request {}: Auction failed to finalize", new_work_request.request_id);
                    }
                });
            }
            else {
                // ignore event 
            }
        }
    });

    let app_state = web::Data::new(AppState {
        wallet: account.clone(),
        rest_client: rest_client.clone(),
        db_pool: pool,
    });

    HttpServer::new(move || {
        App::new()
            .wrap(
                Cors::default()
                    .allow_any_origin()
                    .allow_any_method()
                    .allow_any_header()
                    .max_age(3600), // Optional, caching the preflight response
            )
            .app_data(app_state.clone())
            .service(request_details)
            .service(request_payload)
            .service(submit_text)
            .service(submit_image)
            .service(submit_voice)
            .service(get_output)
    })
    .bind(("127.0.0.1", orchestrator_port.parse().unwrap()))?
    .run()
    .await
}

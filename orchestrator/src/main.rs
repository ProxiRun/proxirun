use std::fs;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use actix_multipart::Multipart;
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use aptos_sdk::{rest_client::Client, types::LocalAccount};
use chain_listener::events_listener::run_listener;
use proxirun_sdk::constants::{CONTRACT_ADDRESS, CONTRACT_MODULE, MODULE_IDENTIFIER};
use proxirun_sdk::contract_interact::commit;
use proxirun_sdk::{
    contract_interact::finalize_auction,
    events::ContractEvent,
    orchestrator::{TaskDefinition, TaskPayload, TextGenerationPayload, TextGenerationSettings},
};

use dotenv::dotenv;
use tokio::io::AsyncWriteExt;
use tokio::time::{sleep_until, Instant};
use tokio_stream::StreamExt;

use std::io::Write;

const INDEXER_URL: &'static str = "https://grpc.testnet.aptoslabs.com";
const TESTNET_NODE: &'static str = "https://fullnode.testnet.aptoslabs.com";

const DELTA_TIME: u64 = 1;


#[derive(Clone)]
pub struct AppState {
    pub wallet: Arc<LocalAccount>,
    pub rest_client: Arc<Client>, // Mutex for concurrent access
}

#[get("/request-details/{id}")]
async fn request_details(_id: web::Path<u64>) -> impl Responder {
    return serde_json::to_string(&TaskDefinition::TextGeneration(TextGenerationSettings {
        model: "ChatGPT".into(),
    }))
    .unwrap();
}

#[get("/request-payload/{id}")]
async fn request_payload(_id: web::Path<u64>) -> impl Responder {
    return serde_json::to_string(&TaskPayload::TextGeneration(TextGenerationPayload {
        system_prompt: "You are a helpful assistant".into(),
        user_prompt: "What is the best kind of pasta? I usually only eat spaghettis".into(),
    }))
    .unwrap();
}

#[post("/submit-text/{id}")]
async fn submit_text(
    id: web::Path<u64>,
    payload: String,
    app_state: web::Data<AppState>,
) -> impl Responder {
    // update on smart contract
    commit(*id, &app_state.wallet, &app_state.rest_client)
        .await
        .unwrap();

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


    // make sure that the uploads folder exists
    fs::create_dir_all("./uploads")?;

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
            println!("Received new event: {:?}", e);
            match e {
                ContractEvent::OnNewWorkRequest(new_work_request) => {
                    println!("Scheduling auction finalization");
                    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
                    let now_as_instant = Instant::now();
                    let target_time = now_as_instant
                        + Duration::from_micros(new_work_request.time_limit + 500000)
                        - now;

                    let task_account = temp_account.clone();
                    let task_client = temp_rest_client.clone();
                    tokio::spawn(async move {
                        // Calculate the target time as an Instant
                        sleep_until(target_time).await;

                        println!("Sending finalization");
                        // notify the blockchain client to finalize the auction
                        finalize_auction(new_work_request.request_id, &task_account, &task_client)
                            .await
                            .unwrap();
                        println!("Finalized");
                    });
                }
                _ => (),
            }
        }
    });

    let app_state = web::Data::new(AppState {
        wallet: account.clone(),
        rest_client: rest_client.clone(),
    });

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .service(request_details)
            .service(request_payload)
    })
    .bind((orchestrator_url, orchestrator_port))?
    .run()
    .await
}

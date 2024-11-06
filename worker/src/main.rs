use aptos_sdk::rest_client::{Client, FaucetClient};
use aptos_sdk::types::LocalAccount;
use openai_api_rust::chat::{ChatApi, ChatBody};
use openai_api_rust::{Auth, Message, OpenAI, Role};
use proxirun_sdk::constants::{CONTRACT_ADDRESS, CONTRACT_MODULE, MODULE_IDENTIFIER};
use rand::rngs::OsRng;
use rand::Rng;
use reqwest::Client as ReqwestClient;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::fs;
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinSet;
use fal_rust::{
    client::{ClientCredentials, FalClient},
    utils::download_image,
};


use chain_listener::events_listener::run_listener;
use proxirun_sdk::events::{ContractEvent, OnBidWon, OnNewWorkRequest};
use proxirun_sdk::orchestrator::{AspectRatio, TaskDefinition, TaskPayload, TextGenerationSettings};

use dotenv::dotenv;


#[derive(Debug, Serialize, Deserialize)]
struct FalImageResult {
    url: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct FalOutput {
    images: Vec<FalImageResult>,
}


const INDEXER_URL: &'static str = "https://grpc.testnet.aptoslabs.com";

const TESTNET_NODE: &'static str = "https://fullnode.testnet.aptoslabs.com";
const FAUCET_URL: &'static str = "https://faucet.testnet.aptoslabs.com";

async fn upload_generated(
    content: Vec<u8>,
    orchestrator_url: &str,
    endpoint: &str,
    request_id: u64,
) -> bool {
    // submit to orchestrator
    let client = ReqwestClient::new();
    let file_part = reqwest::multipart::Part::bytes(content)
        .mime_str("application/octet-stream")
        .unwrap();
    let form = reqwest::multipart::Form::new().part("file", file_part);

    let target_url = format!("{}/{}/{}", orchestrator_url, endpoint, request_id);
    let response = client
        .post(target_url)
        .multipart(form)
        .send()
        .await
        .unwrap();

    if response.status().is_success() {
        println!("Request {} - Commit successful", request_id);
        return true;
    } else {
        println!(
            "Request {} - Commit failed. Status: {}",
            request_id,
            response.status()
        );
        return false;
    }
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let auth_token = std::env::var("INDEXER_AUTH_KEY").expect("INDEXER_AUTH_KEY must be set.");
    let orchestrator_url =
        std::env::var("ORCHESTRATOR_URL").expect("ORCHESTRATOR_URL must be set.");
    let orchestrator_port =
        std::env::var("ORCHESTRATOR_PORT").expect("ORCHESTRATOR_PORT must be set.");
    let full_orchestrator_url = format!("{}:{}", orchestrator_url, orchestrator_port);

    let openai_token = std::env::var("OPENAI_KEY").expect("OPENAI_KEY must be set.");
    let fal_token = std::env::var("FALAI_KEY").expect("FALAI_KEY must be set.");


    let rest_client = Client::new(TESTNET_NODE.parse().unwrap());
    let faucet_client =
        FaucetClient::new(FAUCET_URL.parse().unwrap(), TESTNET_NODE.parse().unwrap());

    //let account = LocalAccount::from_private_key(PRIVATE_KEY, 0)?;

    let mut rng = OsRng::default();
    let account = LocalAccount::generate(&mut rng);
    let account_address = account.address();

    println!(
        "Starting worker with address: {}",
        account_address.to_string()
    );
    println!("Funding testnet account");

    faucet_client
        .fund(account.address(), 100_000_000)
        .await
        .unwrap();

    let mut task_set = JoinSet::new();

    // create channels for communication
    let (sender_events, mut receiver_events) = mpsc::unbounded_channel::<ContractEvent>();
    let (sender_new_work_request, mut receiver_new_work_request) =
        mpsc::unbounded_channel::<OnNewWorkRequest>();
    let (sender_on_bid_won, mut receiver_on_bid_won) = mpsc::unbounded_channel::<OnBidWon>();

    //
    let task_records: Arc<Mutex<HashMap<u64, TaskDefinition>>> =
        Arc::new(Mutex::new(HashMap::new()));

    // start the service to handle new work requests
    let clone = task_records.clone();
    let cloned_url = full_orchestrator_url.clone();
    task_set.spawn(async move {
        while let Some(req) = receiver_new_work_request.recv().await {
            println!("New auction with request_id: {}", req.request_id);

            // fetch work details from server
            let target_url = format!("{}/{}/{}", cloned_url, "request-details", req.request_id);
            let res = reqwest::get(target_url).await.unwrap();
            let deets: TaskDefinition = res.json().await.unwrap();

            //let deets = TaskDefinition::TextGeneration(TextGenerationSettings {});

            // and save work details to the task_records set
            {
                let mut lock = clone.lock().await;
                lock.insert(req.request_id, deets);
            }

            // bid in all cases
            // choose a random price
            let chosen_price = rng.gen_range(1, req.max_price);
            println!(
                "Bidding on request {} with price of: {} APT",
                req.request_id,
                (chosen_price as f64) * 10_f64.powi(-8)
            );

            // and send tx
            let res = proxirun_sdk::contract_interact::bid(
                req.request_id,
                chosen_price,
                &account,
                &rest_client,
            )
            .await
            .unwrap();
        }
    });

    // start the service to handle tasks when bids are won
    let clone = task_records.clone();
    task_set.spawn(async move {
        let cloned_url = full_orchestrator_url.clone(); //.as_str();
        let task_payload_base_url = format!("{}/{}/", &cloned_url, "request-payload");

        // set up worker externals
        let openai = {
            Arc::new(OpenAI::new(
                Auth::new(openai_token.as_str()),
                "https://api.openai.com/v1/",
            ))
        };
        let fal = Arc::new(FalClient::new(ClientCredentials::Key(fal_token)));
        while let Some(req) = receiver_on_bid_won.recv().await {
            // check if is winner of the auction
            if req.winner != account_address.to_string() {
                continue;
            }

            println!("Won auction with request_id: {}", req.request_id);
            let task_definition = {
                let lock = clone.lock().await;
                lock.get(&req.request_id).unwrap().to_owned()
            };

            // need to query the payloads for generation
            let task_payload_target_url = format!("{}{}", task_payload_base_url, req.request_id);
            let res = reqwest::get(task_payload_target_url).await.unwrap();
            let task_payload: TaskPayload = res.json().await.unwrap();

            let temp_url = cloned_url.clone();
            let openai_client = openai.clone();
            let fal_client = fal.clone();
            // then process the work
            tokio::spawn(async move {
                // do the work then submit to orchestrator
                match task_definition {
                    TaskDefinition::TextGeneration(task_def) => {
                        if let TaskPayload::TextGeneration(payload) = task_payload {
                            // process work
                            let body = ChatBody {
                                model: "gpt-3.5-turbo".to_string(),
                                max_tokens: None, //Some(7),
                                temperature: Some(0_f32),
                                top_p: Some(0_f32),
                                n: Some(2),
                                stream: Some(false),
                                stop: None,
                                presence_penalty: None,
                                frequency_penalty: None,
                                logit_bias: None,
                                user: None,
                                messages: vec![
                                    Message {
                                        role: Role::System,
                                        content: payload.system_prompt,
                                    },
                                    Message {
                                        role: Role::User,
                                        content: payload.user_prompt,
                                    },
                                ],
                            };

                            let rs = openai_client.chat_completion_create(&body);
                            let choice = rs.unwrap().choices;
                            let message = choice[0].message.clone().unwrap().content;

                            // submit to orchestrator
                            // Send the text payload to the server using a POST request
                            let client = ReqwestClient::new();
                            let response = client
                                .post(&format!("{}/submit-text/{}", temp_url, req.request_id))
                                .body(message)
                                .send()
                                .await
                                .unwrap();

                            // Check the response from the server
                            if response.status().is_success() {
                                println!("Request {} - Commit successful", req.request_id);
                            } else {
                                println!(
                                    "Request {} - Commit failed. Status: {}",
                                    req.request_id,
                                    response.status()
                                );
                            }
                        } else {
                            println!(
                                "Mismatch between task definition and task payload for request {}",
                                req.request_id
                            );
                        }
                    }
                    TaskDefinition::ImageGeneration(task_def) => {
                        if let TaskPayload::ImageGeneration(payload) = task_payload {
                            let image_size = match payload.aspect_ratio {
                                AspectRatio::Landscape => "landscape_4_3",
                                AspectRatio::Portrait => "portrait_4_3",
                                AspectRatio::Square => "square"
                            };

                            let res = fal_client
                            .run(
                                "fal-ai/fast-sdxl",
                                serde_json::json!({
                                    "prompt": payload.positive_prompt,
                                    "image_size": image_size,
                                    "negative_prompt": payload.negative_prompt
                    
                                }),
                            )
                            .await
                            .unwrap();
                    
                            let output: FalOutput = res.json::<FalOutput>().await.unwrap();
                        
                            let url = output.images[0].url.clone();
                            //let filename = url.split('/').last().unwrap();
                            
                            let out_file = format!("./{}/{}.jpeg", "temp", req.request_id);
                            download_image(&url, out_file.as_str())
                                .await
                                .unwrap();

                            // process work
                            let file_content = fs::read(out_file).await.unwrap();
                            // submit to orchestrator
                            upload_generated(
                                file_content,
                                temp_url.as_str(),
                                "submit-image",
                                req.request_id,
                            )
                            .await;
                        } else {
                            println!(
                                "Mismatch between task definition and task payload for request {}",
                                req.request_id
                            );
                        }
                    }
                    TaskDefinition::VoiceGeneration(task_def) => {
                        if let TaskPayload::VoiceGeneration(payload) = task_payload {
                            // process work
                            let file_content = fs::read("./temp/audio.wav").await.unwrap();
                            // submit to orchestrator
                            upload_generated(
                                file_content,
                                temp_url.as_str(),
                                "submit-voice",
                                req.request_id,
                            )
                            .await;
                        } else {
                            println!(
                                "Mismatch between task definition and task payload for request {}",
                                req.request_id
                            );
                        }
                    }
                }
            });
        }
    });

    // start chain listener
    task_set.spawn(async move {
        run_listener(
            &auth_token,
            INDEXER_URL,
            CONTRACT_MODULE.to_owned(),
            sender_events,
        )
        .await
        .unwrap();
    });

    task_set.spawn(async move {
        while let Some(e) = receiver_events.recv().await {
            match e {
                ContractEvent::OnNewWorkRequest(data) => {
                    sender_new_work_request.send(data).unwrap();
                }
                ContractEvent::OnBidWon(data) => {
                    sender_on_bid_won.send(data).unwrap();
                }
                _ => (), // ignore other events
            }
        }
    });

    while let Some(res) = task_set.join_next().await {
        res.unwrap();
    }

    return Ok(());
}

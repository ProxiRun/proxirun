use aptos_sdk::rest_client::{Client, FaucetClient};
use aptos_sdk::types::LocalAccount;
use proxirun_sdk::constants::{CONTRACT_ADDRESS, CONTRACT_MODULE, MODULE_IDENTIFIER};
use rand::rngs::OsRng;
use rand::Rng;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinSet;

use chain_listener::events_listener::run_listener;
use proxirun_sdk::events::{ContractEvent, OnBidWon, OnNewWorkRequest};
use proxirun_sdk::orchestrator::{TaskDefinition, TextGenerationSettings};

use dotenv::dotenv;

const INDEXER_URL: &'static str = "https://grpc.testnet.aptoslabs.com";

const TESTNET_NODE: &'static str = "https://fullnode.testnet.aptoslabs.com";
const FAUCET_URL: &'static str = "https://faucet.testnet.aptoslabs.com";

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok(); 
    let auth_token = std::env::var("INDEXER_AUTH_KEY").expect("INDEXER_AUTH_KEY must be set.");
    let orchestrator_url = std::env::var("ORCHESTRATOR_URL").expect("ORCHESTRATOR_URL must be set.");
    let orchestrator_port = std::env::var("ORCHESTRATOR_PORT").expect("ORCHESTRATOR_PORT must be set.");
    let full_orchestrator_url = format!("{}:{}", orchestrator_url, orchestrator_port);


    let rest_client = Client::new(TESTNET_NODE.parse().unwrap());
    let faucet_client =
        FaucetClient::new(FAUCET_URL.parse().unwrap(), TESTNET_NODE.parse().unwrap());

    //let account = LocalAccount::from_private_key(PRIVATE_KEY, 0)?;

    let mut rng = OsRng::default();
    let account = LocalAccount::generate(&mut rng);
    let account_address = account.address();
    println!("account address: {:?}", account);

    faucet_client
        .fund(account.address(), 100_000_000)
        .await
        .unwrap();

    // check balance
    let balance_res = rest_client
        .get_account_balance(account.address())
        .await
        .unwrap();
    println!("account balance: {:?}", balance_res);

    let mut task_set = JoinSet::new();

    // create channels for communication
    let (sender_events, mut receiver_events) =
    mpsc::unbounded_channel::<ContractEvent>();
    let (sender_new_work_request, mut receiver_new_work_request) =
        mpsc::unbounded_channel::<OnNewWorkRequest>();
    let (sender_on_bid_won, mut receiver_on_bid_won) = mpsc::unbounded_channel::<OnBidWon>();

    //
    let task_records: Arc<Mutex<HashMap<u64, TaskDefinition>>> =
        Arc::new(Mutex::new(HashMap::new()));

    // start the service to handle new work requests
    let clone = task_records.clone();
    task_set.spawn(async move {
        while let Some(req) = receiver_new_work_request.recv().await {
            println!("New auction with request_id: {}", req.request_id);

            // fetch work details from server
            let target_url = format!(
                "{}/{}/{}",
                full_orchestrator_url, "request-details", req.request_id
            );
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

            // and send tx
            //let price = rng.gen_range(1, req.)
            let res =
                proxirun_sdk::contract_interact::bid(req.request_id, 10, &account, &rest_client)
                    .await
                    .unwrap();
        }
    });

    // start the service to handle tasks when bids are won
    let clone = task_records.clone();
    task_set.spawn(async move {
        while let Some(req) = receiver_on_bid_won.recv().await {
            // check if is winner of the auction
            if req.winner != account_address.to_string() {
                continue;
            }

            println!("Won auction with request_id: {}", req.request_id);
            let deets = {
                let lock = clone.lock().await;
                lock.get(&req.request_id).unwrap().to_owned()
            };

            // need to query the payloads for generation

            // then process the work
            tokio::spawn(async move {
                // do the work

                // and submit
            });
        }
    });

    // start chain listener
    task_set.spawn(async move {
        run_listener(
            &auth_token,
            INDEXER_URL,
            CONTRACT_MODULE.to_owned(),
            sender_events
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
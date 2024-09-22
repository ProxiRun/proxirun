use std::str::FromStr;

use aptos_protos::indexer::v1::raw_data_client::RawDataClient;
use aptos_protos::indexer::v1::GetTransactionsRequest;
use aptos_protos::transaction::v1::transaction::TxnData;
use aptos_protos::transaction::v1::{BlockMetadataTransaction, Event, UserTransaction};
use aptos_sdk::move_types::language_storage::ModuleId;
use proxirun_sdk::events::ContractEvent;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use tokio::sync::mpsc::UnboundedSender;
use tokio_stream::StreamExt;
use tonic::service::Interceptor;
use tonic::transport::{Channel, Endpoint};

use proxirun_sdk::events::{OnBidWon, OnNewWorkRequest};
use crate::events::ContractEventExtractor;
use proxirun_sdk::orchestrator::TaskDefinition;
use tonic::codegen::InterceptedService;
use tonic::metadata::MetadataValue;
use tonic::{IntoRequest, Request, Status};


#[derive(Clone)]
struct AuthInterceptor {
    pub token: String,
}

impl Interceptor for AuthInterceptor {
    fn call(&mut self, mut request: Request<()>) -> Result<Request<()>, tonic::Status> {
        // Add the authorization header with the token
        let token = format!("Bearer {}", self.token);
        let metadata_value = MetadataValue::from_str(&token)
            .map_err(|_| tonic::Status::unauthenticated("Invalid token"))?;
        request.metadata_mut().insert("authorization", metadata_value);
        
        Ok(request)
    }
}

/*
fn intercept(req: Request<()>) -> Result<Request<()>, Status> {
    let mut req = req.into_request();
    let token = format!("Bearer {}", API_KEY);
    let metadata_value =
        MetadataValue::from_str(&token).map_err(|_| Status::unauthenticated("Invalid token"))?;
    req.metadata_mut().insert("authorization", metadata_value);
    Ok(req)
}
*/

pub async fn run_listener(
    api_key: &str,
    indexer_url: &str,
    module_id: ModuleId,
    sender_events: UnboundedSender<ContractEvent>,
    //sender_new_work_request: UnboundedSender<OnNewWorkRequest>,
    //sender_on_bid_won: UnboundedSender<OnBidWon>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create a gRPC channel
    let channel = Channel::from_shared(indexer_url.to_owned()).unwrap().connect().await?;

    //let mut client = RawDataClient::connect(INDEXER_URL).await?;
    let interceptor = AuthInterceptor {
        token: api_key.to_owned()
    };
    let mut client = RawDataClient::with_interceptor(channel, interceptor);

    let chain_listener = tokio::spawn(async move {
        let req = GetTransactionsRequest {
            starting_version: None,
            transactions_count: None,
            batch_size: None,
        };
        let response = client.get_transactions(req).await.unwrap();

        let mut resp_stream = response.into_inner();

        while let Some(received) = resp_stream.next().await {
            let received = received.unwrap();
            let filtered_events: Vec<Vec<Event>> = received
                .transactions
                .par_iter()
                .filter_map(|txn| {
                    if let Some(txn_data) = &txn.txn_data {
                        match txn_data {
                            TxnData::User(data) => Some(data.events.to_owned()),
                            _ => None,
                        }
                    } else {
                        None
                    }
                })
                .collect();

            let mut flattened: Vec<Event> = vec![];
            for events in filtered_events {
                for e in events {
                        flattened.push(e);
                    
                }
            }

            let filtered_event: Vec<ContractEvent> = flattened
                .par_iter()
                .filter_map(|e| {
                    ContractEvent::extract_event_data_with_filters(
                        e,
                        &module_id.address.to_string(),
                        &module_id.name.to_string(),
                    )
                })
                .collect();

            for e in filtered_event {
                sender_events.send(e).unwrap();
            }
        }
    });

    return Ok(());
}

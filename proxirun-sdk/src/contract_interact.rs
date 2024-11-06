use std::time::{SystemTime, UNIX_EPOCH};

use aptos_sdk::{
    move_types::identifier::Identifier,
    rest_client::{error::RestError, Client, PendingTransaction, Response},
    transaction_builder::TransactionBuilder,
    types::{
        chain_id::ChainId,
        transaction::{EntryFunction, TransactionPayload},
        LocalAccount,
    },
};

use crate::constants::CONTRACT_MODULE;

const TIME_OUT: u64 = 10;

pub async fn bid(
    request_id: u64,
    price: u64,
    account: &LocalAccount,
    client: &Client,
) -> Result<Response<PendingTransaction>, RestError> {
    let chain_id = client.get_index().await.unwrap().into_inner();

    let builder = TransactionBuilder::new(
        TransactionPayload::EntryFunction(EntryFunction::new(
            CONTRACT_MODULE.to_owned(),
            Identifier::new("bid_work_request").unwrap(),
            vec![],
            vec![
                bcs::to_bytes(&request_id).unwrap(),
                bcs::to_bytes(&price).unwrap(),
            ],
        )),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + TIME_OUT,
        ChainId::new(chain_id.chain_id),
    )
    .gas_unit_price(100)
    .max_gas_amount(1_000)
    .sender(account.address())
    .sequence_number(account.sequence_number());

    let signed_txn = account.sign_with_transaction_builder(builder);
    return client.submit(&signed_txn).await;
}

pub async fn finalize_auction(
    request_id: u64,
    account: &LocalAccount,
    client: &Client,
) -> Result<Response<PendingTransaction>, RestError> {
    let chain_id = client.get_index().await.unwrap().into_inner();

    let builder = TransactionBuilder::new(
        TransactionPayload::EntryFunction(EntryFunction::new(
            CONTRACT_MODULE.to_owned(),
            Identifier::new("finalize_auction").unwrap(),
            vec![],
            vec![bcs::to_bytes(&request_id).unwrap()],
        )),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + TIME_OUT,
        ChainId::new(chain_id.chain_id),
    )
    .gas_unit_price(100)
    .max_gas_amount(1_000)
    .sender(account.address())
    .sequence_number(account.sequence_number());

    let signed_txn = account.sign_with_transaction_builder(builder);
    return client.submit(&signed_txn).await;
}

pub async fn commit(
    request_id: u64,
    account: &LocalAccount,
    client: &Client,
) -> Result<Response<PendingTransaction>, RestError> {
    let chain_id = client.get_index().await.unwrap().into_inner();

    let builder = TransactionBuilder::new(
        TransactionPayload::EntryFunction(EntryFunction::new(
            CONTRACT_MODULE.to_owned(),
            Identifier::new("commit").unwrap(),
            vec![],
            vec![bcs::to_bytes(&request_id).unwrap()],
        )),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + TIME_OUT,
        ChainId::new(chain_id.chain_id),
    )
    .gas_unit_price(100)
    .max_gas_amount(1_000)
    .sender(account.address())
    .sequence_number(account.sequence_number());

    let signed_txn = account.sign_with_transaction_builder(builder);
    return client.submit(&signed_txn).await;
}

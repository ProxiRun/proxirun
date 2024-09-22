use serde::Deserialize;

mod string_to_u64 {
    use serde::Deserializer;
    use std::str::FromStr;

    use super::*;
    
    pub fn deserialize<'de, D>(deserializer: D) -> Result<u64, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        u64::from_str(&s).map_err(serde::de::Error::custom)
    }
}


#[derive(Debug, Deserialize)]
pub struct OnNewWorkRequest {
    #[serde(with = "string_to_u64")]
    pub request_id: u64,
    pub requester: String,
    #[serde(with = "string_to_u64")]
    pub time_limit: u64,
}

#[derive(Debug, Deserialize)]
pub struct OnWorkRequestCompleted {
    #[serde(with = "string_to_u64")]
    pub request_id: u64,
}

#[derive(Debug, Deserialize)]
pub struct OnNewWorkRequestBid {
    #[serde(with = "string_to_u64")]
    pub request_id: u64,
    pub bidder: String,
    #[serde(with = "string_to_u64")]
    pub price: u64,
}

#[derive(Debug, Deserialize)]
pub struct OnBidWon {
    #[serde(with = "string_to_u64")]
    pub request_id: u64,
    pub winner: String,
    #[serde(with = "string_to_u64")]
    pub bid_price: u64,
}

#[derive(Debug, Deserialize)]
pub struct OnAuctionFailure {
    #[serde(with = "string_to_u64")]
    pub request_id: u64,
}

#[derive(Debug)]
pub enum ContractEvent {
    OnNewWorkRequest(OnNewWorkRequest),
    OnWorkRequestCompleted(OnWorkRequestCompleted),
    OnNewWorkRequestBid(OnNewWorkRequestBid),
    OnBidWon(OnBidWon),
    OnAuctionFailure(OnAuctionFailure),
}
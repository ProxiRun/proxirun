use aptos_protos::transaction::v1::{move_type::Content, Event};
use proxirun_sdk::events::*;

pub trait ContractEventExtractor where Self: Sized {
    fn extract_event_data_with_filters(
        event: &Event,
        contract_address: &str,
        module_name: &str,
    ) -> Option<Self>;

    fn extract_event_data(event: Event) -> Option<Self>;
}

impl ContractEventExtractor for ContractEvent {
    fn extract_event_data_with_filters(
        event: &Event,
        contract_address: &str,
        module_name: &str,
    ) -> Option<Self> {
        if let Some(event_type) = &event.r#type {
            if let Some(event_content) = &event_type.content {
                match event_content {
                    Content::Struct(s) => {
                        if s.module != module_name && &s.address != contract_address {
                            return None;
                        }

                        return match s.name.as_str() {
                            "OnNewWorkRequest" => Some(ContractEvent::OnNewWorkRequest(
                                serde_json::from_str(&event.data).unwrap(),
                            )),
                            "OnWorkRequestCompleted" => {
                                Some(ContractEvent::OnWorkRequestCompleted(
                                    serde_json::from_str(&event.data).unwrap(),
                                ))
                            }
                            "OnNewWorkRequestBid" => Some(ContractEvent::OnNewWorkRequestBid(
                                serde_json::from_str(&event.data).unwrap(),
                            )),
                            "OnBidWon" => Some(ContractEvent::OnBidWon(
                                serde_json::from_str(&event.data).unwrap(),
                            )),
                            "OnAuctionFailure" => Some(ContractEvent::OnAuctionFailure(
                                serde_json::from_str(&event.data).unwrap(),
                            )),
                            _ => None, //panic!("Unexpected event tag"),
                        };
                    }
                    _ => return None, // panic!("Unexpected"),
                }
            } else {
                return None;
            }
        } else {
            return None;
        }
    }

    fn extract_event_data(event: Event) -> Option<Self> {
        if let Some(event_type) = event.r#type {
            match event_type.content.unwrap() {
                Content::Struct(s) => {
                    return match s.name.as_str() {
                        "OnNewWorkRequest" => Some(ContractEvent::OnNewWorkRequest(
                            serde_json::from_str(&event.data).unwrap(),
                        )),
                        "OnWorkRequestCompleted" => Some(ContractEvent::OnWorkRequestCompleted(
                            serde_json::from_str(&event.data).unwrap(),
                        )),
                        "OnNewWorkRequestBid" => Some(ContractEvent::OnNewWorkRequestBid(
                            serde_json::from_str(&event.data).unwrap(),
                        )),
                        "OnBidWon" => Some(ContractEvent::OnBidWon(
                            serde_json::from_str(&event.data).unwrap(),
                        )),
                        "OnAuctionFailure" => Some(ContractEvent::OnAuctionFailure(
                            serde_json::from_str(&event.data).unwrap(),
                        )),
                        _ => panic!("Unexpected event tag"),
                    }
                }
                _ => panic!("Unexpected"),
            }
        } else {
            return None;
        }
    }
}




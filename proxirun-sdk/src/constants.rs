use std::str::FromStr;

use aptos_sdk::move_types::identifier::Identifier;
use aptos_sdk::move_types::language_storage::ModuleId;
use aptos_sdk::{crypto::_once_cell::sync::Lazy, types::PeerId};
use aptos_sdk::types::account_address::AccountAddress;



const _CONTRACT_ADDRESS: &'static str = "0x9476528b38675eaf7fcc4d18c9472f22efd24532cad25a27794c6f7300df06cc";
const _CONTRACT_MODULE: &'static str = "proxirun";

pub const CONTRACT_ADDRESS: Lazy<AccountAddress> = Lazy::new(|| {
    AccountAddress::from_str(&_CONTRACT_ADDRESS).unwrap()
});

pub const MODULE_IDENTIFIER: Lazy<Identifier> = Lazy::new(|| {
    Identifier::new(_CONTRACT_MODULE).unwrap()
});

pub const CONTRACT_MODULE: Lazy<ModuleId> = Lazy::new(|| {
    ModuleId::new(AccountAddress::from_str(&_CONTRACT_ADDRESS).unwrap(), Identifier::new(_CONTRACT_MODULE).unwrap())
});
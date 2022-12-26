use alloc::borrow::ToOwned;
use casper_contract::{contract_api::runtime, unwrap_or_revert::UnwrapOrRevert};
use casper_types::{system::CallStackElement, Key};

pub(crate) fn get_verified_caller() -> Key {
    match *runtime::get_call_stack()
        .iter()
        .nth_back(1)
        .to_owned()
        .unwrap_or_revert()
    {
        CallStackElement::Session {
            account_hash: calling_account_hash,
        } => Key::Account(calling_account_hash),
        CallStackElement::StoredSession { contract_hash, .. }
        | CallStackElement::StoredContract { contract_hash, .. } => contract_hash.into(),
    }
}

use crate::{
    data::{self, Allowances, Metadata, OwnedTokens, Owners},
    event::CEP47Event,
    utils, Meta, TokenId,
};
use alloc::{string::String, vec::Vec};
use casper_contract::{contract_api::runtime, unwrap_or_revert::UnwrapOrRevert};
use casper_types::{account::AccountHash, ApiError, ContractHash, Key, KeyTag, Tagged, U256};
use contract_utils::{ContractContext, ContractStorage};
use core::convert::TryInto;

#[repr(u16)]
pub enum Error {
    PermissionDenied = 1,
    WrongArguments = 2,
    TokenIdAlreadyExists = 3,
    TokenIdDoesntExist = 4,
    InvalidKey = 69,
    UnlistedContractHash = 81,
    MissingAdminRights = 204,
    MissingMintRights = 205,
    MissingMetadataRights = 206,
    InvalidLength = 207,
    MissingCheckingProperty = 208,
    MissingMetadata = 209,
    MissingMetadataValue = 210,
    DifferentMetadata = 211,
}

impl From<Error> for ApiError {
    fn from(error: Error) -> ApiError {
        ApiError::User(error as u16)
    }
}

#[repr(u8)]
pub enum PermissionsMode {
    /// Installer
    Admins = 0,
    // Installer, whitelisted accounts or contracts
    Mint = 1,
    /// Installer, whitelisted accounts, contracts or whitelisted metadata
    Metadata = 2,
}

pub trait CEP47<Storage: ContractStorage>: ContractContext<Storage> {
    fn init(
        &mut self,
        name: String,
        symbol: String,
        meta: Meta,
        whitelist_accounts: Vec<AccountHash>,
        whitelist_contracts: Vec<ContractHash>,
    ) {
        data::set_name(name);
        data::set_symbol(symbol);
        data::set_meta(meta);
        data::set_total_supply(U256::zero());
        data::set_token_id_counter(U256::zero());
        data::set_whitelist_accounts(whitelist_accounts);
        data::set_whitelist_contracts(whitelist_contracts);
        Owners::init();
        OwnedTokens::init();
        Metadata::init();
        Allowances::init();
    }

    fn name(&self) -> String {
        data::name()
    }

    fn symbol(&self) -> String {
        data::symbol()
    }

    fn meta(&self) -> Meta {
        data::meta()
    }

    fn total_supply(&self) -> U256 {
        data::total_supply()
    }

    fn token_id_counter(&self) -> U256 {
        data::token_id_counter()
    }

    fn balance_of(&self, owner: Key) -> U256 {
        OwnedTokens::instance().get_balances(&owner)
    }

    fn owner_of(&self, token_id: TokenId) -> Option<Key> {
        Owners::instance().get(&token_id)
    }

    fn token_meta(&self, token_id: TokenId) -> Option<Meta> {
        Metadata::instance().get(&token_id)
    }

    fn set_token_meta(&mut self, token_id: TokenId, meta: Meta) -> Result<(), Error> {
        if self.owner_of(token_id).is_none() {
            return Err(Error::TokenIdDoesntExist);
        };

        self.require_permissions(PermissionsMode::Metadata);

        let metadata_dict = Metadata::instance();
        metadata_dict.set(&token_id, meta);

        self.emit(CEP47Event::MetadataUpdate { token_id });
        Ok(())
    }

    fn get_whitelist_accounts(&self) -> Vec<AccountHash> {
        data::get_whitelist_accounts()
    }

    fn set_whitelist_accounts(&mut self, value: Vec<AccountHash>) {
        self.require_permissions(PermissionsMode::Admins);
        data::set_whitelist_accounts(value)
    }

    fn get_whitelist_contracts(&self) -> Vec<ContractHash> {
        data::get_whitelist_contracts()
    }

    fn set_whitelist_contracts(&mut self, value: Vec<ContractHash>) {
        self.require_permissions(PermissionsMode::Admins);
        data::set_whitelist_contracts(value)
    }

    fn get_token_by_index(&self, owner: Key, index: U256) -> Option<TokenId> {
        OwnedTokens::instance().get_token_by_index(&owner, &index)
    }

    fn mint(&mut self, recipient: Key, token_metas: Vec<Meta>) -> Result<Vec<TokenId>, Error> {
        self.require_permissions(PermissionsMode::Mint);

        let owners_dict = Owners::instance();
        let owned_tokens_dict = OwnedTokens::instance();
        let metadata_dict = Metadata::instance();

        let mut token_id = data::token_id_counter();
        let mut token_ids = vec![];

        for token_meta in token_metas {
            token_ids.push(token_id);

            metadata_dict.set(&token_id, token_meta.clone());
            owners_dict.set(&token_id, recipient);
            owned_tokens_dict.set_token(&recipient, &token_id);

            token_id = token_id.checked_add(U256::one()).unwrap_or_revert();
        }

        data::set_token_id_counter(token_id);

        let minted_tokens_count: U256 = From::<u64>::from(token_ids.len().try_into().unwrap());
        let new_total_supply = data::total_supply()
            .checked_add(minted_tokens_count)
            .unwrap();
        data::set_total_supply(new_total_supply);

        self.emit(CEP47Event::Mint {
            recipient,
            token_ids: token_ids.clone(),
        });
        Ok(token_ids)
    }

    fn mint_copies(
        &mut self,
        recipient: Key,
        token_meta: Meta,
        count: u32,
    ) -> Result<Vec<TokenId>, Error> {
        let token_metas = vec![token_meta; count.try_into().unwrap()];
        self.mint(recipient, token_metas)
    }

    fn burn(&mut self, owner: Key, token_ids: Vec<TokenId>) -> Result<(), Error> {
        let spender = self.get_caller();
        if spender != owner {
            for token_id in &token_ids {
                if !self.is_approved(owner, *token_id, spender) {
                    return Err(Error::PermissionDenied);
                }
            }
        }
        self.burn_internal(owner, token_ids)
    }

    fn burn_internal(&mut self, owner: Key, token_ids: Vec<TokenId>) -> Result<(), Error> {
        let owners_dict = Owners::instance();
        let owned_tokens_dict = OwnedTokens::instance();
        let metadata_dict = Metadata::instance();
        let allowances_dict = Allowances::instance();

        for token_id in &token_ids {
            match owners_dict.get(token_id) {
                Some(owner_of_key) => {
                    if owner_of_key != owner {
                        return Err(Error::PermissionDenied);
                    }
                }
                None => {
                    return Err(Error::TokenIdDoesntExist);
                }
            }
        }

        for token_id in &token_ids {
            owned_tokens_dict.remove_token(&owner, token_id);
            metadata_dict.remove(token_id);
            owners_dict.remove(token_id);
            allowances_dict.remove(&owner, token_id);
        }

        let burnt_tokens_count: U256 = From::<u64>::from(token_ids.len().try_into().unwrap());
        let new_total_supply = data::total_supply()
            .checked_sub(burnt_tokens_count)
            .unwrap();
        data::set_total_supply(new_total_supply);

        self.emit(CEP47Event::Burn { owner, token_ids });
        Ok(())
    }

    fn approve(&mut self, spender: Key, token_ids: Vec<TokenId>) -> Result<(), Error> {
        let caller = self.get_caller();
        for token_id in &token_ids {
            match self.owner_of(*token_id) {
                None => return Err(Error::WrongArguments),
                Some(owner) if owner != caller => return Err(Error::PermissionDenied),
                Some(_) => Allowances::instance().set(&caller, token_id, spender),
            }
        }
        self.emit(CEP47Event::Approve {
            owner: caller,
            spender,
            token_ids,
        });
        Ok(())
    }

    fn get_approved(&self, owner: Key, token_id: TokenId) -> Option<Key> {
        Allowances::instance().get(&owner, &token_id)
    }

    fn transfer(&mut self, recipient: Key, token_ids: Vec<TokenId>) -> Result<(), Error> {
        self.transfer_from(self.get_caller(), recipient, token_ids)
    }

    fn transfer_from(
        &mut self,
        owner: Key,
        recipient: Key,
        token_ids: Vec<TokenId>,
    ) -> Result<(), Error> {
        let spender = self.get_caller();

        if owner != spender {
            let allowances_dict = Allowances::instance();
            for token_id in &token_ids {
                if !self.is_approved(owner, *token_id, spender) {
                    return Err(Error::PermissionDenied);
                }
                allowances_dict.remove(&owner, token_id);
            }
        }
        self.transfer_from_internal(owner, recipient, token_ids)
    }

    fn transfer_from_internal(
        &mut self,
        owner: Key,
        recipient: Key,
        token_ids: Vec<TokenId>,
    ) -> Result<(), Error> {
        let owners_dict = Owners::instance();
        let owned_tokens_dict = OwnedTokens::instance();

        for token_id in &token_ids {
            match owners_dict.get(token_id) {
                Some(owner_of_key) => {
                    if owner_of_key != owner {
                        return Err(Error::PermissionDenied);
                    }
                }
                None => {
                    return Err(Error::TokenIdDoesntExist);
                }
            }
        }

        for token_id in &token_ids {
            owned_tokens_dict.remove_token(&owner, token_id);
            owned_tokens_dict.set_token(&recipient, token_id);
            owners_dict.set(token_id, recipient);
        }

        self.emit(CEP47Event::Transfer {
            sender: owner,
            recipient,
            token_ids,
        });
        Ok(())
    }

    fn is_approved(&self, owner: Key, token_id: TokenId, spender: Key) -> bool {
        let allowances_dict = Allowances::instance();
        if let Some(spender_of) = allowances_dict.get(&owner, &token_id) {
            if spender_of == spender {
                return true;
            }
        }
        false
    }

    fn merge(&mut self, mut token_ids: Vec<TokenId>, check_prop: &str) -> Result<(), Error> {
        if token_ids.len() < 2 {
            return Err(Error::InvalidLength);
        }
        if check_prop.is_empty() {
            return Err(Error::MissingCheckingProperty);
        }

        let owner = self.get_caller();
        let metadata_dict = Metadata::instance();

        // Keep the last token in list
        let last_id = *token_ids.last().unwrap_or_revert();
        // Confirm owner
        match self.owner_of(last_id) {
            Some(owner_of_key) => {
                if owner_of_key != owner {
                    return Err(Error::PermissionDenied);
                }
            }
            None => {
                return Err(Error::TokenIdDoesntExist);
            }
        }

        let last_metadata = metadata_dict
            .get(&last_id)
            .unwrap_or_revert_with(Error::MissingMetadata);
        let last_prop = last_metadata
            .get(check_prop)
            .unwrap_or_revert_with(Error::MissingMetadataValue);
        if last_prop.is_empty() {
            return Err(Error::MissingMetadataValue);
        }

        // Remove the last item
        token_ids.truncate(token_ids.len() - 1);

        // Verify that they have the same type
        for other_id in &token_ids {
            let other_metadata = metadata_dict
                .get(other_id)
                .unwrap_or_revert_with(Error::MissingMetadata);
            let other_prop = other_metadata
                .get(check_prop)
                .unwrap_or_revert_with(Error::MissingMetadataValue);

            if other_prop != last_prop {
                return Err(Error::DifferentMetadata);
            }
        }

        // Let's burn others
        self.burn_internal(owner, token_ids)
    }

    fn emit(&mut self, event: CEP47Event) {
        data::emit(&event);
    }

    fn require_permissions(&mut self, mode: PermissionsMode) {
        let caller = utils::get_verified_caller();
        match caller.tag() {
            KeyTag::Hash => {
                let calling_contract = caller
                    .into_hash()
                    .map(ContractHash::new)
                    .unwrap_or_default();
                let whitelist_contracts = self.get_whitelist_contracts();
                // We should allow only specific contracts to mint
                if whitelist_contracts.is_empty()
                    || !whitelist_contracts.contains(&calling_contract)
                {
                    runtime::revert(Error::UnlistedContractHash)
                }
            }
            KeyTag::Account => {
                let installer = runtime::get_key("installer")
                    .unwrap_or_revert()
                    .into_account()
                    .unwrap_or_revert();

                let caller_account = runtime::get_caller();
                if installer != caller_account {
                    if let PermissionsMode::Admins = mode {
                        runtime::revert(Error::MissingAdminRights)
                    } else {
                        let whitelist_accounts = self.get_whitelist_accounts();

                        // If no whitelist accounts, we allow everyone to mint
                        // Otherwise, only whitelisted accounts can perform actions
                        let valid = whitelist_accounts.is_empty()
                            || whitelist_accounts.contains(&caller_account);
                        if !valid {
                            if let PermissionsMode::Mint = mode {
                                runtime::revert(Error::MissingMintRights);
                            } else {
                                runtime::revert(Error::MissingMetadataRights);
                            }
                        }
                    }
                }
            }
            _ => runtime::revert(Error::InvalidKey),
        }
    }
}

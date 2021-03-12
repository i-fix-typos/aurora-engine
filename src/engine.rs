#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
#[cfg(feature = "std")]
use std::vec::Vec;

use crate::sdk;
use crate::types::{
    address_to_key, bytes_to_hex, log_to_bytes, storage_to_key, u256_to_arr, FunctionCallArgs,
    KeyPrefix, ViewCallArgs,
};
use borsh::BorshDeserialize;
use evm::backend::{Apply, ApplyBackend, Backend, Basic, Log};
use evm::{ExitReason, ExitSucceed};
use primitive_types::{H160, H256, U256};

pub struct Engine {
    chain_id: U256,
    origin: H160,
}

impl Engine {
    pub fn new(chain_id: u64, origin: H160) -> Self {
        Self {
            chain_id: U256::from(chain_id),
            origin,
        }
    }

    pub fn set_code(address: &H160, code: &[u8]) {
        sdk::write_storage(&address_to_key(KeyPrefix::Code, address), code);
    }

    pub fn remove_code(address: &H160) {
        sdk::remove_storage(&address_to_key(KeyPrefix::Code, address))
    }

    pub fn get_code(address: &H160) -> Vec<u8> {
        sdk::read_storage(&address_to_key(KeyPrefix::Code, address)).unwrap_or_else(Vec::new)
    }

    pub fn get_code_size(address: &H160) -> usize {
        Engine::get_code(&address).len()
    }

    pub fn set_nonce(address: &H160, nonce: &U256) {
        sdk::write_storage(
            &address_to_key(KeyPrefix::Nonce, address),
            &u256_to_arr(nonce),
        );
    }

    pub fn remove_nonce(address: &H160) {
        sdk::remove_storage(&address_to_key(KeyPrefix::Nonce, address))
    }

    pub fn get_nonce(address: &H160) -> U256 {
        sdk::read_storage(&address_to_key(KeyPrefix::Nonce, address))
            .map(|value| U256::from_big_endian(&value))
            .unwrap_or_else(U256::zero)
    }

    pub fn set_balance(address: &H160, balance: &U256) {
        sdk::write_storage(
            &address_to_key(KeyPrefix::Balance, address),
            &u256_to_arr(balance),
        );
    }

    pub fn remove_balance(address: &H160) {
        sdk::remove_storage(&address_to_key(KeyPrefix::Balance, address))
    }

    pub fn get_balance(address: &H160) -> U256 {
        sdk::read_storage(&address_to_key(KeyPrefix::Balance, address))
            .map(|value| U256::from_big_endian(&value))
            .unwrap_or_else(U256::zero)
    }

    pub fn remove_storage(address: &H160, key: &H256) {
        sdk::remove_storage(&storage_to_key(address, key));
    }

    pub fn set_storage(address: &H160, key: &H256, value: &H256) {
        sdk::write_storage(&storage_to_key(address, key), &value.0);
    }

    pub fn get_storage(address: &H160, key: &H256) -> H256 {
        sdk::read_storage(&storage_to_key(address, key))
            .map(|value| H256::from_slice(&value))
            .unwrap_or_else(H256::default)
    }

    pub fn is_account_empty(address: &H160) -> bool {
        let balance = Self::get_balance(address);
        let nonce = Self::get_nonce(address);
        let code_len = Self::get_code_size(address);
        balance == U256::zero() && nonce == U256::zero() && code_len == 0
    }

    /// Removes all storage for the given address.
    pub fn remove_all_storage(_address: &H160) {
        // FIXME: there is presently no way to prefix delete trie state.
    }

    /// Removes an account.
    pub fn remove_account(address: &H160) {
        Self::remove_nonce(address);
        Self::remove_balance(address);
        Self::remove_code(address);
        Self::remove_all_storage(address);
    }

    /// Removes an account if it is empty.
    pub fn remove_account_if_empty(address: &H160) {
        if Self::is_account_empty(address) {
            Self::remove_account(address);
        }
    }

    pub fn deploy_code(&self, _input: &[u8]) -> (ExitReason, H160) {
        let _origin = self.origin();
        let _value = U256::zero();
        (ExitReason::Succeed(ExitSucceed::Stopped), H160::zero()) // TODO
    }

    pub fn call(&self, input: &[u8]) -> (ExitReason, Vec<u8>) {
        let _args = FunctionCallArgs::try_from_slice(&input).unwrap();
        let _origin = self.origin();
        let _value = U256::zero();
        (ExitReason::Succeed(ExitSucceed::Stopped), [].to_vec()) // TODO
    }

    pub fn view(&self, args: ViewCallArgs) -> (ExitReason, Vec<u8>) {
        let _value = U256::from_big_endian(&args.amount);
        (ExitReason::Succeed(ExitSucceed::Stopped), [].to_vec()) // TODO
    }
}

impl evm::backend::Backend for Engine {
    fn gas_price(&self) -> U256 {
        U256::zero()
    }

    fn origin(&self) -> H160 {
        self.origin
    }

    fn block_hash(&self, _number: U256) -> H256 {
        H256::zero() // TODO: https://github.com/near/nearcore/issues/3456
    }

    fn block_number(&self) -> U256 {
        U256::from(sdk::block_index())
    }

    fn block_coinbase(&self) -> H160 {
        H160::zero()
    }

    fn block_timestamp(&self) -> U256 {
        U256::from(sdk::block_timestamp())
    }

    fn block_difficulty(&self) -> U256 {
        U256::zero()
    }

    fn block_gas_limit(&self) -> U256 {
        U256::zero() // TODO
    }

    fn chain_id(&self) -> U256 {
        self.chain_id
    }

    fn exists(&self, address: H160) -> bool {
        !Engine::is_account_empty(&address)
    }

    fn basic(&self, address: H160) -> Basic {
        Basic {
            nonce: Engine::get_nonce(&address),
            balance: Engine::get_balance(&address),
        }
    }

    fn code(&self, address: H160) -> Vec<u8> {
        Engine::get_code(&address)
    }

    fn storage(&self, address: H160, index: H256) -> H256 {
        Engine::get_storage(&address, &index)
    }

    fn original_storage(&self, _address: H160, _index: H256) -> Option<H256> {
        None
    }
}

impl ApplyBackend for Engine {
    fn apply<A, I, L>(&mut self, values: A, logs: L, delete_empty: bool)
    where
        A: IntoIterator<Item = Apply<I>>,
        I: IntoIterator<Item = (H256, H256)>,
        L: IntoIterator<Item = Log>,
    {
        for apply in values {
            match apply {
                Apply::Modify {
                    address,
                    basic,
                    code,
                    storage,
                    reset_storage,
                } => {
                    Engine::set_nonce(&address, &basic.nonce);
                    Engine::set_balance(&address, &basic.balance);
                    if let Some(code) = code {
                        Engine::set_code(&address, &code)
                    }

                    if reset_storage {
                        Engine::remove_all_storage(&address)
                    }

                    for (index, value) in storage {
                        if value == H256::default() {
                            Engine::remove_storage(&address, &index)
                        } else {
                            Engine::set_storage(&address, &index, &value)
                        }
                    }

                    if delete_empty {
                        Engine::remove_account_if_empty(&address)
                    }
                }
                Apply::Delete { address } => Engine::remove_account(&address),
            }
        }

        for log in logs {
            sdk::log_utf8(&bytes_to_hex(&log_to_bytes(log)).into_bytes())
        }
    }
}

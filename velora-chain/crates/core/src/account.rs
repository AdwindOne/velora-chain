use crate::types::{Nonce, U256, H256};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Account {
    pub nonce: Nonce,
    pub balance: U256,
    pub storage_root: H256,
    pub code_hash: H256,
}

impl Account {
    pub fn new(nonce: Nonce, balance: U256, storage_root: H256, code_hash: H256) -> Self {
        Self {
            nonce,
            balance,
            storage_root,
            code_hash,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.balance == U256::zero() && self.nonce == 0 && self.code_hash == H256::zero()
    }
}

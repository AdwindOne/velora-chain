use crate::types::{Address, U256, H256, Nonce};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transaction {
    pub nonce: Nonce,
    pub gas_price: U256,
    pub gas_limit: U256,
    pub to: Option<Address>,
    pub value: U256,
    pub data: Vec<u8>,
    pub v: U256,
    pub r: U256,
    pub s: U256,
    pub from: Option<Address>,
    pub hash: H256,
}

impl Transaction {
    pub fn new(
        nonce: Nonce,
        gas_price: U256,
        gas_limit: U256,
        to: Option<Address>,
        value: U256,
        data: Vec<u8>,
    ) -> Self {
        // In a real implementation, we would calculate the `from` address and `hash`
        Self {
            nonce,
            gas_price,
            gas_limit,
            to,
            value,
            data,
            v: U256::zero(),
            r: U256::zero(),
            s: U256::zero(),
            from: None,
            hash: H256::random(),
        }
    }
}

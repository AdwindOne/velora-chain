use crate::types::{Address, U256, H256, Nonce};
use rlp::{RlpStream, Encodable, Decodable};
use serde::{Deserialize, Serialize};
use sha3::{Keccak256, Digest};
use ethers::types::Signature;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encodable, Decodable)]
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
        let mut tx = Self {
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
            hash: H256::zero(),
        };
        tx.hash = tx.hash();
        tx
    }

    pub fn hash(&self) -> H256 {
        let mut stream = RlpStream::new();
        self.encode(&mut stream);
        H256::from_slice(Keccak256::digest(stream.as_raw()).as_slice())
    }

    pub fn recover_from(&self) -> Result<Address, anyhow::Error> {
        let sig = Signature {
            v: self.v.as_u64(),
            r: self.r.into(),
            s: self.s.into(),
        };
        let recovered = sig.recover(self.hash())?;
        Ok(recovered)
    }
}

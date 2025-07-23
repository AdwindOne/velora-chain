use crate::types::{Address, BlockNumber, H256, U256, Bloom};
use crate::transaction::Transaction;
use rlp::{RlpStream, Encodable, Decodable, Rlp};
use serde::{Deserialize, Serialize};
use sha3::{Keccak256, Digest};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encodable, Decodable)]
pub struct BlockHeader {
    pub parent_hash: H256,
    pub ommers_hash: H256,
    pub beneficiary: Address,
    pub state_root: H256,
    pub transactions_root: H256,
    pub receipts_root: H256,
    pub logs_bloom: Bloom,
    pub difficulty: U256,
    pub number: BlockNumber,
    pub gas_limit: u64,
    pub gas_used: u64,
    pub timestamp: u64,
    pub extra_data: Vec<u8>,
    pub mix_hash: H256,
    pub nonce: H256,
}

impl BlockHeader {
    pub fn hash(&self) -> H256 {
        let mut stream = RlpStream::new();
        self.encode(&mut stream);
        H256::from_slice(Keccak256::digest(stream.as_raw()).as_slice())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Transaction>,
    pub ommers: Vec<BlockHeader>,
}

impl Block {
    pub fn new(header: BlockHeader, transactions: Vec<Transaction>, ommers: Vec<BlockHeader>) -> Self {
        Self {
            header,
            transactions,
            ommers,
        }
    }

    pub fn hash(&self) -> H256 {
        self.header.hash()
    }
}

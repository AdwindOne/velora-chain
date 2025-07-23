use crate::types::{Address, BlockNumber, H256, U256, Bloom};
use crate::transaction::Transaction;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
        // In a real implementation, this would be a proper RLP hash
        H256::random()
    }
}

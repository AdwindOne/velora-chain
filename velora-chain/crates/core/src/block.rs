use crate::types::{Address, BlockNumber, H256, U256, Bloom};
use crate::transaction::Transaction;
use rlp::{RlpStream, Encodable, Decodable, Rlp, DecoderError};
use serde::{Deserialize, Serialize};
use sha3::{Keccak256, Digest};

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

impl Encodable for BlockHeader {
    fn rlp_append(&self, s: &mut RlpStream) {
        s.begin_list(15);
        s.append(&self.parent_hash);
        s.append(&self.ommers_hash);
        s.append(&self.beneficiary);
        s.append(&self.state_root);
        s.append(&self.transactions_root);
        s.append(&self.receipts_root);
        s.append(&self.logs_bloom);
        s.append(&self.difficulty);
        s.append(&self.number);
        s.append(&self.gas_limit);
        s.append(&self.gas_used);
        s.append(&self.timestamp);
        s.append(&self.extra_data);
        s.append(&self.mix_hash);
        s.append(&self.nonce);
    }
}

impl Decodable for BlockHeader {
    fn decode(rlp: &Rlp) -> Result<Self, DecoderError> {
        Ok(Self {
            parent_hash: rlp.val_at(0)?,
            ommers_hash: rlp.val_at(1)?,
            beneficiary: rlp.val_at(2)?,
            state_root: rlp.val_at(3)?,
            transactions_root: rlp.val_at(4)?,
            receipts_root: rlp.val_at(5)?,
            logs_bloom: rlp.val_at(6)?,
            difficulty: rlp.val_at(7)?,
            number: rlp.val_at(8)?,
            gas_limit: rlp.val_at(9)?,
            gas_used: rlp.val_at(10)?,
            timestamp: rlp.val_at(11)?,
            extra_data: rlp.val_at(12)?,
            mix_hash: rlp.val_at(13)?,
            nonce: rlp.val_at(14)?,
        })
    }
}

impl BlockHeader {
    pub fn hash(&self) -> H256 {
        let mut stream = RlpStream::new();
        self.rlp_append(&mut stream);
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

use crate::types::{Address, U256, H256, Nonce};
use rlp::{RlpStream, Encodable, Decodable, Rlp, DecoderError};
use serde::{Deserialize, Serialize};
use sha3::{Keccak256, Digest};
use ethers::types::transaction::eip2718::TypedTransaction;

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
    #[serde(skip)]
    pub from: Option<Address>,
    #[serde(skip)]
    pub hash: H256,
}

impl Encodable for Transaction {
    fn rlp_append(&self, s: &mut RlpStream) {
        s.begin_list(9);
        s.append(&self.nonce);
        s.append(&self.gas_price);
        s.append(&self.gas_limit);
        if let Some(to) = self.to {
            s.append(&to);
        } else {
            s.append(&"");
        }
        s.append(&self.value);
        s.append(&self.data);
        s.append(&self.v);
        s.append(&self.r);
        s.append(&self.s);
    }
}

impl Decodable for Transaction {
    fn decode(rlp: &Rlp) -> Result<Self, DecoderError> {
        Ok(Self {
            nonce: rlp.val_at(0)?,
            gas_price: rlp.val_at(1)?,
            gas_limit: rlp.val_at(2)?,
            to: rlp.val_at(3)?,
            value: rlp.val_at(4)?,
            data: rlp.val_at(5)?,
            v: rlp.val_at(6)?,
            r: rlp.val_at(7)?,
            s: rlp.val_at(8)?,
            from: None,
            hash: H256::zero(),
        })
    }
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
        self.rlp_append(&mut stream);
        H256::from_slice(Keccak256::digest(stream.as_raw()).as_slice())
    }

    pub fn recover_from(raw_tx: &[u8]) -> Result<Address, anyhow::Error> {
        let typed_tx: TypedTransaction = rlp::decode(raw_tx)?;
        let sighash = typed_tx.sighash();
        let sig = typed_tx.signature();
        let from = sig.recover(sighash)?;
        Ok(from)
    }
}

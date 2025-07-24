use velora_core::{Block, Transaction, Genesis, Account};
use revm::{
    Database,
    primitives::{AccountInfo, Bytecode, CfgEnv, TxEnv, TransactTo, B160, B256, U256 as RevmU256},
};
use rocksdb::{DB, Options};
use anyhow::Result;
use velora_core::types::{Address, H256, U256};
use std::sync::Arc;
use std::path::Path;
use bincode::{serialize, deserialize};

const ACCOUNTS_CF: &str = "accounts";
const CODE_CF: &str = "code";
const STORAGE_CF: &str = "storage";
const BLOCK_HASHES_CF: &str = "block_hashes";

pub struct Executor {
    db: Arc<DB>,
}

impl Executor {
    pub fn new(path: &Path) -> Result<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        let cfs = [ACCOUNTS_CF, CODE_CF, STORAGE_CF, BLOCK_HASHES_CF];
        let db = Arc::new(DB::open_cf(&opts, path, cfs)?);

        Ok(Self { db })
    }

    pub fn apply_block(&self, block: &Block) -> Result<()> {
        // In a real implementation, we would execute each transaction in the block
        // and update the state.
        Ok(())
    }

    pub fn apply_genesis(&self, genesis: &Genesis) -> Result<()> {
        let accounts_cf = self.db.cf_handle(ACCOUNTS_CF).unwrap();
        for (addr, acc) in &genesis.alloc {
            let account_info = AccountInfo {
                balance: acc.balance.into(),
                nonce: 0,
                code_hash: B256::zero(), // Will be calculated if code is present
                code: None,
            };
            self.db.put_cf(accounts_cf, addr.as_bytes(), serialize(&account_info)?)?;
        }
        Ok(())
    }
}

impl Database for Executor {
    type Error = anyhow::Error;

    fn basic(&mut self, address: B160) -> Result<Option<AccountInfo>, Self::Error> {
        let accounts_cf = self.db.cf_handle(ACCOUNTS_CF).unwrap();
        match self.db.get_cf(accounts_cf, address.as_bytes())? {
            Some(bytes) => Ok(Some(deserialize(&bytes)?)),
            None => Ok(None),
        }
    }

    fn code_by_hash(&mut self, code_hash: B256) -> Result<Bytecode, Self::Error> {
        let code_cf = self.db.cf_handle(CODE_CF).unwrap();
        match self.db.get_cf(code_cf, code_hash.as_bytes())? {
            Some(bytes) => Ok(Bytecode::new_raw(bytes.into())),
            None => Ok(Bytecode::default()),
        }
    }

    fn storage(&mut self, address: B160, index: RevmU256) -> Result<RevmU256, Self::Error> {
        let storage_cf = self.db.cf_handle(STORAGE_CF).unwrap();
        let mut key = [0u8; 64];
        key[..32].copy_from_slice(address.as_bytes());
        key[32..].copy_from_slice(&index.to_be_bytes::<32>());

        match self.db.get_cf(storage_cf, &key)? {
            Some(bytes) => Ok(RevmU256::from_be_bytes(bytes.try_into().unwrap())),
            None => Ok(RevmU256::ZERO),
        }
    }

    fn block_hash(&mut self, number: RevmU256) -> Result<B256, Self::Error> {
        let block_hashes_cf = self.db.cf_handle(BLOCK_HASHES_CF).unwrap();
        let num_bytes = number.to_be_bytes::<32>();
        match self.db.get_cf(block_hashes_cf, &num_bytes[24..])? { // u64
            Some(bytes) => Ok(B256::from_slice(&bytes)),
            None => Ok(B256::ZERO),
        }
    }
}

use crate::Consensus;
use async_trait::async_trait;
use velora_core::{Block, BlockHeader, Address};
use anyhow::{Result, anyhow};
use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

const MIN_BLOCK_PERIOD_SECS: u64 = 2;

pub struct Poa {
    validators: HashSet<Address>,
}

impl Poa {
    pub fn new(validators: Vec<Address>) -> Self {
        Self {
            validators: validators.into_iter().collect(),
        }
    }
}

#[async_trait]
impl Consensus for Poa {
    async fn verify_header(&self, header: &BlockHeader) -> Result<()> {
        // 这里需要获取 parent header，实际实现中应通过外部 context 获取。
        // 暂时只校验 validator。
        if !self.validators.contains(&header.beneficiary) {
            return Err(anyhow!("Invalid validator: {}", header.beneficiary));
        }
        // 其它校验略去
        Ok(())
    }

    async fn prepare_header(&self, parent: &BlockHeader) -> Result<BlockHeader> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let timestamp = if now <= parent.timestamp {
            parent.timestamp + MIN_BLOCK_PERIOD_SECS
        } else {
            now
        };

        Ok(BlockHeader {
            parent_hash: parent.hash(),
            number: parent.number + 1,
            timestamp,
            beneficiary: *self.validators.iter().next().unwrap(), // Simple leader selection
            ..Default::default()
        })
    }

    async fn finalize_block(&self, block: &mut Block) -> Result<()> {
        // In a real PoA implementation, we would add a signature to the extra_data
        // For now, we'll just leave it empty
        block.header.extra_data = vec![];
        Ok(())
    }
}

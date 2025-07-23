use crate::Consensus;
use async_trait::async_trait;
use velora_core::{Block, BlockHeader, Address};
use anyhow::Result;
use std::collections::HashSet;

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
        if !self.validators.contains(&header.beneficiary) {
            return Err(anyhow::anyhow!("Invalid validator"));
        }
        Ok(())
    }

    async fn prepare_header(&self, parent: &BlockHeader) -> Result<BlockHeader> {
        // In a real implementation, we would do more here, like setting the timestamp
        Ok(BlockHeader {
            parent_hash: parent.hash(),
            number: parent.number + 1,
            ..Default::default()
        })
    }

    async fn finalize_block(&self, block: &mut Block) -> Result<()> {
        // In a real PoA implementation, we might add a signature to the extra_data
        Ok(())
    }
}

use anyhow::Result;
use async_trait::async_trait;
use velora_core::{Block, BlockHeader};

#[async_trait]
pub trait Consensus: Send + Sync {
    async fn verify_header(&self, header: &BlockHeader) -> Result<()>;
    async fn prepare_header(&self, parent: &BlockHeader) -> Result<BlockHeader>;
    async fn finalize_block(&self, block: &mut Block) -> Result<()>;
}

pub mod poa;

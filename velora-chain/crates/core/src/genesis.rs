use crate::types::{Address, U256};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Genesis {
    pub config: GenesisConfig,
    pub nonce: u64,
    pub timestamp: u64,
    pub extra_data: String,
    pub gas_limit: u64,
    pub difficulty: u64,
    pub mix_hash: String,
    pub coinbase: Address,
    pub alloc: HashMap<Address, GenesisAccount>,
    pub number: u64,
    pub gas_used: u64,
    pub parent_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenesisConfig {
    pub chain_id: u64,
    pub homestead_block: u64,
    pub eip150_block: u64,
    pub eip155_block: u64,
    pub eip158_block: u64,
    pub byzantium_block: u64,
    pub constantinople_block: u64,
    pub petersburg_block: u64,
    pub istanbul_block: u64,
    pub berlin_block: u64,
    pub london_block: u64,
    pub arrow_glacier_block: u64,
    pub gray_glacier_block: u64,
    pub merge_netsplit_block: u64,
    pub shanghai_block: u64,
    pub cancun_block: u64,
    pub prague_block: u64,
    pub verkle_block: u64,
    pub poa: Option<PoaConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisAccount {
    pub balance: U256,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoaConfig {
    pub validators: Vec<Address>,
}

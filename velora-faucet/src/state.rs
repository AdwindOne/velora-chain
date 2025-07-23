use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;
use ethers::types::Address;

pub struct AppState {
    pub requests: Mutex<HashMap<Address, Instant>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            requests: Mutex::new(HashMap::new()),
        }
    }
}

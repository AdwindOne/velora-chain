use ethers::prelude::*;
use std::env;
use std::str::FromStr;
use std::sync::Arc;
use once_cell::sync::Lazy;

static PROVIDER: Lazy<Arc<Provider<Http>>> = Lazy::new(|| {
    let rpc_url = env::var("RPC_URL").expect("RPC_URL must be set");
    Arc::new(Provider::<Http>::try_from(rpc_url).expect("Failed to connect to provider"))
});

static WALLET: Lazy<LocalWallet> = Lazy::new(|| {
    let private_key = env::var("PRIVATE_KEY").expect("PRIVATE_KEY must be set");
    private_key.parse::<LocalWallet>().expect("Failed to parse private key")
});

pub async fn send_transaction(to: Address, value: U256) -> Result<TxHash, anyhow::Error> {
    let tx = TransactionRequest::new().to(to).value(value);
    let pending_tx = WALLET.send_transaction(tx, None).await?;
    Ok(*pending_tx)
}

use std::sync::Arc;
use std::thread;
use std::time::Duration;
use ethers::abi::AbiDecode;
use ethers::prelude::{Address, Filter, H160, H256, ProviderError};
use ethers::providers::{Middleware, Provider, StreamExt, Ws};
use ethers::solc::utils::RuntimeOrHandle::Runtime;
use ethers::types::U256;
use eyre::Result;

pub async fn listen_deposit(address: &str) -> Result<(), ProviderError> {
    let client =
        Provider::<Ws>::connect("ws://127.0.0.1:8546")
            .await?;
    let client = Arc::new(client);

    let erc20_transfer_filter =
        Filter::new()
            .from_block(1)
            .address(address.parse::<Address>().expect("failed"))
            .event("Transfer(address,address,uint256)");

    let mut stream = client.subscribe_logs(&erc20_transfer_filter).await?.take(2);

    while let Some(log) = stream.next().await {
        println!(
            "block: {:?}, tx: {:?}, token: {:?}, from: {:?}, to: {:?}, amount: {:?}",
            log.block_number,
            log.transaction_hash,
            log.address,
            Address::from(log.topics[1]),
            Address::from(log.topics[2]),
            U256::decode(log.data)
        )
    }
    Ok(())
}


#[test]
fn test_logs() {
    tokio::runtime::Builder::new_current_thread().enable_io().build().unwrap().block_on(async {
        println!("1");
        let ret = listen_deposit("").await.expect("failed");
        println!("2");
    });
    thread::sleep(Duration::from_secs(5));
}

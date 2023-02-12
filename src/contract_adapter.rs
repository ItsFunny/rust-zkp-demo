use std::sync::Arc;
use ethers::contract::{ContractError, ContractFactory, ContractInstance};
use ethers::core::k256::ecdsa::SigningKey;
use ethers::middleware::SignerMiddleware;
use ethers::prelude::TransactionReceipt;
use ethers::providers::{Http, Middleware, Provider};
use ethers::signers::{LocalWallet, Signer, Wallet};
use ethers_core::k256::elliptic_curve::weierstrass::add;
use crate::get_contract;

pub struct SimpleDeployer {}

impl Default for SimpleDeployer {
    fn default() -> Self {
        SimpleDeployer {}
    }
}

impl SimpleDeployer {
    pub async fn deploy(self, name: &str) -> (SimpleDeployer, ContractInstance<Arc<SignerMiddleware<ethers::providers::Provider<Http>, Wallet<ethers_core::k256::ecdsa::SigningKey>>>, SignerMiddleware<ethers::providers::Provider<Http>, Wallet<ethers_core::k256::ecdsa::SigningKey>>>, TransactionReceipt) {
        let (abi, bytecode, c) = get_contract("KeyedVerifier");
        let abi_code = abi.unwrap();
        let key = "b7700998b973a2cae0cb8e8a328171399c043e57289735aca5f2419bd622297a";
        let wallet = key.parse::<LocalWallet>().unwrap();
        let address = wallet.address();

        let provider = Provider::<Http>::try_from("http://127.0.0.1:26659")
            .unwrap();
        let client = SignerMiddleware::new(provider, (wallet as Wallet<ethers_core::k256::ecdsa::SigningKey>).with_chain_id(100 as u64));
        let client = Arc::new(client);

        let factory = ContractFactory::new(abi_code.clone(), bytecode.unwrap(), client.clone());
        let deployer = factory.deploy(()).expect("f");


        let pending_tx = deployer
            .client()
            .send_transaction(deployer.tx.clone(), None)
            .await.expect("fff");
        let receipt = pending_tx
            .confirmations(1)
            .await.expect("fail")
            .expect("fail2");
        let address = receipt.contract_address.unwrap();
        let contract: ContractInstance<Arc<SignerMiddleware<ethers::providers::Provider<Http>, Wallet<ethers_core::k256::ecdsa::SigningKey>>>, SignerMiddleware<ethers::providers::Provider<Http>, Wallet<ethers_core::k256::ecdsa::SigningKey>>> = ContractInstance::new(address, abi_code.clone(), client.clone());

        println!("合约:{} 部署成功,合约地址:{}", name, address.to_string());

        (self, contract, receipt)
    }
}

#[test]
pub fn test_deploy() {
    let dep = SimpleDeployer::default();
    tokio::runtime::Builder::new_current_thread().enable_time().enable_io().build().unwrap().block_on(async {
        dep.deploy("KeyedVerifier").await;
    });
}
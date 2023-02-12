// use std::borrow::Borrow;
// use std::path::{Path, PathBuf};
// use std::sync::Arc;
// use ethers::abi::Abi;
// use ethers::contract::{abigen, ContractError, ContractFactory, ContractInstance};
// use ethers::contract::builders::Deployer;
// use ethers::middleware::gas_escalator::{Frequency, GasEscalatorMiddleware, GeometricGasPrice};
// use ethers::middleware::gas_oracle::GasNow;
// use ethers::middleware::SignerMiddleware;
// use ethers::prelude::{DeploymentTxFactory, PendingTransaction};
// use ethers::providers::{Http, JsonRpcClient, Middleware, Provider, ProviderError};
// use ethers::signers::{LocalWallet, Signer, Wallet};
// use ethers::solc::{Artifact, Project, ProjectPathsConfig};
// use ethers::types::{Bytes, TransactionReceipt};
// use ethers_core::k256::elliptic_curve::weierstrass::add;
// use ethers_core::types::BlockId;
//
// pub fn deploy() {}
//
// async fn deploy_plonk_verify_contract() -> Result<(), ProviderError> {
//     let (abi, bytecode, c) = get_contract("KeyedVerifier");
//     let key = "b7700998b973a2cae0cb8e8a328171399c043e57289735aca5f2419bd622297a";
//     let wallet = key.parse::<LocalWallet>().unwrap();
//     let address = wallet.address();
//
//
//     let provider = Provider::<Http>::try_from("http://127.0.0.1:26659")
//         .unwrap();
//     let client = SignerMiddleware::new(provider, (wallet as Wallet<ethers_core::k256::ecdsa::SigningKey>).with_chain_id(100 as u64));
//     let client = Arc::new(client);
//
//
//     let factory = ContractFactory::new(abi.unwrap(), bytecode.unwrap(), client.clone());
//     let deployer = factory.deploy(()).expect("f");
//     // // let couple = deployer.send_with_receipt2().await.expect("fail 2");
//     // let pending_tx = deployer
//     //     .client()
//     //     .send_transaction(deployer.tx.clone(), None)
//     //     .await.expect("fff");
//     // let receipt = pending_tx
//     //     .confirmations(1)
//     //     .await
//     //     .expect("3").unwrap();
//     // let address = receipt.contract_address.ok_or(ContractError::ContractNotDeployed).expect("3");
//     //
//     // let contract = ContractInstance::new(address, abi.unwrap().clone(), client);
//     //
//
//     // let res = do_deploy2(abi.unwrap(), bytecode.unwrap(), client.clone()).await;
//     // let contract = couple.0;
//     // // 7. get the contract's address
//     // // let addr = contract.address();
//     // println!("部署成功:{}", contract.address());
//
//
//     {
//         let pending_tx = deployer
//             .client()
//             .send_transaction(deployer.tx.clone(), None)
//             .await.expect("fff");
//         do_deploy()
//     }
//     Ok(())
// }
//
// // pub async fn do_deploy2<B: Borrow<M> + Clone, M: Middleware, >(abi: Abi, byte_code: Bytes, client: B) -> (ContractInstance<B, M>, TransactionReceipt) {
// //     let factory: DeploymentTxFactory<Arc<Middleware>, Middleware> = ContractFactory::new(abi, byte_code, client.clone());
// //     let deployer = factory.deploy(()).expect("f");
// //     // let couple = deployer.send_with_receipt2().await.expect("fail 2");
// //     let pending_tx = deployer
// //         .client()
// //         .send_transaction(deployer.tx.clone(), None)
// //         .await.expect("fff");
// //     let receipt = pending_tx
// //         .confirmations(1)
// //         .await
// //         .expect("fail")
// //         .ok_or(ContractError::ContractNotDeployed).expect("unwrap err");
// //     let address = receipt.contract_address.ok_or(ContractError::ContractNotDeployed).expect("fail add");
// //
// //     let contract = ContractInstance::new(address, abi.clone(), client.clone());
// //     (contract, receipt)
// // }
//
// async fn do_deploy<B: Borrow<M> + Clone, M: JsonRpcClient + Middleware>
// (pending_tx: PendingTransaction<'_, M>, abi: Abi, client: B) -> Result<(ContractInstance<B, M>, TransactionReceipt), ContractError<M>> {
//     let receipt = pending_tx
//         .confirmations(1)
//         .await
//         .map_err(|_| ContractError::ContractNotDeployed)?
//         .ok_or(ContractError::ContractNotDeployed)?;
//     let address = receipt.contract_address.ok_or(ContractError::ContractNotDeployed)?;
//
//     let contract = ContractInstance::new(address, abi, client);
//     Ok((contract, receipt))
// }
//
// pub fn get_contract(name: &str) -> (Option<Abi>, Option<Bytes>, Option<Bytes>) {
//     let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("config/contracts");
//     println!("{:?}", root);
//     let paths = ProjectPathsConfig::builder().root(&root).sources(&root).build().unwrap();
//     println!("{:?}", paths);
//     let project = Project::builder().paths(paths).ephemeral().no_artifacts().build().unwrap();
//     let output = project.compile().unwrap();
//     let contract = output.find_first(name).expect("could not find contract").clone();
//     contract.into_parts()
// }
//
//
// #[test]
// pub fn test_get_contract() {
//     let (a, b, c) = get_contract("KeyedVerifier");
//     println!("{:?},{:?},{:?}", a, b, c);
// }
//
// #[test]
// pub fn test_deploy_plonk() {
//     tokio::runtime::Builder::new_current_thread().enable_time().enable_io().build().unwrap().block_on(async {
//         deploy_plonk_verify_contract().await
//     });
// }
//

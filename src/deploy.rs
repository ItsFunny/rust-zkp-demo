use std::path::{Path, PathBuf};
use ethers::abi::Abi;
use ethers::solc::{Artifact, Project, ProjectPathsConfig};
use ethers::types::Bytes;

pub fn deploy() {}

fn deploy_plonk_verify_contract<P: AsRef<Path>>(path: P) {}

pub fn get_contract(name: &str) -> (Option<Abi>, Option<Bytes>, Option<Bytes>) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("config/contracts");
    println!("{:?}", root);
    let paths = ProjectPathsConfig::builder().root(&root).sources(&root).build().unwrap();
    println!("{:?}", paths);
    let project = Project::builder().paths(paths).ephemeral().no_artifacts().build().unwrap();
    let output = project.compile().unwrap();
    let contract = output.find_first(name).expect("could not find contract").clone();
    contract.into_parts()
}


#[test]
pub fn test_get_contract() {
    let (a, b, c) = get_contract("KeyedVerifier");
    println!("{:?},{:?},{:?}", a, b, c);
}
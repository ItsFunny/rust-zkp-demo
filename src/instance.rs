use std::collections::HashMap;
use std::error::Error;
use std::fmt::format;
use std::fs;
use std::fs::{OpenOptions, read};
use std::io::{BufReader, Read, Seek};
use std::sync::{Arc, Mutex, MutexGuard, RwLock};
use ethers::prelude::artifacts::BinaryOperator::LessThan;
use plonkit::bellman_ce::{Circuit, Engine};
use plonkit::bellman_ce::bn256::Bn256;
use plonkit::circom_circuit::CircomCircuit;
use plonkit::{bellman_ce, plonk, reader};
use plonkit::bellman_ce::plonk::better_cs::cs::PlonkCsWidth4WithNextStepParams;
use plonkit::bellman_ce::plonk::VerificationKey;
use plonkit::plonk::SetupForProver;
use serde::{Serialize, Deserialize};

const MONOMIAL_KEY_FILE: &'static str = concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/plonk/setup/setup_2^10.key");
const TEMPLATE_SOL: &'static str = concat!(env!("CARGO_MANIFEST_DIR"), "/config/template.sol");
const SAVE_TEMP_PATH: &'static str = concat!(env!("CARGO_MANIFEST_DIR", "/temp"));
// pub struct ZKPInstance<E: Engine> {}


pub struct ZKPCircomInstance {
    pub id: String,
    pub prover: SetupForProver,
    pub vk: VerificationKey<Bn256, PlonkCsWidth4WithNextStepParams>,
}

#[derive(Default)]
pub struct ZKPFactory {}

impl ZKPFactory {
    pub fn build<R: Read + Seek>(self, id: String, r: R) -> ZKPCircomInstance {
        let (r1cs, _) = reader::load_r1cs_from_bin(r);
        let circuit = CircomCircuit {
            r1cs: r1cs,
            witness: None,
            wire_mapping: None,
            aux_offset: plonk::AUX_OFFSET,
        };

        let setup = plonk::SetupForProver::prepare_setup_for_prover(
            circuit.clone(),
            reader::load_key_monomial_form(MONOMIAL_KEY_FILE),
            reader::maybe_load_key_lagrange_form(None),
        )
            .unwrap();

        let vk = setup.make_verification_key().expect("fail to create vk");

        ZKPCircomInstance { id: id, prover: setup, vk: (vk.clone() as VerificationKey<Bn256, PlonkCsWidth4WithNextStepParams>) }
    }
}

impl ZKPCircomInstance {
    pub fn get(&mut self) -> (Vec<u8>, Vec<u8>) {
        let mut vk_bytes = Vec::<u8>::new();
        self.vk.clone().write(&mut vk_bytes).unwrap();
        let path: String = SAVE_TEMP_PATH.to_string() + &(format!("{}.sol", self.id.clone()));
        bellman_vk_codegen::render_verification_key(&self.vk, TEMPLATE_SOL, path.clone().as_str());
        let sol_bytes = fs::read(path.clone()).expect("fail");
        (vk_bytes, sol_bytes)
    }
}


pub struct ZKPProverContainer {
    mutex: RwLock<HashMap<String, Arc<Mutex<ZKPCircomInstance>>>>,
    pub nodes: HashMap<String, Arc<ZKPCircomInstance>>,
}

impl Default for ZKPProverContainer {
    fn default() -> Self {
        Self {
            mutex: Default::default(),
            nodes: Default::default(),
        }
    }
}

impl ZKPProverContainer {
    pub fn register<R: Read + Seek>(&mut self, req: RegisterRequest<R>) -> RegisterResponse {
        let mut cache = self.mutex.write().unwrap();
        let instance = cache.entry(req.key.clone()).or_insert(
            Arc::new(Mutex::new(ZKPFactory::default().build(req.id, req.reader)))
        );
        let (vk, sol) = instance.clone().lock().unwrap().get();
        RegisterResponse { vk, sol }
    }
}

pub struct RegisterRequest<R: Read + Seek> {
    pub key: String,
    pub reader: R,
    pub id: String,
}

impl<R: Read + Seek> RegisterRequest<R> {
    pub fn new(id: String, key: String, reader: R) -> Self {
        Self { key, reader, id }
    }
}


#[derive(Serialize, Deserialize, Debug)]
pub struct RegisterResponse {
    pub vk: Vec<u8>,
    pub sol: Vec<u8>,
}


#[test]
pub fn test_container() {
    let mut container = ZKPProverContainer::default();
    let r1cs_file_path = concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/circoms/mycircuit.r1cs");
    let reader = OpenOptions::new().read(true).open(r1cs_file_path).expect("unable to open.");
    let ret = container.register(RegisterRequest::new(String::from("123"), String::from("demo"), reader));
    println!("{:?}", ret);
}
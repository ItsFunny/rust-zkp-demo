use std::collections::HashMap;
use std::fmt::{format};
use std::fs;
use std::fs::{OpenOptions, read};
use std::io::{BufReader, Error, ErrorKind, Read, Seek};
use std::sync::{Arc, Mutex, MutexGuard, RwLock};
use ethers::prelude::artifacts::BinaryOperator::LessThan;
use plonkit::bellman_ce::{Circuit, Engine};
use plonkit::bellman_ce::bn256::Bn256;
use plonkit::circom_circuit::{CircomCircuit, R1CS};
use plonkit::{bellman_ce, plonk, reader};
use plonkit::bellman_ce::plonk::better_cs::cs::PlonkCsWidth4WithNextStepParams;
use plonkit::bellman_ce::plonk::{Proof, VerificationKey};
use plonkit::plonk::SetupForProver;
use plonkit::reader::load_witness_from_array;
use primitive_types::U256;
use serde::{Serialize, Deserialize};

const MONOMIAL_KEY_FILE: &'static str = concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/plonk/setup/setup_2^10.key");
const TEMPLATE_SOL: &'static str = concat!(env!("CARGO_MANIFEST_DIR"), "/config/template.sol");
const SAVE_TEMP_PATH: &'static str = concat!(env!("CARGO_MANIFEST_DIR"), "/temp");
const DEFAULT_TRANSCRIPT: &'static str = "keccak";


pub struct ZKPCircomInstance {
    pub r1cs: R1CS<Bn256>,
    pub key: String,
    pub prover: SetupForProver,
    pub vk: VerificationKey<Bn256, PlonkCsWidth4WithNextStepParams>,
}

#[derive(Default)]
pub struct ZKPFactory {}

impl ZKPFactory {
    pub fn build<R: Read + Seek>(self, id: String, r: R) -> ZKPCircomInstance {
        let (r1cs, _) = reader::load_r1cs_from_bin(r);
        let circuit = CircomCircuit {
            r1cs: r1cs.clone(),
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


        ZKPCircomInstance { r1cs: r1cs.clone(), key: id, prover: setup, vk: (vk.clone() as VerificationKey<Bn256, PlonkCsWidth4WithNextStepParams>) }
    }
}

impl ZKPCircomInstance {
    pub fn get(&mut self) -> (Vec<u8>, Vec<u8>) {
        let mut vk_bytes = Vec::<u8>::new();
        self.vk.clone().write(&mut vk_bytes).unwrap();
        let path: String = SAVE_TEMP_PATH.to_string() + &(format!("{}.sol", self.key.clone()));
        println!("path:{}", path);
        bellman_vk_codegen::render_verification_key(&self.vk, TEMPLATE_SOL, path.clone().as_str());
        let sol_bytes = fs::read(path.clone()).expect("fail");
        // rm
        fs::remove_file(path.clone());
        (vk_bytes, sol_bytes)
    }
    pub fn prove(&self, witness: Vec<u8>) -> Result<Proof<Bn256, PlonkCsWidth4WithNextStepParams>, Error> {
        let witness = load_witness_from_array::<Bn256>(witness).map_err(|e| {
            Error::new(ErrorKind::InvalidData, e)
        })?;
        let circuit = CircomCircuit {
            r1cs: self.r1cs.clone(),
            witness: Some(witness),
            wire_mapping: None,
            aux_offset: plonk::AUX_OFFSET,
        };
        let res = self.prover.prove(circuit, DEFAULT_TRANSCRIPT).map_err(|e| {
            Error::new(ErrorKind::InvalidData, e)
        })?;
        let b = plonk::verify(&self.vk.clone(), &res, DEFAULT_TRANSCRIPT).map_err(|e| {
            Error::new(ErrorKind::InvalidData, e)
        })?;
        if !b {
            panic!("fail to verify");
        }
        Ok(res)
    }

    pub fn verify(&self, proof_bytes: Vec<u8>) -> Result<bool, Error> {
        let proof = reader::load_proof_from_bytes::<Bn256>(proof_bytes);
        plonk::verify(&self.vk.clone(), &proof, DEFAULT_TRANSCRIPT).map_err(|e| {
            Error::new(ErrorKind::InvalidData, e)
        })
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
            Arc::new(Mutex::new(ZKPFactory::default().build(req.key.clone(), req.reader)))
        );
        let (vk, sol) = instance.clone().lock().unwrap().get();
        RegisterResponse { vk, sol }
    }
    pub fn prove(&self, req: ProveRequest) -> Result<ProveResponse, Error> {
        let cache = self.mutex.read().unwrap();
        if let Some(instance) = cache.get(req.key.as_str()) {
            let v = instance.lock().unwrap();
            let proof = v.prove(req.wtns)?;
            let (inputs, serialized_proof) = bellman_vk_codegen::serialize_proof(&proof);
            let ser_proof_str = serde_json::to_string_pretty(&serialized_proof).unwrap();
            let ser_inputs_str = serde_json::to_string_pretty(&inputs).unwrap();
            let mut proof_bytes = Vec::<u8>::new();
            proof.write(&mut proof_bytes).map_err(|e| {
                Error::new(ErrorKind::InvalidData, e)
            })?;
            Ok(ProveResponse {
                proof: proof_bytes,
                json_proof: ser_proof_str,
                inputs: inputs.clone(),
                inputs_json: ser_inputs_str,
            })
        } else {
            panic!("asd")
        }
    }
    pub fn verify(&self, req: VerifyRequest) -> Result<VerifyResponse, Error> {
        let cache = self.mutex.read().unwrap();
        if let Some(instance) = cache.get(req.key.as_str()) {
            let v = instance.lock().unwrap();
            let res = v.verify(req.proof_bytes)?;
            Ok(VerifyResponse { verify: res })
        } else {
            panic!("asd")
        }
    }

    pub fn prove_json(&self, req: ProveRequest) {
        let res = self.prove(req).unwrap();
    }
}

pub struct VerifyRequest {
    pub key: String,
    pub proof_bytes: Vec<u8>,
}

pub struct VerifyResponse {
    pub verify: bool,
}

pub struct RegisterRequest<R: Read + Seek> {
    pub key: String,
    pub reader: R,
}

impl<R: Read + Seek> RegisterRequest<R> {
    pub fn new(key: String, reader: R) -> Self {
        Self { key, reader }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RegisterResponse {
    pub vk: Vec<u8>,
    pub sol: Vec<u8>,
}


#[derive(Serialize, Deserialize, Debug)]
pub struct ProveRequest {
    pub key: String,
    pub wtns: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProveResponse {
    pub proof: Vec<u8>,
    pub json_proof: String,
    pub inputs: Vec<U256>,
    pub inputs_json: String,
}

#[test]
pub fn test_container() {
    register_simple();
}

fn register_simple() -> ZKPProverContainer {
    let mut container = ZKPProverContainer::default();
    let r1cs_file_path = concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/circoms/mycircuit.r1cs");
    let reader = OpenOptions::new().read(true).open(r1cs_file_path).expect("unable to open.");
    container.register(RegisterRequest::new(String::from("demo"), reader));
    container
}

#[test]
pub fn test_prove() {
    let mut container = register_simple();
    let wit_file = concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/circoms/witness.wtns");
    let wtns = fs::read(wit_file).expect("fail");
    let res = container.prove(ProveRequest { key: String::from("demo"), wtns }).expect("fail to prove");
    println!("{:?}", res);
}

#[test]
pub fn test_verify() {
    let mut container = register_simple();
    let wit_file = concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/circoms/witness.wtns");
    let wtns = fs::read(wit_file).expect("fail");
    let key = String::from("demo");
    let res = container.prove(ProveRequest { key: key.clone(), wtns }).expect("fail to prove");
    println!("{:?}", res);
    let v = container.verify(VerifyRequest { key: key.clone(), proof_bytes: res.proof }).expect("fail to verify");
    assert!(v.verify);
}
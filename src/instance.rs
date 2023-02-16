use std::collections::HashMap;
use std::fmt::{Display, format, Formatter};
use std::{error, fs, panic};
use std::any::Any;
use std::fs::{OpenOptions, read};
use std::io::{BufReader, Cursor, Error, ErrorKind, Read, Seek};
use std::rc::Rc;
use std::sync::{Arc, Mutex, MutexGuard, RwLock};
use crossbeam::channel::{Receiver, Select, Sender};
use ethers::prelude::artifacts::BinaryOperator::LessThan;
use ethers::utils::hex;
use plonkit::bellman_ce::{Circuit, Engine, SynthesisError};
use plonkit::bellman_ce::bn256::Bn256;
use plonkit::circom_circuit::{CircomCircuit, R1CS};
use plonkit::{bellman_ce, plonk, reader};
use plonkit::bellman_ce::plonk::better_cs::cs::PlonkCsWidth4WithNextStepParams;
use plonkit::bellman_ce::plonk::{Proof, VerificationKey};
use plonkit::plonk::SetupForProver;
use plonkit::reader::load_witness_from_array;
use primitive_types::U256;
use rocket_multipart_form_data::multer::bytes;
use serde::{Serialize, Deserialize};
use tokio::runtime::Runtime;
use tokio::sync::oneshot;
use crate::ZKPInstance;

const MONOMIAL_KEY_FILE: &'static str = concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/plonk/setup/setup_2^10.key");
const TEMPLATE_SOL: &'static str = concat!(env!("CARGO_MANIFEST_DIR"), "/config/template.sol");
const SAVE_TEMP_PATH: &'static str = concat!(env!("CARGO_MANIFEST_DIR"), "/temp");
const DEFAULT_TRANSCRIPT: &'static str = "keccak";

#[derive(Debug, Clone)]
pub struct TempError;

impl Display for TempError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "temp error")
    }
}

impl error::Error for TempError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

#[async_trait]
pub trait ZKComponent: Prover + Verifier + Helper + Send + Sync {
    async fn start_zk(self);
}

#[async_trait]
pub trait Prover {
    async fn async_prove(&self, req: ProveRequest) -> Result<ProveResponse, Error>;
    fn prove(&self, req: ProveRequest) -> Result<ProveResponse, Error>;
}

#[async_trait]
pub trait Verifier {
    async fn async_verify(&self, req: VerifyRequest) -> Result<VerifyResponse, Error>;
    fn verify(&self, req: VerifyRequest) -> Result<VerifyResponse, Error>;
}

pub trait Helper {
    fn get_vk_and_sol(&self) -> Result<(Vec<u8>, Vec<u8>), Error>;
}


pub struct ZKPCircomInstance {
    pub sender: Sender<Cmd>,
    receiver: Receiver<Cmd>,
    pub r1cs: R1CS<Bn256>,
    pub key: String,
    pub prover: Arc<SetupForProver>,
    pub vk: VerificationKey<Bn256, PlonkCsWidth4WithNextStepParams>,
}

impl Clone for ZKPCircomInstance {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            receiver: self.receiver.clone(),
            r1cs: self.r1cs.clone(),
            key: self.key.clone(),
            prover: self.prover.clone(),
            vk: self.vk.clone(),
        }
    }
}

impl ZKPCircomInstance {
    pub fn get(&self) -> (Vec<u8>, Vec<u8>) {
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

    fn do_prove(&self, req: ProveRequest) -> Result<ProveResponse, Error> {
        let witness = req.wtns;
        let witness = load_witness_from_array::<Bn256>(witness).map_err(|e| {
            Error::new(ErrorKind::InvalidData, e)
        })?;
        let circuit = CircomCircuit {
            r1cs: self.r1cs.clone(),
            witness: Some(witness),
            wire_mapping: None,
            aux_offset: plonk::AUX_OFFSET,
        };
        let proof = self.prover.prove(circuit, DEFAULT_TRANSCRIPT).map_err(|e| {
            Error::new(ErrorKind::InvalidData, e)
        })?;
        let b = plonk::verify(&self.vk.clone(), &proof, DEFAULT_TRANSCRIPT).map_err(|e| {
            Error::new(ErrorKind::InvalidData, e)
        })?;
        if !b {
            return Err(Error::new(ErrorKind::InvalidData, TempError {}));
        }
        let (inputs, serialized_proof) = bellman_vk_codegen::serialize_proof(&proof);
        let ser_proof_str = serde_json::to_string_pretty(&serialized_proof).unwrap();
        let ser_inputs_str = serde_json::to_string_pretty(&inputs).unwrap();
        let vv: Vec<U256> = serde_json::from_str(ser_proof_str.clone().as_str()).unwrap();
        assert_eq!(vv, serialized_proof);
        let mut proof_bytes = Vec::<u8>::new();
        proof.write(&mut proof_bytes).map_err(|e| {
            Error::new(ErrorKind::InvalidData, e)
        })?;

        Ok(ProveResponse {
            proof: proof_bytes.clone(),
            hex_proof: hex::encode(proof_bytes.clone()),
            json_proof: ser_proof_str,
            inputs: inputs.clone(),
            inputs_json: ser_inputs_str,
        })
    }

    fn do_verify(&self, req: VerifyRequest) -> Result<VerifyResponse, Error> {
        let proof = reader::load_proof_from_bytes::<Bn256>(req.proof_bytes);
        let v = plonk::verify(&self.vk.clone(), &proof, DEFAULT_TRANSCRIPT).map_err(|e| {
            Error::new(ErrorKind::InvalidData, e)
        })?;
        Ok(VerifyResponse { verify: v })
    }
}

// TODO,这里的,全丢到async fn中
#[async_trait]
impl Prover for ZKPCircomInstance {
    fn prove(&self, req: ProveRequest) -> Result<ProveResponse, Error> {
        futures::executor::block_on(self.async_prove(req))
    }

    async fn async_prove(&self, req: ProveRequest) -> Result<ProveResponse, Error> {
        let (ts, mut rs) = oneshot::channel();
        self.sender.send(Cmd::new(Operation::Prove(req), ts)).map_err(|e| {
            Error::new(ErrorKind::InvalidData, e)
        })?;

        rs.await.map_err(|e| {
            Error::new(ErrorKind::InvalidData, e)
        }).map(|v| {
            if let ResultOperation::Proof(value) = v {
                value
            } else {
                unreachable!()
            }
        })
    }
}

#[async_trait]
impl Verifier for ZKPCircomInstance {
    async fn async_verify(&self, req: VerifyRequest) -> Result<VerifyResponse, Error> {
        let (ts, mut rs) = oneshot::channel();
        self.sender.send(Cmd::new(Operation::Verify(req), ts)).map_err(|e| {
            Error::new(ErrorKind::InvalidData, e)
        })?;
        rs.await.map_err(|e| {
            Error::new(ErrorKind::InvalidData, e)
        }).map(|v| {
            if let ResultOperation::Verify(value) = v {
                value
            } else {
                unreachable!()
            }
        })
    }

    fn verify(&self, req: VerifyRequest) -> Result<VerifyResponse, Error> {
        futures::executor::block_on(self.async_verify(req))
    }
}

impl Helper for ZKPCircomInstance {
    fn get_vk_and_sol(&self) -> Result<(Vec<u8>, Vec<u8>), Error> {
        Ok(self.get())
    }
}

pub enum Operation {
    Prove(ProveRequest),
    Verify(VerifyRequest),
}

#[derive(Debug)]
pub enum ResultOperation {
    Proof(ProveResponse),
    Verify(VerifyResponse),
    Fail(Error),
}

pub trait Event: Send + Sync {}

pub struct Cmd
{
    pub op: Operation,
    pub sender: oneshot::Sender<ResultOperation>,
}

unsafe impl Send for Cmd {}

unsafe impl Sync for Cmd {}

impl Cmd where
{
    pub fn new(op: Operation, sender: oneshot::Sender<ResultOperation>) -> Self {
        Self { op, sender }
    }
}


#[async_trait]
impl ZKComponent for ZKPCircomInstance {
    async fn start_zk(self) {
        let clone_sub = self.receiver.clone();
        let mut sel = Select::new();
        sel.recv(&clone_sub);
        loop {
            let res = clone_sub.try_recv();
            // If the operation turns out not to be ready, retry.
            if let Err(e) = res {
                if e.is_empty() {
                    continue;
                }
            }
            let cmd: Cmd = res.unwrap();
            let mut send_ret: Option<ResultOperation> = None;

            match cmd.op {
                Operation::Prove(value) => {
                    let mut res = self.do_prove(value);
                    match res {
                        Ok(prove_resp) => {
                            send_ret = Some(ResultOperation::Proof(prove_resp));
                        }
                        Err(e) => {
                            send_ret = Some(ResultOperation::Fail(e));
                        }
                    }
                }
                Operation::Verify(value) => {
                    let mut res = self.do_verify(value);
                    match res {
                        Ok(resp) => {
                            send_ret = Some(ResultOperation::Verify(resp));
                        }
                        Err(e) => {
                            send_ret = Some(ResultOperation::Fail(e));
                        }
                    }
                }
                _ => {}
            }
            if let Some(v) = send_ret {
                cmd.sender.send(v).expect("fail to send");
            }
        }
    }
}

#[derive(Default)]
pub struct ZKPFactory {}

impl ZKPFactory {
    pub fn build(self, id: String, r: Vec<u8>) -> ZKPCircomInstance {
        let res = self.build_with_key_type(MONOMIAL_KEY_FILE, id.clone(), r.clone());
        if let Err(e) = res {
            let new_file = concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/plonk/setup/setup_2^20.key");
            self.build_with_key_type(new_file, id.clone(), r.clone()).unwrap()
        } else {
            return res.unwrap();
        }
    }
    // TODO: pass runtime
    pub fn build_and_start(self, rt: Arc<Runtime>, id: String, r: Vec<u8>) -> Box<dyn ZKComponent> {
        let ret = self.build(id, r);
        let v = ret.clone();
        rt.clone().spawn(async move {
            v.clone().start_zk().await
        });
        return Box::new(ret.clone());
    }

    fn build_with_key_type(&self, path: &str, id: String, r: Vec<u8>) -> Result<ZKPCircomInstance, Error> {
        let reader = Cursor::new(r);
        let (r1cs, _) = reader::load_r1cs_from_bin(reader);
        let circuit = CircomCircuit {
            r1cs: r1cs.clone(),
            witness: None,
            wire_mapping: None,
            aux_offset: plonk::AUX_OFFSET,
        };

        let setup = plonk::SetupForProver::prepare_setup_for_prover(
            circuit.clone(),
            reader::load_key_monomial_form(path),
            reader::maybe_load_key_lagrange_form(None),
        )
            .unwrap();

        let res = panic::catch_unwind(|| {
            let _ = setup.get_srs_lagrange_form_from_monomial_form();
        });
        if res.is_err() {
            return Err(Error::new(ErrorKind::InvalidData, TempError));
        }

        let vk = setup.make_verification_key().map_err(|e| {
            Error::new(ErrorKind::InvalidData, e)
        })?;

        let (sender, receiver) = crossbeam::channel::bounded::<Cmd>(10);
        Ok(ZKPCircomInstance { sender: sender.clone(), receiver: receiver.clone(), r1cs: r1cs.clone(), key: id, prover: Arc::new(setup), vk: (vk.clone() as VerificationKey<Bn256, PlonkCsWidth4WithNextStepParams>) })
    }
}


pub struct ZKPProverContainer {
    mutex: RwLock<HashMap<String, Arc<Mutex<Box<dyn ZKComponent>>>>>,
    rt: Arc<Runtime>,
}

impl Default for ZKPProverContainer {
    fn default() -> Self {
        Self {
            mutex: Default::default(),
            rt: Arc::new(tokio::runtime::Builder::new_multi_thread().enable_time().enable_io().build().unwrap()),
        }
    }
}

impl ZKPProverContainer {
    pub fn register(&mut self, req: RegisterRequest) -> RegisterResponse {
        let mut cache = self.mutex.write().unwrap();
        let instance = cache.entry(req.key.clone()).or_insert(
            Arc::new(Mutex::new(ZKPFactory::default().build_and_start(self.rt.clone(), req.key.clone(), req.reader)))
        );
        let (vk, sol) = instance.clone().lock().unwrap().get_vk_and_sol().unwrap();
        let v = String::from_utf8_lossy(sol.as_slice()).to_string();
        RegisterResponse { vk: vk, sol: v }
    }
    pub fn prove(&self, req: ProveRequest) -> Result<ProveResponse, Error> {
        let cache = self.mutex.read().unwrap();
        if let Some(instance) = cache.get(req.key.as_str()) {
            let v = instance.lock().unwrap();
            v.prove(req.clone())
        } else {
            Err(Error::new(ErrorKind::InvalidData, TempError {}))
        }
    }
    pub fn verify(&self, req: VerifyRequest) -> Result<VerifyResponse, Error> {
        let cache = self.mutex.read().unwrap();
        if let Some(instance) = cache.get(req.key.as_str()) {
            let v = instance.lock().unwrap();
            v.verify(req.clone())
        } else {
            Err(Error::new(ErrorKind::InvalidData, TempError {}))
        }
    }
}

pub struct PrettyVerifyRequest {
    pub key: String,
    pub proof: String,
}

impl Into<VerifyRequest> for PrettyVerifyRequest {
    fn into(self) -> VerifyRequest {
        let vv: Vec<U256> = serde_json::from_str(self.proof.as_str()).unwrap();
        VerifyRequest { key: self.key, proof_bytes: vec![] }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct VerifyRequest {
    pub key: String,
    pub proof_bytes: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VerifyResponse {
    pub verify: bool,
}

#[derive(Clone)]
pub struct RegisterRequest {
    pub key: String,
    pub reader: Vec<u8>,
}

impl RegisterRequest {
    pub fn new(key: String, reader: Vec<u8>) -> Self {
        Self { key, reader }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RegisterResponse {
    pub vk: Vec<u8>,
    pub sol: String,
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProveRequest {
    pub key: String,
    pub wtns: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProveResponse {
    pub proof: Vec<u8>,
    pub hex_proof: String,
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
    let mut reader = BufReader::new(reader);
    let mut buffer = Vec::<u8>::new();
    reader.read_to_end(&mut buffer).expect("fail");
    container.register(RegisterRequest::new(String::from("demo"), buffer));
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

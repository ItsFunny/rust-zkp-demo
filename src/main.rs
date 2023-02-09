mod sync;

#[macro_use]
extern crate rocket;
extern crate core;

use tracing::{span, event, Level};
use tracing_subscriber::layer::SubscriberExt;
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt::{Display, Error};
use std::io::Bytes;
use ark_bn254::{Bn254, Fq12, Fr, G1Affine, G2Affine, Parameters};
use ark_circom::{CircomBuilder, CircomCircuit, CircomConfig, read_zkey};
use ark_ff::to_bytes;
use ark_groth16::{generate_random_parameters, prepare_verifying_key, verify_proof, create_random_proof as prove, ProvingKey, Proof};
use ark_std::rand::thread_rng;
use num_bigint::{BigInt, BigUint};
use ark_relations::r1cs::{ConstraintLayer, ConstraintMatrices, ConstraintSynthesizer, ConstraintSystem, ConstraintTrace, OptimizationGoal, Result as R1CSResult, SynthesisError, TracingMode};
use ark_serialize::CanonicalSerialize;
use lazy_static::lazy_static;
use num_traits::{Num, Zero};
use rocket::serde::{Serialize, Deserialize};
use rocket::figment::map;
use rocket::futures::future::{ok, OkInto};
use rocket::http::hyper::body::to_bytes;
use rocket::Request;
use rocket::response::Responder;
use rocket::serde::json::{Json, Value, json, serde_json};



lazy_static! {
    static ref ZKPInstance: ZKP = init_zkp();
}

fn init_zkp() -> ZKP {
    ZKP::new()
}


pub struct ZKP {
    pub zkp_config: CircomConfig<Bn254>,
    pub zkp_params: ProvingKey<Bn254>,
}


impl ZKP {
    pub fn new() -> ZKP {

        // Tracing to help with debugging
        let mut layer = ConstraintLayer::default();
        layer.mode = TracingMode::OnlyConstraints;
        let subscriber = tracing_subscriber::Registry::default().with(layer);
        let _guard = tracing::subscriber::set_default(subscriber);

        let trace = ConstraintTrace::capture();
        println!("Trace is: {:?}", trace);

        let mut rng = thread_rng();
        let cfg = CircomConfig::<Bn254>::new(
            "/Users/lvcong/rust/zk-rust-demo/src/circoms/tpke_single.wasm",
            "/Users/lvcong/rust/zk-rust-demo/src/circoms/tpke_single.r1cs",
        ).unwrap();
        // Test
        let trace = ConstraintTrace::capture();
        println!("Trace is: {:?}", trace);

        // let accounts_root: BigInt = BigInt::from(1);
        // let intermediate_root: BigInt = BigInt::from(2);
        // let accounts_pubkeys: Vec<BigInt> = vec![BigInt::from(3), BigInt::from(4)];
        // let accounts_balances: Vec<BigInt> = vec![BigInt::from(5), BigInt::from(6)];
        // let sender_pubkey: BigInt = BigInt::from(7);
        // let sender_balance: BigInt = BigInt::from(8);
        // let receiver_pubkey: Vec<BigInt> = vec![BigInt::from(9), BigInt::from(10)];
        // let receiver_balance: BigInt = BigInt::from(11);
        // let amount: BigInt = BigInt::from(12);
        // let signature_R8x: BigInt = BigInt::from(13);
        // let signature_R8y: BigInt = BigInt::from(14);
        // let signature_S: BigInt = BigInt::from(15);
        // let sender_proof: BigInt = BigInt::from(16);
        // let sender_proof_pos: BigInt = BigInt::from(17);
        // let receiver_proof: BigInt = BigInt::from(18);
        // let receiver_proof_pos: BigInt = BigInt::from(19);
        //
        let mut builder = CircomBuilder::new(cfg.clone());
        // builder.push_input("accounts_root", accounts_root);
        // builder.push_input("intermediate_root", intermediate_root);
        // for v in accounts_pubkeys {
        //     builder.push_input("accounts_pubkeys", v);
        // }
        // for v in accounts_balances {
        //     builder.push_input("accounts_balances", v);
        // }
        // builder.push_input("sender_pubkey", sender_pubkey);
        // builder.push_input("sender_balance", sender_balance);
        // for v in receiver_pubkey {
        //     builder.push_input("receiver_pubkey", v);
        // }
        // builder.push_input("receiver_balance", receiver_balance);
        // builder.push_input("amount", amount);
        // builder.push_input("signature_R8x", signature_R8x);
        // builder.push_input("signature_R8y", signature_R8y);
        // builder.push_input("signature_S", signature_S);
        // builder.push_input("sender_proof", sender_proof);
        // builder.push_input("sender_proof_pos", sender_proof_pos);
        // builder.push_input("receiver_proof", receiver_proof);
        // builder.push_input("receiver_proof_pos", receiver_proof_pos);
        let trace = ConstraintTrace::capture();
        println!("Trace is: {:?}", trace);
        let circom=builder.setup();
        let trace = ConstraintTrace::capture();
        println!("Trace is: {:?}", trace);

        let params = generate_random_parameters::<Bn254, _, _>(circom, &mut rng).unwrap();
        let trace = ConstraintTrace::capture();
        println!("Trace is: {:?}", trace);
        let cir = builder.build().unwrap();
        let trace = ConstraintTrace::capture();
        println!("Trace is: {:?}", trace);
        ZKP {
            zkp_config: cfg.clone(),
            zkp_params: params.clone(),
        }
    }

    pub fn generate_prove(&self, a: BigInt, b: BigInt) -> (R1CSResult<Proof<Bn254>>, Vec<Fr>) {
        let mut builder = CircomBuilder::new(self.zkp_config.clone());
        builder.push_input("a", a);
        builder.push_input("b", b);
        let mut rng = thread_rng();
        let circom = builder.build().unwrap();
        let mut rng = thread_rng();
        let inputs = circom.get_public_inputs().unwrap();
        (prove(circom, &self.zkp_params, &mut rng), inputs)
    }

    pub fn generate_single_tx_prove(&self,
                                    accounts_root: BigInt,
                                    intermediate_root: BigInt,
                                    accounts_pubkeys: Vec<BigInt>,
                                    accounts_balances: Vec<BigInt>,
                                    sender_pubkey: BigInt,
                                    sender_balance: BigInt,
                                    receiver_pubkey: Vec<BigInt>,
                                    receiver_balance: BigInt,
                                    amount: BigInt,
                                    signature_R8x: BigInt,
                                    signature_R8y: BigInt,
                                    signature_S: BigInt,
                                    sender_proof: BigInt,
                                    sender_proof_pos: BigInt,
                                    receiver_proof: BigInt,
                                    receiver_proof_pos: BigInt,
    ) -> (R1CSResult<Proof<Bn254>>, Vec<Fr>) {
        let mut builder = CircomBuilder::new(self.zkp_config.clone());
        builder.push_input("accounts_root", accounts_root);
        builder.push_input("intermediate_root", intermediate_root);
        for v in accounts_pubkeys {
            builder.push_input("accounts_pubkeys", v);
        }
        for v in accounts_balances {
            builder.push_input("accounts_balances", v);
        }
        builder.push_input("sender_pubkey", sender_pubkey);
        builder.push_input("sender_balance", sender_balance);
        for v in receiver_pubkey {
            builder.push_input("receiver_pubkey", v);
        }
        builder.push_input("receiver_balance", receiver_balance);
        builder.push_input("amount", amount);
        builder.push_input("signature_R8x", signature_R8x);
        builder.push_input("signature_R8y", signature_R8y);
        builder.push_input("signature_S", signature_S);
        builder.push_input("sender_proof", sender_proof);
        builder.push_input("sender_proof_pos", sender_proof_pos);
        builder.push_input("receiver_proof", receiver_proof);
        builder.push_input("receiver_proof_pos", receiver_proof_pos);
        let mut rng = thread_rng();
        let circom = builder.build().unwrap();
        let mut rng = thread_rng();
        let inputs = circom.get_public_inputs().unwrap();
        let res = prove(circom, &self.zkp_params, &mut rng);
        if let Err(e) = res {
            return (Err(e), inputs);
        }
        let proof = res.unwrap();
        let pvk = prepare_verifying_key(&ZKPInstance.zkp_params.vk);
        match verify_proof(&pvk, &proof, &inputs) {
            Err(e) => {
                return (Err(e), inputs);
            }
            Ok(b) => {
                if !b {
                    return (Err(SynthesisError::AssignmentMissing), inputs);
                }
            }
        }

        (Ok(proof), inputs)
    }


    pub fn simple_verify(&self) {}
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
struct ProveRequest<'r> {
    a: Cow<'r, str>,
    b: Cow<'r, str>,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
struct ProveResponse {
    proof: String,
    vk: String,
    public_input: String,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
struct VerifyRequest<'r> {
    proof: Cow<'r, str>,
    vk: Cow<'r, str>,
    public_input: Cow<'r, str>,
}

struct VerifyResponse {}

#[post("/", format = "json", data = "<message>")]
fn simple_prove_api(message: Json<ProveRequest<'_>>) -> String {
    match message.a.parse::<BigInt>() {
        Ok(v) => {
            match message.b.parse::<BigInt>() {
                Ok(vv) => {
                    let zkp_res = ZKPInstance.generate_prove(v, vv);
                    match zkp_res.0 {
                        Ok(data) => {
                            let proof_bytes = to_bytes!(data).unwrap();
                            let vk_bytes = to_bytes!(ZKPInstance.zkp_params.vk).unwrap();
                            let public_input_bytes = to_bytes!(zkp_res.1).unwrap();
                            let ret = ProveResponse { proof: hex::encode(proof_bytes), vk: hex::encode(vk_bytes), public_input: hex::encode(public_input_bytes) };
                            let ret = json!(ret);
                            format!("{}", ret.to_string())
                        }
                        Err(e) => {
                            format!("{}", "err")
                        }
                    }
                }
                Err(e) => {
                    format!("{}", "err")
                }
            }
        }
        Err(e) => {
            format!("{}", "err")
        }
    }
}

#[post("/", format = "json", data = "<message>")]
fn verify_api(message: Json<VerifyRequest>) -> String {
    // let cfg = ZKPInstance.zkp_config.clone();
    // let mut builder = CircomBuilder::new(cfg);
    // builder.push_input("a", 3);
    // builder.push_input("b", 11);
    let proof = message.proof.clone();
    let vk = message.vk.clone();
    let pb = message.public_input.clone();

    if let Ok(proof_resp) = hex::decode(proof.to_string()) {
        if let Ok(vk_resp) = hex::decode(vk.to_string()) {
            if let Ok(pb_resp) = hex::decode(pb.to_string()) {
                // let pvk = prepare_verifying_key(&params.vk);
                // let verified = verify_proof(&pvk, &proof, &inputs).unwrap();
            }
        }
    }
    error_return("asd")
}

fn error_return(str: &'static str) -> String {
    String::from(str)
}

// #[launch]
// fn rocket() -> _ {
//     rocket::build()
//         .mount("/prove", routes![simple_prove_api])
//         .mount("/verify", routes![verify_api])
// }

fn main() {
    let cc = ZKPInstance.zkp_config.clone();
}

#[test]
fn test_generate_tx_proof() {
    let cc = ZKPInstance.zkp_config.clone();
    //
    // let accounts_root: BigInt = BigInt::from(1);
    // let intermediate_root: BigInt = BigInt::from(2);
    // let accounts_pubkeys: Vec<BigInt> = vec![BigInt::from(3), BigInt::from(4)];
    // let accounts_balances: Vec<BigInt> = vec![BigInt::from(5), BigInt::from(6)];
    // let sender_pubkey: BigInt = BigInt::from(7);
    // let sender_balance: BigInt = BigInt::from(8);
    // let receiver_pubkey: Vec<BigInt> = vec![BigInt::from(9), BigInt::from(10)];
    // let receiver_balance: BigInt = BigInt::from(11);
    // let amount: BigInt = BigInt::from(12);
    // let signature_R8x: BigInt = BigInt::from(13);
    // let signature_R8y: BigInt = BigInt::from(14);
    // let signature_S: BigInt = BigInt::from(15);
    // let sender_proof: BigInt = BigInt::from(16);
    // let sender_proof_pos: BigInt = BigInt::from(17);
    // let receiver_proof: BigInt = BigInt::from(18);
    // let receiver_proof_pos: BigInt = BigInt::from(19);
    // let couple = ZKPInstance.generate_single_tx_prove(
    //     accounts_root,
    //     intermediate_root,
    //     accounts_pubkeys,
    //     accounts_balances,
    //     sender_pubkey,
    //     sender_balance,
    //     receiver_pubkey,
    //     receiver_balance,
    //     amount,
    //     signature_R8x,
    //     signature_R8y,
    //     signature_S,
    //     sender_proof,
    //     sender_proof_pos,
    //     receiver_proof,
    //     receiver_proof_pos,
    // );
    // if let Err(e) = couple.0 {
    //     panic!("{}", e.to_string());
    // }
}
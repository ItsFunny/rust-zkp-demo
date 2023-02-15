extern crate core;

#[macro_use]
extern crate rocket;

use std::fs::OpenOptions;
use std::io::Cursor;
use std::sync::{Arc, Mutex};
use ethers::utils::hex;
use lazy_static::lazy_static;
use rocket::form::{Context, Contextual, Form};
use rocket::fs::FileServer;
use rocket::http::{ContentType, Status};
use rocket::{Data, routes};
use rocket_multipart_form_data::{MultipartFormData, MultipartFormDataField, MultipartFormDataOptions};
use crate::instance::{ProveRequest, RegisterRequest, VerifyRequest, ZKPProverContainer};


mod events;
mod anvil;
mod contract_adapter;
pub mod verifier_contract;
pub mod circuits;
mod instance;
pub mod ddd;

lazy_static! {
    static ref ZKPInstance: Arc<Mutex<ZKPProverContainer>> = init_zkp();
}

fn init_zkp() -> Arc<Mutex<ZKPProverContainer>> {
    let zkp = ZKPProverContainer::default();
    Arc::new(Mutex::new(zkp))
}


#[post("/", data = "<data>")]
async fn register<'r>(content_type: &ContentType, data: Data<'_>) -> String {
    let mut options = MultipartFormDataOptions::with_multipart_form_data_fields(
        vec![
            MultipartFormDataField::raw("r1cs").size_limit(1024 * 1024 * 1024),
            MultipartFormDataField::text("key"),
        ]
    );
    let mut multipart_form_data_res = MultipartFormData::parse(content_type, data, options).await;
    if let Err(e) = multipart_form_data_res {
        return e.to_string();
    }
    let mut multipart_form_data = multipart_form_data_res.unwrap();
    let r1cs_field = multipart_form_data.raw.get_mut("r1cs").unwrap().remove(0); //
    let key_field = multipart_form_data.texts.get_mut("key").unwrap().remove(0).text;
    println!("key:{}", key_field);

    let mut binding = ZKPInstance.clone();
    let mut vv = binding.lock().unwrap();
    let req = RegisterRequest { key: key_field, reader: r1cs_field.raw };
    let resp = vv.register(req);
    serde_json::json!(resp).to_string()
}

#[post("/", data = "<data>")]
async fn prove<'r>(content_type: &ContentType, data: Data<'_>) -> String {
    let mut options = MultipartFormDataOptions::with_multipart_form_data_fields(
        vec![
            MultipartFormDataField::raw("witness").size_limit(1024 * 1024 * 1024),
            MultipartFormDataField::text("key"),
        ]
    );
    let mut multipart_form_data_res = MultipartFormData::parse(content_type, data, options).await;
    if let Err(e) = multipart_form_data_res {
        return e.to_string();
    }
    let mut multipart_form_data = multipart_form_data_res.unwrap();
    let file_field = multipart_form_data.raw.get_mut("witness").unwrap().remove(0); //
    let key_field = multipart_form_data.texts.get_mut("key").unwrap().remove(0).text;

    let mut binding = ZKPInstance.clone();
    let mut vv = binding.lock().unwrap();
    let req = ProveRequest { key: key_field, wtns: file_field.raw };
    let resp = vv.prove(req);
    if let Err(e) = resp {
        return e.to_string();
    }
    serde_json::json!(resp.unwrap()).to_string()
}

#[post("/", data = "<data>")]
async fn verify<'r>(content_type: &ContentType, data: Data<'_>) -> String {
    let mut options = MultipartFormDataOptions::with_multipart_form_data_fields(
        vec![
            MultipartFormDataField::text("hex_proof").size_limit(1024 * 1024 * 1024),
            MultipartFormDataField::text("key").size_limit(1024 * 1024 * 1024),
        ]
    );
    let mut multipart_form_data_res = MultipartFormData::parse(content_type, data, options).await;
    if let Err(e) = multipart_form_data_res {
        return e.to_string();
    }
    let mut multipart_form_data = multipart_form_data_res.unwrap();
    let hex_proof = multipart_form_data.texts.get_mut("hex_proof").unwrap().remove(0).text;
    let key = multipart_form_data.texts.get_mut("key").unwrap().remove(0).text;


    let mut binding = ZKPInstance.clone();
    let mut vv = binding.lock().unwrap();
    let req = HexVerifyRequest { key: key, hex_proof: hex_proof };
    let resp = vv.verify(req.into());
    if let Err(e) = resp {
        return e.to_string();
    }
    serde_json::json!(resp.unwrap()).to_string()
}

pub struct HexVerifyRequest {
    pub key: String,
    pub hex_proof: String,
}

impl Into<VerifyRequest> for HexVerifyRequest {
    fn into(self) -> VerifyRequest {
        VerifyRequest { key: self.key, proof_bytes: hex::decode(self.hex_proof).unwrap() }
    }
}

#[post("/", format = "json", data = "<message>")]
fn test(message: String) -> String {
    error_return("asd")
}

fn error_return(str: &'static str) -> String {
    String::from(str)
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/register", routes![register])
        .mount("/prove", routes![prove])
        .mount("/verify", routes![verify])
        .mount("/test", routes!(test))
}
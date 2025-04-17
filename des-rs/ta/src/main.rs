#![no_std]
#![no_main]

extern crate alloc;

use alloc::vec;
use alloc::boxed::Box;
use optee_utee::{
    ta_close_session, ta_create, ta_destroy, ta_invoke_command, ta_open_session, trace_println,
};
use optee_utee::{is_algorithm_supported};
use optee_utee::{AlgorithmId, ElementId, Cipher, OperationMode};
use optee_utee::{AttributeId, AttributeMemref, TransientObject, TransientObjectType};
use optee_utee::{Error, ErrorKind, Parameters, Result};
use proto::{Algo, Command, KeySize, Mode};

pub struct DesCipher {
    pub key_size: usize,
    pub cipher: Cipher,
    pub key_object: TransientObject,
}

impl Default for DesCipher {
    fn default() -> Self {
        Self {
            key_size: 0,
            cipher: Cipher::null(),
            key_object: TransientObject::null_object(),
        }
    }
}

#[ta_create]
fn create() -> Result<()> {
    trace_println!("[+] TA create");
    Ok(())
}

#[ta_open_session]
fn open_session(_params: &mut Parameters, _sess_ctx: &mut DesCipher) -> Result<()> {
    trace_println!("[+] TA open session");
    Ok(())
}

#[ta_close_session]
fn close_session(_sess_ctx: &mut DesCipher) {
    trace_println!("[+] TA close session");
}

#[ta_destroy]
fn destroy() {
    trace_println!("[+] TA destroy");
}

#[ta_invoke_command]
fn invoke_command(sess_ctx: &mut DesCipher, cmd_id: u32, params: &mut Parameters) -> Result<()> {
    trace_println!("[+] TA invoke command");
    match Command::from(cmd_id) {
        Command::Prepare => {
            return alloc_resources(sess_ctx, params);
        }
        Command::SetKey => {
            return set_des_key(sess_ctx, params);
        }
        Command::SetIV => {
            return reset_des_iv(sess_ctx, params);
        }
        Command::Cipher => {
            return cipher_buffer(sess_ctx, params);
        }
        _ => {
            return Err(Error::new(ErrorKind::BadParameters));
        }
    }
}

pub fn ta2tee_algo_id(algo_id: u32) -> Result<AlgorithmId> {
    match Algo::from(algo_id) {
        Algo::ECB => Ok(AlgorithmId::DesEcbNopad),
        Algo::CBC => Ok(AlgorithmId::DesCbcNopad),
        _ => Err(Error::new(ErrorKind::BadParameters)),
    }
}

pub fn ta2tee_key_size(key_sz: u32) -> Result<usize> {
    match KeySize::from(key_sz) {
        KeySize::Bit64 => Ok(key_sz as usize),
        _ => Err(Error::new(ErrorKind::BadParameters)),
    }
}

pub fn ta2tee_mode_id(mode: u32) -> Result<OperationMode> {
    match Mode::from(mode) {
        Mode::Encode => Ok(OperationMode::Encrypt),
        Mode::Decode => Ok(OperationMode::Decrypt),
        _ => Err(Error::new(ErrorKind::BadParameters)),
    }
}

pub fn alloc_resources(des: &mut DesCipher, params: &mut Parameters) -> Result<()> {
    let algo_value = unsafe { params.0.as_value().unwrap().a() };
    let key_size_value = unsafe { params.1.as_value().unwrap().a() };
    let mode_id_value = unsafe { params.2.as_value().unwrap().a() };

    des.key_size = ta2tee_key_size(key_size_value).unwrap();

    // 检查算法是否支持
    is_algorithm_supported(ta2tee_algo_id(algo_value).unwrap() as u32, ElementId::ElementNone as u32)?;

    des.cipher = Cipher::allocate(
        ta2tee_algo_id(algo_value).unwrap(),
        ta2tee_mode_id(mode_id_value).unwrap(),
        des.key_size * 8,
    )
    .unwrap();
    des.key_object = TransientObject::allocate(TransientObjectType::Des, des.key_size * 8).unwrap();
    let key = vec![0u8; des.key_size as usize];
    let attr = AttributeMemref::from_ref(AttributeId::SecretValue, &key);
    des.key_object.populate(&[attr.into()])?;
    des.cipher.set_key(&des.key_object)?;
    Ok(())
}

pub fn set_des_key(des: &mut DesCipher, params: &mut Parameters) -> Result<()> {
    let mut param0 = unsafe { params.0.as_memref().unwrap() };
    let key = param0.buffer();

    if key.len() != des.key_size {
        trace_println!("[+] Invalid key size: expected {}, got {}", des.key_size, key.len());
        return Err(Error::new(ErrorKind::BadParameters));
    }

    let attr = AttributeMemref::from_ref(AttributeId::SecretValue, &key);

    des.key_object.reset();
    des.key_object.populate(&[attr.into()])?;
    des.cipher.set_key(&des.key_object)?;
    Ok(())
}

pub fn reset_des_iv(des: &mut DesCipher, params: &mut Parameters) -> Result<()> {
    let mut param0 = unsafe { params.0.as_memref().unwrap() };
    let iv = param0.buffer();

    if iv.len() != 8 {
        trace_println!("[+] Invalid IV size: expected 8, got {}", iv.len());
        return Err(Error::new(ErrorKind::BadParameters));
    }

    des.cipher.init(iv);
    trace_println!("[+] IV reset successfully!");
    Ok(())
}

pub fn cipher_buffer(des: &mut DesCipher, params: &mut Parameters) -> Result<()> {
    let mut param0 = unsafe { params.0.as_memref().unwrap() };
    let mut param1 = unsafe { params.1.as_memref().unwrap() };

    let input = param0.buffer();
    let output = param1.buffer();

    if output.len() < input.len() {
        return Err(Error::new(ErrorKind::BadParameters));
    }

    trace_println!("[+] TA tries to update ciphers!");

    let tmp_size = des.cipher.update(input, output).unwrap();
    param1.set_updated_size(tmp_size);
    Ok(())
}

include!(concat!(env!("OUT_DIR"), "/user_ta_header.rs"));
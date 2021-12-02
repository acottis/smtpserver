use std::mem::MaybeUninit;
use std::sync::{Once};
use std::io::prelude::{Write, Read};
use std::collections::HashMap;
use crate::error::{Error, Result};

pub static MAX_BAD_ATTEMPTS: u8 = 3;

/// Generic Global lookup for the config file
/// 
pub fn lookup(key: &str) -> String {
    static mut CONFIG: MaybeUninit<HashMap<String,String>> = MaybeUninit::uninit();
    static ONCE: Once = Once::new();
    unsafe{
        ONCE.call_once(|| {
            let config = aml::load("config.aml");
            CONFIG.write(config);
        });      
        match CONFIG.assume_init_ref().get(key){
            Some(val) => val.to_owned(),
            None => panic!("Config file issue, key: {}", key),
        }
    }
}
/// Runs a check to get a public IP and stores as static memory
/// 
pub fn public_ip() -> &'static String {
    static mut PUBLIC_IP: MaybeUninit<String> = MaybeUninit::uninit();
    static ONCE: Once = Once::new();
    unsafe {
        ONCE.call_once(|| {
            let lookup = icanhazip().unwrap_or("".into());
            PUBLIC_IP.write(lookup);
        });
        PUBLIC_IP.assume_init_ref()
    }
}
/// Grabs public IP from icanhazip.com
/// 
fn icanhazip() -> Result<String> {
    let ip_addr_regex: regex::Regex =  regex::Regex::new(r"\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}").map_err(Error::Regex)?;
    let buf = &mut [0u8;1000];
    let mut stream = std::net::TcpStream::connect("icanhazip.com:80").map_err(Error::IO)?;
    stream.write(b"GET / HTTP/1.1\nHost: icanhazip.com\r\n\r\n").map_err(Error::IO)?;
    stream.read(buf).map_err(Error::IO)?;
    let s = String::from_utf8_lossy(buf);
    match ip_addr_regex.find(&s){
        Some(ip) => Ok(ip.as_str().to_owned()),
        None => Err(Error::GetPublicIP),
    }
}

use std::mem::MaybeUninit;
use std::sync::{Mutex, Once};
use std::io::prelude::{Write, Read};
use crate::error::{Error, Result};

pub static MAX_BAD_ATTEMPTS: u8 = 3;

pub fn public_ip() -> &'static Mutex<String> {
    // Create an uninitialized static
    static mut PUBLIC_IP: MaybeUninit<Mutex<String>> = MaybeUninit::uninit();
    static ONCE: Once = Once::new();
    unsafe {
        ONCE.call_once(|| {
            // Make it
            let lookup = icanhazip().unwrap_or("".into());
            let public_ip = Mutex::new(lookup);
            // Store it to the static var, i.e. initialize it
            PUBLIC_IP.write(public_ip);
        });
        // Now we give out a shared reference to the data, which is safe to use
        // concurrently.
        PUBLIC_IP.assume_init_ref()
    }
}
pub fn hostname() -> &'static Mutex<String> {
    // Create an uninitialized static
    static mut HOSTNAME: MaybeUninit<Mutex<String>> = MaybeUninit::uninit();
    static ONCE: Once = Once::new();
    unsafe {
        ONCE.call_once(|| {
            // Make it
            let config = aml::load("config.aml".into());
            let hostname = Mutex::new(config.get("hostname").unwrap().to_owned());
            // Store it to the static var, i.e. initialize it
            HOSTNAME.write(hostname);
        });
        // Now we give out a shared reference to the data, which is safe to use
        // concurrently.
        HOSTNAME.assume_init_ref()
    }
}
pub fn bind_addr() -> &'static Mutex<String> {
    // Create an uninitialized static
    static mut BIND_ADDRESS: MaybeUninit<Mutex<String>> = MaybeUninit::uninit();
    static ONCE: Once = Once::new();
    unsafe {
        ONCE.call_once(|| {
            // Make it
            let config = aml::load("config.aml".into());
            let bind_address = Mutex::new(config.get("bind_addr").unwrap().to_owned());
            // Store it to the static var, i.e. initialize it
            BIND_ADDRESS.write(bind_address);
        });
        // Now we give out a shared reference to the data, which is safe to use
        // concurrently.
        BIND_ADDRESS.assume_init_ref()
    }
}

/// Grabs public IP from icanhazip.com
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

use std::mem::MaybeUninit;
use std::sync::{Mutex, Once};
use std::io::prelude::{Write, Read};

pub static BIND_ADDRESS: &str = "0.0.0.0:25";
pub static HOSTNAME: &str = "mx1.domain.tld"; //mx1.domain.tld Will read this from config
pub static MAX_BAD_ATTEMPTS: u8 = 3;

pub fn public_ip() -> &'static Mutex<String> {
    // Create an uninitialized static
    static mut PUBLIC_IP: MaybeUninit<Mutex<String>> = MaybeUninit::uninit();
    static ONCE: Once = Once::new();

    unsafe {
        ONCE.call_once(|| {
            // Make it
            let public_ip = Mutex::new(icanhazip().unwrap_or("".into()));
            // Store it to the static var, i.e. initialize it
            PUBLIC_IP.write(public_ip);
        });

        // Now we give out a shared reference to the data, which is safe to use
        // concurrently.
        PUBLIC_IP.assume_init_ref()
    }
}

/// Grabs public IP from icanhazip.com
fn icanhazip() -> Result<String, ()> {
    let ip_addr_regex: regex::Regex =  regex::Regex::new(r"\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}").unwrap();
    let buf = &mut [0u8;1000];
    let mut stream = std::net::TcpStream::connect("icanhazip.com:80").unwrap();
    stream.write(b"GET / HTTP/1.1\nHost: icanhazip.com\r\n\r\n").unwrap();
    stream.read(buf).unwrap();
    let s = String::from_utf8_lossy(buf);
    Ok(ip_addr_regex.find(&s).unwrap().as_str().to_owned())
}
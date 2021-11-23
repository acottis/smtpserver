//!
//! #Basic Usage:
//! ```cargo run```
//! This will start the web server listening on all interfaces on port 25 only
//! 
//! # TODO
//! - ~~Handle DATA msg with the . stop if it comes in mutiple parts~~
//! - Implement port 587
//! - Implement POP/IMAP
//! - Send emails (Already have written just need logic to hook), Also need to impl AUTH for security, also bruteforce protection. Subnet allow list is probably best early protection
//! - Better threading (limit)
//! - config file with hostname and port options
//! - Mail Size checking
//! 
use std::net::{TcpListener, TcpStream};
use std::io::{BufReader, BufRead, Write};
use smtpclient::StatusCodes;

#[cfg(test)]
mod test;

mod email;
use email::Email;

mod error;
use error::{Error, Result};

mod global;
use global::{HOSTNAME, MAX_BAD_ATTEMPTS, BIND_ADDRESS};

mod command;
use command::Command;

/// Program Entry Point, calls the TCP listener `listen`
/// 
fn main() {
    listen().unwrap();
}

/// Listens for TCP connections then starts a thread to deal with them `smtp_main`
/// 
fn listen() -> Result<()>{
    println!("Starting SMTP Server...");
    let listener = TcpListener::bind(BIND_ADDRESS).map_err(Error::IO)?;
    println!("Listening on {}", listener.local_addr().map_err(Error::IO)?);
    for stream in listener.incoming() {
        match stream {
            Ok(s) => {
                std::thread::spawn(|| -> Result<()> {
                    println!("Received connection from: {}", &s.peer_addr().map_err(Error::IO)?);
                    smtp_main(s)?;
                    Ok(())
                });
            },
            Err(e) => panic!("Encountered IO error: {}", e),   
        }
    }
    Ok(())
}
/// This function reads a TCP stream until a CLRF `[13, 10]` is sent then collects into a [Vec]
/// 
fn read<T>(stream: T) -> Result<Vec<u8>> where T: std::io::Read {  
    let mut reader = BufReader::new(stream);
    let mut data: Vec<u8> = vec![];
    loop{
        let buffer = reader.fill_buf();      
        match buffer {
            Ok(bytes) => {
                let length = bytes.len();
                data.extend_from_slice(bytes); 
                reader.consume(length);
                // Okay checks for CLFR if more than one byte is in buffer
                if (data.len() > 1) && (&data[data.len()-2..] == [13, 10]){
                    break;
                }
            },
            _ => {}
        }      
    }
    println!("{}", String::from_utf8_lossy(&data));
    Ok(data)
}
/// Wrapper around writing to TCP stream, handles the no whitespace requirement of the HELO response
/// 
fn write(mut stream: &TcpStream, status: StatusCodes, msg: String) -> Result<()> {
    let mut res = String::new();
    if msg.contains("-Hello"){
        res = format!("{}{}", String::from(status), msg);
    }else{
        res = format!("{} {}", String::from(status), msg);
    }
    stream.write(res.as_bytes()).map_err(Error::IO)?;
    Ok(())
}
/// Once a TCP session is established this is the main loop for handling the transaction
/// 
fn smtp_main(stream: TcpStream) -> Result<()>{

    let mut sender = String::new();
    let mut recipient = String::new();
    let mut bad_attempts = 0;
    let mut authenticated = false;

    let mut email = Email::new();

    let welcome = format!("{} SMTP MAIL Service Ready [{}]\r\n", HOSTNAME, global::public_ip().lock().unwrap());
    // Inital connection
    write(&stream, StatusCodes::ServiceReady, welcome.into())?;
        
    loop{
        let res_raw = read(&stream)?;
        let res = String::from_utf8(res_raw).map_err(Error::UTF8)?;

        let cmd = Command::lookup(res.as_ref());  
        match cmd {
            Command::Ehlo => {
                email.set_domain(res);
                write(&stream, StatusCodes::Ok, format!("-Hello {}\r\n250 AUTH LOGIN PLAIN\r\n", email.domain()))?;
            }
            Command::Helo => {
                email.set_domain(res);
                write(&stream, StatusCodes::Ok, format!("-Hello {}\r\n250 AUTH LOGIN PLAIN\r\n", email.domain()))?;
            }
            Command::AuthPlain => {
                write(&stream, StatusCodes::AuthenticationSuceeded, "\r\n".into())?;
                authenticated = true;
            }
            Command::AuthLogin => {
                write(&stream, StatusCodes::ServerChallenge, "VXNlcm5hbWU6\r\n".into())?;
                let user = read(&stream);
                write(&stream, StatusCodes::ServerChallenge, "UGFzc3dvcmQ6\r\n".into())?;
                let pass = read(&stream);
                write(&stream, StatusCodes::AuthenticationSuceeded, "2.7.0 Authentication successful\r\n".into())?;
                authenticated = true;

            }
            Command::MailFrom => {
                write(&stream, StatusCodes::Ok, "\r\n".into())?;
                let tmp = res.splitn(2, "mail from:").last().unwrap_or("");
                sender = tmp.replace(&[' ', '\r','\n','<','>'][..], "");
            }
            Command::RcptTo => {
                write(&stream, StatusCodes::Ok, "Ok\r\n".into())?;
                let tmp = res.splitn(2, "rcpt to:").last().unwrap_or("");
                recipient = tmp.replace(&[' ', '\r','\n','<','>'][..], "");
            }
            Command::Data => {
                if sender == "" || email.domain() == "" || recipient == "" {
                    write(&stream, StatusCodes::BadCommandSequence, "EHLO/HELO, MAIL FROM: and RCPT: are required before DATA\r\n".into())?;
                    continue;
                }
                write(&stream, StatusCodes::StartingMailInput, "Ok\r\n".into())?;
                let mut data: Vec<u8> = Vec::new();
                loop {
                    let mut tmp = read(&stream)?;
                    data.append(&mut tmp);
                    // Checks for `CLRF.CLRF`
                    if &data[&data.len()-5..] == &[13, 10, 46, 13, 10] { break }
                }
                email.save_email(data)?;
                println!("Sender: {}, Domain: {}, Recipient: {}", sender, email.domain(), recipient);
                write(&stream, StatusCodes::Ok, "Recieved Data\r\n".into())?;
            }
            Command::Quit => {
                write(&stream, StatusCodes::ServiceClosed, "Goodbye\r\n".into())?;
                break;
            }
            _ => { 
                bad_attempts += 1;
                println!("{:?} found", cmd);
                write(&stream, StatusCodes::CommandUnrecognised, format!("Command Unrecognised, Attempts Remaining: {}\r\n", (MAX_BAD_ATTEMPTS - bad_attempts)))?;
                if bad_attempts > 3 { 
                    let _ = &stream.shutdown(std::net::Shutdown::Both);
                    break;
                }
            }
        }
        //std::thread::sleep(std::time::Duration::from_secs(1));
    }  
    Ok(())
}

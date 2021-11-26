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
use global::{MAX_BAD_ATTEMPTS};

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
    let mail_root = global::mail_root().lock().expect("No mail route set in config");
    println!("Mail root is at: {}", mail_root);
    // Drop value as this function never ends
    drop(mail_root);

    let bind_addr = global::bind_addr().lock().unwrap().to_string();
    let listener = TcpListener::bind(bind_addr).map_err(Error::IO)?;
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
    print!("C: {}", String::from_utf8_lossy(&data));
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
    print!("S: {}", res);
    stream.write(res.as_bytes()).map_err(Error::IO)?;
    Ok(())
}
/// Once a TCP session is established this is the main loop for handling the transaction
/// 
fn smtp_main(stream: TcpStream) -> Result<()>{

    let mut bad_attempts = 0;

    // Struct to handle the data associated with the email
    let mut email = Email::new();
    email.set_sender_ip(stream.peer_addr().unwrap().ip().to_string());
    // Inital connection Response
    let welcome = format!("{} SMTP MAIL Service Ready [{}]\r\n", global::hostname().lock().unwrap(), global::public_ip().lock().unwrap());
    write(&stream, StatusCodes::ServiceReady, welcome.into())?;
       
    loop{
        // Raw bytes from stream
        let res_raw = read(&stream)?;
        // String representation of bytes
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
            // Currently no authentication checking TODO
            Command::AuthPlain => {
                match email.auth_plain(res){
                    Ok(_) => write(&stream, StatusCodes::AuthenticationSuceeded, "2.7.0 Authentication successful\r\n".into())?,
                    Err(e) => write(&stream, StatusCodes::AuthenticationFailed, format!("{:?}\r\n", e))?,
                }
            }
            // Currently no authentication checking TODO
            Command::AuthLogin => {
                write(&stream, StatusCodes::ServerChallenge, "VXNlcm5hbWU6\r\n".into())?;
                let _user = read(&stream);
                write(&stream, StatusCodes::ServerChallenge, "UGFzc3dvcmQ6\r\n".into())?;
                let _pass = read(&stream);
                write(&stream, StatusCodes::AuthenticationSuceeded, "2.7.0 Authentication successful\r\n".into())?;
            }
            Command::MailFrom => {
                email.set_sender(res);
                write(&stream, StatusCodes::Ok, "Ok\r\n".into())?;
            }
            Command::RcptTo => {
                email.set_recipient(res);
                write(&stream, StatusCodes::Ok, "Ok\r\n".into())?;
            }
            Command::Data => {
                if email.sender() == "" || email.domain() == "" || email.recipient() == "" { // || !email.authenticated() {
                    write(&stream, StatusCodes::BadCommandSequence, format!("EHLO/HELO, MAIL FROM: and RCPT: are required before DATA, Attempts Remaining: {}\r\n", MAX_BAD_ATTEMPTS - bad_attempts))?;
                    bad_attempts += 1;
                    continue;
                }
                write(&stream, StatusCodes::StartingMailInput, "Ok\r\n".into())?;
                let mut data: Vec<u8> = Vec::new();
                // Loops until `CLRF.CLRF`
                loop {
                    let mut tmp = read(&stream)?;
                    data.append(&mut tmp);
                    // Checks for `CLRF.CLRF`
                    if &data[&data.len()-5..] == &[13, 10, 46, 13, 10] { 
                        data.truncate(data.len()-5); // Removes the `CLRF.CLRF`
                        break; 
                    }
                }     
                email.save_email(data).unwrap();
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
            }
        }
        // Kill session if being spammed by bad commands
        if bad_attempts > 3 { 
            let _ = &stream.shutdown(std::net::Shutdown::Both);
            break;
        }
    }
    // Sends the email if its for an external addresss
    if email.domain() != global::hosted_email_domain().lock().unwrap().to_owned(){
        email.send()?;  
    } else{
        // Stores the email to user mailbox
        match email.store() {
            Ok(_) => println!("Email moved sucessfully"),
            Err(e) => println!("Email could not be moved to folder: {:?}", e),
        }
    }
    Ok(())
}

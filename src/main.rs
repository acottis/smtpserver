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
use std::io::{BufReader, BufRead, Write, Read};
use smtpclient::StatusCodes;
use std::thread;
use native_tls::{TlsAcceptor,TlsStream, Identity};
use std::time::Duration;

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

mod stream;
use stream::Stream;

/// Program Entry Point, calls the TCP listener `listen`
/// 
fn main() {

    println!("Starting SMTP Server...");
    let mail_root = global::mail_root().lock().expect("No mail route set in config");
    println!("Mail root is at: {}", mail_root);
    println!("-------------------------------------------");
    // Drop value as this function never ends
    drop(mail_root);

    thread::spawn(|| {
        listen(TcpListener::bind("0.0.0.0:25").unwrap()).expect("Failed to Start Mail Server");
    });
    listen(TcpListener::bind("0.0.0.0:587").unwrap()).expect("Failed to Start Mail Server");

    //loop { thread::sleep(Duration::from_secs(5))};
}

/// Listens for TCP connections then starts a thread to deal with them `smtp_main`
/// 
fn listen(listener: TcpListener) -> Result<()>{
    println!("Listening on {}", listener.local_addr().map_err(Error::IO)?);
    for stream in listener.incoming() {
        match stream {
            Ok(s) => {
                std::thread::spawn(move || -> Result<()> {
                    println!("Received connection from: {}", &s.peer_addr().map_err(Error::IO)?);
                    s.set_read_timeout(Some(std::time::Duration::from_secs(15))).unwrap();
                    s.set_write_timeout(Some(std::time::Duration::from_secs(15))).unwrap();
                    smtp_main(s)?;
                    Ok(())
                });
            },
            Err(e) => panic!("Encountered IO error: {}", e),   
        }
    }
    Ok(())
}
/// Once a TCP session is established this is the main loop for handling the transaction
/// 
fn smtp_main(stream: TcpStream) -> Result<()>{

    let mut stream = Stream{
        tcp_stream: stream,
        tls_stream: None
    };

    let mut bad_attempts = 0;

    // Struct to handle the data associated with the email
    let mut email = Email::new();
    email.set_sender_ip(stream.tcp_stream.peer_addr().unwrap().ip().to_string());
    // Inital connection Response
    let welcome = format!("{} SMTP MAIL Service Ready [{}]\r\n", global::hostname().lock().unwrap(), global::public_ip().lock().unwrap());
    stream.write(StatusCodes::ServiceReady, welcome.into())?;
       
    loop{
        // Raw bytes from stream
        let res_raw = stream.read()?;
        // String representation of bytes
        let res = String::from_utf8(res_raw).map_err(Error::UTF8)?;

        let cmd = Command::lookup(res.as_ref());  
        match cmd {
            Command::Ehlo => {
                email.set_domain(res);
                stream.write(StatusCodes::Ok, format!("-Hello {}\r\n250-AUTH LOGIN PLAIN\r\n250 STARTTLS\r\n", email.domain()))?;
            }
            Command::Helo => {
                email.set_domain(res);
                stream.write(StatusCodes::Ok, format!("-Hello {}\r\n250-AUTH LOGIN PLAIN\r\n250 STARTTLS\r\n", email.domain()))?;
            }
            Command::Starttls =>{
                stream.write(StatusCodes::ServiceReady, format!("Ready to start TLS 1.2\r\n"))?;
                stream.start_tls().expect("Starttls Failed");
            }
            // Currently no authentication checking TODO
            Command::AuthPlain => {
                match email.auth_plain(res){
                    Ok(_) => stream.write(StatusCodes::AuthenticationSuceeded, "2.7.0 Authentication successful\r\n".into())?,
                    Err(e) => stream.write(StatusCodes::AuthenticationFailed, format!("{:?}\r\n", e))?,
                }
            }
            // Currently no authentication checking TODO
            Command::AuthLogin => {
                stream.write(StatusCodes::ServerChallenge, "VXNlcm5hbWU6\r\n".into())?;
                let _user = stream.read();
                stream.write(StatusCodes::ServerChallenge, "UGFzc3dvcmQ6\r\n".into())?;
                let _pass = stream.read();
                stream.write(StatusCodes::AuthenticationSuceeded, "2.7.0 Authentication successful\r\n".into())?;
            }
            Command::MailFrom => {
                email.set_sender(res);
                stream.write(StatusCodes::Ok, "Ok\r\n".into())?;
            }
            Command::RcptTo => {
                email.set_recipient(res);
                stream.write(StatusCodes::Ok, "Ok\r\n".into())?;
            }
            Command::Data => {
                if email.sender() == "" || email.domain() == "" || email.recipient() == "" { // || !email.authenticated() {
                    stream.write(StatusCodes::BadCommandSequence, format!("EHLO/HELO, MAIL FROM: and RCPT: are required before DATA, Attempts Remaining: {}\r\n", MAX_BAD_ATTEMPTS - bad_attempts))?;
                    bad_attempts += 1;
                    continue;
                }
                stream.write(StatusCodes::StartingMailInput, "Ok\r\n".into())?;
                let mut data: Vec<u8> = Vec::new();
                // Loops until `CLRF.CLRF`
                loop {
                    let mut tmp = match stream.read() {
                        Ok(t) => t,
                        Err(e) => {
                            println!("SMTP Client time limit exceeded, closing. Error: {:?}", e);
                            return Ok(())
                        },
                    };
                    data.append(&mut tmp);
                    // Checks for `CLRF.CLRF`
                    if &data[&data.len()-5..] == &[13, 10, 46, 13, 10] { 
                        data.truncate(data.len()-5); // Removes the `CLRF.CLRF`
                        break; 
                    }
                }     
                email.save_email(data).unwrap();
                stream.write(StatusCodes::Ok, "Recieved Data\r\n".into())?;

                // Sends the email if its for an external addresss
                if email.recipient_domain() != global::hosted_email_domain().lock().unwrap().to_owned(){
                    email.send().unwrap();
                } else{
                    // Stores the email to user mailbox
                    match email.store() {
                        Ok(_) => println!("Email moved sucessfully"),
                        Err(e) => println!("Email could not be moved to folder: {:?}", e),
                    }
                }
            }
            Command::Quit => {
                stream.write(StatusCodes::ServiceClosed, "Goodbye\r\n".into())?;
                break;
            }
            _ => { 
                bad_attempts += 1;
                println!("{:?} found", cmd);
                if bad_attempts < 3 {
                    stream.write(StatusCodes::CommandUnrecognised, format!("Command Unrecognised, Attempts Remaining: {}\r\n", (MAX_BAD_ATTEMPTS - bad_attempts)))?;
                }    
            }
        }
        // Kill session if being spammed by bad commands
        if bad_attempts >= 3 { 
            stream.write(StatusCodes::CommandUnrecognised, format!("Goodbye\r\n"))?;
            return stream.tcp_stream.shutdown(std::net::Shutdown::Both).map_err(Error::IO)
        }
    }
    Ok(())
}

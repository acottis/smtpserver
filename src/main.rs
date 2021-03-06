//!
//! #Basic Usage:
//! ```cargo run```
//! This will start the web server listening on all interfaces on port 25 only
//! 
//! # TODO
//! - ~~Handle DATA msg with the . stop if it comes in mutiple parts~~
//! - ~~Implement port 587~~
//! - ~~Send emails (Already have written just need logic to hook), Also need to impl AUTH for security, also bruteforce protection. Subnet allow list is probably best early protection~~
//! - Better threading (limit)
//! - ~~config file with hostname and port options~~
//! - Mail Size checking
//! - Multiple To's
//! - Ban IP's after multiple attempts
//! 
use std::net::{TcpListener, TcpStream};
use smtpclient::StatusCodes;
use std::thread;

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
    let mail_root = global::lookup("mail_root");
    println!("Mail root is at: {}", mail_root);
    println!("-------------------------------------------");

    thread::spawn(|| {
        listen(TcpListener::bind("0.0.0.0:25").unwrap()).expect("Failed to Start Mail Server");
    });
    listen(TcpListener::bind("0.0.0.0:587").unwrap()).expect("Failed to Start Mail Server");
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

    let mut stream = Stream::new(stream);

    let mut bad_attempts = 0;

    // Struct to handle the data associated with the email
    let mut email = Email::new();
    email.set_sender_ip(stream.peer_addr().ip().to_string());
    // Inital connection Response
    let welcome = format!("{} SMTP MAIL Service Ready [{}]\r\n", global::lookup("hostname"), global::public_ip());
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
                let user = String::from_utf8(stream.read().unwrap()).unwrap();
                stream.write(StatusCodes::ServerChallenge, "UGFzc3dvcmQ6\r\n".into())?;
                let pass = String::from_utf8(stream.read().unwrap()).unwrap();
                match email.auth_login(&user, &pass){
                    Ok(_) => stream.write(StatusCodes::AuthenticationSuceeded, "2.7.0 Authentication successful\r\n".into())?,
                    Err(e) => stream.write(StatusCodes::AuthenticationFailed, format!("{:?}\r\n", e))?,
                }
                
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
                assert_eq!(email.recipients().len(), email.recipient_domains().len(), "There is not an equal amount of recipients and domains");
                if email.sender() == "" || email.domain() == "" || email.recipients().is_empty() { // || !email.authenticated() {
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
                // Sends the email if its for an external addresss

                for (recipient, domain) in email.recipients().iter().zip(email.recipient_domains()) {
                    
                    if domain != global::lookup("my_domain") {
                        match email.send(&recipient, &domain) {
                            Ok(_) => println!("Email sent sucessfully"),
                            Err(e) => {
                                println!("Error: {:?}", e);
                                stream.write(StatusCodes::AuthenticationRequired, format!("Goodbye\r\n"))?;
                                break;
                            }
                        }
                    } else{
                        // Stores the email to user mailbox
                        match email.store(&recipient) {
                            Ok(_) => println!("Email moved sucessfully"),
                            Err(e) => println!("Email could not be moved to folder: {:?}", e),
                        }
                    }
                    stream.write(StatusCodes::Ok, "Recieved Data\r\n".into())?;
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
            stream.shutdown().expect("Stream could not shutdown")
        }
    }
    stream.shutdown().expect("Stream could not shutdown");
    Ok(())
}

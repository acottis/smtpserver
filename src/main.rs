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
//! 


use std::net::{TcpListener, TcpStream};
use std::io::{BufReader, BufRead, Write};
use smtpclient::SmtpStatusCodes;

type Result<T> = std::result::Result<T, self::Error>;

static BIND_ADDRESS: &str = "0.0.0.0:25";
static HOSTNAME: &str = ""; //mx1.domain.tld Will read this from config
static MAX_BAD_ATTEMTPS: u8 = 3;

mod global;

#[derive(Debug)]
enum Error{
    IO(std::io::Error),
    UTF8(std::string::FromUtf8Error),
    SystemTime(std::time::SystemTimeError),
}

#[derive(Debug)]
enum SmtpCommand{
    Ehlo,
    Helo,
    AuthLogin,
    MailFrom,
    RcptTo,
    Data,
    Quit,
    CommandUnrecognised,
}

impl SmtpCommand{
    fn lookup(string: &str) -> Self {
        let mut text = string.to_owned();
        text.make_ascii_uppercase();
        if text.starts_with("EHLO") { return Self::Ehlo }
        if text.starts_with("HELO") {  return Self::Helo }
        if text.starts_with("AUTH LOGIN") {  return Self::AuthLogin }
        if text.starts_with("MAIL FROM:") {  return Self::MailFrom }
        if text.starts_with("RCPT TO:") {  return Self::RcptTo }
        if text.starts_with("DATA") {  return Self::Data }
        if text.starts_with("QUIT") {  return Self::Quit }
        Self::CommandUnrecognised
    }
}

fn main() {

    listen().unwrap();
}


fn listen() -> Result<()>{

    println!("Starting SMTP Server...");
    let listener = TcpListener::bind(BIND_ADDRESS).map_err(Error::IO)?;
   // listener.set_nonblocking(true).expect("Cannot set non-blocking"); Dont need this?
    println!("Listening on {}", listener.local_addr().map_err(Error::IO)?);


    for stream in listener.incoming() {
        match stream {
            Ok(s) => {
                std::thread::spawn(|| -> Result<()> {
                    println!("Recieved connection from: {}", &s.peer_addr().map_err(Error::IO)?);
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
fn read(stream: &TcpStream) -> Result<Vec<u8>>{
    
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
    //println!("Data from client: {:?}", data);
    println!("{}", String::from_utf8_lossy(&data));
    Ok(data)
}

fn write(mut stream: &TcpStream, status: SmtpStatusCodes, msg: String) -> Result<()> {

    let res = format!("{} {}", String::from(status), msg);
    stream.write(res.as_bytes()).map_err(Error::IO)?;

    Ok(())
}

// File name is time in seconds from EPOCH
fn save_email(data: Vec<u8>) -> Result<()>{
    let time = std::time::SystemTime::now();
    let filename = format!("{:?}.eml", time.duration_since(std::time::SystemTime::UNIX_EPOCH).map_err(Error::SystemTime)?);
    let mut file = std::fs::File::create(filename).map_err(Error::IO)?;
    file.write(&data).map_err(Error::IO)?;
    Ok(())
}  

fn smtp_main(stream: TcpStream) -> Result<()>{

    let mut domain = String::new();
    let mut sender = String::new();
    let mut recipient = String::new();
    let mut bad_attempts = 0;


    let welcome = format!("{} SMTP MAIL Service Ready [{}]\r\n",HOSTNAME, global::public_ip().lock().unwrap());
    // Inital connection
    write(&stream, SmtpStatusCodes::ServiceReady, welcome.into())?;
        
    loop{
        let res_raw = read(&stream)?;
        let res = String::from_utf8(res_raw).map_err(Error::UTF8)?.to_lowercase();

        let cmd = SmtpCommand::lookup(res.as_ref());  
        match cmd {
            SmtpCommand::Ehlo => {
                write(&stream, SmtpStatusCodes::Ok, "\r\n".into())?;
                let tmp = res.splitn(2, "ehlo").last().unwrap_or("");
                domain = tmp.replace(&[' ', '\r','\n'][..], "");
            }
            SmtpCommand::Helo => {
                write(&stream, SmtpStatusCodes::Ok, "\r\n".into())?;
                let tmp = res.splitn(2, "helo").last().unwrap_or("");
                domain = tmp.replace(&[' ', '\r','\n'][..], "");
            }
            SmtpCommand::MailFrom => {
                write(&stream, SmtpStatusCodes::Ok, "\r\n".into())?;
                let tmp = res.splitn(2, "mail from:").last().unwrap_or("");
                sender = tmp.replace(&[' ', '\r','\n','<','>'][..], "");
            }
            SmtpCommand::RcptTo => {
                write(&stream, SmtpStatusCodes::Ok, "Ok\r\n".into())?;
                let tmp = res.splitn(2, "rcpt to:").last().unwrap_or("");
                recipient = tmp.replace(&[' ', '\r','\n','<','>'][..], "");
            }
            SmtpCommand::Data => {
                if sender == "" || domain == "" || recipient == "" {
                    write(&stream, SmtpStatusCodes::BadCommandSequence, "EHLO/HELO, MAIL FROM: and RCPT: are required before DATA\r\n".into())?;
                    continue;
                }
                write(&stream, SmtpStatusCodes::StartingMailInput, "Ok\r\n".into())?;
                let mut data: Vec<u8> = Vec::new();
                loop {
                    let mut tmp = read(&stream)?;
                    data.append(&mut tmp);
                    // Checks for CLRF.CLRF
                    if &data[&data.len()-5..] == &[13, 10, 46, 13, 10] { break }
                }
                save_email(data)?;
                println!("Sender: {}, Domain: {}, Recipient: {}", sender, domain, recipient);
                write(&stream, SmtpStatusCodes::Ok, "Recieved Data\r\n".into())?;
            }
            SmtpCommand::Quit => {
                write(&stream, SmtpStatusCodes::ServiceClosed, "Goodbye\r\n".into())?;
                break;
            }
            _ => { 
                bad_attempts += 1;
                println!("{:?} found", cmd);
                write(&stream, SmtpStatusCodes::CommandUnrecognised, format!("Command Unrecognised, Attempts Remaining: {}\r\n", (MAX_BAD_ATTEMTPS - bad_attempts)))?;
                if bad_attempts > 3 { 
                    let _ = &stream.shutdown(std::net::Shutdown::Both);
                    break;
                }
            }
        }
    }  
    Ok(())
}

#[cfg(test)]
mod test;
//! # TODO
//! - Handle DATA msg with the . stop if it comes in mutiple parts
//! - Better threading (limit)

use std::thread;
use std::net::{TcpListener, TcpStream};
use std::io::{BufReader, BufRead, Write};
use smtpclient::SmtpStatusCodes;

// type Result<T> = std::result::Result<T, self::Error>;

// #[derive(Debug)]
// enum Error{
//     IO(std::io::Error),
//     UTF8(std::str::Utf8Error),
// }
static BIND_ADDRESS: &str = "127.0.0.1:25";
static MAX_BAD_ATTEMTPS: u8 = 3;

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

fn listen() -> Result<(), std::io::Error>{

    println!("Starting SMTP Server...");
    let listener = TcpListener::bind(BIND_ADDRESS)?;
   // listener.set_nonblocking(true).expect("Cannot set non-blocking"); Dont need this?

    println!("Listening on {}", listener.local_addr()?);

    for stream in listener.incoming() {
        match stream {
            Ok(s) => {
                thread::spawn(|| -> Result<(), std::io::Error> {
                    println!("Recieved connection from: {}", &s.peer_addr()?);
                    smtp_main(s)?;
                    Ok(())
                });
            },
            Err(e) => panic!("encountered IO error: {}", e),   
        }
    }
    Ok(())
}
/// This function reads a TCP stream until a CLRF `[13, 10]` is sent then collects into a [Vec]
fn read(stream: &TcpStream) -> Result<Vec<u8>, std::io::Error>{
    
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
    println!("Data from client: {:?}", data);
    println!("{}", String::from_utf8_lossy(&data));
    Ok(data)
}

fn write(mut stream: &TcpStream, status: SmtpStatusCodes, msg: String) -> Result<(), std::io::Error> {

    let res = format!("{} {}", String::from(status), msg);
    stream.write(res.as_bytes())?;

    Ok(())
}

fn save_email(data: Vec<u8>) -> Result<(), std::io::Error>{
    let mut file = std::fs::File::create("test.msg")?;
    file.write(&data)?;
    Ok(())
}  

fn smtp_main(stream: TcpStream) -> Result<(), std::io::Error>{

    let domain = 0;
    let mut bad_attempts = 0;

    // Inital connection
    write(&stream, SmtpStatusCodes::ServiceReady,"MX1.ashdown.scot SMTP MAIL Service Ready\r\n".into())?;
        
    loop{
        let res_raw = read(&stream)?;
        let res = String::from_utf8(res_raw).unwrap();

        let cmd = SmtpCommand::lookup(res.as_ref());  
        match cmd {
            SmtpCommand::Ehlo | SmtpCommand::Helo => {
                write(&stream, SmtpStatusCodes::Ok, "".into())?;
            }
            SmtpCommand::MailFrom => {
                write(&stream, SmtpStatusCodes::Ok, "".into())?;
            }
            SmtpCommand::RcptTo => {
                write(&stream, SmtpStatusCodes::Ok, "".into())?;
            }
            SmtpCommand::Data => {
                write(&stream, SmtpStatusCodes::StartingMailInput, "".into())?;
                let data = read(&stream)?;
                save_email(data)?;
                write(&stream, SmtpStatusCodes::Ok, "".into())?;
            }
            SmtpCommand::Quit => {
                write(&stream, SmtpStatusCodes::ServiceClosed, "".into())?;
                break;
            }
            _ => { 
                bad_attempts += 1;
                println!("{:?} found", cmd);
                write(&stream, SmtpStatusCodes::CommandUnrecognised, format!("Command Unrecognised, Attempts Remaining: {}\r\n", (MAX_BAD_ATTEMTPS - bad_attempts)))?;
                if bad_attempts > 3 { 
                    &stream.shutdown(std::net::Shutdown::Both);
                    break;
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod test;
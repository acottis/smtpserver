use chrono::{DateTime, Utc};
use std::io::{Write, Read};
use crate::error::{Result, Error};
use crate::global;

/// Struct handles information about a single email and provides getter/setter methods of access
/// 
#[derive(Default, Debug)]
pub struct Email{
    domain: String,
    sender: String,
    recipient: String,
    sender_ip: String,
    authenticated: bool,
    filename: String,
}

impl Email{
    /// Creates new [Email] with defaults 
    /// 
    pub fn new() -> Self{
        Default::default()
    }
    /// Setter for `self.sender_ip`
    /// 
    pub fn set_sender_ip(&mut self, ip: String){
        self.sender_ip = ip;
    }
    /// Setter for `self.domain`
    /// 
    pub fn set_domain(&mut self, res: String){
        let tmp = res.splitn(2, " ").last().unwrap_or("");
        self.domain = tmp.replace(&[' ', '\r','\n'][..], "");
    }
    /// Getter for `self.domain`
    /// 
    pub fn domain(&self) -> String{
        self.domain.clone()
    }
    /// Setter for `self.sender`
    /// 
    pub fn set_sender(&mut self, res: String){
        let tmp = res.splitn(2, ":").last().unwrap_or("");
        self.sender = tmp.replace(&[' ','\r','\n','<','>'][..], "");
    }
    /// Getter for `self.sender`
    /// 
    pub fn sender(&self) -> String{
        self.sender.clone()
    }
    /// Setter for `self.recipient`
    /// 
    pub fn set_recipient(&mut self, res: String){
        let tmp = res.splitn(2, ":").last().unwrap_or("");
        self.recipient = tmp.replace(&[' ','\r','\n','<','>'][..], "");
    }
    /// Getter for `self.recipient`
    /// 
    pub fn recipient(&self) -> String{
        self.recipient.clone()
    }
    /// Getter for `self.authenticated`
    /// 
    pub fn authenticated(&self) -> bool{
        self.authenticated
    }
    /// Generate receieved header into email after recieving it
    /// 
    fn gen_recv_header(&self) -> Vec<u8> {
        let now: DateTime<Utc> = Utc::now();
        let header = format!(
            "Received: from {from_mx} ({from_mx_ip}) by {my_mx} (AdaMPT) with {encryption} id {id}; {date_received}\r\n",
            from_mx = self.domain,
            from_mx_ip = self.sender_ip,
            my_mx = global::hostname().lock().unwrap(),
            encryption = "SMTP",
            date_received = now.to_rfc2822(),
            id = now.timestamp(),
        );
        // println!("{}", header);
        header.as_bytes().to_vec()
    }
    /// File name is time in seconds from EPOCH, saves file to mailbox
    /// 
    pub fn save_email(&mut self, data: Vec<u8>) -> Result<()>{
        let time = std::time::SystemTime::now();
        self.filename = format!("{:?}.eml", time.duration_since(std::time::SystemTime::UNIX_EPOCH).map_err(Error::SystemTime)?);
        let mut file = std::fs::File::create(self.filename.clone()).map_err(Error::IO)?;
        let recv_header = self.gen_recv_header();
        file.write(&recv_header).map_err(Error::IO)?;
        file.write(&data).map_err(Error::IO)?;
        Ok(())
    }
    /// Controls authentication for auth plain
    /// 
    pub fn auth_plain(&mut self, res: String) -> Result<()>{
        let creds = res.split(" ").last().unwrap().replace(&['\r','\n'][..], "").to_string();
        let secrets = aml::load("config.aml".into());
        if &creds == secrets.get("password").unwrap(){
            self.authenticated = true;
            Ok(())
        }else{
            Err(Error::BadAuth)
        }
    }
    /// Sends the email on if required, IP locked for now
    /// 
    pub fn send(&self) -> Result<()>{
        if self.sender_ip != "127.0.0.1"{
            return Err(Error::SendSecurityPolicy(format!("Bad IP: {}", self.sender_ip)))
        }
        if !self.authenticated(){
            return Err(Error::SendSecurityPolicy("Not Authenticated".into()));
        } 
        println!("-------------------------------------------");
        println!("-------------SENDING EMAIL NOW-------------");
        println!("-------------------------------------------");
        let mut buf = vec![];
        let mut f = std::fs::File::open(&self.filename).unwrap();
        f.read_to_end(&mut buf).map_err(Error::IO)?;
        buf.extend_from_slice(&[b'\r',b'\n',b'.',b'\r',b'\n']);
        let secrets = aml::load("secret.aml".into());
        let smtp_client_builder = smtpclient::SmtpBuilder::new(
            secrets.get("host").unwrap().into(), //host 
            secrets.get("port").unwrap().into(), //port
            self.sender.to_owned(), //sender
            self.recipient.to_owned(), //recipient
            self.domain.to_owned() //domain
        );
        smtp_client_builder
            .raw_bytes(buf)
            .starttls()
            .auth_login(secrets.get("username").unwrap().into(), secrets.get("password").unwrap().into())
            .send().unwrap();
        Ok(())
    }
}

#[test]
#[should_panic]
fn test_auth(){
    let mut email = Email::new();
    email.auth_plain("AUTH PLAIN notthecorrectanswer".into()).unwrap();
}
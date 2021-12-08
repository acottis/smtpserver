use chrono::{DateTime, Utc};
use std::io::{Write, Read};
use std::fs::{File, OpenOptions};
use crate::error::{Result, Error};
use crate::global;

/// Struct handles information about a single email and provides getter/setter methods of access
/// 
#[derive(Default, Debug)]
pub struct Email{
    sender_domain: String,
    sender: String,
    recipients: Vec<String>,
    recipient_domains: Vec<String>,
    sender_ip: String,
    authenticated: bool,
    filename: String,
    mail_root: String,
}

impl Email{
    /// Creates new [Email] with defaults 
    /// 
    pub fn new() -> Self{   
        Self {
            mail_root: global::lookup("mail_root"),
            ..Default::default()
        }
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
        self.sender_domain = tmp.replace(&[' ', '\r','\n'][..], "");
    }
    /// Getter for `self.domain`
    /// 
    pub fn domain(&self) -> String{
        self.sender_domain.clone()
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
        let recipient = tmp.replace(&[' ','\r','\n','<','>'][..], "");

        self.recipients.push(recipient.clone());
        self.recipient_domains.push(recipient.split("@").last().unwrap().to_string());
    }
    /// Getter for `self.recipient`
    /// 
    pub fn recipients(&self) -> Vec<String>{
        self.recipients.clone()
    }
    /// Getter for `self.recipient_domain`
    /// 
    pub fn recipient_domains(&self) -> Vec<String>{
        self.recipient_domains.clone()
    }
    /// Getter for `self.authenticated`
    /// 
    pub fn authenticated(&self) -> bool{
        self.authenticated
    }
    /// Generate receieved header into email after recieving it
    /// 
    fn generate_recveived_header(&self, file: &mut File) -> Result<()> {
        let now: DateTime<Utc> = Utc::now();
        let header = format!(
            "Received: from {from_mx} ({from_mx_ip}) by {my_mx} (AdaMPT) with {encryption} id {id}; {date_received}\r\n",
            from_mx = self.sender_domain,
            from_mx_ip = self.sender_ip,
            my_mx = global::lookup("hostname"),
            encryption = "SMTP",
            date_received = now.to_rfc2822(),
            id = now.timestamp(),
        );
        //println!("{}", header);
        file.write(header.as_bytes()).map_err(|e| Error::AddingReceivedHeader(e))?;
        Ok(())
    }
    /// File name is time in seconds from EPOCH, saves file to mailbox
    /// 
    pub fn save_email(&mut self, data: Vec<u8>) -> Result<()>{
        let time = std::time::SystemTime::now();
        self.filename = format!("{:?}.eml", time.duration_since(std::time::SystemTime::UNIX_EPOCH).map_err(Error::SystemTime)?); 
        let path = format!("{}/{}", self.mail_root, self.filename);
        let mut file = OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open(path)
            .map_err(|e|Error::ProcessingEmailData(e))?;

        // Insert Recieved Header
        self.generate_recveived_header(&mut file)?;

        // Write email to file
        file.write(&data).map_err(|e|Error::ProcessingEmailData(e))?;
        Ok(())
    }
    /// Controls authentication for auth plain
    /// 
    pub fn auth_plain(&mut self, res: String) -> Result<()>{
        let creds = res.split(" ").last().unwrap().replace(&['\r','\n'][..], "").to_string();
        let secrets = aml::load("config.aml");
        if &creds == secrets.get("password").unwrap(){
            self.authenticated = true;
            Ok(())
        }else{
            Err(Error::BadAuth)
        }
    }
    /// Controls authentication for auth plain
    /// 
    pub fn auth_login(&mut self, user: &str, pass: &str) -> Result<()>{
        let secrets = aml::load("config.aml");
        let user = user.replace(&['\r','\n'][..], "").to_string();
        let pass = pass.replace(&['\r','\n'][..], "").to_string();
        println!("user: {}, pass: {}", user, pass);
        if &user == secrets.get("login_user").unwrap() && &pass == secrets.get("login_pass").unwrap() {
            self.authenticated = true;
            Ok(())
        }else{
            Err(Error::BadAuth)
        }
    }
    /// Moves an email to a user mailbox
    /// 
    pub fn store(&self, recipient: &str) -> Result<()>{
        let user = &recipient.split("@").next().unwrap();
        let email = format!("{}/{}", self.mail_root, self.filename);
        let user_folder = format!("{}/mail/{}/Inbox/", &self.mail_root, user);
        
        println!("Attempting to move Email: {}, to Folder: {} ...", &email, user_folder);
        let destination = match std::path::Path::new(&user_folder).exists() {
            true => format!("{}/{}", user_folder, &self.filename),
            false => {
                println!("{} does not exist... defaulting to catch-all", user_folder);
                format!("{}/mail/{}/{}", &self.mail_root, "catch-all", &self.filename)
            },
        };
        
        let copy = std::fs::copy(&email, &destination);
        match copy {
            Ok(_) => { 
                //std::fs::remove_file(&email).map_err(|e| Error::FileDelete((e, format!("Filename: {}", &email))))?;
                return Ok(())
            },
            Err(e) => return Err(Error::FileCopy((e, format!("From: {}, To: {}", &email, &destination))))
        }
    }
    /// Sends the email on if required, IP locked for now
    /// 
    pub fn send(&self, recipient: &str, domain: &str) -> Result<()>{
        if self.sender_ip != "127.0.0.1"{
            return Err(Error::SendSecurityPolicy(format!("Bad IP: {}", self.sender_ip)))
        }
        if !self.authenticated(){
            return Err(Error::SendSecurityPolicy("Not Authenticated".into()));
        } 
        println!("-------------------------------------------");
        println!("-------------SENDING EMAIL NOW-------------");
        println!("----Filename: {}-------------", &self.filename);
        println!("----------Insert webhook here TODO---------");
        println!("-------------------------------------------");
        let mut buf = vec![];
        let email = format!("{}/{}", self.mail_root, self.filename);
        let mut f = std::fs::File::open(email).unwrap();
        f.read_to_end(&mut buf).map_err(Error::IO)?;
        buf.extend_from_slice(&[b'\r',b'\n',b'.',b'\r',b'\n']);
        let config = aml::load("config.aml");
        let smtp_client_builder = smtpclient::SmtpBuilder::new(
            config.get("forwarder_hostname").unwrap().into(), //host 
            config.get("forwarder_port").unwrap().into(), //port
            self.sender.to_owned(), //sender
            recipient.to_owned(), //recipient
            domain.to_owned() //domain
        );
        //println!("{}", String::from_utf8(buf.clone()).unwrap());
        let send = smtp_client_builder
            .raw_bytes(buf)
            .starttls()
            .auth_login(config.get("forwarder_username").unwrap().into(), config.get("forwarder_password").unwrap().into())
            .send();
        match send{
            Ok(_) => Ok(()),
            Err(e) => Err(Error::CouldNotSend(format!("{:?}", e))),
        }
    }
}

#[test]
#[should_panic]
fn test_auth(){
    let mut email = Email::new();
    email.auth_plain("AUTH PLAIN notthecorrectanswer".into()).unwrap();
}
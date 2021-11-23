use chrono::{DateTime, Utc};
use std::io::Write;
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
        let tmp = res.splitn(2, "mail from:").last().unwrap_or("");
        self.sender = tmp.replace(&[' ', '\r','\n','<','>'][..], "");
    }
    /// Getter for `self.sender`
    /// 
    pub fn sender(&self) -> String{
        self.sender.clone()
    }
    /// Setter for `self.recipient`
    /// 
    pub fn set_recipient(&mut self, res: String){
        let tmp = res.splitn(2, "ehlo").last().unwrap_or("");
        self.recipient = tmp.replace(&[' ', '\r','\n'][..], "");
    }
    /// Getter for `self.recipient`
    /// 
    pub fn recipient(&self) -> String{
        self.recipient.clone()
    }
    /// Generate receieved header into email after recieving it
    /// 
    fn gen_recv_header(&self) -> Vec<u8> {
        let now: DateTime<Utc> = Utc::now();
        let header = format!(
            "Received: from {from_mx} ({from_mx_ip}) by {my_mx} with {encryption}; {date_received}\r\n",
            from_mx = self.domain,
            from_mx_ip = self.sender_ip,
            my_mx = global::hostname().lock().unwrap(),
            encryption = "SMTP",
            date_received = now.to_rfc2822(),
        );
        // println!("{}", header);
        header.as_bytes().to_vec()
    }

    /// File name is time in seconds from EPOCH, saves file to mailbox
    /// 
    pub fn save_email(&self, data: Vec<u8>) -> Result<()>{
        let time = std::time::SystemTime::now();
        let filename = format!("{:?}.eml", time.duration_since(std::time::SystemTime::UNIX_EPOCH).map_err(Error::SystemTime)?);
        let mut file = std::fs::File::create(filename).map_err(Error::IO)?;
        let recv_header = self.gen_recv_header();
        file.write(&recv_header).map_err(Error::IO)?;
        file.write(&data).map_err(Error::IO)?;
        Ok(())
    }
}


// #[test]
// fn test_received_header(){
//     Email::insert_recv_header("");
// }
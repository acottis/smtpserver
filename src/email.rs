use chrono::{DateTime, Utc};
use std::io::Write;
use crate::error::{Result, Error};
use crate::global::{HOSTNAME};

/// Struct handles information about a single email and provides getter/setter methods of access
/// 
#[derive(Default, Debug)]
pub struct Email{
    domain: String,
    sender: String,
    recipient: String,
}

impl Email{
    /// Creates new [Email] with defaults 
    /// 
    pub fn new() -> Self{
        Default::default()
    }

    /// Setter for `self.domain`
    /// 
    pub fn set_domain(&mut self, res: String){
        let tmp = res.splitn(2, "ehlo").last().unwrap_or("");
        self.domain = tmp.replace(&[' ', '\r','\n'][..], "");
    }
    /// Getter for `self.domain`
    /// 
    pub fn domain(&self) -> String{
        self.domain.clone()
    }
    /// Insert receieved header into email after recieving it
    /// 
    fn insert_receieved_header(){

        let now: DateTime<Utc> = Utc::now();
    
        let header = format!(
            "Received: from {from_mx} ({from_mx_ip}) by {my_mx} with {encryption}; \
            {date_received}",
            from_mx = "mx9.example.com",
            from_mx_ip = "99.99.99.99",
            my_mx = HOSTNAME,
            encryption = "HTTPS",
            date_received = now.to_rfc2822(),
        );
    
        println!("{}", header);
    }

    /// File name is time in seconds from EPOCH, saves file to mailbox
    /// 
    pub fn save_email(&self, data: Vec<u8>) -> Result<()>{
        let time = std::time::SystemTime::now();
        let filename = format!("{:?}.eml", time.duration_since(std::time::SystemTime::UNIX_EPOCH).map_err(Error::SystemTime)?);
        let mut file = std::fs::File::create(filename).map_err(Error::IO)?;

        file.write(&data).map_err(Error::IO)?;
        Ok(())
    }
}


#[test]
fn test_received_header(){
    Email::insert_receieved_header();
}
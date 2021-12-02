use std::net::TcpStream;
use native_tls::{Identity, TlsAcceptor, TlsStream};
use std::io::{BufReader, BufRead, Write, Read};
use crate::error::{Result, Error};
use smtpclient::StatusCodes;


/// Struct for managing the reading and writing from TLS and TCP streams in a way that abstracts from the rest of the code
///
#[derive(Debug)] 
pub struct Stream{
    pub tcp_stream: TcpStream,
    pub tls_stream: Option<TlsStream<TcpStream>>,
}
impl Stream{
    /// Abstraction for choosing stream for read
    /// 
    // pub fn read(&mut self) -> Result<Vec<u8>> {

    //     // match self.tls_stream{
    //     //     Some(mut stream) => {
    //     //         self.private_read(Read::by_ref(&mut stream))
    //     //     },
    //     //     None => {
    //     //         self.private_read(self.tcp_stream.try_clone().unwrap())
    //     //     },
    //     // }
    //     if self.tls_stream.is_some(){
    //         println!("Hit tls: {:?}", self.tls_stream);
    //         let mut tls_stream = self.tls_stream.as_mut().unwrap();
    //         let res = self.private_read(tls_stream);
    //         //self.tls_stream = Some(tls_stream);
    //         return res;
    //     }else{
    //         println!("Hit Tcp: {:?}", self.tcp_stream);
    //         return self.private_read(self.tcp_stream.try_clone().unwrap())
    //     }
    // }
    /// Abstraction for choosing stream for write
    /// 
    // pub fn write(&mut self, status: StatusCodes, msg: String) -> Result<()> {
    //     if self.tls_stream.is_some(){
    //         let tls_stream = self.tls_stream.take().unwrap();
    //         self.private_write(tls_stream, status, msg).unwrap();
    //     }else{
    //         self.private_write(self.tcp_stream.try_clone().unwrap(), status, msg).unwrap();
    //     }
    //     Ok(())
    // }
    // /// This function reads a TCP stream until a CLRF `[13, 10]` is sent then collects into a [Vec]
    // /// 
    // pub fn read<T>(&mut self, stream: T) -> Result<Vec<u8>> where T: std::io::Read {  
    //     let mut reader = BufReader::new(stream);
    //     let mut data: Vec<u8> = vec![];
    //     let now = std::time::SystemTime::now();
    //     loop{
    //         let buffer = reader.fill_buf();  
    //         match buffer {
    //             Ok(bytes) => {
    //                 let length = bytes.len();
    //                 data.extend_from_slice(bytes); 
    //                 reader.consume(length);
    //                 // Okay checks for CLFR if more than one byte is in buffer
    //                 if (data.len() > 1) && (&data[data.len()-2..] == [13, 10]){
    //                     break;
    //                 }
    //             },
    //             _ => {}
    //         }
    //         if now.elapsed().unwrap() > std::time::Duration::from_secs(60) {
    //             return Err(Error::TCPReadTimeout)
    //         }      
    //     }
    //     print!("C: {}", String::from_utf8_lossy(&data));
    //     Ok(data)
    // }
    /// This function reads a TCP stream until a CLRF `[13, 10]` is sent then collects into a [Vec]
    /// 
    pub fn read(&mut self) -> Result<Vec<u8>>  {  

        if self.tls_stream.is_some(){
            let stream = self.tls_stream.as_mut().unwrap();
            let mut reader = BufReader::new(stream);
            let mut data: Vec<u8> = vec![];
            let now = std::time::SystemTime::now();
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
                if now.elapsed().unwrap() > std::time::Duration::from_secs(60) {
                    return Err(Error::TCPReadTimeout)
                }      
            }
            print!("C: {}", String::from_utf8_lossy(&data));
            Ok(data)
        }else{
            let stream = &self.tcp_stream;
            let mut reader = BufReader::new(stream);
            let mut data: Vec<u8> = vec![];
            let now = std::time::SystemTime::now();
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
                if now.elapsed().unwrap() > std::time::Duration::from_secs(60) {
                    return Err(Error::TCPReadTimeout)
                }      
            }
            print!("C: {}", String::from_utf8_lossy(&data));
            Ok(data)

        }
    }
    /// Wrapper around writing to TCP stream, handles the no whitespace requirement of the HELO response
    /// 
    pub fn write(&mut self, status: StatusCodes, msg: String) -> Result<()> {
    
        let res = match msg.contains("-Hello"){
            true => format!("{}{}", String::from(status), msg),
            false => format!("{} {}", String::from(status), msg),
        };
        print!("S: {}", res);
        if self.tls_stream.is_some(){
            let stream = self.tls_stream.as_mut().unwrap();
            stream.write(res.as_bytes()).map_err(Error::IO)?;
        }else{
            let mut stream = &self.tcp_stream;
            stream.write(res.as_bytes()).map_err(Error::IO)?;
        }
        Ok(())
    }
    // /// Wrapper around writing to TCP stream, handles the no whitespace requirement of the HELO response
    // /// 
    // fn write<T>(&self, mut stream: T, status: StatusCodes, msg: String) -> Result<()> where T: std::io::Write {
    
    //     let res = match msg.contains("-Hello"){
    //         true => format!("{}{}", String::from(status), msg),
    //         false => format!("{} {}", String::from(status), msg),
    //     };
    //     print!("S: {}", res);
    //     stream.write(res.as_bytes()).map_err(Error::IO)?;
    //     Ok(())
    // }

    /// Takes a TCP stream and inits a TLS stream if successful
    /// 
    pub fn start_tls(&mut self) -> Result<()> {
        let mut file = std::fs::File::open("cert.pfx").unwrap();
        let config = aml::load("config.aml");
        let mut raw_cert = vec![];
        file.read_to_end(&mut raw_cert).unwrap();
        let identity = Identity::from_pkcs12(&raw_cert, config.get("cert_passphrase").unwrap()).unwrap();
        //let acceptor = TlsAcceptor::builder(identity).min_protocol_version(Some(native_tls::Protocol::Tlsv12)).build().unwrap();
        let acceptor = TlsAcceptor::new(identity).unwrap();
        let tls_stream = acceptor.accept(self.tcp_stream.try_clone().unwrap()).unwrap();
        self.tls_stream = Some(tls_stream);
        Ok(())
    }
}
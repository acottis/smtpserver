use std::thread;
use std::net::{TcpListener, TcpStream};
use std::io::{BufReader, BufRead, Write};

// type Result<T> = std::result::Result<T, self::Error>;

// #[derive(Debug)]
// enum Error{
//     IO(std::io::Error),
// }

fn main() {
    listen().unwrap();
}

fn listen() -> Result<(), std::io::Error>{

    println!("Starting SMTP Server...");
    let listener = TcpListener::bind("127.0.0.1:25")?;
   // listener.set_nonblocking(true).expect("Cannot set non-blocking"); Dont need this?

    println!("Listening on {}", listener.local_addr()?);

    for stream in listener.incoming() {
        match stream {
            Ok(s) => {
                thread::spawn(|| -> Result<(), std::io::Error> {
                    println!("Recieved connection from: {}", s.peer_addr()?);
                    smtp_main(s)?;
                    Ok(())
                });
            },
            Err(e) => panic!("encountered IO error: {}", e),   
        }
    }

    Ok(())
}

fn read(){
    
}

fn smtp_main(stream: TcpStream) -> Result<(), std::io::Error>{

    let mut reader = BufReader::new(&stream);
    let mut writer = &stream;
    let mut data: Vec<u8> = vec![];

    writer.write("220 MX1.ashdown.scot SMTP MAIL Service Ready\r\n".as_bytes()).unwrap();

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

    Ok(())
}

#[cfg(test)]
mod test;
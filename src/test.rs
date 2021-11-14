use super::*;
use smtpclient::SmtpBuilder;

// #[test]
// fn test_listen(){
//     main()
// }

#[test]
fn recv_email(){

    std::thread::spawn(||{
        main(); 
    });

    let smtp_client_builder = SmtpBuilder::new(
        ("127.0.0.1").into(), //host 
        ("25").into(), //port
        ("sender").into(), //sender
        ("recipient").into(), //recipient
        ("domain").into() //domain
    );
    smtp_client_builder
        .subject("Testing Email".into())
        .body("This is a body - Generated by builder\nCan I have 한글? 안녕하세요~~".into())
        .display_name("Adam the Rusty".into())
        .send().unwrap();

}
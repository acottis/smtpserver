use super::*;
use smtpclient::SmtpBuilder;

#[test]
fn recv_email_local(){

    std::thread::spawn(||{
        main();
    });

    std::thread::sleep(std::time::Duration::from_secs(1));

    let secrets = aml::load("secret.aml");

    let smtp_client_builder = SmtpBuilder::new(
        secrets.get("hostname").unwrap().into(), //host 
        ("25").into(), //port
        secrets.get("sender").unwrap().into(), //sender
        secrets.get("recipient").unwrap().into(), //recipient
        ("example.com").into() //domain
    );
    smtp_client_builder
        .subject("Testing Email".into())
        .body("This is a body - Generated by builder\nCan I have 한글? 안녕하세요~~".into())
        .starttls()
        .auth_plain(secrets.get("plain_password").unwrap().into())
        .display_name("Adam the Rusty".into())
        .send().unwrap();
}


#[test]
fn test_email_struct(){
    let email = Email::new();
    println!("{:?}", email)
}
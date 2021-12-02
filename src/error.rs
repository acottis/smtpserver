pub type Result<T> = std::result::Result<T, self::Error>;

#[derive(Debug)]
pub enum Error{
    IO(std::io::Error),
    UTF8(std::string::FromUtf8Error),
    SystemTime(std::time::SystemTimeError),
    BadAuth,
    SendSecurityPolicy(String),
    Regex(regex::Error),
    GetPublicIP,
    AddingReceivedHeader(std::io::Error),
    ProcessingEmailData(std::io::Error),
    FileCopy((std::io::Error, String)),
    FileDelete((std::io::Error, String)),
    TCPReadTimeout,
    CouldNotSend(String),
}
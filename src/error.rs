pub type Result<T> = std::result::Result<T, self::Error>;

#[derive(Debug)]
pub enum Error{
    IO(std::io::Error),
    UTF8(std::string::FromUtf8Error),
    SystemTime(std::time::SystemTimeError),
}
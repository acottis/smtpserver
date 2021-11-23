
#[derive(Debug)]
pub enum Command{
    Ehlo,
    Helo,
    AuthLogin,
    AuthPlain,
    MailFrom,
    RcptTo,
    Data,
    Quit,
    CommandUnrecognised,
}

impl Command{
    pub fn lookup(string: &str) -> Self {
        let mut text = string.to_owned();
        text.make_ascii_uppercase();
        if text.starts_with("EHLO") { return Self::Ehlo }
        if text.starts_with("HELO") {  return Self::Helo }
        if text.starts_with("AUTH LOGIN") {  return Self::AuthLogin }
        if text.starts_with("AUTH PLAIN") {  return Self::AuthPlain }
        if text.starts_with("MAIL FROM:") {  return Self::MailFrom }
        if text.starts_with("RCPT TO:") {  return Self::RcptTo }
        if text.starts_with("DATA") {  return Self::Data }
        if text.starts_with("QUIT") {  return Self::Quit }
        Self::CommandUnrecognised
    }
}
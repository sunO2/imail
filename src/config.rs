use std::sync::LazyLock;


pub struct Config {
    pub imap_user: String,
    pub imap_email: String,
    pub imap_server: String,
    pub smtp_user: String,
    pub smtp_email: String,
    pub smtp_server: String,
    pub email_password: String,
}

pub static CONFIG: LazyLock<Config> = LazyLock::new(|| {
    let imap_user = std::env::var("IMAP_USER").expect("IMAP_USER 必须设置");
    let imap_email = std::env::var("IMAP_EMAIL").expect("IMAP_EMAIL 必须设置");
    let imap_server = std::env::var("IMAP_SERVER").expect("IMAP_SERVER 必须设置");

    let smtp_user = std::env::var("SMTP_USER").expect("SMTP_USER 必须设置");
    let smtp_email = std::env::var("SMTP_EMAIL").expect("SMTP_EMAIL 必须设置");
    let smtp_server = std::env::var("SMTP_SERVER").expect("SMTP_SERVER 必须设置");
    let email_password = std::env::var("EMAIL_PASSWORD").expect("EMAIL_PASSWORD 必须设置");
    Config{
        imap_user: imap_user,
        imap_email: imap_email,
        imap_server: imap_server,
        smtp_user: smtp_user,
        smtp_email: smtp_email,
        smtp_server: smtp_server,
        email_password: email_password,
    }
});
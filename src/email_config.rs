#[derive(Debug, Clone)]
pub struct EmailConfig {
    pub smtp_relay: String,
    pub smtp_username: String,
    pub smtp_password: String,
    pub from_email: String,
    pub to_email: String,
    pub subject: String,
}

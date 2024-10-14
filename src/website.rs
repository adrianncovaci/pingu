use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;

#[derive(Clone, Serialize, Deserialize)]
pub struct ResponseDetails {
    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub content_length: Option<u64>,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum CheckStatus {
    Up(ResponseDetails),
    Down {
        status_code: u16,
        error_message: String,
    },
}

impl CheckStatus {
    pub fn is_up(&self) -> bool {
        match self {
            CheckStatus::Up(_) => true,
            CheckStatus::Down { .. } => false,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Check {
    pub status: CheckStatus,
    pub timestamp: SystemTime,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Website {
    pub url: String,
    pub last_check: SystemTime,
    pub is_up: bool,
    pub total_checks: Vec<Check>,
    pub successful_checks: u64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct FailReport {
    pub url: String,
    pub status_code: u16,
    pub error_message: String,
    pub timestamp: SystemTime,
}

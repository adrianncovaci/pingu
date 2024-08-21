use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use tokio::time::interval;

#[derive(Clone, Serialize, Deserialize)]
pub struct Website {
    pub url: String,
    pub last_check: SystemTime,
    pub is_up: bool,
    pub total_checks: u64,
    pub successful_checks: u64,
}

#[derive(Clone)]
pub struct WebsiteMonitor {
    websites: Arc<RwLock<HashMap<String, Website>>>,
    client: Client,
}

impl Default for WebsiteMonitor {
    fn default() -> Self {
        WebsiteMonitor {
            websites: Arc::new(RwLock::new(HashMap::new())),
            client: Client::new(),
        }
    }
}

impl WebsiteMonitor {
    pub async fn add_website(&self, url: String) {
        let mut websites = self.websites.write().await;
        websites.insert(
            url.clone(),
            Website {
                url,
                last_check: SystemTime::now(),
                is_up: false,
                total_checks: 0,
                successful_checks: 0,
            },
        );
    }

    pub async fn check_website(&self, url: &str) -> bool {
        match self
            .client
            .get(url)
            .timeout(Duration::from_secs(10))
            .send()
            .await
        {
            Ok(response) => response.status().is_success(),
            Err(_) => false,
        }
    }

    pub async fn update_website_status(&self) {
        let mut websites = self.websites.write().await;

        for website in websites.values_mut() {
            let is_up = self.check_website(&website.url).await;
            website.is_up = is_up;
            website.last_check = SystemTime::now();
            website.total_checks += 1;
            if is_up {
                website.successful_checks += 1;
            }
        }
    }

    pub async fn get_status(&self) -> Vec<Website> {
        let websites = self.websites.read().await;
        websites
            .values()
            .map(|w| {
                Website {
                    url: w.url.clone(),
                    is_up: w.is_up,
                    last_check: w.last_check,
                    total_checks: w.total_checks,
                    successful_checks: w.successful_checks
                }
            })
            .collect()
    }

    pub async fn start_monitoring(&self, interval_secs: u64) {
        let monitor = self.clone();
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(interval_secs));
            loop {
                interval.tick().await;
                monitor.update_website_status().await;
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_add_website() {
        let monitor = WebsiteMonitor::default();
        monitor
            .add_website("https://www.example.com".to_string())
            .await;
        assert_eq!(monitor.websites.read().await.len(), 1);
    }

    #[tokio::test]
    async fn test_check_website() {
        let monitor = WebsiteMonitor::default();
        let result = monitor.check_website("https://www.example.com").await;
        assert!(result);
    }
}

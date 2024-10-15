use reqwest::Client;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use tokio::time::interval;

use crate::website::{Check, CheckStatus, ResponseDetails, Website};

#[cfg(feature = "email_notifications")]
use crate::email_config::EmailConfig;
#[cfg(feature = "email_notifications")]
use crate::website::FailReport;
#[cfg(feature = "email_notifications")]
use std::error::Error;

#[derive(Clone)]
pub struct WebsiteMonitor {
    websites: Arc<RwLock<HashMap<String, Website>>>,
    client: Client,
    #[cfg(feature = "email_notifications")]
    email_config: EmailConfig,
}

impl WebsiteMonitor {
    pub fn new(#[cfg(feature = "email_notifications")] email_config: EmailConfig) -> Self {
        WebsiteMonitor {
            websites: Arc::new(RwLock::new(HashMap::new())),
            client: Client::new(),
            #[cfg(feature = "email_notifications")]
            email_config,
        }
    }
}

impl WebsiteMonitor {
    pub async fn websites(&self) -> HashMap<String, Website> {
        self.websites.read().await.clone()
    }

    pub async fn add_website(&self, url: String) {
        let mut websites = self.websites.write().await;
        websites.insert(
            url.clone(),
            Website {
                url,
                last_check: SystemTime::now(),
                is_up: false,
                total_checks: vec![],
                successful_checks: 0,
            },
        );
    }

    pub async fn check_website(&self, url: &str) -> CheckStatus {
        match self
            .client
            .get(url)
            .timeout(Duration::from_secs(15))
            .send()
            .await
        {
            Ok(response) => {
                if !response.status().is_success() {
                    let status_code = response.status().as_u16();
                    let error_message = response.text().await.unwrap();
                    #[cfg(feature = "email_notifications")]
                    {
                        let fail_report = FailReport {
                            url: url.to_string(),
                            status_code,
                            error_message: error_message.clone(),
                            timestamp: SystemTime::now(),
                        };
                        self.send_email_notification(fail_report).await.unwrap();
                    }
                    CheckStatus::Down {
                        status_code,
                        error_message,
                    }
                } else {
                    let status_code = response.status().as_u16();
                    let headers = response.headers().clone();
                    let content_length = response.content_length();
                    CheckStatus::Up(ResponseDetails {
                        status_code,
                        headers: headers
                            .iter()
                            .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap().to_string()))
                            .collect(),
                        content_length,
                    })
                }
            }
            Err(err) => {
                eprintln!("err = {:?}", err);
                #[cfg(feature = "email_notifications")]
                {
                    let fail_report = FailReport {
                        url: url.to_string(),
                        status_code: u16::MAX,
                        error_message: err.to_string(),
                        timestamp: SystemTime::now(),
                    };
                    self.send_email_notification(fail_report).await.unwrap();
                }
                CheckStatus::Down {
                    status_code: 0,
                    error_message: err.to_string(),
                }
            }
        }
    }

    pub async fn update_website_status(&self) {
        let mut websites = self.websites.write().await;

        for website in websites.values_mut() {
            let status = self.check_website(&website.url).await;
            let timestamp = SystemTime::now();
            website.last_check = timestamp;
            website.is_up = status.is_up();
            if status.is_up() {
                website.successful_checks += 1;
            }
            website.total_checks.push(Check { status, timestamp });
        }
    }

    pub async fn get_status(&self) -> Vec<Website> {
        let websites = self.websites.read().await;
        websites
            .values()
            .map(|w| Website {
                url: w.url.clone(),
                is_up: w.is_up,
                last_check: w.last_check,
                total_checks: w.total_checks.clone(),
                successful_checks: w.successful_checks,
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

    #[cfg(feature = "email_notifications")]
    pub async fn send_email_notification(
        &self,
        fail_report: FailReport,
    ) -> Result<(), Box<dyn Error>> {
        use lettre::{
            message::{header::ContentType, Mailbox},
            transport::smtp::authentication::Credentials,
            AsyncSmtpTransport, AsyncStd1Executor, AsyncTransport, Message,
        };

        let to_email: Mailbox = self.email_config.to_email.parse()?;

        let email = Message::builder()
            .from(self.email_config.from_email.clone().parse().unwrap())
            .to(to_email)
            .subject(&format!(
                "{} {} is down!",
                self.email_config.subject, fail_report.url
            ))
            .header(ContentType::TEXT_PLAIN)
            .body(format!(
                "The website {} is down with status code {}. Error message: {} At: {:?}",
                fail_report.url,
                fail_report.status_code,
                fail_report.error_message,
                fail_report.timestamp
            ))?;

        let creds = Credentials::new(
            self.email_config.smtp_username.clone(),
            self.email_config.smtp_password.clone(),
        );

        // Open a remote connection to gmail
        let mailer: AsyncSmtpTransport<AsyncStd1Executor> =
            AsyncSmtpTransport::<AsyncStd1Executor>::relay(&self.email_config.smtp_relay.clone())
                .unwrap()
                .credentials(creds)
                .build();

        match mailer.send(email).await {
            Ok(_) => println!("Email sent successfully!"),
            Err(e) => eprintln!("Could not send email: {e:?}"),
        }

        Ok(())
    }
}

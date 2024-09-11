pub mod monitor;
pub mod website;

#[cfg(test)]
mod tests {
    use crate::monitor::WebsiteMonitor;

    #[tokio::test]
    async fn test_add_website() {
        let monitor = WebsiteMonitor::default();
        monitor
            .add_website("https://www.example.com".to_string())
            .await;
        assert_eq!(monitor.websites().await.len(), 1);
    }

    #[tokio::test]
    async fn test_check_website() {
        let monitor = WebsiteMonitor::default();
        let result = monitor.check_website("https://www.example.com").await;
        assert!(result.is_up());
    }
}

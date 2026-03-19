use std::time::Duration;
use tokio::time::timeout;

/// Network connectivity status
#[derive(Debug, Clone, PartialEq)]
pub enum NetworkStatus {
    Online,
    Offline,
}

/// Check if network connectivity is available
pub async fn check_connectivity() -> NetworkStatus {
    // Try to make a simple HTTP request with a short timeout
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .unwrap_or_default();

    // Test with a reliable endpoint
    let test_urls = [
        "https://httpbin.org/status/200",
        "https://www.google.com",
        "https://public-node.testnet.rsk.co",
    ];

    for url in &test_urls {
        if let Ok(Ok(response)) = timeout(Duration::from_secs(2), client.get(*url).send()).await {
            if response.status().is_success() {
                return NetworkStatus::Online;
            }
        }
    }

    NetworkStatus::Offline
}

/// Features available in offline mode
pub fn get_offline_features() -> Vec<&'static str> {
    vec![
        "Wallet Management",
        "Contact Management", 
        "Token Management",
        "Configuration",
        "System",
    ]
}

/// Check if a feature is available offline
pub fn is_offline_feature(feature: &str) -> bool {
    get_offline_features().contains(&feature)
}

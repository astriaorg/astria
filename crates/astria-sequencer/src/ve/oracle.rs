use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;

// Define the configuration for the providers
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    url: String,
    // Add other configuration fields as needed
}

impl ProviderConfig {
    // Create a new instance of ProviderConfig from a URL string
    pub fn new(url: &str) -> Self {
        ProviderConfig {
            url: url.to_string(),
        }
    }
}

// Define the Oracle struct
pub struct Oracle {
    client: Client,
    providers: Vec<ProviderConfig>,
    prices: RwLock<HashMap<String, String>>,
}

// Implement the Oracle struct
impl Oracle {
     // Create a new instance of Oracle
    pub fn new(provider_urls: &[&str]) -> Self {
        let providers = provider_urls
            .iter()
            .map(|&url| ProviderConfig::new(url))
            .collect();

        Oracle {
            client: Client::new(),
            providers,
            prices: RwLock::new(HashMap::new()),
        }
    }

    // Fetch prices from the providers and update the internal prices map
    pub async fn fetch_prices(&self) {
        let mut prices = HashMap::new();

        for provider in &self.providers {
            // Make the API call to fetch prices from the provider
            let response = self.client.get(&provider.url).send().await;

            // Parse the response and update the prices map
            if let Ok(response) = response {
                if let Ok(provider_prices) = response.json::<HashMap<String, String>>().await {
                    prices.extend(provider_prices);
                }
            }
        }

        // Update the internal prices map
        *self.prices.write().await = prices;
    }

    // Get the current prices
    pub async fn prices(&self) -> HashMap<String, String> {
        self.prices.read().await.clone()
    }
}

mod anthropic;
mod gemini;
mod open_router;

use anthropic::Anthropic;
use forge_domain::{Provider, ProviderService};
use gemini::Gemini;
use open_router::{OpenRouter, Provider as OpenRouterProvider};

#[derive(Debug)]
pub struct ProviderBuilder {
    url: String,
    api_key: Option<String>,
}

impl ProviderBuilder {
    pub fn from_url<S: Into<String>>(url: S) -> Self {
        Self { url: url.into(), api_key: None }
    }

    pub fn with_key<S: Into<String>>(mut self, key: S) -> Self {
        self.api_key = Some(key.into());
        self
    }

    pub fn build(self) -> Result<Box<dyn ProviderService>, anyhow::Error> {
        let provider = Provider::from_url(&self.url)
            .ok_or_else(|| anyhow::anyhow!("Failed to detect provider from URL: {}", self.url))?;
        let api_key = self
            .api_key
            .ok_or_else(|| anyhow::anyhow!("API key is required for provider: {}", provider))?;
        // println!("provider :: {:?}", provider);
        // println!("api_key :: {:?}", api_key);
        Ok(match provider {
            Provider::OpenRouter => Box::new(
                OpenRouter::builder()
                    .provider(OpenRouterProvider::OpenRouter)
                    .api_key(api_key)
                    .build()?,
            ),
            Provider::OpenAI => Box::new(
                OpenRouter::builder()
                    .provider(OpenRouterProvider::OpenAI)
                    .api_key(api_key)
                    .build()?,
            ),
            Provider::Anthropic => Box::new(
                Anthropic::builder()
                    .api_key(api_key)
                    .base_url(self.url)
                    .build()?,
            ),
            Provider::Gemini => Box::new(
                Gemini::builder()
                    .api_key(api_key)
                    .base_url(self.url)
                    .build()?,
            ),
        })
    }
}

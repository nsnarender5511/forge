use anyhow::Context as _;
use derive_setters::Setters;
use forge_domain::{
    ChatCompletionMessage, Context, Model, ModelId, Parameters, ProviderService, ResultStream
};
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{Client, Url};
use tokio_stream;

use super::request::Request;
use super::response::{EventData, ListModelResponse};

#[derive(Debug, Default, Clone, Setters)]
#[setters(into, strip_option)]
pub struct GeminiBuilder {
    api_key: Option<String>,
    base_url: Option<String>,
}

impl GeminiBuilder {
    pub fn build(self) -> anyhow::Result<Gemini> {
        let client = Client::builder().build()?;
        let base_url = self
            .base_url
            .as_deref()
            .unwrap_or("https://generativelanguage.googleapis.com/v1beta/");

        let base_url = Url::parse(base_url)
            .with_context(|| format!("Failed to parse base URL: {}", base_url))?;
        let api_key = self
            .api_key
            .ok_or_else(|| anyhow::anyhow!("API key is required"))?;

        Ok(Gemini { client, base_url, api_key })
    }
}

#[derive(Clone)]
pub struct Gemini {
    client: Client,
    api_key: String,
    base_url: Url,
}

impl Gemini {
    pub fn builder() -> GeminiBuilder {
        GeminiBuilder::default()
    }

    fn url(&self, path: &str) -> anyhow::Result<Url> {
        // Validate the path doesn't contain certain patterns
        if path.contains("://") || path.contains("..") {
            anyhow::bail!("Invalid path: Contains forbidden patterns");
        }

        // Remove leading slash to avoid double slashes
        let path = path.trim_start_matches('/');

        // Create the URL with the path
        let mut url = self.base_url
            .join(path)
            .with_context(|| format!("Failed to append {} to base URL: {}", path, self.base_url))?;
        
        // Add the API key as a query parameter
        url.query_pairs_mut().append_pair("key", &self.api_key);
        
        Ok(url)
    }

    fn headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            "Content-Type",
            HeaderValue::from_static("application/json"),
        );
        headers
    }
}

#[async_trait::async_trait]
impl ProviderService for Gemini {
    async fn chat(
        &self,
        _id: &ModelId,
        context: Context,
    ) -> ResultStream<ChatCompletionMessage, anyhow::Error> {
        // Using hardcoded model name as requested
        let model_name = "gemini-1.5-flash"; // Using the latest stable version
        
        // Create a request from the context
        let request = match Request::try_from(context) {
            Ok(req) => req,
            Err(e) => return Err(anyhow::anyhow!("Failed to create request: {}", e)).into(),
        };

        // Use the non-streaming endpoint for now
        let path = format!("models/{}:generateContent", model_name);
        let url = self.url(&path)?;

        // println!("{}", "*".repeat(100));
        // println!("request :: {}", serde_json::to_string_pretty(&request).unwrap());
        // println!("{}", "*".repeat(100));
        
        let response = self
            .client
            .post(url)
            .headers(self.headers())
            .json(&request)
            .send()
            .await;

        println!("response :: {:?}", response);
        
        if let Err(err) = &response {
            return Err(anyhow::anyhow!("Gemini API error: {:?}", err)).into();
        }
        
        let response = response.unwrap();
        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Gemini API error: Status {}, Body: {}", status, error_body)).into();
        }
        
        // Process the non-streaming response
        let bytes = response.bytes().await?;
        let text = String::from_utf8_lossy(&bytes);
        
        match serde_json::from_str::<EventData>(&text) {
            Ok(event_data) => {
                match ChatCompletionMessage::try_from(event_data) {
                    Ok(message) => {
                        // Return a stream with just this one message
                        Ok(Box::pin(tokio_stream::once(Ok(message))))
                    },
                    Err(err) => Err(anyhow::anyhow!("Failed to create completion message: {:?}", err)).into(),
                }
            },
            Err(err) => {
                Err(anyhow::anyhow!("Failed to parse Gemini response: {:?}", err)).into()
            },
        }
    }

    async fn models(&self) -> anyhow::Result<Vec<Model>> {
        let text = self
            .client
            .get(self.url("models")?)
            .headers(self.headers())
            .send()
            .await?
            .error_for_status()
            .with_context(|| "Failed because of a non 200 status code".to_string())?
            .text()
            .await?;
        let response: ListModelResponse = serde_json::from_str(&text)?;
        Ok(response.models.into_iter().map(Into::into).collect())
    }

    async fn parameters(&self, _model: &ModelId) -> anyhow::Result<Parameters> {
        // For now, we'll assume all Gemini models support tools
        Ok(Parameters { tool_supported: true })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_url_for_models() {
        let gemini = Gemini::builder().api_key("your-api-key").build().unwrap();
        let url = gemini.url("models").unwrap();
        assert!(url.as_str().starts_with("https://generativelanguage.googleapis.com/v1beta/models"));
        assert!(url.as_str().contains("key=your-api-key"));
    }
} 
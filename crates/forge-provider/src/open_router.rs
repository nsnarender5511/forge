use super::error::Result;
use super::open_ai::Role; // Importing Role
use super::provider::{InnerProvider, Provider};
use futures::stream::Stream as FuturesStream;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize}; // Importing Stream trait

#[derive(Debug, Clone)]
struct Config {
    api_key: String,
    base_url: Option<String>,
}

impl Config {
    fn api_key(&self) -> &str {
        &self.api_key
    }

    fn api_base(&self) -> &str {
        self.base_url
            .as_deref()
            .unwrap_or("https://openrouter.ai/api/v1")
    }

    fn headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", self.api_key)).unwrap(),
        );
        headers.insert("X-Title", HeaderValue::from_static("Tailcall"));
        headers
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.api_base(), path)
    }

    fn query(&self) -> Vec<(&str, &str)> {
        Vec::new()
    }
}

#[derive(Clone)]
pub struct OpenRouter {
    http_client: reqwest::Client,
    config: Config,
    model: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct Model {
    pub id: String,
    pub name: String,
    pub created: u64,
    pub description: String,
    pub context_length: u64,
    pub architecture: Architecture,
    pub pricing: Pricing,
    pub top_provider: TopProvider,
    pub per_request_limits: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct Architecture {
    pub modality: String,
    pub tokenizer: String,
    pub instruct_type: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct Pricing {
    pub prompt: String,
    pub completion: String,
    pub image: String,
    pub request: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct TopProvider {
    pub context_length: Option<u64>,
    pub max_completion_tokens: Option<u64>,
    pub is_moderated: bool,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Serialize)]
pub struct ListModelResponse {
    pub data: Vec<Model>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Request {
    pub messages: Option<Vec<Message>>,
    pub prompt: Option<String>,
    pub model: Option<String>,
    pub response_format: Option<ResponseFormat>,
    pub stop: Option<Vec<String>>,
    pub stream: Option<bool>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub tools: Option<Vec<Tool>>,
    pub tool_choice: Option<ToolChoice>,
    pub seed: Option<u32>,
    pub top_p: Option<f32>,
    pub top_k: Option<u32>,
    pub frequency_penalty: Option<f32>,
    pub presence_penalty: Option<f32>,
    pub repetition_penalty: Option<f32>,
    pub logit_bias: Option<std::collections::HashMap<u32, f32>>,
    pub top_logprobs: Option<u32>,
    pub min_p: Option<f32>,
    pub top_a: Option<f32>,
    pub prediction: Option<Prediction>,
    pub transforms: Option<Vec<String>>,
    pub models: Option<Vec<String>>,
    pub route: Option<String>,
    pub provider: Option<ProviderPreferences>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TextContent {
    pub r#type: String,
    pub text: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ImageContentPart {
    pub r#type: String,
    pub image_url: ImageUrl,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ImageUrl {
    pub url: String,
    pub detail: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum ContentPart {
    Text(TextContent),
    Image(ImageContentPart),
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Message {
    pub role: String,
    pub content: ContentPart,
    pub name: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FunctionDescription {
    pub description: Option<String>,
    pub name: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Tool {
    pub r#type: String,
    pub function: FunctionDescription,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum ToolChoice {
    None,
    Auto,
    Function { name: String },
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ResponseFormat {
    pub r#type: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Prediction {
    pub r#type: String,
    pub content: String,
}

// New ResponseType struct based on expected API response
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ResponseType {
    pub status: String,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
}

// Defining ProviderPreferences struct
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ProviderPreferences {
    // Define fields as necessary
}

impl OpenRouter {
    fn new(api_key: String, model: Option<String>, base_url: Option<String>) -> Self {
        let config = Config { api_key, base_url };
        let http_client = reqwest::Client::new();

        Self {
            http_client,
            config,
            model: model.unwrap_or("openai/gpt-4o-mini".to_string()),
        }
    }

    fn new_message(&self, role: Role, input: &str) -> Message {
        Message {
            role: role.to_string(),
            content: ContentPart::Text(TextContent {
                r#type: "text".to_string(),
                text: input.to_string(),
            }),
            name: None,
        }
    }

    fn prompt_request(&self, input: String) -> Result<Request> {
        Ok(Request {
            model: Some(self.model.clone()),
            messages: Some(vec![self.new_message(Role::User, &input)]),
            temperature: Some(0.7),
            stream: Some(true),
            ..Default::default()
        })
    }
}

#[async_trait::async_trait]
impl InnerProvider for OpenRouter {
    fn name(&self) -> &'static str {
        "Open Router"
    }

    async fn prompt(
        &self,
        input: String,
    ) -> Result<Box<dyn FuturesStream<Item = Result<String>> + Unpin>> {
        let request = self.prompt_request(input)?;
        let response = self
            .http_client
            .post(self.config.url("/chat/completions"))
            .headers(self.config.headers())
            .json(&request)
            .send()
            .await?
            .json::<ResponseType>() // Adjusted to use ResponseType
            .await?;

        // Handle the response and return a stream
        let stream = futures::stream::iter(
            response.data.into_iter().map(|data| Ok(data.to_string())), // Adjusted to match expected output
        );

        Ok(Box::new(stream))
    }

    async fn models(&self) -> Result<Vec<String>> {
        Ok(self
            .http_client
            .get(self.config.url("/models"))
            .headers(self.config.headers())
            .send()
            .await?
            .json::<ListModelResponse>()
            .await?
            .data
            .iter()
            .map(|r| r.name.clone())
            .collect::<Vec<String>>())
    }
}

impl Provider {
    pub fn open_router(api_key: String, model: Option<String>, base_url: Option<String>) -> Self {
        Provider::new(OpenRouter::new(api_key, model, base_url))
    }
}

#[cfg(test)]
mod test {
    use crate::open_router::ListModelResponse;

    fn models() -> &'static str {
        (r#"{
            "data": [
              {
                "id": "sao10k/l3.3-euryale-70b",
                "name": "Sao10K: Llama 3.3 Euryale 70B",
                "created": 1734535928,
                "description": "Euryale L3.3 70B is a model focused on creative roleplay from [Sao10k](https://ko-fi.com/sao10k). It is the successor of [Euryale L3 70B v2.2](/models/sao10k/l3-euryale-70b).",
                "context_length": 8000,
                "architecture": {
                  "modality": "text-\u003Etext",
                  "tokenizer": "Llama3",
                  "instruct_type": "llama3"
                },
                "pricing": {
                  "prompt": "0.0000015",
                  "completion": "0.0000015",
                  "image": "0",
                  "request": "0"
                },
                "top_provider": {
                  "context_length": 8000,
                  "max_completion_tokens": null,
                  "is_moderated": false
                },
                "per_request_limits": null
              },
              {
                "id": "inflatebot/mn-mag-mell-r1",
                "name": "Inflatebot: Mag Mell R1 12B",
                "created": 1734535439,
                "description": "Mag Mell is a merge of pre-trained language models created using mergekit, based on [Mistral Nemo](/mistralai/mistral-nemo). It is a great roleplay and storytelling model which combines the best parts of many other models to be a general purpose solution for many usecases.\n\nIntended to be a general purpose \"Best of Nemo\" model for any fictional, creative use case. \n\nMag Mell is composed of 3 intermediate parts:\n- Hero (RP, trope coverage)\n- Monk (Intelligence, groundedness)\n- Deity (Prose, flair)",
                "context_length": 16000,
                "architecture": {
                  "modality": "text-\u003Etext",
                  "tokenizer": "Mistral",
                  "instruct_type": "chatml"
                },
                "pricing": {
                  "prompt": "0.0000009",
                  "completion": "0.0000009",
                  "image": "0",
                  "request": "0"
                },
                "top_provider": {
                  "context_length": 16000,
                  "max_completion_tokens": null,
                  "is_moderated": false
                },
                "per_request_limits": null
              }
            ]
          }"#) as _
    }

    #[test]
    fn test_ser_of_models() {
        let response: Result<ListModelResponse, serde_json::Error> = serde_json::from_str(models());
        assert!(response.is_ok())
    }
}

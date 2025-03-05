use forge_domain::{ChatCompletionMessage, Content, FinishReason, ModelId, ToolCall, ToolCallFull, ToolCallId, ToolName};
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
pub struct ListModelResponse {
    pub models: Vec<Model>,
}

#[derive(Deserialize)]
pub struct Model {
    name: String,
    display_name: String,
    description: Option<String>,
}

impl From<Model> for forge_domain::Model {
    fn from(value: Model) -> Self {
        // Extract model ID from the full name (e.g., "models/gemini-pro" -> "gemini-pro")
        let id = value.name.split('/').last().unwrap_or(&value.name).to_string();
        
        Self {
            id: ModelId::new(id),
            name: value.display_name,
            description: value.description,
            context_length: None,
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct EventData {
    #[serde(default)]
    candidates: Vec<Candidate>,
    // Fields for non-streaming response
    #[serde(default)]
    contents: Option<Vec<ContentBlock>>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Candidate {
    content: Option<ContentBlock>,
    finish_reason: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ContentBlock {
    parts: Option<Vec<Part>>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Part {
    text: Option<String>,
    function_call: Option<FunctionCall>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct FunctionCall {
    name: String,
    args: Value,
}

impl TryFrom<EventData> for ChatCompletionMessage {
    type Error = anyhow::Error;
    fn try_from(value: EventData) -> Result<Self, Self::Error> {
        println!("DEBUG [Gemini::chat]: EventData: {:?}", value);
        // First try to process streaming response format
        if !value.candidates.is_empty() {
            // Process the first candidate if available
            if let Some(candidate) = value.candidates.first() {
                if let Some(content_block) = &candidate.content {
                    if let Some(parts) = &content_block.parts {
                        // Process text content
                        let text_content: String = parts
                            .iter()
                            .filter_map(|part| part.text.clone())
                            .collect::<Vec<String>>()
                            .join("");
                        
                        // Process function calls
                        let tool_calls: Vec<ToolCallFull> = parts
                            .iter()
                            .filter_map(|part| {
                                part.function_call.as_ref().map(|fc| ToolCallFull {
                                    call_id: Some(ToolCallId::new(format!("call-{}", fc.name))),
                                    name: ToolName::new(fc.name.clone()),
                                    arguments: fc.args.clone(),
                                })
                            })
                            .collect();
                        
                        // Create the completion message
                        let content = if !text_content.is_empty() {
                            Some(Content::part(text_content))
                        } else {
                            None
                        };
                        
                        // Map finish reason
                        let finish_reason = candidate.finish_reason.as_deref().map(|reason| {
                            match reason {
                                "STOP" => FinishReason::Stop,
                                "MAX_TOKENS" => FinishReason::Length,
                                "SAFETY" => FinishReason::ContentFilter,
                                "RECITATION" => FinishReason::ContentFilter,
                                "TOOL_CALLS" => FinishReason::ToolCalls,
                                _ => FinishReason::Stop,
                            }
                        });
                        
                        return Ok(ChatCompletionMessage {
                            content,
                            tool_call: tool_calls.into_iter().map(ToolCall::Full).collect(),
                            finish_reason,
                            usage: None,
                        });
                    }
                }
            }
        }
        
        // Try to process non-streaming response format
        if let Some(contents) = &value.contents {
            if let Some(content_block) = contents.first() {
                if let Some(parts) = &content_block.parts {
                    // Process text content
                    let text_content: String = parts
                        .iter()
                        .filter_map(|part| part.text.clone())
                        .collect::<Vec<String>>()
                        .join("");
                    
                    // Process function calls
                    let tool_calls: Vec<ToolCallFull> = parts
                        .iter()
                        .filter_map(|part| {
                            part.function_call.as_ref().map(|fc| ToolCallFull {
                                call_id: Some(ToolCallId::new(format!("call-{}", fc.name))),
                                name: ToolName::new(fc.name.clone()),
                                arguments: fc.args.clone(),
                            })
                        })
                        .collect();
                    
                    // Create the completion message
                    let content = if !text_content.is_empty() {
                        Some(Content::part(text_content))
                    } else {
                        None
                    };
                    
                    return Ok(ChatCompletionMessage {
                        content,
                        tool_call: tool_calls.into_iter().map(ToolCall::Full).collect(),
                        finish_reason: Some(FinishReason::Stop), // Assume STOP for non-streaming
                        usage: None,
                    });
                }
            }
        }
        
        // If we couldn't extract a valid message, return an empty one
        Ok(ChatCompletionMessage {
            content: None,
            tool_call: Vec::new(),
            finish_reason: None,
            usage: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_data_deserialization() {
        let json_str = r#"{
            "candidates": [
                {
                    "content": {
                        "parts": [
                            {
                                "text": "Hello, how can I help you today?"
                            }
                        ],
                        "role": "model"
                    },
                    "finishReason": "STOP",
                    "index": 0
                }
            ]
        }"#;
        
        let event_data: EventData = serde_json::from_str(json_str).unwrap();
        assert_eq!(event_data.candidates.len(), 1);
        assert_eq!(
            event_data.candidates[0].content.as_ref().unwrap().parts.as_ref().unwrap()[0].text.as_ref().unwrap(),
            "Hello, how can I help you today?"
        );
    }

    #[test]
    fn test_model_conversion() {
        let model = Model {
            name: "models/gemini-pro".to_string(),
            display_name: "Gemini Pro".to_string(),
            description: Some("A large language model for text generation".to_string()),
        };
        
        let domain_model = forge_domain::Model::from(model);
        assert_eq!(domain_model.id.to_string(), "gemini-pro");
        assert_eq!(domain_model.name, "Gemini Pro");
        assert_eq!(
            domain_model.description.unwrap(),
            "A large language model for text generation"
        );
    }
} 
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::convert::TryFrom;
use std::fmt::Debug;
use anyhow::{Result, Error};
use forge_domain::{ ContextMessage, Role};
use derive_setters::Setters;

/// ----------------------------------------------------------------
/// Request types for the Gemini API
/// ----------------------------------------------------------------

#[derive(Serialize, Default, Debug)]
pub struct Request {
    /// "system_instruction": { "parts": { "text": ... } }
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<SingleMessage>,
    
    /// "contents": { "role": "user", "parts": { "text": ... } }
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contents: Option<SingleMessageWithRole>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    
    /// "tool_config": { "function_calling_config": { "mode": "ANY" } }
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_config: Option<ToolConfig>,
}

/// For system_instruction, which lacks a `role`. 
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct SingleMessage {
    /// { "parts": { "text": "..." } }
    pub parts: SinglePart,
}

/// For contents, which has `role` plus `parts`.
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct SingleMessageWithRole {
    pub role: String,
    pub parts: SinglePart,
}

/// A single part containing text only, matching your target structure.
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct SinglePart {
    pub text: String,
}

/// Tools are mostly the same as your original code: an array of objects,
/// each holding a list of `function_declarations`.
#[derive(Serialize, Deserialize, Debug)]
pub struct Tool {
    pub function_declarations: Vec<FunctionDeclaration>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FunctionDeclaration {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Value>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ToolConfig {
    pub function_calling_config: FunctionCallingConfig,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FunctionCallingConfig {
    /// e.g. "ANY"
    pub mode: String,
}

/// ----------------------------------------------------------------
/// 3) Implement `TryFrom<forge_domain::Context>` for `Request`
///    so we can build this new request format from the old context
/// ----------------------------------------------------------------

impl TryFrom<forge_domain::Context> for Request {
    type Error = Error;

    fn try_from(context: forge_domain::Context) -> Result<Self> {
        // Find the first system message
        let system_instruction = context.messages.iter().find_map(|message| {
            if let ContextMessage::ContentMessage(chat_message) = message {
                if chat_message.role == forge_domain::Role::System {
                    Some(SingleMessage {
                        parts: SinglePart {
                            text: chat_message.content.clone(),
                        },
                    })
                } else {
                    None
                }
            } else {
                None
            }
        });

        // Find the first user message
        let contents = context.messages.iter().find_map(|message| {
            if let ContextMessage::ContentMessage(chat_message) = message {
                if chat_message.role == forge_domain::Role::User {
                    Some(SingleMessageWithRole {
                        role: "user".to_string(),
                        parts: SinglePart {
                            text: chat_message.content.clone(),
                        }
                    })
                } else {
                    None
                }
            } else {
                None
            }
        });

        // Convert tools
        let tools = if !context.tools.is_empty() {
            Some(context.tools
                .into_iter()
                .map(Tool::try_from)
                .collect::<Result<Vec<_>>>()?
            )
        } else {
            None
        };

        println!("{}", "*".repeat(100));
        println!("tools :: {:?}", serde_json::to_string_pretty(&tools).unwrap());
        println!("{}", "*".repeat(100));

        // Set tool config if tools are present
        let tool_config = tools.as_ref().map(|_| ToolConfig {
            function_calling_config: FunctionCallingConfig {
                mode: "ANY".to_string(),
            }
        });

        Ok(Request {
            system_instruction,
            contents,
            tools,
            tool_config,
        })
    }
}

// Add TryFrom implementation for Tool
impl TryFrom<forge_domain::ToolDefinition> for Tool {
    type Error = Error;

    fn try_from(value: forge_domain::ToolDefinition) -> Result<Self> {
        // Convert input schema to Value and extract only what we need
        let mut schema = serde_json::Map::new();
        let input_schema = serde_json::to_value(&value.input_schema)?;

        // Copy only the fields we need
        if let Some(obj) = input_schema.as_object() {
            // Copy properties and simplify types
            if let Some(properties) = obj.get("properties").and_then(|p| p.as_object()) {
                let mut simplified_props = serde_json::Map::new();
                
                for (key, prop) in properties {
                    let mut prop_obj = prop.as_object()
                        .ok_or_else(|| Error::msg("Invalid property object"))?
                        .clone();
                    
                    // Simplify type if it's an array
                    if let Some(types) = prop_obj.get("type").and_then(|t| t.as_array()) {
                        // Before the .find() call, create the default value
                        let default_type = serde_json::Value::String("string".to_string());

                        let simple_type = types.iter()
                            .find(|t| t.as_str() != Some("null"))
                            .unwrap_or(&default_type);
                        prop_obj.insert("type".to_string(), simple_type.clone());
                    }
                    
                    simplified_props.insert(key.clone(), serde_json::Value::Object(prop_obj));
                }
                
                schema.insert("properties".to_string(), serde_json::Value::Object(simplified_props));
            }

            // Copy required fields as is
            if let Some(required) = obj.get("required") {
                schema.insert("required".to_string(), required.clone());
            }
        }

        // Add type: "object"
        schema.insert("type".to_string(), serde_json::Value::String("object".to_string()));

        Ok(Tool {
            function_declarations: vec![FunctionDeclaration {
                name: value.name.into_string(),
                description: Some(value.description),
                parameters: Some(serde_json::Value::Object(schema))
            }]
        })
    }
}




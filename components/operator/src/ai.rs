use cosmwasm_std::{Decimal256};
use serde::{Deserialize, Serialize};
use std::env;
use std::collections::BTreeMap;
use wstd::{
    http::{Client, HeaderValue, IntoBody, Request, StatusCode},
    io::AsyncRead,
};

const SYSTEM_PROMPT: &str = r#"You are a monkey throwing darts at a board. The user will provide you a list of names and you provide a number of points for each one. Examples:

NAMES: NTRN, ATOM, USDC
NTRN 42
ATOM 83
USDC 11

NAMES: OSMO, JUNO, DYDX, NTRN
OSMO 72
JUNO 3
DYDX 157
NTRN 65
"#;

pub async fn monkey_advisor(denoms: Vec<String>, tvl: Decimal256, seed: u32) -> Result<BTreeMap<String, Decimal256>, String> {
    // call the monkey to get some rankings from these various denoms
    // let model = "llama3.1:8b".to_string();
    let model = "llama3:8b-instruct-q4_0".to_string();

    let llm_config = LlmOptions {
        context_window: Some(16384),
        max_tokens: Some(2048),
        seed,
        temperature: 0.7,
        top_p: 0.9,
    };
    let llm_client = with_config(model.clone(), llm_config).map_err(|e| e.to_string())?;

    let prompt = format!("NAMES: {}", denoms.join(", "));

    let response =
        llm_client.chat_completion_text(vec![Message {
            role: "system".to_string(),
            content: Some(SYSTEM_PROMPT.to_string()),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }, Message {
            role: "user".to_string(),
            content: Some(prompt.clone()),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }]).await.map_err(|e| e.to_string())?;

    let monkey = parse_response(&response, &denoms)?;
    Ok(normalize_monkey(monkey, tvl))
}

fn parse_response(resp: &str, denoms: &[String]) -> Result<BTreeMap<String, u64>, String> {
    let mut matches = BTreeMap::new();

    for line in resp.split_terminator("\n") {
        let items = line.trim().split(' ').collect::<Vec<&str>>();
        if items.len() == 2 {
            if let Ok(v) = u64::from_str_radix(items[1], 10) {
                matches.insert(items[0].to_string(), v);
            }
        }
    }
    if matches.len() != denoms.len() {
        return Err(format!("Only found {} of {} tokens in response", matches.len(), denoms.len()))
    }
    Ok(matches)
}

fn normalize_monkey(advice: BTreeMap<String, u64>, tvl: Decimal256) -> BTreeMap<String, Decimal256> {
    let total: u64 = advice.values().map(|x| *x).sum();
    let mut targets = BTreeMap::new();
    for (denom, allocation) in advice {
        let target_value = tvl
            .checked_mul(Decimal256::from_ratio(allocation, total)).unwrap();
        targets.insert(denom.clone(), target_value);
    }
    targets

}

// JSON serializable version of LlmOptions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmOptions {
    pub temperature: f32,
    pub top_p: f32,
    pub seed: u32,
    pub max_tokens: Option<u32>,
    pub context_window: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct LlmClient {
    /// The model name to use
    pub model: String,
    /// Configuration options for LLM requests
    pub config: LlmOptions,
    /// The API URL to send requests to
    pub api_url: String,
    /// Optional API key for authenticated services
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: Option<String>,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub tool_call_id: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub tool_type: String,
    pub function: ToolCallFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallFunction {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub tool_type: String,
    pub function: Function,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Function {
    pub name: String,
    pub description: Option<String>,
    pub parameters: Option<String>,
}

pub fn with_config(model: String, config: LlmOptions) -> Result<LlmClient, String> {
    // Get API key if using OpenAI models
    let api_key = match model.as_str() {
        "gpt-3.5-turbo" | "gpt-4" | "gpt-4o" | "gpt-4o-mini" | "gpt-4.1" | "gpt-4-turbo" => {
            match std::env::var("WAVS_ENV_OPENAI_API_KEY") {
                Ok(key) => Some(key),
                Err(_) => None,
            }
        }
        _ => None,
    };

    // Set API URL based on model type
    let api_url = match model.as_str() {
        "gpt-3.5-turbo" | "gpt-4" | "gpt-4o" | "gpt-4o-mini" | "gpt-4.1" | "gpt-4-turbo" => {
            "https://api.openai.com/v1/chat/completions".to_string()
        }
        _ => format!(
            "{}/api/chat",
            env::var("WAVS_ENV_OLLAMA_API_URL")
                .unwrap_or_else(|_| "http://localhost:11434".to_string())
        ),
    };

    // Create the new client instance
    Ok(LlmClient { model, config, api_url, api_key })
}

impl LlmClient {
    fn get_model(&self) -> String {
        self.model.clone()
    }

    fn get_config(&self) -> LlmOptions {
        self.config.clone()
    }

    async fn chat_completion(
        &self,
        messages: Vec<Message>,
        tools: Option<Vec<Tool>>,
    ) -> Result<Message, String> {
            // Validate messages
            if messages.is_empty() {
                return Err("Messages cannot be empty".into());
            }

            println!("Sending chat completion request:");

            // Check if OpenAI models have an API key
            let is_openai_model = matches!(
                self.model.as_str(),
                "gpt-3.5-turbo" | "gpt-4" | "gpt-4o" | "gpt-4o-mini" | "gpt-4.1" | "gpt-4-turbo"
            );

            println!("is_openai_model: {}", is_openai_model);

            if is_openai_model && self.api_key.is_none() {
                return Err("OpenAI API key is required for OpenAI models".into());
            }

            // Calculate max tokens based on tools presence if not explicitly set
            let max_tokens =
                self.config.max_tokens.unwrap_or_else(|| if tools.is_some() { 1024 } else { 100 });

            println!("api key: {}", self.api_key.is_some());
            println!("api url: {}", self.api_url);

            if self.api_url.is_empty() {
                return Err("API URL is empty".into());
            }

            // Create request body with configurable settings
            let body = if self.api_key.is_some() {
                // OpenAI format
                let mut request = serde_json::json!({
                    "model": self.model,
                    "messages": messages,
                    "temperature": self.config.temperature,
                    "top_p": self.config.top_p,
                    "seed": self.config.seed,
                    "stream": false,
                    "max_tokens": max_tokens
                });

                // Add tools if provided
                if let Some(tools_list) = tools {
                    request["tools"] = serde_json::to_value(tools_list).map_err(|e| e.to_string())?;
                }

                request
            } else {
                // Ollama chat format
                let mut request = serde_json::json!({
                    "model": self.model,
                    "messages": messages,
                    "stream": false,
                    "options": {
                        "temperature": self.config.temperature,
                        "top_p": self.config.top_p,
                        "seed": self.config.seed,
                        "num_predict": max_tokens,
                    }
                });

                // Add context window if specified
                if let Some(ctx) = self.config.context_window {
                    request["options"]["num_ctx"] = serde_json::json!(ctx);
                }

                // Add tools if provided for Ollama (using the format Ollama expects)
                if let Some(tools_list) = tools.clone() {
                    // Standard tools format
                    request["tools"] = serde_json::to_value(tools_list.clone()).map_err(|e| e.to_string())?;

                    // Also include functions key which some Ollama versions might need
                    // Convert tools to format compatible with Ollama
                    let functions = tools_list
                        .iter()
                        .map(|tool| {
                            serde_json::json!({
                                "name": tool.function.name,
                                "description": tool.function.description,
                                "parameters": tool.function.parameters
                            })
                        })
                        .collect::<Vec<_>>();

                    request["functions"] = serde_json::json!(functions);
                }

                request
            };

            println!("Request body: {}", serde_json::to_string_pretty(&body).unwrap_or_default());

            // Create request
            let mut req = Request::post(&self.api_url)
                .body(serde_json::to_vec(&body).unwrap().into_body())
                .map_err(|e| format!("Failed to create request: {}", e))?;

            // Add headers
            req.headers_mut().insert("Content-Type", HeaderValue::from_static("application/json"));
            req.headers_mut().insert("Accept", HeaderValue::from_static("application/json"));

            // Add authorization if needed
            if let Some(api_key) = &self.api_key {
                req.headers_mut().insert(
                    "Authorization",
                    HeaderValue::from_str(&format!("Bearer {}", api_key))
                        .map_err(|e| format!("Invalid API key format: {}", e))?,
                );
            }

            println!("Sending request to: {}", req.uri());

            // Send request
            let mut res = Client::new()
                .send(req)
                .await
                .map_err(|e| format!("Request failed: {}", e))?;

            println!("Received response with status: {}", res.status());

            if res.status() != StatusCode::OK {
                let mut error_body = Vec::new();
                res.body_mut()
                    .read_to_end(&mut error_body)
                    .await
                    .map_err(|e| format!("Failed to read error response: {}", e))?;
                let error_msg = format!(
                    "API error: status {} - {}",
                    res.status(),
                    String::from_utf8_lossy(&error_body)
                );
                println!("Error: {}", error_msg);
                return Err(error_msg);
            }

            // Read response body
            let mut body_buf = Vec::new();
            res.body_mut()
                .read_to_end(&mut body_buf)
                .await
                .map_err(|e| format!("Failed to read response body: {}", e))?;

            let body_str = String::from_utf8(body_buf)
                .map_err(|e| format!("Invalid UTF-8 in response: {}", e))?;

            println!("Raw response: {}", body_str);

            // Parse response based on provider
            if self.api_key.is_some() {
                // Parse OpenAI response format
                #[derive(Deserialize)]
                struct ChatResponse {
                    choices: Vec<Choice>,
                }

                #[derive(Deserialize)]
                struct Choice {
                    message: OpenAIMessage,
                }

                #[derive(Deserialize, Clone)]
                struct OpenAIMessage {
                    role: String,
                    #[serde(default)]
                    content: Option<String>,
                    #[serde(default)]
                    tool_calls: Option<Vec<OpenAIToolCall>>,
                }

                #[derive(Deserialize, Clone)]
                struct OpenAIToolCall {
                    id: String,
                    #[serde(rename = "type")]
                    tool_type: String,
                    function: OpenAIFunction,
                }

                #[derive(Deserialize, Clone)]
                struct OpenAIFunction {
                    name: String,
                    arguments: String,
                }

                let resp: ChatResponse = serde_json::from_str(&body_str).map_err(|e| {
                    format!("Failed to parse OpenAI response: {}", e)
                })?;

                resp.choices
                    .first()
                    .map(|choice| Message {
                        role: choice.message.role.clone(),
                        content: choice.message.content.clone(),
                        tool_calls: choice.message.tool_calls.clone().map(|tool_calls| {
                            tool_calls
                                .into_iter()
                                .map(|tool_call| ToolCall {
                                    id: tool_call.id,
                                    tool_type: tool_call.tool_type,
                                    function: ToolCallFunction {
                                        name: tool_call.function.name,
                                        arguments: tool_call.function.arguments,
                                    },
                                })
                                .collect()
                        }),
                        tool_call_id: None,
                        name: None,
                    })
                    .ok_or_else(|| "No response choices returned".into())
            } else {
                // Parse Ollama chat response format
                // Create a custom deserialization logic for Ollama responses
                let parsed_json: serde_json::Value =
                    serde_json::from_str(&body_str).map_err(|e| {
                        format!("Failed to parse Ollama response as JSON: {}", e)
                    })?;

                println!("Successfully parsed Ollama response to JSON Value");

                // Extract message contents
                let role =
                    parsed_json["message"]["role"].as_str().unwrap_or("assistant").to_string();

                let content = parsed_json["message"]["content"].as_str().map(|s| s.to_string());

                // Create base message
                let mut message =
                    Message { role, content, tool_calls: None, tool_call_id: None, name: None };

                // Process tool calls if present
                if let Some(tool_calls_array) = parsed_json["message"]["tool_calls"].as_array() {
                    println!("Found tool calls in Ollama response: {}", tool_calls_array.len());

                    let mut processed_tool_calls = Vec::new();

                    for (idx, tool_call) in tool_calls_array.iter().enumerate() {
                        if let Some(name) = tool_call["function"]["name"].as_str() {
                            println!("Processing tool call: {}", name);

                            // Get arguments value (could be object or string)
                            let args = &tool_call["function"]["arguments"];

                            // Convert arguments to string if they're an object
                            let arguments = if args.is_object() {
                                serde_json::to_string(args).unwrap_or_default()
                            } else if args.is_string() {
                                args.as_str().unwrap_or_default().to_string()
                            } else {
                                serde_json::to_string(args).unwrap_or_default()
                            };

                            println!("Arguments converted to string: {}", arguments);

                            processed_tool_calls
                                .push(ToolCall {
                                id: format!("call_{}", idx),
                                tool_type: "function".to_string(),
                                function:
                                    ToolCallFunction {
                                        name: name.to_string(),
                                        arguments,
                                    },
                            });
                        }
                    }

                    if !processed_tool_calls.is_empty() {
                        message.tool_calls = Some(processed_tool_calls);
                    }
                }

                Ok(message)
            }
    }

    async fn chat_completion_text(&self, messages: Vec<Message>) -> Result<String, String> {
        let response = self.chat_completion(messages, None).await?;
        Ok(response.content.unwrap_or_default())
    }

}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parsing() {
        let response = 
r#"OOH OOH AH AH! THE MONKEY'S GOT A FEW CHOICE WORDS FOR YA, BUT I'LL TRY TO KEEP IT CLEAN AND JUST THROW SOME DARTS!

FOO 17
WINNER 98
SUCK 35
"#;
        let denoms = vec!["FOO".to_string(), "WINNER".to_string(), "SUCK".to_string()];
        let parsed = parse_response(response, &denoms).unwrap();
        assert_eq!(parsed.len(), 3);
        assert_eq!(*parsed.get("FOO").unwrap(), 17);
        assert_eq!(*parsed.get("WINNER").unwrap(), 98);
        assert_eq!(*parsed.get("SUCK").unwrap(), 35);


        let normie = normalize_monkey(parsed, Decimal256::from_atomics(15000u128, 0).unwrap());
        assert_eq!(normie.len(), 3);
        // assert_eq!(*normie.get("FOO").unwrap(), Decimal256::from_atomics(1700u128, 0).unwrap());
        // assert_eq!(*normie.get("WINNER").unwrap(), Decimal256::from_atomics(9800u128, 0).unwrap());
        // assert_eq!(*normie.get("SUCK").unwrap(), Decimal256::from_atomics(3500u128, 0).unwrap());
        println!("{}", *normie.get("FOO").unwrap());
        println!("{}", *normie.get("WINNER").unwrap());
        println!("{}", *normie.get("SUCK").unwrap());
    }
}
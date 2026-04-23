use anyhow::{Context, Result, bail};
use log::{debug, warn};
use reqwest::Client;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;
use tokio::time::timeout;
use parking_lot::RwLock;
use crate::config::{AiConfig, OllamaConfig};

const MAX_CONCURRENT_REQUESTS: usize = 3;
const PLAYER_COOLDOWN_SECS: u64 = 2;

#[derive(Debug, Clone)]
pub struct AiClient {
    client: Client,
    config: AiConfig,
    ollama_config: Option<OllamaConfig>,
    semaphore: Arc<Semaphore>,
    last_request: Arc<RwLock<HashMap<String, Instant>>>,
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    max_tokens: u32,
    temperature: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Option<Vec<Choice>>,
    error: Option<ApiError>,
}

#[derive(Debug, Deserialize)]
struct ApiError {
    message: String,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
}

// Ollama /api/chat 端点请求格式
#[derive(Debug, Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
}

#[derive(Debug, Serialize, Clone)]
struct OllamaMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OllamaChatResponse {
    message: OllamaResponseMessage,
}

#[derive(Debug, Deserialize)]
struct OllamaResponseMessage {
    content: String,
}

pub enum ChatResult {
    Success(String),
    RateLimited(String),
}

impl AiClient {
    pub fn new(config: AiConfig, ollama_config: Option<OllamaConfig>) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            client,
            config,
            ollama_config,
            semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_REQUESTS)),
            last_request: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub async fn chat(&self, messages: Vec<Message>, player: &str) -> Result<ChatResult> {
        debug!("[AI] Chat request: player='{}', messages_count={}", player, messages.len());
        
        {
            let last_requests = self.last_request.read();
            if let Some(last_time) = last_requests.get(player) {
                let elapsed = last_time.elapsed();
                if elapsed < Duration::from_secs(PLAYER_COOLDOWN_SECS) {
                    let wait_secs = PLAYER_COOLDOWN_SECS - elapsed.as_secs();
                    debug!("[AI] Player '{}' rate limited, elapsed={}ms, wait={}s", player, elapsed.as_millis(), wait_secs);
                    return Ok(ChatResult::RateLimited(format!(
                        "Please wait {} seconds before asking again.",
                        wait_secs
                    )));
                }
            }
        }

        debug!("[AI] Acquiring semaphore permit (max concurrent: {})", MAX_CONCURRENT_REQUESTS);
        let _permit = timeout(Duration::from_secs(10), self.semaphore.acquire())
            .await
            .context("AI request timeout: too many concurrent requests")?;
        
        {
            let mut last_requests = self.last_request.write();
            last_requests.insert(player.to_string(), Instant::now());
        }

        debug!("[AI] Routing to backend: ollama_enabled={}", 
            self.ollama_config.as_ref().is_some_and(|o| o.enabled));
        
        let result = if let Some(ref ollama) = self.ollama_config {
            if ollama.enabled {
                debug!("[AI] Using Ollama backend");
                self.chat_ollama(ollama, messages).await
            } else {
                debug!("[AI] Using OpenAI-compatible backend");
                self.chat_openai(messages).await
            }
        } else {
            debug!("[AI] No Ollama config, using OpenAI-compatible backend");
            self.chat_openai(messages).await
        };

        match result {
            Ok(response) => {
                debug!("[AI] Chat successful, response_length={}", response.len());
                Ok(ChatResult::Success(response))
            }
            Err(e) => {
                warn!("[AI] Chat error: {}", e);
                Err(e)
            }
        }
    }

    async fn chat_openai(&self, messages: Vec<Message>) -> Result<String> {
        let request = ChatRequest {
            model: self.config.model.clone(),
            messages,
            max_tokens: self.config.max_tokens,
            temperature: self.config.temperature,
        };
        debug!("[AI] OpenAI request: model={}, max_tokens={}, temperature={}, api_url={}", 
            self.config.model, self.config.max_tokens, self.config.temperature, self.config.api_url);

        debug!("[AI] Sending request to OpenAI API...");

        let response = self.client
            .post(&self.config.api_url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .json(&request)
            .send()
            .await
            .context("Failed to send request to OpenAI API. Please check your network connection and api_url in config.toml")?;

        let status = response.status();
        debug!("[AI] OpenAI API response status: {}", status);

        if !response.status().is_success() {
            let body = match response.text().await {
                Ok(text) => {
                    debug!("[AI] OpenAI error response body: {}", text);
                    text
                },
                Err(e) => format!("<failed to read error response: {}>", e),
            };
            
            if status.as_u16() == 401 {
                bail!(
                    "OpenAI API authentication failed. \n\
                     Please check that api_key in config.toml is correct.\n\
                     Response: {}", body
                );
            } else if status.as_u16() == 429 {
                bail!(
                    "OpenAI API rate limit exceeded. Please try again later.\n\
                     Response: {}", body
                );
            }
            
            warn!("[AI] OpenAI API error: {} - {}", status, body);
            bail!("OpenAI API returned error: {} - {}", status, body);
        }

        debug!("[AI] Parsing OpenAI response...");
        let chat_response: ChatResponse = response
            .json()
            .await
            .context("Failed to parse OpenAI response")?;

        // 检查 API 错误
        if let Some(error) = chat_response.error {
            debug!("[AI] OpenAI API returned error in response: {}", error.message);
            bail!("OpenAI API error: {}", error.message);
        }

        // 获取响应内容
        let content = chat_response
            .choices
            .and_then(|c| c.first().map(|c| c.message.content.clone()))
            .ok_or_else(|| anyhow::anyhow!("No response from AI API"))?;
        
        debug!("[AI] OpenAI response parsed successfully, content_length={}", content.len());
        Ok(content)
    }

    async fn chat_ollama(&self, ollama: &OllamaConfig, messages: Vec<Message>) -> Result<String> {
        debug!("[AI] Preparing Ollama request: model={}, messages_count={}", ollama.model, messages.len());
        
        // 转换为 Ollama 格式（使用 /api/chat 端点）
        let ollama_messages: Vec<OllamaMessage> = messages
            .into_iter()
            .map(|m| OllamaMessage {
                role: m.role,
                content: m.content,
            })
            .collect();

        // 构造 /api/chat 请求
        let chat_url = ollama.url.replace("/api/generate", "/api/chat");
        let request = OllamaChatRequest {
            model: ollama.model.clone(),
            messages: ollama_messages,
            stream: false,
        };

        debug!("[AI] Sending request to Ollama /api/chat: {}", chat_url);
        debug!("[AI] Ollama request: model={}, url={}", ollama.model, chat_url);

        let response = self.client
            .post(&chat_url)
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Ollama API. Please ensure Ollama is running and the URL in config.toml is correct")?;

        let status = response.status();
        debug!("[AI] Ollama API response status: {}", status);

        if !response.status().is_success() {
            let body = match response.text().await {
                Ok(text) => {
                    debug!("[AI] Ollama error response body: {}", text);
                    text
                },
                Err(e) => format!("<failed to read error response: {}>", e),
            };
            warn!("[AI] Ollama API error: {} - {}", status, body);
            bail!("Ollama API returned error: {} - {}", status, body);
        }

        debug!("[AI] Parsing Ollama response...");
        let ollama_response: OllamaChatResponse = response
            .json()
            .await
            .context("Failed to parse Ollama response")?;

        debug!("[AI] Ollama response parsed successfully, content_length={}", ollama_response.message.content.len());
        Ok(ollama_response.message.content)
    }

    pub fn get_trigger(&self) -> &str {
        &self.config.trigger
    }
}

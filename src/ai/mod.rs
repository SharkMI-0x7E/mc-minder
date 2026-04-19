use anyhow::{Context, Result, bail};
use log::{debug, warn};
use reqwest::Client;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;
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
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    response: String,
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
        {
            let last_requests = self.last_request.read();
            if let Some(last_time) = last_requests.get(player) {
                let elapsed = last_time.elapsed();
                if elapsed < Duration::from_secs(PLAYER_COOLDOWN_SECS) {
                    let wait_secs = PLAYER_COOLDOWN_SECS - elapsed.as_secs();
                    return Ok(ChatResult::RateLimited(format!(
                        "Please wait {} seconds before asking again.",
                        wait_secs
                    )));
                }
            }
        }

        let _permit = self.semaphore.acquire().await;
        
        {
            let mut last_requests = self.last_request.write();
            last_requests.insert(player.to_string(), Instant::now());
        }

        let result = if let Some(ref ollama) = self.ollama_config {
            if ollama.enabled {
                self.chat_ollama(ollama, messages).await
            } else {
                self.chat_openai(messages).await
            }
        } else {
            self.chat_openai(messages).await
        };

        match result {
            Ok(response) => Ok(ChatResult::Success(response)),
            Err(e) => {
                warn!("AI chat error: {}", e);
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

        debug!("Sending request to OpenAI API");

        let response = self.client
            .post(&self.config.api_url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .json(&request)
            .send()
            .await
            .context("Failed to send request to OpenAI API. Please check your network connection and api_url in config.toml")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            
            if status.as_u16() == 401 {
                bail!(
                    "OpenAI API authentication failed. \n\
                     Please check that api_key in config.toml is correct."
                );
            } else if status.as_u16() == 429 {
                bail!(
                    "OpenAI API rate limit exceeded. Please try again later."
                );
            }
            
            warn!("OpenAI API error: {} - {}", status, body);
            bail!("OpenAI API returned error: {}", status);
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .context("Failed to parse OpenAI response")?;

        chat_response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| anyhow::anyhow!("No response from OpenAI"))
    }

    async fn chat_ollama(&self, ollama: &OllamaConfig, messages: Vec<Message>) -> Result<String> {
        let prompt = messages
            .iter()
            .map(|m| format!("{}: {}", m.role, m.content))
            .collect::<Vec<_>>()
            .join("\n");

        let request = OllamaRequest {
            model: ollama.model.clone(),
            prompt,
            stream: false,
        };

        debug!("Sending request to Ollama API");

        let response = self.client
            .post(&ollama.url)
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Ollama API. Please ensure Ollama is running and the URL in config.toml is correct")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            warn!("Ollama API error: {} - {}", status, body);
            bail!("Ollama API returned error: {}", status);
        }

        let ollama_response: OllamaResponse = response
            .json()
            .await
            .context("Failed to parse Ollama response")?;

        Ok(ollama_response.response)
    }

    pub fn get_trigger(&self) -> &str {
        &self.config.trigger
    }
}

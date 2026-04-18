use anyhow::{Result, Context};
use log::{debug, warn};
use reqwest::Client;
use serde::{Serialize, Deserialize};
use std::time::Duration;
use crate::config::{AiConfig, OllamaConfig};

#[derive(Debug, Clone)]
pub struct AiClient {
    client: Client,
    config: AiConfig,
    ollama_config: Option<OllamaConfig>,
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
        })
    }

    pub async fn chat(&self, messages: Vec<Message>) -> Result<String> {
        if let Some(ref ollama) = self.ollama_config {
            if ollama.enabled {
                return self.chat_ollama(ollama, messages).await;
            }
        }
        
        self.chat_openai(messages).await
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
            .context("Failed to send request to OpenAI API")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            warn!("OpenAI API error: {} - {}", status, body);
            anyhow::bail!("OpenAI API returned error: {}", status);
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
            .context("Failed to send request to Ollama API")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            warn!("Ollama API error: {} - {}", status, body);
            anyhow::bail!("Ollama API returned error: {}", status);
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

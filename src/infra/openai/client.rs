use anyhow::{Context, Result};
use reqwest::blocking::multipart::{Form, Part};
use reqwest::blocking::Client;
use reqwest::Url;
use serde::{Deserialize, Serialize};

const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";
const DEFAULT_TEMPLATE_MODEL: &str = "gpt-5.2";
const DEFAULT_TRANSCRIBE_MODEL: &str = "gpt-4o-transcribe";

#[derive(Debug, Clone)]
pub struct OpenAiConfig {
    pub api_key: String,
    pub base_url: Url,
    pub template_model: String,
    pub transcribe_model: String,
}

impl OpenAiConfig {
    pub fn from_env() -> Result<Self> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .context("OPENAI_API_KEY is not set. Provide it via environment or .env file.")?;
        let base_url =
            std::env::var("OPENAI_BASE_URL").unwrap_or_else(|_| DEFAULT_BASE_URL.to_string());
        let base_url = format!("{}/", base_url.trim_end_matches('/'));
        let base_url = Url::parse(&base_url).context("OPENAI_BASE_URL is not a valid URL")?;
        let template_model = std::env::var("TEMPLIFY_TEMPLATE_MODEL")
            .or_else(|_| std::env::var("OPENAI_TEMPLATE_MODEL"))
            .unwrap_or_else(|_| DEFAULT_TEMPLATE_MODEL.to_string());
        let transcribe_model = std::env::var("TEMPLIFY_TRANSCRIBE_MODEL")
            .unwrap_or_else(|_| DEFAULT_TRANSCRIBE_MODEL.to_string());

        Ok(Self {
            api_key,
            base_url,
            template_model,
            transcribe_model,
        })
    }
}

#[derive(Clone)]
pub struct OpenAiClient {
    client: Client,
    config: OpenAiConfig,
}

impl OpenAiClient {
    pub fn new(config: OpenAiConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .context("Failed to build HTTP client")?;
        Ok(Self { client, config })
    }

    pub fn template_model(&self) -> &str {
        &self.config.template_model
    }

    pub fn complete_chat(
        &self,
        model: &str,
        messages: Vec<ChatMessage>,
        temperature: f32,
        response_format: Option<ChatResponseFormat>,
    ) -> Result<String> {
        let request = ChatCompletionRequest {
            model: model.to_string(),
            messages,
            temperature,
            response_format: response_format.map(|f| f.into_request()),
        };

        let url = self
            .config
            .base_url
            .join("chat/completions")
            .context("Failed to build OpenAI request URL")?;

        let response = self
            .client
            .post(url)
            .bearer_auth(&self.config.api_key)
            .json(&request)
            .send()
            .context("Failed to send request to OpenAI")?
            .error_for_status()
            .context("OpenAI request failed")?;

        let body: ChatCompletionResponse =
            response.json().context("Failed to parse OpenAI response")?;

        let content = body
            .choices
            .first()
            .and_then(|choice| choice.message.as_ref().and_then(|msg| msg.content.as_ref()))
            .cloned()
            .unwrap_or_default();

        Ok(strip_fences(content))
    }

    pub fn transcribe_audio(&self, file: AudioFile) -> Result<String> {
        if file.bytes.is_empty() {
            return Ok(String::new());
        }

        let part = Part::bytes(file.bytes)
            .file_name(file.name)
            .mime_str("audio/wav")
            .context("Failed to set audio MIME type")?;

        let form = Form::new()
            .part("file", part)
            .text("model", self.config.transcribe_model.clone())
            .text("response_format", "json".to_string());

        let url = self
            .config
            .base_url
            .join("audio/transcriptions")
            .context("Failed to build OpenAI request URL")?;

        let response = self
            .client
            .post(url)
            .bearer_auth(&self.config.api_key)
            .multipart(form)
            .send()
            .context("Failed to send request to OpenAI")?
            .error_for_status()
            .context("OpenAI request failed")?;

        let body: AudioResponse = response.json().context("Failed to parse OpenAI response")?;
        Ok(body.text.unwrap_or_default().trim().to_string())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ChatResponseFormat {
    JsonObject,
}

impl ChatResponseFormat {
    fn into_request(self) -> ChatResponseFormatRequest {
        match self {
            ChatResponseFormat::JsonObject => ChatResponseFormatRequest {
                kind: "json_object".to_string(),
            },
        }
    }
}

#[derive(Debug)]
pub struct AudioFile {
    pub name: String,
    pub bytes: Vec<u8>,
}

impl AudioFile {
    pub fn new(name: impl Into<String>, bytes: Vec<u8>) -> Self {
        Self {
            name: name.into(),
            bytes,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ChatMessage {
    role: String,
    content: String,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: content.into(),
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
        }
    }
}

#[derive(Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ChatResponseFormatRequest>,
}

#[derive(Debug, Serialize)]
struct ChatResponseFormatRequest {
    #[serde(rename = "type")]
    kind: String,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: Option<ChatMessageResponse>,
}

#[derive(Debug, Deserialize)]
struct ChatMessageResponse {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AudioResponse {
    text: Option<String>,
}

fn strip_fences(text: String) -> String {
    let trimmed = text.trim();
    let body = match trimmed.strip_prefix("```") {
        Some(stripped) => {
            let stripped = stripped.trim_start();
            if let Some(end) = stripped.rfind("```") {
                stripped[..end].trim().to_string()
            } else {
                stripped.trim().to_string()
            }
        }
        None => trimmed.to_string(),
    };
    body
}

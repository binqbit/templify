use anyhow::{Context, Result};
use serde::Deserialize;

use crate::infra::openai::client::{AudioFile, ChatMessage, ChatResponseFormat, OpenAiClient};
use crate::infra::storage::template_store::{now_timestamp, TemplateRecord};

pub struct TemplateService {
    ai_client: OpenAiClient,
}

pub struct TemplateBuildInput {
    pub user_request: String,
    pub source_data: String,
}

impl TemplateService {
    pub fn new(ai_client: OpenAiClient) -> Self {
        Self { ai_client }
    }

    pub fn build_record(&self, key: &str, input: &TemplateBuildInput) -> Result<TemplateRecord> {
        let content = self.ai_client.complete_chat(
            self.ai_client.template_model(),
            build_prompt_messages(input),
            0.2,
            Some(ChatResponseFormat::JsonObject),
        )?;
        let payload: TemplateGenerationPayload = serde_json::from_str(&content)
            .context("Failed to parse template JSON response from OpenAI")?;
        let template = payload.template.trim().to_string();
        let description_output = payload.description.trim().to_string();

        Ok(TemplateRecord {
            key: key.to_string(),
            description: description_output,
            template,
            created_at: now_timestamp(),
        })
    }

    pub fn transcribe_description(&self, sequence_id: u64, audio_bytes: Vec<u8>) -> Result<String> {
        let file = AudioFile::new(format!("segment-{sequence_id}.wav"), audio_bytes);
        self.ai_client.transcribe_audio(file)
    }

    pub fn render(&self, template: &str, params: &[String], paste: Option<&str>) -> String {
        let mut rendered = template.to_string();

        if let Some(paste_value) = paste {
            rendered = rendered.replace("$PASTE", paste_value);
        }

        // Replace high indexes first to avoid "$1" mutating "$10".
        for index in (1..=params.len()).rev() {
            let placeholder = format!("${index}");
            rendered = rendered.replace(&placeholder, &params[index - 1]);
        }

        rendered
    }
}

fn build_prompt_messages(input: &TemplateBuildInput) -> Vec<ChatMessage> {
    let user_request = if input.user_request.trim().is_empty() {
        "<empty>"
    } else {
        input.user_request.trim()
    };
    let clipboard_context = input.source_data.trim();
    let clipboard_section = if clipboard_context.is_empty() {
        "data: <empty>".to_string()
    } else {
        format!("data (for understanding only; never copy literally):\n{clipboard_context}")
    };

    vec![
        ChatMessage::system(
            "Generate Templify template records. Return strictly valid JSON with exactly two fields: description and template.",
        ),
        ChatMessage::user(format!(
            "Input:\n\
user_request: {user_request}\n\
{clipboard_section}\n\n\
Interpret `user_request` as the intent for how input data should be transformed into a reusable template. \
Treat `data` as source content used to understand structure, variable parts, and what should become placeholders. \
Build the template according to user intent, not by copying the request text. \
The `description` field must be a short English description of what the final rendered data represents and what role runtime parameters play in this template. \
The description must focus on the template result, not on the generation process. \
The `template` field must be final render-ready output text only, with no extra explanations, labels, metadata, or markdown fences. \
The template may be technical or non-technical, but it must be usable by direct replacement only. \
Replacement is raw string substitution with no preprocessing, escaping, or type conversion. \
So template structure must already define correct placement context for placeholders (quotes, separators, wrappers, casting, and parameter position) to keep final output valid. \
For structured formats (SQL/JSON/query/config/code), keep exact syntax so direct replacement works correctly. \
Use `$PASTE` for clipboard text and `$1..$N` for positional runtime parameters when needed, and do not invent placeholders when they are not required. \
Do not copy literal `data` into `template` unless it is explicitly requested. \
Return only valid JSON with exactly these fields: {{\"description\":\"...\",\"template\":\"...\"}}"
        )),
    ]
}

#[derive(Debug, Deserialize)]
struct TemplateGenerationPayload {
    description: String,
    template: String,
}

mod app;
mod cli;
mod infra;

use anyhow::{anyhow, Result};
use chrono::{DateTime, Local, Utc};
use std::io::{self, Write};

use crate::app::template_service::{TemplateBuildInput, TemplateService};
use crate::cli::config::{
    AppConfig, CliCommand, GetTemplateCommand, NewTemplateCommand, ShowTemplateCommand,
};
use crate::infra::clipboard::Clipboard;
use crate::infra::openai::client::OpenAiClient;
use crate::infra::speech::listener::{SpeechConfig, SpeechListener};
use crate::infra::storage::template_store::TemplateStore;

fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let config = AppConfig::from_env_and_args()?;
    let store_path = config.data_dir.join("templates.json");
    let client = OpenAiClient::new(config.openai.clone())?;
    let mut app = App {
        config,
        template_service: TemplateService::new(client),
        store: TemplateStore::open(store_path)?,
        clipboard: None,
    };

    app.run()
}

struct App {
    config: AppConfig,
    template_service: TemplateService,
    store: TemplateStore,
    clipboard: Option<Clipboard>,
}

impl App {
    fn run(&mut self) -> Result<()> {
        match self.config.command.clone() {
            CliCommand::New(cmd) => self.run_new(cmd),
            CliCommand::Get(cmd) => self.run_get(cmd),
            CliCommand::Show(cmd) => self.run_show(cmd),
        }
    }

    fn run_new(&mut self, cmd: NewTemplateCommand) -> Result<()> {
        let clipboard_text = self
            .ensure_clipboard()
            .and_then(|clipboard| clipboard.get_text())
            .unwrap_or_default();

        let description = if let Some(prompt) = cmd.prompt.clone() {
            prompt
        } else {
            println!("Listening for speech... (Ctrl+C to exit)");
            let listener = SpeechListener::new(SpeechConfig::default(), cmd.device_index)?;
            let segment = listener.listen_once()?;
            println!("Transcribing audio...");
            let text = self
                .template_service
                .transcribe_description(segment.sequence_id, segment.audio_bytes)?;
            println!("Description: {text}");
            text
        };

        let build_input = TemplateBuildInput {
            user_request: description.trim().to_string(),
            source_data: clipboard_text.trim().to_string(),
        };

        if build_input.user_request.is_empty() && build_input.source_data.is_empty() {
            return Err(anyhow!(
                "Template input is empty. Voice transcription returned no text and clipboard is empty."
            ));
        }

        if self.store.contains(&cmd.key) && !cmd.force {
            return Err(anyhow!(
                "Template key '{key}' already exists. Use --force to overwrite.",
                key = cmd.key
            ));
        }

        println!("Generating template via OpenAI...");
        let record = self.template_service.build_record(&cmd.key, &build_input)?;

        self.store.upsert(record)?;

        println!(
            "Template '{}' saved in {}",
            cmd.key,
            self.store.path().to_string_lossy()
        );
        Ok(())
    }

    fn run_get(&mut self, cmd: GetTemplateCommand) -> Result<()> {
        let template = self
            .store
            .get(&cmd.key)
            .ok_or_else(|| anyhow!("Template key '{key}' not found", key = cmd.key))?
            .clone();

        let paste_value = if template.template.contains("$PASTE") {
            Some(self.ensure_clipboard()?.get_text()?)
        } else {
            None
        };

        let rendered =
            self.template_service
                .render(&template.template, &cmd.params, paste_value.as_deref());
        self.ensure_clipboard()?.set_text(&rendered)?;

        println!("Template '{}' copied to clipboard", template.key);
        println!("{rendered}");
        Ok(())
    }

    fn run_show(&mut self, cmd: ShowTemplateCommand) -> Result<()> {
        let template = self
            .store
            .get(&cmd.key)
            .ok_or_else(|| anyhow!("Template key '{key}' not found", key = cmd.key))?;

        println!("Description: {}", template.description);
        println!(
            "Created at: {} (unix: {})",
            format_unix_timestamp(template.created_at),
            template.created_at
        );
        println!("Template:\n{}", template.template);
        print!("\nPress Enter to continue...");
        io::stdout().flush()?;
        let mut line = String::new();
        io::stdin().read_line(&mut line)?;
        Ok(())
    }

    fn ensure_clipboard(&mut self) -> Result<&mut Clipboard> {
        if self.clipboard.is_none() {
            self.clipboard = Some(Clipboard::new()?);
        }

        self.clipboard
            .as_mut()
            .ok_or_else(|| anyhow!("Clipboard unavailable"))
    }
}

fn format_unix_timestamp(timestamp: u64) -> String {
    let Ok(seconds) = i64::try_from(timestamp) else {
        return "invalid timestamp".to_string();
    };

    let Some(utc_time) = DateTime::<Utc>::from_timestamp(seconds, 0) else {
        return "invalid timestamp".to_string();
    };

    utc_time
        .with_timezone(&Local)
        .format("%Y-%m-%d %H:%M:%S %Z")
        .to_string()
}

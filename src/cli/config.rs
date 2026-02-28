use std::ffi::OsStr;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use crate::infra::openai::client::OpenAiConfig;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub command: CliCommand,
    pub openai: OpenAiConfig,
    pub data_dir: PathBuf,
}

impl AppConfig {
    pub fn from_env_and_args() -> Result<Self> {
        let args = Args::parse();
        let openai = OpenAiConfig::from_env()?;
        let command = match args.command {
            CliCommandInput::New {
                key,
                prompt,
                device_index,
                force,
            } => CliCommand::New(NewTemplateCommand {
                key,
                prompt,
                device_index,
                force,
            }),
            CliCommandInput::Get { key, params } => {
                CliCommand::Get(GetTemplateCommand { key, params })
            }
            CliCommandInput::Show { key } => CliCommand::Show(ShowTemplateCommand { key }),
        };

        let data_dir = match args.data_dir {
            Some(path) => path,
            None => resolve_data_dir()?,
        };

        Ok(Self {
            command,
            openai,
            data_dir,
        })
    }
}

#[derive(Debug, Clone)]
pub enum CliCommand {
    New(NewTemplateCommand),
    Get(GetTemplateCommand),
    Show(ShowTemplateCommand),
}

#[derive(Debug, Clone)]
pub struct NewTemplateCommand {
    pub key: String,
    pub prompt: Option<String>,
    pub device_index: Option<usize>,
    pub force: bool,
}

#[derive(Debug, Clone)]
pub struct GetTemplateCommand {
    pub key: String,
    pub params: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ShowTemplateCommand {
    pub key: String,
}

#[derive(Parser, Debug)]
#[command(
    name = "templify",
    version,
    about = "Create and reuse templates built from clipboard/context prompts"
)]
struct Args {
    #[command(subcommand)]
    command: CliCommandInput,

    /// Folder where template store is kept (default: <project_dir>/data)
    #[arg(long = "data-dir")]
    data_dir: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
enum CliCommandInput {
    /// Create template from description and save it locally
    New {
        /// Template key (identifier used later in `get`)
        key: String,
        /// Optional description override (if not set, it uses voice transcription)
        #[arg(short = 'p', long)]
        prompt: Option<String>,
        /// Optional input device index for voice capture
        #[arg(long = "device", value_parser = clap::value_parser!(usize))]
        device_index: Option<usize>,
        /// Replace existing template with the same key
        #[arg(short = 'f', long)]
        force: bool,
    },
    /// Render a template by key with positional params, then copy to clipboard
    Get {
        /// Template key
        key: String,
        /// Positional params used for $1, $2 ... placeholders
        params: Vec<String>,
    },
    /// Print raw template text by key and wait for Enter
    Show {
        /// Template key
        key: String,
    },
}

fn resolve_data_dir() -> Result<PathBuf> {
    let exe_path = std::env::current_exe().context("Failed to determine executable path")?;
    if let Some(exe_dir) = exe_path.parent() {
        if exe_dir.file_name() == Some(OsStr::new("bin")) {
            if let Some(project_dir) = exe_dir.parent() {
                return Ok(project_dir.join("data"));
            }
        }

        if exe_dir.file_name() == Some(OsStr::new("release"))
            && exe_dir.parent().and_then(|p| p.file_name()) == Some(OsStr::new("target"))
        {
            if let Some(project_dir) = exe_dir.parent().and_then(|p| p.parent()) {
                return Ok(project_dir.join("data"));
            }
        }
    }

    let current_dir = std::env::current_dir()
        .context("Failed to determine current working directory for default data path")?;
    Ok(current_dir.join("data"))
}

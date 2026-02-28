use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateRecord {
    #[serde(alias = "name")]
    pub key: String,
    /// Optional short description/summary for local reference
    pub description: String,
    pub template: String,
    pub created_at: u64,
}

pub struct TemplateStore {
    path: PathBuf,
    templates: HashMap<String, TemplateRecord>,
}

impl TemplateStore {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).context("Failed to create template storage directory")?;
        }

        let templates = if path.exists() {
            let data = fs::read_to_string(&path).context("Failed to read template store")?;
            if data.trim().is_empty() {
                HashMap::new()
            } else {
                serde_json::from_str(&data).context("Failed to parse template store JSON")?
            }
        } else {
            HashMap::new()
        };

        Ok(Self { path, templates })
    }

    pub fn get(&self, key: &str) -> Option<&TemplateRecord> {
        self.templates.get(key)
    }

    pub fn contains(&self, key: &str) -> bool {
        self.templates.contains_key(key)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn upsert(&mut self, record: TemplateRecord) -> Result<()> {
        self.templates.insert(record.key.clone(), record);

        let data = serde_json::to_string_pretty(&self.templates)
            .context("Failed to serialize template store")?;
        fs::write(&self.path, data).context("Failed to save template store")?;
        Ok(())
    }
}

pub fn now_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

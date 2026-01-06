use crate::schema::cards::{SchemaCards, WorkspaceIndex};
use anyhow::Context;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SchemaRegistry {
    pub index: WorkspaceIndex,
    pub cards: SchemaCards,
}

impl SchemaRegistry {
    pub fn load(index_path: &str) -> anyhow::Result<Self> {
        let idx = std::fs::read_to_string(index_path)
            .with_context(|| format!("read workspace index: {}", index_path))?;
        let index: WorkspaceIndex = serde_json::from_str(&idx)?;

        // Resolve schema_cards_path relative to the repo root (two levels up from index file)
        let cards_path = if std::path::Path::new(&index.schema_cards_path).is_absolute() {
            index.schema_cards_path.clone()
        } else {
            let index_path_buf = std::path::Path::new(index_path);
            let repo_root = index_path_buf
                .parent() // config/workspaces
                .and_then(|p| p.parent()) // config
                .and_then(|p| p.parent()) // repo root
                .unwrap_or_else(|| std::path::Path::new("."));
            repo_root
                .join(&index.schema_cards_path)
                .to_str()
                .unwrap()
                .to_string()
        };

        let cards_raw = std::fs::read_to_string(&cards_path)
            .with_context(|| format!("read schema cards: {}", index.schema_cards_path))?;
        let cards: SchemaCards = serde_json::from_str(&cards_raw)?;

        Ok(Self { index, cards })
    }
}

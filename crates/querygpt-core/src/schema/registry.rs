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

        let cards_raw = std::fs::read_to_string(&index.schema_cards_path)
            .with_context(|| format!("read schema cards: {}", index.schema_cards_path))?;
        let cards: SchemaCards = serde_json::from_str(&cards_raw)?;

        Ok(Self { index, cards })
    }
}

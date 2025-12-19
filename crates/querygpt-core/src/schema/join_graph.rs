use crate::schema::cards::{JoinEdge, SchemaCards};
use anyhow::anyhow;

pub fn find_edge<'a>(cards: &'a SchemaCards, from: &str, to: &str) -> Option<&'a JoinEdge> {
    cards.join_graph.edges.iter().find(|e| e.from == from && e.to == to)
}

pub fn assert_edge_safe(cards: &SchemaCards, from: &str, to: &str) -> anyhow::Result<()> {
    let edge = find_edge(cards, from, to).ok_or_else(|| anyhow!("no join edge {} -> {}", from, to))?;
    if !edge.safe {
        return Err(anyhow!("join edge {} -> {} is marked unsafe", from, to));
    }
    Ok(())
}
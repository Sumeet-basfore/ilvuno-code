//! Second-opinion warning (PRD §4). When the reviewer runs on the same provider as the builder, the
//! round gets no independent review. That is a warning in the readiness report, never a refusal: the
//! user decides.

use anyhow::Result;
use rusqlite::{params, Connection};
use std::collections::BTreeMap;

/// Warning lines for one round, from the provider each of its dispatches ran on.
pub fn for_round(conn: &Connection, round_id: &str) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT d.slot, t.provider FROM dispatch d JOIN threads t ON t.id = d.thread_id
         WHERE d.round_id = ?1 ORDER BY d.rowid",
    )?;
    let rows = stmt.query_map(params![round_id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
    let mut providers = BTreeMap::new();
    for row in rows {
        let (slot, provider) = row?;
        providers.insert(slot, provider);
    }
    Ok(warnings(&providers))
}

/// The rule, on slot → provider id. Slots that are absent from the round are not compared.
pub fn warnings(providers: &BTreeMap<String, String>) -> Vec<String> {
    match (providers.get("builder"), providers.get("reviewer")) {
        (Some(builder), Some(reviewer)) if builder == reviewer => {
            vec![format!("reviewer uses the same provider as builder ({builder}): no second opinion")]
        }
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn providers(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs.iter().map(|(s, p)| (s.to_string(), p.to_string())).collect()
    }

    #[test]
    fn same_provider_for_builder_and_reviewer_warns_with_the_id() {
        let warned = warnings(&providers(&[("builder", "claude-main"), ("reviewer", "claude-main")]));
        assert_eq!(warned, vec!["reviewer uses the same provider as builder (claude-main): no second opinion"]);
    }

    #[test]
    fn different_providers_do_not_warn() {
        let warned = warnings(&providers(&[("builder", "claude-main"), ("reviewer", "codex-main")]));
        assert!(warned.is_empty());
    }

    #[test]
    fn a_round_without_both_roles_is_not_compared() {
        assert!(warnings(&providers(&[("researcher-a", "claude-main"), ("synthesizer", "claude-main")])).is_empty());
        assert!(warnings(&providers(&[("builder", "claude-main")])).is_empty());
    }
}

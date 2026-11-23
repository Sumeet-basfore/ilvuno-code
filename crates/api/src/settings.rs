//! `GET /settings` and `PUT /settings`: the window's defaults, stored as JSON under fixed keys in the
//! `settings` table. A read fills in the defaults; a write takes only the keys it is given.

use crate::blocking::{blocking, open_store};
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;
use axum::body::Bytes;
use axum::extract::State;
use axum::Json;
use rusqlite::Connection;
use serde::Deserialize;
use serde_json::{json, Map, Value};

const ROUNDS_ANY: std::ops::RangeInclusive<u32> = 1..=20;
const TOKENS_ANY: std::ops::RangeInclusive<u32> = 1000..=10_000_000;
const MESSAGE_POLICIES: [&str; 3] = ["anytime", "round", "never"];
const THEMES: [&str; 3] = ["dark", "light", "system"];

/// A write may name any of these keys, all of them optional. Anything else is a 400.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Patch {
    budget_rounds: Option<u32>,
    budget_tokens: Option<u32>,
    message_policy: Option<String>,
    theme: Option<String>,
    human_gate: Option<bool>,
}

impl Patch {
    fn validate(&self) -> ApiResult<()> {
        if let Some(rounds) = self.budget_rounds {
            if !ROUNDS_ANY.contains(&rounds) {
                return Err(ApiError::BadRequest(format!("budget_rounds must be {}–{}", ROUNDS_ANY.start(), ROUNDS_ANY.end())));
            }
        }
        if let Some(tokens) = self.budget_tokens {
            if !TOKENS_ANY.contains(&tokens) {
                return Err(ApiError::BadRequest(format!("budget_tokens must be {}–{}", TOKENS_ANY.start(), TOKENS_ANY.end())));
            }
        }
        if let Some(policy) = &self.message_policy {
            if !MESSAGE_POLICIES.contains(&policy.as_str()) {
                return Err(ApiError::BadRequest(format!("message_policy must be one of {}", MESSAGE_POLICIES.join(", "))));
            }
        }
        if let Some(theme) = &self.theme {
            if !THEMES.contains(&theme.as_str()) {
                return Err(ApiError::BadRequest(format!("theme must be one of {}", THEMES.join(", "))));
            }
        }
        Ok(())
    }

    /// Only the keys the body carried, as the values to store.
    fn pairs(&self) -> Vec<(&'static str, Value)> {
        let mut out = Vec::new();
        if let Some(rounds) = self.budget_rounds {
            out.push(("budget_rounds", json!(rounds)));
        }
        if let Some(tokens) = self.budget_tokens {
            out.push(("budget_tokens", json!(tokens)));
        }
        if let Some(policy) = &self.message_policy {
            out.push(("message_policy", json!(policy)));
        }
        if let Some(theme) = &self.theme {
            out.push(("theme", json!(theme)));
        }
        if let Some(gate) = self.human_gate {
            out.push(("human_gate", json!(gate)));
        }
        out
    }
}

fn defaults() -> Map<String, Value> {
    let mut out = Map::new();
    out.insert("budget_rounds".into(), json!(3));
    out.insert("budget_tokens".into(), json!(150_000));
    out.insert("message_policy".into(), json!("round"));
    out.insert("theme".into(), json!("dark"));
    out.insert("human_gate".into(), json!(true));
    out
}

/// Every setting, with the stored value where there is a good one and the default where there is not.
pub fn read(conn: &Connection) -> ApiResult<Value> {
    let mut out = defaults();
    let mut stmt = conn.prepare("SELECT key, value FROM settings").map_err(|e| ApiError::Internal(format!("read settings: {e}")))?;
    let rows = stmt
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
        .map_err(|e| ApiError::Internal(format!("read settings: {e}")))?;
    for row in rows {
        let (key, text) = row.map_err(|e| ApiError::Internal(format!("read settings: {e}")))?;
        if !out.contains_key(&key) {
            continue;
        }
        match serde_json::from_str(&text) {
            Ok(value) => {
                out.insert(key, value);
            }
            // Only a hand-edited row can be bad: every write is validated. Keep the screen usable.
            Err(err) => eprintln!("ilvunod: settings row {key} is not JSON ({err}); using the default"),
        }
    }
    Ok(Value::Object(out))
}

/// Write the keys the patch carries, all or nothing.
pub fn write(conn: &Connection, patch: &Patch) -> ApiResult<()> {
    patch.validate()?;
    let tx = conn.unchecked_transaction().map_err(|e| ApiError::Internal(format!("write settings: {e}")))?;
    for (key, value) in patch.pairs() {
        tx.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            rusqlite::params![key, value.to_string()],
        )
        .map_err(|e| ApiError::Internal(format!("write settings: {e}")))?;
    }
    tx.commit().map_err(|e| ApiError::Internal(format!("write settings: {e}")))?;
    Ok(())
}

pub async fn read_route<P>(State(state): State<AppState<P>>) -> ApiResult<Json<Value>>
where
    P: Send + Sync + 'static,
{
    let home = state.home.clone();
    let settings = blocking(move || {
        let store = open_store(&home)?;
        read(store.conn())
    })
    .await?;
    Ok(Json(settings))
}

pub async fn write_route<P>(State(state): State<AppState<P>>, body: Bytes) -> ApiResult<Json<Value>>
where
    P: Send + Sync + 'static,
{
    let patch: Patch = crate::actions::parse_body(&body)?;
    let home = state.home.clone();
    // The answer is the full object after the write, read in the same transaction's connection.
    let settings = blocking(move || {
        let store = open_store(&home)?;
        write(store.conn(), &patch)?;
        read(store.conn())
    })
    .await?;
    Ok(Json(settings))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> runner::Store {
        let dir = std::env::temp_dir().join(format!("ilvuno-settings-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("scratch dir");
        runner::Store::open(&dir.join("state.db")).expect("store")
    }

    fn patch(text: &str) -> Patch {
        serde_json::from_str(text).expect("patch")
    }

    #[test]
    fn an_empty_table_reads_as_the_defaults() {
        let store = store();
        let settings = read(store.conn()).expect("read");
        assert_eq!(
            settings,
            json!({ "budget_rounds": 3, "budget_tokens": 150_000, "message_policy": "round", "theme": "dark", "human_gate": true })
        );
    }

    #[test]
    fn a_write_is_read_back_and_leaves_the_keys_it_did_not_name_alone() {
        let store = store();
        write(store.conn(), &patch(r#"{"theme":"light","human_gate":false}"#)).expect("first write");
        write(store.conn(), &patch(r#"{"budget_rounds":7}"#)).expect("partial write");
        let settings = read(store.conn()).expect("read");
        assert_eq!(settings["budget_rounds"], json!(7));
        assert_eq!(settings["theme"], json!("light"));
        assert_eq!(settings["human_gate"], json!(false));
        assert_eq!(settings["budget_tokens"], json!(150_000));
    }

    #[test]
    fn each_out_of_range_or_unknown_value_is_refused_and_writes_nothing() {
        let store = store();
        for bad in [
            r#"{"budget_rounds":0}"#,
            r#"{"budget_rounds":21}"#,
            r#"{"budget_tokens":999}"#,
            r#"{"budget_tokens":10000001}"#,
            r#"{"message_policy":"sometimes"}"#,
            r#"{"theme":"sepia"}"#,
        ] {
            assert!(write(store.conn(), &patch(bad)).is_err(), "{bad} should be refused");
        }
        assert_eq!(read(store.conn()).expect("read"), Value::Object(defaults()));
    }

    #[test]
    fn the_edges_of_each_range_are_allowed() {
        let store = store();
        write(store.conn(), &patch(r#"{"budget_rounds":1,"budget_tokens":1000}"#)).expect("lower edges");
        write(store.conn(), &patch(r#"{"budget_rounds":20,"budget_tokens":10000000}"#)).expect("upper edges");
        let settings = read(store.conn()).expect("read");
        assert_eq!(settings["budget_rounds"], json!(20));
        assert_eq!(settings["budget_tokens"], json!(10_000_000));
    }
}

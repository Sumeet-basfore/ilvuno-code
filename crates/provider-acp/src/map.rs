//! Pure translation between ACP frames and the runner's events. No I/O: every rule here is tested on
//! frames shaped like the ones an ACP agent sends (see the tests at the end of this file).
//!
//! Tool calls are translated into the names the runner's policy already decides on (`Read`, `Edit`,
//! `Bash`). Anything it cannot name precisely is given a name the policy denies, so it fails closed.

use agent_client_protocol::v1::schema::{
    ContentBlock, ContentChunk, Cost, PermissionOption, PermissionOptionId, PermissionOptionKind, PromptResponse,
    SessionNotification, SessionUpdate, StopReason, ToolCall, ToolCallContent, ToolCallStatus, ToolCallUpdate,
    ToolKind, UsageUpdate,
};
use provider_core::{Decision, Event};
use serde_json::{json, Value};

/// Longest tool output or raw payload carried in an event, in characters.
const OUTPUT_LIMIT: usize = 2_000;

/// Turns the agent's cumulative USD cost into per-update deltas, and keeps the running total so the
/// adapter can stop a call that passes its cap. Other currencies are not summed: they are not USD.
#[derive(Debug, Default)]
pub struct CostTracker {
    last_total: f64,
    spent: f64,
}

impl CostTracker {
    pub fn record(&mut self, cost: Option<&Cost>) -> Option<f64> {
        let cost = cost.filter(|c| c.currency == "USD")?;
        let delta = (cost.amount - self.last_total).max(0.0);
        self.last_total = self.last_total.max(cost.amount);
        self.spent += delta;
        Some(delta)
    }

    pub fn spent(&self) -> f64 {
        self.spent
    }
}

/// Events for one `session/update` notification.
pub fn update_events(note: &SessionNotification, cost: &mut CostTracker) -> Vec<Event> {
    match &note.update {
        SessionUpdate::AgentMessageChunk(chunk) => text_of(chunk).map(|text| Event::TextDelta { text }).into_iter().collect(),
        SessionUpdate::AgentThoughtChunk(chunk) => {
            text_of(chunk).map(|text| Event::ThinkingDelta { text }).into_iter().collect()
        }
        SessionUpdate::ToolCall(call) => vec![tool_call_event(call)],
        SessionUpdate::ToolCallUpdate(update) => vec![tool_update_event(update)],
        SessionUpdate::UsageUpdate(UsageUpdate { used, cost: spend, .. }) => vec![Event::Usage {
            tokens_in: 0,
            tokens_out: 0,
            cache_read_tokens: 0,
            context_used: Some(*used),
            cost_usd: cost.record(spend.as_ref()),
        }],
        other => vec![Event::Raw { payload: clip_value(serde_json::to_value(other).unwrap_or(Value::Null)) }],
    }
}

fn text_of(chunk: &ContentChunk) -> Option<String> {
    match &chunk.content {
        ContentBlock::Text(t) if !t.text.is_empty() => Some(t.text.clone()),
        _ => None,
    }
}

fn tool_call_event(call: &ToolCall) -> Event {
    Event::ToolCall {
        id: call.tool_call_id.to_string(),
        name: call.title.clone(),
        input: clip_value(call.raw_input.clone().unwrap_or(Value::Null)),
        kind: kind_name(&call.kind).to_string(),
        status: status_name(&call.status).to_string(),
    }
}

/// A status change for a tool call. Terminal statuses become a result; the rest stay raw.
fn tool_update_event(update: &ToolCallUpdate) -> Event {
    let fields = &update.fields;
    match fields.status {
        Some(status @ (ToolCallStatus::Completed | ToolCallStatus::Failed)) => Event::ToolResult {
            id: update.tool_call_id.to_string(),
            output: clip_text(&output_text(fields.raw_output.as_ref(), fields.content.as_deref())),
            status: status_name(&status).to_string(),
        },
        _ => Event::Raw {
            payload: json!({ "tool_call_id": update.tool_call_id.to_string(), "status": fields.status.as_ref().map(status_name) }),
        },
    }
}

/// The tool, and its input in the runner's policy vocabulary, for a permission request.
pub fn translate_tool(update: &ToolCallUpdate) -> (String, Value) {
    let fields = &update.fields;
    let paths: Vec<String> = fields
        .locations
        .iter()
        .flatten()
        .map(|l| l.path.to_string_lossy().to_string())
        .collect();
    match fields.kind {
        Some(ToolKind::Execute) => {
            let raw = fields.raw_input.clone().unwrap_or(Value::Null);
            let command = command_text(&raw).or_else(|| fields.title.clone()).unwrap_or_default();
            ("Bash".to_string(), json!({ "command": command }))
        }
        Some(ToolKind::Read) => single_path("Read", &paths),
        Some(ToolKind::Edit) => single_path("Edit", &paths),
        Some(other) => (kind_name(&other).to_string(), json!({})),
        None => ("unknown".to_string(), json!({})),
    }
}

/// One path, or a name the policy denies: a change that touches several files is never judged by
/// its first file alone.
fn single_path(tool: &str, paths: &[String]) -> (String, Value) {
    match paths {
        [only] => (tool.to_string(), json!({ "file_path": only })),
        _ => ("multi-file change".to_string(), json!({ "paths": paths })),
    }
}

/// The command text of a shell call. A `bash -lc "<text>"` wrapper is unwrapped one level, so the
/// policy sees the command the agent meant to run. Anything else is joined as argv.
fn command_text(raw: &Value) -> Option<String> {
    match raw.get("command")? {
        Value::String(s) => Some(s.clone()),
        Value::Array(parts) => {
            let words: Vec<&str> = parts.iter().map(Value::as_str).collect::<Option<_>>()?;
            match words.as_slice() {
                [shell, flag, text] if matches!(*shell, "bash" | "sh" | "zsh") && matches!(*flag, "-c" | "-lc") => {
                    Some(text.to_string())
                }
                _ => Some(words.join(" ")),
            }
        }
        _ => None,
    }
}

/// The option the runner's decision maps to. Chosen by kind, never by position: allow-once is
/// preferred over allow-always, reject-once over reject-always. `None` means cancel.
pub fn select_option(options: &[PermissionOption], decision: &Decision) -> Option<PermissionOptionId> {
    let order: [PermissionOptionKind; 2] = match decision {
        Decision::Allow { .. } => [PermissionOptionKind::AllowOnce, PermissionOptionKind::AllowAlways],
        Decision::Deny { .. } => [PermissionOptionKind::RejectOnce, PermissionOptionKind::RejectAlways],
    };
    order.iter().find_map(|kind| options.iter().find(|o| o.kind == *kind).map(|o| o.option_id.clone()))
}

/// The turn's end: the stop reason. ACP v1 reports no per-turn token counts, so none are invented here.
pub fn turn_finished(response: &PromptResponse, cancel_requested: bool) -> Vec<Event> {
    vec![Event::TurnFinished {
        stop_reason: Some(stop_name(&response.stop_reason).to_string()),
        interrupted: cancel_requested || matches!(response.stop_reason, StopReason::Cancelled),
        error: None,
    }]
}

/// A prompt that failed on the wire. The runner treats a turn error as a failed dispatch.
pub fn turn_error(message: &str) -> Event {
    Event::TurnFinished { stop_reason: None, interrupted: false, error: Some(clip_text(message)) }
}

fn kind_name(kind: &ToolKind) -> &'static str {
    match kind {
        ToolKind::Read => "read",
        ToolKind::Edit => "edit",
        ToolKind::Delete => "delete",
        ToolKind::Move => "move",
        ToolKind::Search => "search",
        ToolKind::Execute => "execute",
        ToolKind::Think => "think",
        ToolKind::Fetch => "fetch",
        ToolKind::SwitchMode => "switch_mode",
        _ => "other",
    }
}

fn status_name(status: &ToolCallStatus) -> &'static str {
    match status {
        ToolCallStatus::Pending => "pending",
        ToolCallStatus::InProgress => "in_progress",
        ToolCallStatus::Completed => "completed",
        ToolCallStatus::Failed => "failed",
        _ => "unknown",
    }
}

fn stop_name(stop: &StopReason) -> &'static str {
    match stop {
        StopReason::EndTurn => "end_turn",
        StopReason::MaxTokens => "max_tokens",
        StopReason::MaxTurnRequests => "max_turn_requests",
        StopReason::Refusal => "refusal",
        StopReason::Cancelled => "cancelled",
        _ => "unknown",
    }
}

/// A tool's output as text: the raw output when the agent sent one, else its text content blocks.
fn output_text(raw: Option<&Value>, content: Option<&[ToolCallContent]>) -> String {
    match raw {
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
        None => content
            .unwrap_or_default()
            .iter()
            .filter_map(|block| match block {
                ToolCallContent::Content(c) => match &c.content {
                    ContentBlock::Text(t) => Some(t.text.clone()),
                    _ => None,
                },
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

fn clip_text(text: &str) -> String {
    if text.chars().count() <= OUTPUT_LIMIT {
        text.to_string()
    } else {
        format!("{}…", text.chars().take(OUTPUT_LIMIT).collect::<String>())
    }
}

fn clip_value(value: Value) -> Value {
    if value.to_string().chars().count() <= OUTPUT_LIMIT {
        value
    } else {
        Value::String(clip_text(&value.to_string()))
    }
}

#[cfg(test)]
mod tests {
    //! Frames below are in the wire shape of ACP v1 (camelCase, `sessionUpdate` and `type` tags), the
    //! same bytes a live agent sends. They are parsed exactly as the connection parses them.
    use super::*;
    use agent_client_protocol::v1::schema::{PermissionOption, RequestPermissionRequest};
    use serde_json::from_value;

    fn note(update: Value) -> SessionNotification {
        from_value(json!({ "sessionId": "s1", "update": update })).expect("session/update frame")
    }

    fn permission(tool_call: Value, options: Value) -> RequestPermissionRequest {
        from_value(json!({ "sessionId": "s1", "toolCall": tool_call, "options": options })).expect("permission frame")
    }

    fn option(id: &str, kind: &str) -> PermissionOption {
        from_value(json!({ "optionId": id, "name": id, "kind": kind })).expect("option")
    }

    fn permission_update(fields: Value) -> ToolCallUpdate {
        permission(fields, json!([])).tool_call
    }

    #[test]
    fn agent_text_chunk_becomes_text_delta() {
        let frame = note(json!({
            "sessionUpdate": "agent_message_chunk",
            "content": { "type": "text", "text": "Found two sections." }
        }));
        let events = update_events(&frame, &mut CostTracker::default());
        assert_eq!(events, vec![Event::TextDelta { text: "Found two sections.".into() }]);
    }

    #[test]
    fn empty_text_chunk_produces_no_event() {
        let frame = note(json!({ "sessionUpdate": "agent_message_chunk", "content": { "type": "text", "text": "" } }));
        assert!(update_events(&frame, &mut CostTracker::default()).is_empty());
    }

    #[test]
    fn tool_call_then_completed_update_become_call_and_result() {
        let mut cost = CostTracker::default();
        let call = note(json!({
            "sessionUpdate": "tool_call", "toolCallId": "c1", "title": "printf ok > findings.md",
            "kind": "execute", "status": "in_progress", "rawInput": { "command": "printf ok > findings.md" }
        }));
        let done = note(json!({
            "sessionUpdate": "tool_call_update", "toolCallId": "c1", "status": "completed", "rawOutput": "exit 0"
        }));
        let events: Vec<Event> = [call, done].iter().flat_map(|n| update_events(n, &mut cost)).collect();
        assert_eq!(
            events,
            vec![
                Event::ToolCall {
                    id: "c1".into(),
                    name: "printf ok > findings.md".into(),
                    input: json!({ "command": "printf ok > findings.md" }),
                    kind: "execute".into(),
                    status: "in_progress".into(),
                },
                Event::ToolResult { id: "c1".into(), output: "exit 0".into(), status: "completed".into() },
            ]
        );
    }

    #[test]
    fn completed_update_without_raw_output_carries_its_text_content() {
        let frame = note(json!({
            "sessionUpdate": "tool_call_update", "toolCallId": "c10", "status": "completed",
            "content": [{ "type": "content", "content": { "type": "text", "text": "/usr/bin/ilvuno\nPATH=/x" } }]
        }));
        let events = update_events(&frame, &mut CostTracker::default());
        assert_eq!(
            events,
            vec![Event::ToolResult {
                id: "c10".into(),
                output: "/usr/bin/ilvuno\nPATH=/x".into(),
                status: "completed".into()
            }]
        );
    }

    #[test]
    fn in_progress_update_stays_raw_and_is_not_a_result() {
        let frame = note(json!({ "sessionUpdate": "tool_call_update", "toolCallId": "c2", "status": "in_progress" }));
        let events = update_events(&frame, &mut CostTracker::default());
        assert!(matches!(events.as_slice(), [Event::Raw { .. }]), "{events:?}");
    }

    #[test]
    fn usage_cost_is_reported_as_a_delta_of_the_running_total() {
        let mut cost = CostTracker::default();
        let first = note(json!({ "sessionUpdate": "usage_update", "used": 1000, "size": 258400,
                                 "cost": { "amount": 0.01, "currency": "USD" } }));
        let second = note(json!({ "sessionUpdate": "usage_update", "used": 2000, "size": 258400,
                                  "cost": { "amount": 0.03, "currency": "USD" } }));
        let costs: Vec<Option<f64>> = [first, second]
            .iter()
            .map(|n| match update_events(n, &mut cost).as_slice() {
                [Event::Usage { cost_usd, context_used, .. }] => {
                    assert!(context_used.is_some());
                    *cost_usd
                }
                other => panic!("expected one usage event, got {other:?}"),
            })
            .collect();
        assert!((costs[0].unwrap() - 0.01).abs() < 1e-9);
        assert!((costs[1].unwrap() - 0.02).abs() < 1e-9, "delta, not total: {costs:?}");
        assert!((cost.spent() - 0.03).abs() < 1e-9);
    }

    #[test]
    fn non_usd_cost_is_not_summed_into_the_cap() {
        let mut cost = CostTracker::default();
        let frame = note(json!({ "sessionUpdate": "usage_update", "used": 10, "size": 100,
                                 "cost": { "amount": 5.0, "currency": "EUR" } }));
        let events = update_events(&frame, &mut cost);
        assert!(matches!(events.as_slice(), [Event::Usage { cost_usd: None, .. }]), "{events:?}");
        assert_eq!(cost.spent(), 0.0);
    }

    #[test]
    fn shell_command_wrapper_is_unwrapped_for_the_policy() {
        let (tool, input) = translate_tool(&permission_update(json!({
            "toolCallId": "c3", "kind": "execute", "title": "bash -lc ...",
            "rawInput": { "command": ["bash", "-lc", "ilvuno report --task t1 --status ok --summary done"] }
        })));
        assert_eq!(tool, "Bash");
        assert_eq!(input, json!({ "command": "ilvuno report --task t1 --status ok --summary done" }));
    }

    #[test]
    fn plain_argv_command_is_joined_for_the_policy_to_judge() {
        let (tool, input) = translate_tool(&permission_update(json!({
            "toolCallId": "c4", "kind": "execute", "rawInput": { "command": ["cp", "secret", "/tmp/x"] }
        })));
        assert_eq!((tool.as_str(), input), ("Bash", json!({ "command": "cp secret /tmp/x" })));
    }

    #[test]
    fn single_file_edit_maps_to_edit_with_its_path() {
        let (tool, input) = translate_tool(&permission_update(json!({
            "toolCallId": "c5", "kind": "edit", "locations": [{ "path": "/w/t1/findings.md" }]
        })));
        assert_eq!((tool.as_str(), input), ("Edit", json!({ "file_path": "/w/t1/findings.md" })));
    }

    #[test]
    fn multi_file_edit_is_named_so_the_policy_denies_it() {
        let (tool, _) = translate_tool(&permission_update(json!({
            "toolCallId": "c6", "kind": "edit",
            "locations": [{ "path": "/w/t1/findings.md" }, { "path": "/etc/passwd" }]
        })));
        assert_eq!(tool, "multi-file change");
    }

    #[test]
    fn delete_and_move_keep_their_own_names_so_the_policy_denies_them() {
        let (delete, _) = translate_tool(&permission_update(json!({ "toolCallId": "c7", "kind": "delete" })));
        let (moved, _) = translate_tool(&permission_update(json!({ "toolCallId": "c8", "kind": "move" })));
        assert_eq!((delete.as_str(), moved.as_str()), ("delete", "move"));
    }

    #[test]
    fn read_maps_to_read_with_one_path() {
        let (tool, input) = translate_tool(&permission_update(json!({
            "toolCallId": "c9", "kind": "read", "locations": [{ "path": "/src/09-spike.md" }]
        })));
        assert_eq!((tool.as_str(), input), ("Read", json!({ "file_path": "/src/09-spike.md" })));
    }

    #[test]
    fn allow_picks_allow_once_even_when_allow_always_is_listed_first() {
        let options = [option("always", "allow_always"), option("once", "allow_once"), option("no", "reject_once")];
        let chosen = select_option(&options, &Decision::Allow { updated_input: Value::Null });
        assert_eq!(chosen.map(|id| id.to_string()), Some("once".to_string()));
    }

    #[test]
    fn deny_picks_reject_once_and_never_an_allow_option() {
        let options = [option("yes", "allow_once"), option("nope", "reject_once")];
        let chosen = select_option(&options, &Decision::Deny { message: "outside scope".into() });
        assert_eq!(chosen.map(|id| id.to_string()), Some("nope".to_string()));
    }

    #[test]
    fn no_matching_option_means_cancel() {
        let options = [option("yes", "allow_once")];
        assert!(select_option(&options, &Decision::Deny { message: "x".into() }).is_none());
    }

    #[test]
    fn cancelled_stop_reason_is_an_interrupted_turn() {
        let response: PromptResponse = from_value(json!({ "stopReason": "cancelled" })).expect("response");
        let events = turn_finished(&response, false);
        assert_eq!(
            events,
            vec![Event::TurnFinished { stop_reason: Some("cancelled".into()), interrupted: true, error: None }]
        );
    }

    #[test]
    fn end_turn_after_our_cancel_is_still_interrupted() {
        let response: PromptResponse = from_value(json!({ "stopReason": "end_turn" })).expect("response");
        let events = turn_finished(&response, true);
        assert!(matches!(events.as_slice(), [Event::TurnFinished { interrupted: true, .. }]));
    }
}

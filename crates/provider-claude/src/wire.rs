//! Pure translation of one stream-json frame into normalized events. No I/O here, so it is unit-tested
//! against frames captured from the real CLI (research/09-spike.md §1–2).

use provider_core::{Capabilities, Event};
use serde_json::{json, Value};

const TOOL_OUTPUT_LIMIT: usize = 4000;

pub fn claude_capabilities() -> Capabilities {
    Capabilities {
        resume: true,
        fork: false,
        rewind: false,
        interrupt: true,
        steer: false,
        approvals: true,
        structured_output: true,
        usage: true,
        models: Vec::new(),
    }
}

/// Translate one decoded frame. Unmodelled frames become `Raw`; control acknowledgements are dropped.
pub fn parse_frame(frame: &Value) -> Vec<Event> {
    match frame.get("type").and_then(Value::as_str) {
        Some("system") => parse_system(frame),
        Some("assistant") => parse_assistant(frame),
        Some("user") => parse_user(frame),
        Some("control_request") => parse_control_request(frame),
        Some("control_response") => Vec::new(),
        Some("result") => parse_result(frame),
        _ => vec![Event::Raw { payload: frame.clone() }],
    }
}

fn parse_system(frame: &Value) -> Vec<Event> {
    match (frame.get("subtype").and_then(Value::as_str), frame.get("session_id").and_then(Value::as_str)) {
        (Some("init"), Some(id)) => {
            vec![Event::Session { native_id: id.to_string(), capabilities: claude_capabilities() }]
        }
        _ => vec![Event::Raw { payload: frame.clone() }],
    }
}

fn content_blocks(frame: &Value) -> &[Value] {
    frame
        .pointer("/message/content")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn parse_assistant(frame: &Value) -> Vec<Event> {
    content_blocks(frame)
        .iter()
        .filter_map(|block| match block.get("type").and_then(Value::as_str)? {
            "text" => Some(Event::TextDelta { text: str_field(block, "text") }),
            "thinking" => Some(Event::ThinkingDelta { text: str_field(block, "thinking") }),
            "tool_use" => Some(Event::ToolCall {
                id: str_field(block, "id"),
                name: str_field(block, "name"),
                input: block.get("input").cloned().unwrap_or(Value::Null),
                kind: "tool".to_string(),
                status: "requested".to_string(),
            }),
            _ => None,
        })
        .collect()
}

fn parse_user(frame: &Value) -> Vec<Event> {
    content_blocks(frame)
        .iter()
        .filter(|block| block.get("type").and_then(Value::as_str) == Some("tool_result"))
        .map(|block| {
            let is_error = block.get("is_error").and_then(Value::as_bool).unwrap_or(false);
            Event::ToolResult {
                id: str_field(block, "tool_use_id"),
                output: truncate(&tool_output_text(block.get("content")), TOOL_OUTPUT_LIMIT),
                status: if is_error { "error" } else { "ok" }.to_string(),
            }
        })
        .collect()
}

fn parse_control_request(frame: &Value) -> Vec<Event> {
    let request = frame.get("request").cloned().unwrap_or(Value::Null);
    if request.get("subtype").and_then(Value::as_str) != Some("can_use_tool") {
        return vec![Event::Raw { payload: frame.clone() }];
    }
    vec![Event::PermissionRequest {
        request_id: str_field(frame, "request_id"),
        tool: str_field(&request, "tool_name"),
        input: request.get("input").cloned().unwrap_or(Value::Null),
        options: vec!["allow".to_string(), "deny".to_string()],
    }]
}

fn parse_result(frame: &Value) -> Vec<Event> {
    let usage = frame.get("usage").cloned().unwrap_or(Value::Null);
    let fresh_in = u64_field(&usage, "input_tokens") + u64_field(&usage, "cache_creation_input_tokens");
    let is_error = frame.get("is_error").and_then(Value::as_bool).unwrap_or(false);
    vec![
        Event::Usage {
            tokens_in: fresh_in,
            tokens_out: u64_field(&usage, "output_tokens"),
            cache_read_tokens: u64_field(&usage, "cache_read_input_tokens"),
            context_used: None,
            cost_usd: frame.get("total_cost_usd").and_then(Value::as_f64),
        },
        Event::TurnFinished {
            stop_reason: frame.get("stop_reason").and_then(Value::as_str).map(str::to_string),
            interrupted: false,
            error: is_error.then(|| str_field(frame, "subtype")),
        },
    ]
}

/// The worker-side permission reply frame. `Allow` carries `updatedInput`; `Deny` carries a message.
pub fn permission_reply_frame(request_id: &str, decision: &provider_core::Decision) -> Value {
    let body = match decision {
        provider_core::Decision::Allow { updated_input } => {
            json!({ "behavior": "allow", "updatedInput": updated_input })
        }
        provider_core::Decision::Deny { message } => json!({ "behavior": "deny", "message": message }),
    };
    json!({
        "type": "control_response",
        "response": { "subtype": "success", "request_id": request_id, "response": body }
    })
}

pub fn initialize_frame(request_id: &str) -> Value {
    json!({ "type": "control_request", "request_id": request_id, "request": { "subtype": "initialize" } })
}

pub fn interrupt_frame(request_id: &str) -> Value {
    json!({ "type": "control_request", "request_id": request_id, "request": { "subtype": "interrupt" } })
}

pub fn user_frame(session_id: &str, text: &str) -> Value {
    json!({
        "type": "user",
        "message": { "role": "user", "content": text },
        "parent_tool_use_id": null,
        "session_id": session_id,
    })
}

fn str_field(v: &Value, key: &str) -> String {
    v.get(key).and_then(Value::as_str).unwrap_or_default().to_string()
}

fn u64_field(v: &Value, key: &str) -> u64 {
    v.get(key).and_then(Value::as_u64).unwrap_or(0)
}

fn tool_output_text(content: Option<&Value>) -> String {
    match content {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|i| i.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

fn truncate(s: &str, limit: usize) -> String {
    if s.chars().count() <= limit {
        return s.to_string();
    }
    s.chars().take(limit).collect::<String>() + "…"
}

#[cfg(test)]
mod tests {
    use super::*;
    use provider_core::Decision;

    #[test]
    fn system_init_yields_native_session_id() {
        let f = json!({"type":"system","subtype":"init","session_id":"43688eaf-01fe-40b4-85ee-3f6cf251dee2"});
        let ev = parse_frame(&f);
        assert!(matches!(&ev[0], Event::Session { native_id, .. } if native_id.starts_with("43688eaf")));
    }

    #[test]
    fn assistant_blocks_become_text_thinking_and_tool_calls() {
        let f = json!({"type":"assistant","message":{"content":[
            {"type":"thinking","thinking":"hmm"},
            {"type":"text","text":"hello"},
            {"type":"tool_use","id":"tu_1","name":"Write","input":{"file_path":"/x","content":"ok"}}
        ]}});
        let ev = parse_frame(&f);
        assert_eq!(ev.len(), 3);
        assert!(matches!(&ev[1], Event::TextDelta { text } if text == "hello"));
        assert!(matches!(&ev[2], Event::ToolCall { name, .. } if name == "Write"));
    }

    #[test]
    fn tool_result_is_truncated_and_marked_error() {
        let big = "x".repeat(5000);
        let f = json!({"type":"user","message":{"content":[
            {"type":"tool_result","tool_use_id":"tu_1","content":big,"is_error":true}]}});
        match &parse_frame(&f)[0] {
            Event::ToolResult { status, output, .. } => {
                assert_eq!(status, "error");
                assert_eq!(output.chars().count(), TOOL_OUTPUT_LIMIT + 1);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn can_use_tool_becomes_permission_request_keyed_by_request_id() {
        let f = json!({"type":"control_request","request_id":"req-9","request":{
            "subtype":"can_use_tool","tool_name":"Bash","input":{"command":"ls"}}});
        match &parse_frame(&f)[0] {
            Event::PermissionRequest { request_id, tool, .. } => {
                assert_eq!(request_id, "req-9");
                assert_eq!(tool, "Bash");
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn result_maps_usage_and_turn_end_and_skips_cache_reads_from_fresh_input() {
        let f = json!({"type":"result","subtype":"success","is_error":false,"stop_reason":"end_turn",
            "total_cost_usd":0.0132,"usage":{"input_tokens":6,"cache_creation_input_tokens":61088,
            "cache_read_input_tokens":157707,"output_tokens":840}});
        let ev = parse_frame(&f);
        match &ev[0] {
            Event::Usage { tokens_in, tokens_out, cache_read_tokens, cost_usd, .. } => {
                assert_eq!(*tokens_in, 61094);
                assert_eq!(*tokens_out, 840);
                assert_eq!(*cache_read_tokens, 157707);
                assert_eq!(*cost_usd, Some(0.0132));
            }
            other => panic!("unexpected {other:?}"),
        }
        assert!(matches!(&ev[1], Event::TurnFinished { error: None, .. }));
    }

    #[test]
    fn initialize_frame_has_required_subtype() {
        let f = initialize_frame("init-1");
        assert_eq!(f["request"]["subtype"], "initialize");
        assert_eq!(f["type"], "control_request");
    }

    #[test]
    fn allow_reply_carries_updated_input_and_deny_carries_message() {
        let allow = permission_reply_frame("r1", &Decision::Allow { updated_input: json!({"a":1}) });
        assert_eq!(allow["response"]["request_id"], "r1");
        assert_eq!(allow["response"]["response"]["behavior"], "allow");
        assert_eq!(allow["response"]["response"]["updatedInput"]["a"], 1);
        let deny = permission_reply_frame("r2", &Decision::Deny { message: "no".into() });
        assert_eq!(deny["response"]["response"]["behavior"], "deny");
        assert_eq!(deny["response"]["response"]["message"], "no");
    }
}

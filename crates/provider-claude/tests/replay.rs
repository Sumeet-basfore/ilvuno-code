use provider_claude::wire;
use provider_core::Event;
use serde_json::Value;
use std::{fs, path::Path};

fn replay_fixture(name: &str) -> Vec<Event> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name);
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .flat_map(|line| {
            let frame: Value = serde_json::from_str(line).expect("fixture line is valid JSON");
            wire::parse_frame(&frame)
        })
        .collect()
}

#[test]
fn normal_turn_replays_session_tool_result_finish_and_unknown_frame() {
    let events = replay_fixture("normal-turn.jsonl");

    assert_eq!(events.len(), 7);
    assert!(matches!(&events[0], Event::Session { native_id, .. } if native_id == "scrubbed-session-id"));
    assert!(matches!(&events[1], Event::TextDelta { text } if text == "I will check the directory."));
    assert!(matches!(
        &events[2],
        Event::ToolCall { id, name, input, .. }
            if id == "scrubbed-tool-use-id" && name == "Bash" && input["command"] == "pwd"
    ));
    assert!(matches!(
        &events[3],
        Event::ToolResult { id, output, status }
            if id == "scrubbed-tool-use-id" && output == "/project" && status == "ok"
    ));
    assert!(matches!(
        &events[4],
        Event::Usage { tokens_in: 12, tokens_out: 4, .. }
    ));
    assert!(matches!(
        &events[5],
        Event::TurnFinished { stop_reason: Some(reason), interrupted: false, error: None }
            if reason == "end_turn"
    ));
    assert!(matches!(
        &events[6],
        Event::Raw { payload } if payload["type"] == "something_new"
    ));
}

#[test]
fn permission_prompt_replays_tool_and_options() {
    let events = replay_fixture("permission-prompt.jsonl");

    assert_eq!(events.len(), 1);
    assert!(matches!(
        &events[0],
        Event::PermissionRequest { request_id, tool, input, options }
            if request_id == "scrubbed-request-id"
                && tool == "Bash"
                && input["command"] == "pwd"
                && options == &["allow", "deny"]
    ));
}

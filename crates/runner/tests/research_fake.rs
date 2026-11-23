//! M3 end-to-end on the scripted provider: zero quota. The fake agent plays the worker by calling the
//! same `run_report` path the real `ilvuno report` command uses.

use provider_core::fake::FakeProvider;
use provider_core::{Decision, Event};
use runner::commands::{self, ReportStatus};
use runner::driver::{Driver, RunEnd};
use runner::goal::GoalPlan;
use runner::replay;
use runner::report::run_report;
use runner::run_status::{self, GateDecision};
use runner::store::Outcome;
use runner::{Home, RunStatus, Store};
use std::collections::{BTreeMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

const GOAL: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/research.json");

struct Env {
    dir: PathBuf,
    home: Home,
    store: Store,
    run_id: String,
    provider: FakeProvider,
    /// `true` = write a citation that resolves; `false` = cite a section that does not exist.
    briefs: Arc<Mutex<VecDeque<bool>>>,
}

impl Drop for Env {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn create_run(store: &mut Store, goal_path: &Path) -> String {
    let plan = GoalPlan::load(goal_path).expect("goal");
    let raw = std::fs::read_to_string(goal_path).expect("raw");
    let base = goal_path.parent().unwrap().to_path_buf();
    let new_goal = plan.new_goal();
    match store
        .transact("run.create", None, |tx, p| {
            let id = commands::create_run(tx, p, &new_goal)?;
            runner::goal::snapshot(tx, &id, &raw, &base)?;
            Ok(Outcome::Applied(id))
        })
        .expect("create")
    {
        Outcome::Applied(id) => id,
        other => panic!("{other:?}"),
    }
}

/// Every provider id the goal names maps to the same scripted provider (clones share one script).
fn instances(provider: &FakeProvider, plan: &GoalPlan) -> BTreeMap<String, FakeProvider> {
    plan.providers().iter().map(|(id, _)| (id.clone(), provider.clone())).collect()
}

fn setup(name: &str) -> Env {
    let dir = std::env::temp_dir().join(format!("ilvuno-e2e-{name}-{}", uuid::Uuid::new_v4()));
    let home = Home::new(&dir);
    let mut store = Store::open(&home.db_path()).expect("store");
    let plan = GoalPlan::load(Path::new(GOAL)).expect("goal");
    let run_id = create_run(&mut store, Path::new(GOAL));
    let provider = FakeProvider::new();
    let briefs = Arc::new(Mutex::new(VecDeque::new()));
    provider.set_prompt_hook(agent(home.clone(), briefs.clone(), plan.file.required_sections.clone()));
    Env { dir, home, store, run_id, provider, briefs }
}

/// The scripted worker. It writes its file, then reports through `run_report`.
fn agent(home: Home, briefs: Arc<Mutex<VecDeque<bool>>>, required: Vec<u32>) -> Box<dyn FnMut(&str) + Send> {
    Box::new(move |prompt: &str| {
        let task_id = between(prompt, "Your task id is ", ".");
        let worktree = PathBuf::from(line_after(prompt, "Work only inside your worktree: ").trim_end_matches('.'));
        std::fs::create_dir_all(&worktree).expect("worktree");
        if prompt.contains("Write `brief.html`") {
            let good = briefs.lock().unwrap().pop_front().unwrap_or(true);
            std::fs::write(worktree.join("brief.html"), brief(&required, good)).expect("brief");
            run_report(&home, &task_id, ReportStatus::Ok, "wrote the brief", &["brief.html".to_string()]).expect("report");
        } else {
            let nums = between(prompt, "Read sections ", " of the source");
            let mut body = String::new();
            for n in nums.split(',').filter_map(|s| s.trim().parse::<u32>().ok()) {
                body.push_str(&format!("- [§{n}] finding for section {n}\n"));
            }
            std::fs::write(worktree.join("findings.md"), body).expect("findings");
            run_report(&home, &task_id, ReportStatus::Ok, "three sentences of findings", &["findings.md".to_string()])
                .expect("report");
        }
    })
}

fn brief(required: &[u32], good: bool) -> String {
    let mut html = String::from("<!doctype html><html><head><meta charset=\"utf-8\"><title>Brief</title></head><body>");
    for n in required {
        let target = if good { *n } else { 9 };
        html.push_str(&format!(
            "<section><h2>Section {n}</h2><p>Summary. <cite data-src=\"09-spike.md\" data-section=\"{target}\">§{target}</cite></p></section>"
        ));
    }
    html.push_str("</body></html>");
    html
}

fn between(text: &str, start: &str, end: &str) -> String {
    let from = text.find(start).expect("start marker") + start.len();
    let to = text[from..].find(end).expect("end marker") + from;
    text[from..to].to_string()
}

fn line_after(text: &str, marker: &str) -> String {
    let from = text.find(marker).expect("marker") + marker.len();
    text[from..].lines().next().unwrap_or_default().to_string()
}

fn usage() -> Event {
    Event::Usage { tokens_in: 900, tokens_out: 150, cache_read_tokens: 0, context_used: None, cost_usd: Some(0.01) }
}

fn finished() -> Event {
    Event::TurnFinished { stop_reason: Some("end_turn".into()), interrupted: false, error: None }
}

/// Researcher turn; the first one also tries to write outside its worktree.
fn researcher_turn(with_escape_attempt: bool) -> Vec<Event> {
    let mut events = Vec::new();
    if with_escape_attempt {
        events.push(Event::PermissionRequest {
            request_id: "perm-escape".into(),
            tool: "Write".into(),
            input: serde_json::json!({"file_path": "/etc/ilvuno-escape-test", "content": "x"}),
            options: vec!["allow".into(), "deny".into()],
        });
    }
    events.push(usage());
    events.push(finished());
    events
}

fn synth_turn() -> Vec<Event> {
    vec![usage(), finished()]
}

async fn drive_run(env: &mut Env) -> RunEnd {
    let plan = GoalPlan::load(Path::new(GOAL)).expect("goal");
    let run_id = env.run_id.clone();
    let providers = instances(&env.provider, &plan);
    let home = env.home.clone();
    let driver = Driver { providers: &providers, home: &home, ilvuno_bin_dir: None, max_usd: 0.2 };
    driver.drive(&mut env.store, &plan, &run_id).await.expect("drive")
}

async fn send_back(env: &mut Env, note: &str) -> RunEnd {
    let run_id = env.run_id.clone();
    let decision = GateDecision::SendBack { note: note.to_string() };
    env.store
        .transact("gate.decide", None, |tx, p| run_status::gate_decide(tx, p, &run_id, &decision))
        .expect("gate");
    drive_run(env).await
}

fn count(store: &Store, sql: &str) -> i64 {
    store.conn().query_row(sql, [], |r| r.get(0)).expect("count")
}

fn status(store: &Store, run: &str) -> String {
    store.conn().query_row("SELECT status FROM runs WHERE id = ?1", [run], |r| r.get(0)).expect("status")
}

fn queue_round0(env: &Env, synth_brief_good: bool) {
    env.provider.push_turn(researcher_turn(true));
    env.provider.push_turn(researcher_turn(false));
    env.provider.push_turn(synth_turn());
    env.briefs.lock().unwrap().push_back(synth_brief_good);
}

#[tokio::test]
async fn happy_path_reaches_gate_then_accepts_and_replays() {
    let mut env = setup("happy");
    queue_round0(&env, true);
    let end = drive_run(&mut env).await;
    assert_eq!(end, RunEnd::AtGate);
    assert_eq!(status(&env.store, &env.run_id), "reviewed");

    let snapshot = env.home.artifacts_dir(&env.run_id).join("rounds/r0/synthesizer/brief.html");
    let html = std::fs::read_to_string(&snapshot).expect("brief snapshot");
    assert!(html.contains("data-section=\"1\"") && html.contains("data-section=\"4\""));
    assert!(env.home.artifacts_dir(&env.run_id).join("brief.html").exists());
    assert_eq!(count(&env.store, "SELECT COUNT(*) FROM artifacts WHERE kind = 'report'"), 1);
    assert_eq!(
        count(&env.store, "SELECT COUNT(*) FROM artifacts"),
        count(&env.store, "SELECT COUNT(DISTINCT path) FROM artifacts"),
        "two artifacts must never share a path"
    );

    let escape = env.provider.decisions();
    assert!(
        escape.iter().any(|(id, d)| id == "perm-escape" && matches!(d, Decision::Deny { .. })),
        "write outside the worktree must be denied: {escape:?}"
    );

    let run_id = env.run_id.clone();
    let accept = env
        .store
        .transact("gate.decide", None, |tx, p| run_status::gate_decide(tx, p, &run_id, &GateDecision::Accept))
        .expect("accept");
    assert_eq!(accept, Outcome::Applied(()));
    assert_eq!(status(&env.store, &env.run_id), RunStatus::Accepted.as_str());

    let replayed = replay::replay(env.store.conn(), &env.run_id).expect("replay");
    assert_eq!(replay::compare_with_projection(env.store.conn(), &env.run_id, &replayed).expect("cmp"), Vec::<String>::new());
    assert_eq!(replayed.rounds.len(), 1);
    assert_eq!(replayed.rounds[0].verdict.as_deref(), Some("pass"));
    assert_eq!(replayed.final_status.as_deref(), Some("accepted"));
}

#[tokio::test]
async fn failed_citation_blocks_round_and_revision_round_passes() {
    let mut env = setup("revise");
    queue_round0(&env, false);
    env.provider.push_turn(synth_turn());
    env.briefs.lock().unwrap().push_back(true);
    let end = drive_run(&mut env).await;
    assert_eq!(end, RunEnd::AtGate);

    let prompts = env.provider.prompts();
    let revision = prompts.last().expect("revision prompt");
    assert!(revision.contains("check section-citations-resolve failed"), "notes handed to synthesizer: {revision}");
    let replayed = replay::replay(env.store.conn(), &env.run_id).expect("replay");
    let verdicts: Vec<_> = replayed.rounds.iter().map(|r| r.verdict.clone()).collect();
    assert_eq!(verdicts, vec![Some("block".to_string()), Some("pass".to_string())]);
    assert_eq!(replay::compare_with_projection(env.store.conn(), &env.run_id, &replayed).expect("cmp"), Vec::<String>::new());
}

#[tokio::test]
async fn identical_failures_twice_stop_as_no_progress() {
    let mut env = setup("stall");
    queue_round0(&env, false);
    env.provider.push_turn(synth_turn());
    env.briefs.lock().unwrap().push_back(false);
    let end = drive_run(&mut env).await;
    assert_eq!(end, RunEnd::Stopped { status: RunStatus::NoProgress, reason: "same failing checks two rounds running".into() });
}

#[tokio::test]
async fn fourth_round_is_refused_with_a_reason_and_work_is_kept() {
    let mut env = setup("fourth");
    queue_round0(&env, true);
    assert_eq!(drive_run(&mut env).await, RunEnd::AtGate);

    env.provider.push_turn(synth_turn());
    assert_eq!(send_back(&mut env, "tighten section 2").await, RunEnd::AtGate);
    env.provider.push_turn(synth_turn());
    assert_eq!(send_back(&mut env, "tighten section 3").await, RunEnd::AtGate);

    // Third send-back would need round index 3 of max 3: refused.
    let end = send_back(&mut env, "one more time").await;
    match end {
        RunEnd::Stopped { status, reason } => {
            assert_eq!(status, RunStatus::Blocked);
            assert!(reason.contains("3 of 3"), "reason: {reason}");
        }
        other => panic!("expected refusal, got {other:?}"),
    }
    assert_eq!(count(&env.store, "SELECT COUNT(*) FROM rounds"), 3);
    assert!(env.home.artifacts_dir(&env.run_id).join("rounds/r0/synthesizer/brief.html").exists());
    assert!(env.home.artifacts_dir(&env.run_id).join("rounds/r2/synthesizer/brief.html").exists());
    assert!(env.home.threads_dir(&env.run_id).join("r0-researcher-a").exists());
    let replayed = replay::replay(env.store.conn(), &env.run_id).expect("replay");
    assert!(replayed.refusals.iter().any(|r| r.contains("3 of 3")), "{:?}", replayed.refusals);
    assert_eq!(replay::compare_with_projection(env.store.conn(), &env.run_id, &replayed).expect("cmp"), Vec::<String>::new());
}

#[tokio::test]
async fn resume_continues_from_last_committed_row_not_from_zero() {
    let mut env = setup("resume");
    // Simulate a crash: researcher-a finished and committed; researcher-b was running when it died.
    let plan = GoalPlan::load(Path::new(GOAL)).expect("goal");
    let run_id = env.run_id.clone();
    let slots = runner::research::round_slots(&plan, 0, &[]);
    let root = env.home.threads_dir(&run_id);
    let round = match env
        .store
        .transact("round.plan", None, |tx, p| commands::plan_round(tx, p, &run_id, &slots, &root))
        .expect("plan")
    {
        Outcome::Applied(r) => r,
        other => panic!("{other:?}"),
    };
    let a = round.dispatches.iter().find(|d| d.slot == "researcher-a").unwrap().dispatch_id.clone();
    let b = round.dispatches.iter().find(|d| d.slot == "researcher-b").unwrap().dispatch_id.clone();
    env.store
        .transact("dispatch.start", None, |tx, p| commands::start_dispatch(tx, p, &run_id, &a, "task a"))
        .expect("start a");
    let fa = env.store.conn().query_row("SELECT id FROM dispatch WHERE id = ?1", [&a], |r| r.get::<_, String>(0)).unwrap();
    assert_eq!(fa, a);
    run_report(&env.home, &a, ReportStatus::Ok, "a done", &["findings.md".to_string()]).expect("a reported");
    env.store
        .transact("dispatch.start", None, |tx, p| commands::start_dispatch(tx, p, &run_id, &b, "task b"))
        .expect("start b");
    // Writes for b's file exist, but b never reported. Crash now.
    std::fs::create_dir_all(env.home.threads_dir(&run_id).join("r0-researcher-b")).unwrap();

    // Restart: the new run loop must not re-run researcher-a.
    env.provider.push_turn(researcher_turn(false)); // b
    env.provider.push_turn(synth_turn());
    env.briefs.lock().unwrap().push_back(true);
    let end = drive_run(&mut env).await;
    assert_eq!(end, RunEnd::AtGate);
    let prompts = env.provider.prompts();
    assert_eq!(prompts.iter().filter(|p| p.contains("Read sections 1, 2 of")).count(), 0, "researcher-a re-ran");
    assert_eq!(prompts.iter().filter(|p| p.contains("Read sections 3, 4 of")).count(), 1);
    let replayed = replay::replay(env.store.conn(), &env.run_id).expect("replay");
    assert!(replayed.transitions.iter().any(|t| t.contains("dispatch.requeued")));
    assert_eq!(replay::compare_with_projection(env.store.conn(), &env.run_id, &replayed).expect("cmp"), Vec::<String>::new());
}

#[tokio::test]
async fn each_slot_runs_on_the_provider_its_assignment_names() {
    let dir = std::env::temp_dir().join(format!("ilvuno-e2e-routing-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).expect("dir");
    let source = std::fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research/09-spike.md"))
        .expect("source");
    let goal = serde_json::json!({
        "title": "routing", "goal_text": "route each slot", "template": "research",
        "source": source.to_string_lossy(),
        "groups": [{"slot": "researcher-a", "sections": [1, 2]}, {"slot": "researcher-b", "sections": [3, 4]}],
        "required_sections": [1, 2, 3, 4],
        "checks": ["section-citations-resolve", "required-sections-cited", "no-placeholders"],
        "budget": {"rounds": 2, "tokens": 600000},
        "providers": {"fake-a": {"driver": "claude"}, "fake-b": {"driver": "claude"}},
        "assignments": {"researcher-a": "fake-a", "researcher-b": "fake-b", "synthesizer": "fake-b"}
    });
    let goal_path = dir.join("goal.json");
    std::fs::write(&goal_path, goal.to_string()).expect("goal file");
    let home = Home::new(dir.join("home"));
    let mut store = Store::open(&home.db_path()).expect("store");
    let run_id = create_run(&mut store, &goal_path);
    let plan = GoalPlan::load(&goal_path).expect("goal");

    let (fa, fb) = (FakeProvider::new(), FakeProvider::new());
    let briefs = Arc::new(Mutex::new(VecDeque::from([true])));
    for p in [&fa, &fb] {
        p.set_prompt_hook(agent(home.clone(), briefs.clone(), plan.file.required_sections.clone()));
    }
    fa.push_turn(researcher_turn(false)); // researcher-a
    fb.push_turn(researcher_turn(false)); // researcher-b
    fb.push_turn(synth_turn()); // synthesizer
    let providers = BTreeMap::from([("fake-a".to_string(), fa.clone()), ("fake-b".to_string(), fb.clone())]);
    let driver = Driver { providers: &providers, home: &home, ilvuno_bin_dir: None, max_usd: 0.2 };
    assert_eq!(driver.drive(&mut store, &plan, &run_id).await.expect("drive"), RunEnd::AtGate);

    let a_prompts = fa.prompts();
    assert_eq!(a_prompts.len(), 1, "fake-a runs one dispatch only");
    assert!(a_prompts[0].contains("Read sections 1, 2 of"), "fake-a got researcher-a's task");
    let b_prompts = fb.prompts();
    assert_eq!(b_prompts.len(), 2, "fake-b runs researcher-b and the synthesizer");
    assert!(b_prompts[0].contains("Read sections 3, 4 of"));
    assert!(b_prompts[1].contains("Write `brief.html`"));

    let mut stmt = store
        .conn()
        .prepare("SELECT d.slot, t.provider FROM dispatch d JOIN threads t ON t.id = d.thread_id ORDER BY d.rowid")
        .expect("prepare");
    let rows: Vec<(String, String)> =
        stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?))).expect("query").collect::<rusqlite::Result<_>>().expect("rows");
    assert_eq!(
        rows,
        vec![
            ("researcher-a".to_string(), "fake-a".to_string()),
            ("researcher-b".to_string(), "fake-b".to_string()),
            ("synthesizer".to_string(), "fake-b".to_string()),
        ]
    );
    let _ = std::fs::remove_dir_all(&dir);
}

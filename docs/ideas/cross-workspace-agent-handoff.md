# Cross-Workspace Agent Handoff and Feedback Loop

**Status: PARKED IDEA — NOT VALIDATED — NOT APPROVED FOR IMPLEMENTATION**

Recorded 2026-11-20 as a future product idea for later research. This note is not part of
authorized M0/G0 work. It creates no scope change, no new milestone, and no task. Nothing in it
is evidence; every claim below is an unvalidated hypothesis. The owner may discard or reprioritize
it at any time.

---

## 1. Problem statement and observed workflow

**Observed first-hand (owner report, 2026-11-20):** during Ilvuno development the owner plans,
decides, and reviews engineering reports in ChatGPT, while coding agents (Claude Code, Codex,
DeepSeek, and others) run in T3 Code. Prompts are manually copied from ChatGPT into T3 Code, and
agent results are manually copied back into ChatGPT. This loop repeats throughout planning,
implementation, review, and verification.

**Hypothesis (not measured):** this manual copy loop is a recurring, friction-heavy coordination
cost for owners who separate planning from execution across AI tools, and it may also lose context
precisely where review decisions are made.

## 2. Target users and situations

Hypothesized users/situations where the problem may occur:

- Solo maintainers using one assistant for planning/review and a separate harness for coding.
- Owners coordinating several coding agents in parallel (Ilvuno's own core use case).
- Review-and-approve workflows where the approver's environment is not the executor's environment.
- Multi-repo, multi-project owners juggling context across sessions and machines.

Whether these users exist in sufficient numbers is an open market question (§6), not a fact.

## 3. Candidate solution directions (hypotheses)

- **H1 — Handoff packets:** structured, signed, reviewable "handoff" artifacts (task brief,
  constraints, artifact references) that carry a planning-environment prompt into a coding-agent
  workspace and return a progress/artifact report. Hypothesis: structure, not connectivity, is the
  main missing piece.
- **H2 — Progress reporting loop:** a lightweight report format the agent workspace can emit
  (commits, evidence, open questions) that a planning environment can read without a human
  retyping it. Hypothesis: one-way outbound reporting alone already removes most of the cost.
- **H3 — Feedback channel with approval gates:** replies from the planning environment land as
  review comments that require explicit human approval before agents act on them. Hypothesis:
  keeping a human gate avoids most autonomy risks (§5).
- **H4 — Session-portable context:** a serialized decision/context bundle (like Ilvuno's
  agent-context documents) that can be opened in either environment. Hypothesis: the real problem
  is context continuity, not message transport.

## 4. Integration approaches (feasibility research required first)

Each of the following needs its own feasibility study before any work is considered:

- Does ChatGPT offer supported integration mechanisms for external tools/workflows (connectors,
  actions, file attachments, project/file upload APIs)? Which are sanctioned for third-party use?
- What do coding-agent harnesses (T3 Code, Claude Code, Codex CLI, OpenCode, others) expose as
  supported entry points for external prompts and artifacts (files, URLs, MCP, CLI flags)?
- File-over-a-shared-folder as the lowest-tech transport (no API dependency at all).
- Local bridge service vs. hosted service trade-off (Ilvuno's approved local-first stance favors
  local).
- Whether any of this can be a plain documented convention (format + file locations) instead of
  software.

## 5. Risks

- **Security:** any bridge becomes an untrusted-input surface; prompts crossing environments must
  be treated as data, not instructions. A bridge must never carry credentials or tokens.
- **Privacy:** session content in either environment may be confidential; transport and storage
  need explicit consent and clear data-handling rules.
- **Permissions:** an agent acting on planning-environment feedback must stay inside its approved
  scope; handoff must not become a bypass of Ilvuno's write-scope policy (§10.2 of AGENTS.md) or
  its approval gates.
- **Runaway loops:** agent output → planning reply → agent action without a human in the loop can
  self-amplify; any feedback loop needs rate/iteration limits and a hard human-approval boundary.
- **Auditability:** handoffs must leave evidence (what was sent, what was approved, by whom), or
  the verification story Ilvuno is built on is undermined.

## 6. Competition and market-validation questions

- Is the copy-paste loop actually a top pain point for this user profile, or a tolerable cost?
- Which existing tools partially cover it (chat-platform integrations, agent dispatchers, ticket
  bridges, paste-sync utilities)? What do they leave uncovered?
- Would users pay for, or adopt, a convention-only solution over an integrated product?
- How does this interact with Ilvuno's approved positioning: Ilvuno never talks to an agent
  (ADR-0010); agent adapters are post-MVP (D-15). Any handoff feature that touches agents may
  conflict with an approved boundary and needs an owner decision first.

## 7. Open technical questions

- Does ChatGPT (or other planning environments) provide a *supported* integration mechanism at
  all, and under what terms? (Unknown at recording time; must be researched, not assumed.)
- What is the canonical artifact format both sides can consume losslessly (markdown? JSON?
  ticket payloads?)
- How are approvals represented so they are cryptographically or procedurally attributable?
- How does a handoff interact with Ilvuno's candidate/verification model — is a handoff itself a
  verifiable artifact?
- Identity: how do the two environments agree on who approved what?

## 8. Future research and validation checklist

1. Measure the manual loop: count handoffs and time cost over one real project week (owner log).
2. Survey sanctioned integration surfaces of ChatGPT and of the agent harnesses in use.
3. Draft (paper only) a minimal handoff packet format; review it against Ilvuno's invariants
   (INV-1…INV-14) and write-scope policy.
4. Prototype nothing; instead run the format manually for a week and record friction.
5. Competitive scan of at least three partial solutions and their gaps.
6. Owner decision gate: revisit ADR-0010 / D-15 boundaries before any implementation is even
   proposed.

## 9. Non-goals and current status

**Non-goals (while parked):** no implementation, no dependencies, no integrations, no market
research spend, no roadmap commitment, no change to authorized M0/G0 scope, no touching of
production code or approved architecture documents.

**Current status:** idea recorded only. Not validated, not approved for implementation, not
assigned to any task or milestone. Next step, if the owner chooses to pursue it: the research
checklist in §8, starting with measurement of the real manual-loop cost.

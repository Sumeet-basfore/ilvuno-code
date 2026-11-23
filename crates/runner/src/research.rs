//! The hardcoded research plan (M3): researchers per section group, then one synthesizer. Revision
//! rounds re-run only the synthesizer with the notes. Pure text; the driver supplies DB facts.

use crate::commands::SlotSpec;
use crate::goal::GoalPlan;

pub const SYNTH_SLOT: &str = "synthesizer";
pub const FINDINGS_FILE: &str = "findings.md";
pub const BRIEF_FILE: &str = "brief.html";

/// Slots for a round. Round 0 is the full plan; later rounds re-run the synthesizer only.
pub fn round_slots(plan: &GoalPlan, round_index: u32, notes: &[String]) -> Vec<SlotSpec> {
    let mut slots = Vec::new();
    if round_index == 0 {
        for group in &plan.file.groups {
            slots.push(SlotSpec {
                slot: group.slot.clone(),
                provider: plan.provider_for_slot(&group.slot).to_string(),
                role: "worker".into(),
                scope: vec![FINDINGS_FILE.into()],
                intent: format!(
                    "Summarise sections {} of {} into {FINDINGS_FILE}",
                    join_numbers(&group.sections),
                    plan.source_name
                ),
            });
        }
    }
    slots.push(SlotSpec {
        slot: SYNTH_SLOT.into(),
        provider: plan.provider_for_slot(SYNTH_SLOT).to_string(),
        role: "worker".into(),
        scope: vec![BRIEF_FILE.into()],
        intent: if notes.is_empty() {
            format!("Write {BRIEF_FILE}: one cited section per source section {}", join_numbers(&plan.file.required_sections))
        } else {
            format!("Revise {BRIEF_FILE} to address the notes from the previous round")
        },
    });
    slots
}

pub fn researcher_task(plan: &GoalPlan, sections: &[u32]) -> String {
    format!(
        "Read sections {nums} of the source file at this absolute path (use the Read tool): {source}\n\
         Write your findings into `{FINDINGS_FILE}` in your worktree: one markdown bullet per finding, each \
         starting with `[§N]` (N = the source section) and quoting the fact it relies on. Keep it to the \
         sections assigned to you. Then report once with the report command, listing `{FINDINGS_FILE}` in --files.",
        nums = join_numbers(sections),
        source = plan.source_abs.display(),
    )
}

/// One finding file handed from a researcher to the synthesizer.
pub struct Handoff {
    pub slot: String,
    pub file: String,
    pub content: String,
}

pub fn synth_task(plan: &GoalPlan, handoffs: &[Handoff], notes: &[String]) -> String {
    let mut text = format!(
        "Write `{BRIEF_FILE}` in your worktree: a complete HTML5 document that summarises the source \
         `{source}`.\n\
         For EACH of these source sections: {required}, emit exactly one `<section>` element containing an \
         `<h2>` and a short paragraph, and cite the source section inside it with exactly this form:\n\
         <cite data-src=\"{name}\" data-section=\"N\">§N</cite>\n\
         Every section must contain at least one citation. Do not write TODO, TBD or placeholder text.\n",
        source = plan.source_name,
        required = join_numbers(&plan.file.required_sections),
        name = plan.source_name,
    );
    if !notes.is_empty() {
        text.push_str("\nNotes from the previous round, which you must address:\n");
        for note in notes {
            text.push_str(&format!("- {note}\n"));
        }
    }
    text.push_str("\nResearcher findings (handed over verbatim):\n");
    for h in handoffs {
        text.push_str(&format!("--- from {} ({}) ---\n{}\n", h.slot, h.file, h.content));
    }
    text.push_str(&format!("\nWhen the file is written, report once with `{BRIEF_FILE}` in --files."));
    text
}

fn join_numbers(ns: &[u32]) -> String {
    ns.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(", ")
}

//! Deterministic checks on the brief. The machine decides done (PRD §7); no model judges citations.
//! Citation format the synthesizer is told to use: `<cite data-src="09-spike.md" data-section="3">`.

use crate::goal::source_sections;
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Check {
    pub name: &'static str,
    pub passed: bool,
    pub detail: String,
}

/// Checks named in a goal's `checks` list that v0 cannot evaluate. Reported as skipped, with the reason.
pub const SKIPPED: &[(&str, &str)] = &[
    (
        "findings-cited-or-rejected",
        "not implemented in v0: matching ledger findings to report text needs claim extraction",
    ),
    (
        "no-uncorrected-contradictions",
        "not implemented in v0: contradiction detection is a model judgement, not a machine check",
    ),
];

pub const IMPLEMENTED: &[&str] = &["section-citations-resolve", "required-sections-cited", "no-placeholders"];

pub fn evaluate(html: Option<&str>, source_name: &str, source_text: &str, required: &[u32]) -> Vec<Check> {
    let Some(html) = html.filter(|h| !h.trim().is_empty()) else {
        return vec![Check { name: "brief-exists", passed: false, detail: "no brief.html".into() }];
    };
    let known = source_sections(source_text);
    let sections = section_blocks(html);
    let mut cited: BTreeSet<u32> = BTreeSet::new();

    let mut bad_sections = Vec::new();
    if sections.is_empty() {
        bad_sections.push("no <section> elements".to_string());
    }
    for (i, block) in sections.iter().enumerate() {
        let cites = cites_in(block);
        let valid: Vec<u32> = cites
            .iter()
            .filter(|(src, n)| src == source_name && known.contains(n))
            .map(|(_, n)| *n)
            .collect();
        if valid.is_empty() {
            bad_sections.push(format!("section {} has no citation that resolves", i + 1));
        }
        cited.extend(valid);
    }
    let missing: Vec<String> = required.iter().filter(|n| !cited.contains(n)).map(|n| n.to_string()).collect();
    let placeholders: Vec<&str> = ["TODO", "TBD", "[citation needed]", "lorem ipsum"]
        .into_iter()
        .filter(|m| html.to_lowercase().contains(&m.to_lowercase()))
        .collect();

    vec![
        Check { name: "brief-exists", passed: true, detail: format!("{} section(s)", sections.len()) },
        Check {
            name: "section-citations-resolve",
            passed: bad_sections.is_empty(),
            detail: if bad_sections.is_empty() { "every section cites a resolving source".into() } else { bad_sections.join("; ") },
        },
        Check {
            name: "required-sections-cited",
            passed: missing.is_empty(),
            detail: if missing.is_empty() { "all required source sections cited".into() } else { format!("uncited source sections: {}", missing.join(", ")) },
        },
        Check {
            name: "no-placeholders",
            passed: placeholders.is_empty(),
            detail: if placeholders.is_empty() { "no placeholder markers".into() } else { format!("found {}", placeholders.join(", ")) },
        },
    ]
}

/// Text of each `<section ...> ... </section>` (or up to the next `<section`).
fn section_blocks(html: &str) -> Vec<String> {
    let lower = html.to_lowercase();
    let starts: Vec<usize> = lower.match_indices("<section").map(|(i, _)| i).collect();
    starts
        .iter()
        .enumerate()
        .map(|(k, &start)| {
            let end = starts.get(k + 1).copied().unwrap_or(html.len());
            let chunk = &html[start..end];
            match chunk.to_lowercase().find("</section>") {
                Some(close) => chunk[..close].to_string(),
                None => chunk.to_string(),
            }
        })
        .collect()
}

/// `(data-src, data-section)` for every `<cite>` tag in the text.
fn cites_in(block: &str) -> Vec<(String, u32)> {
    let lower = block.to_lowercase();
    lower
        .match_indices("<cite")
        .filter_map(|(start, _)| {
            let tag_end = block[start..].find('>')? + start;
            let tag = &block[start..tag_end];
            let src = attr(tag, "data-src")?;
            let n: u32 = attr(tag, "data-section")?.trim().parse().ok()?;
            Some((src.to_string(), n))
        })
        .collect()
}

fn attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let key = format!("{name}=\"");
    let start = tag.find(&key)? + key.len();
    let end = tag[start..].find('"')? + start;
    Some(&tag[start..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE: &str = "# Spike\n\n## 1. First\nbody\n\n## 2. Second\nbody\n\n## 3. Third\nbody\n";

    fn brief(body: &str) -> String {
        format!("<!doctype html><html><body>{body}</body></html>")
    }

    fn passed(checks: &[Check], name: &str) -> bool {
        checks.iter().find(|c| c.name == name).map(|c| c.passed).unwrap_or(false)
    }

    #[test]
    fn well_cited_brief_passes_everything() {
        let html = brief(
            r#"<section><h2>A</h2><p>x <cite data-src="spike.md" data-section="1">§1</cite></p></section>
               <section><p><cite data-src="spike.md" data-section="2">§2</cite> and <cite data-src="spike.md" data-section="3">§3</cite></p></section>"#,
        );
        let checks = evaluate(Some(&html), "spike.md", SOURCE, &[1, 2, 3]);
        assert!(checks.iter().all(|c| c.passed), "{checks:?}");
    }

    #[test]
    fn citation_to_missing_section_fails_resolution() {
        let html = brief(r#"<section><cite data-src="spike.md" data-section="9">§9</cite></section>"#);
        let checks = evaluate(Some(&html), "spike.md", SOURCE, &[]);
        assert!(!passed(&checks, "section-citations-resolve"));
    }

    #[test]
    fn citation_to_wrong_source_file_does_not_resolve() {
        let html = brief(r#"<section><cite data-src="other.md" data-section="1">§1</cite></section>"#);
        let checks = evaluate(Some(&html), "spike.md", SOURCE, &[]);
        assert!(!passed(&checks, "section-citations-resolve"));
    }

    #[test]
    fn section_without_any_citation_fails() {
        let html = brief("<section><p>no cite here</p></section>");
        let checks = evaluate(Some(&html), "spike.md", SOURCE, &[]);
        assert!(!passed(&checks, "section-citations-resolve"));
    }

    #[test]
    fn required_section_left_uncited_fails_coverage() {
        let html = brief(r#"<section><cite data-src="spike.md" data-section="1">§1</cite></section>"#);
        let checks = evaluate(Some(&html), "spike.md", SOURCE, &[1, 2]);
        assert!(!passed(&checks, "required-sections-cited"));
        assert!(checks.iter().any(|c| c.detail.contains("uncited source sections: 2")));
    }

    #[test]
    fn placeholder_markers_fail() {
        let html = brief(r#"<section><cite data-src="spike.md" data-section="1">TODO write</cite></section>"#);
        let checks = evaluate(Some(&html), "spike.md", SOURCE, &[]);
        assert!(!passed(&checks, "no-placeholders"));
    }

    #[test]
    fn missing_brief_fails_brief_exists() {
        let checks = evaluate(None, "spike.md", SOURCE, &[]);
        assert_eq!(checks.len(), 1);
        assert!(!checks[0].passed);
    }
}

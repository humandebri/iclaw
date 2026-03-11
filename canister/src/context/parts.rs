// where: standalone/canister/src/context/parts.rs
// what: Rendering and parsing helpers for ICP chat context sections
// why: Keep context.rs focused on orchestration while isolating deterministic text assembly rules

use super::{LightweightSkillAction, SkillDoc, WorkspaceDoc};
use crate::service::policies::is_summary_key;
use iclaw_standalone_core::memory::MemoryEntry;
use std::collections::BTreeSet;

pub(crate) fn render_tooling_context(tool_names: &[String]) -> String {
    if tool_names.is_empty() {
        return "Available canister surfaces: memory_store, memory_recall, memory_forget, http_request.".to_string();
    }
    format!("Available canister surfaces: {}.", tool_names.join(", "))
}

pub(crate) fn render_workspace_context(docs: &[WorkspaceDoc], max_chars: usize) -> String {
    let mut output = String::new();
    let mut remaining = max_chars;
    for doc in docs {
        if remaining == 0 {
            break;
        }
        let heading = format!("### {}\n", doc.file_name);
        if heading.chars().count() >= remaining {
            break;
        }
        let chunk = format!(
            "{}{}",
            heading,
            truncate_chars(&doc.content, remaining - heading.chars().count())
        );
        if !chunk.trim().is_empty() {
            remaining = remaining.saturating_sub(chunk.chars().count());
            if !output.is_empty() {
                output.push_str("\n\n");
            }
            output.push_str(&chunk);
        }
    }
    output
}

pub(crate) fn render_memory_context(entries: &[MemoryEntry], min_score: f64) -> String {
    let mut seen = BTreeSet::new();
    let mut output = String::new();
    for entry in entries {
        if matches!(entry.score, Some(score) if score < min_score)
            || is_assistant_autosave_key(&entry.key)
            || is_summary_key(&entry.key)
        {
            continue;
        }
        let normalized = entry.content.trim();
        if normalized.is_empty() || !seen.insert(normalized.to_string()) {
            continue;
        }
        if output.is_empty() {
            output.push_str("[Memory context]\n");
        }
        output.push_str("- ");
        output.push_str(&entry.key);
        output.push_str(": ");
        output.push_str(normalized);
        output.push('\n');
    }
    output.trim_end().to_string()
}

pub(crate) fn render_skills_context(skills: &[SkillDoc], max_chars: usize) -> String {
    let mut output = String::from("[Available skills]\n");
    for skill in skills {
        let line = format!("- {}: {}", skill.name, skill.summary);
        if output.chars().count() + line.chars().count() + 1 > max_chars {
            break;
        }
        output.push_str(&line);
        output.push('\n');
    }
    if output == "[Available skills]\n" {
        String::new()
    } else {
        output.trim_end().to_string()
    }
}

pub(crate) fn render_action_context(
    prompt: &str,
    docs: &[WorkspaceDoc],
    skills: &[SkillDoc],
    memory_context: &str,
) -> String {
    let lowered = prompt.to_ascii_lowercase();
    let actions = skills
        .iter()
        .flat_map(|skill| skill.actions.iter().copied())
        .collect::<BTreeSet<_>>();
    let mut notes = Vec::new();
    if actions.contains(&LightweightSkillAction::WorkspaceLookup) {
        notes.extend(
            docs.iter()
                .filter(|doc| lowered.contains(&doc.file_name.to_ascii_lowercase()))
                .map(|doc| {
                    format!(
                        "- workspace_lookup: {} => {}",
                        doc.file_name,
                        truncate_chars(&doc.content, 240)
                    )
                }),
        );
    }
    if actions.contains(&LightweightSkillAction::MemoryRecall)
        && !memory_context.is_empty()
        && (lowered.contains("memory") || lowered.contains("remember"))
    {
        notes.push(
            "- memory_recall: durable memory matches are already injected above.".to_string(),
        );
    }
    if actions.contains(&LightweightSkillAction::HttpRequestGuidance)
        && (lowered.contains("http") || lowered.contains("api") || lowered.contains("fetch"))
    {
        notes.push(
            "- http_request: only reserved canister-managed HTTPS guidance is available here."
                .to_string(),
        );
    }
    if actions.contains(&LightweightSkillAction::SkillSummary) {
        notes.extend(
            skills
                .iter()
                .filter(|skill| lowered.contains(&skill.name.to_ascii_lowercase()))
                .map(|skill| {
                    format!(
                        "- skill_summary: {} => {}",
                        skill.name,
                        truncate_chars(&skill.summary, 240)
                    )
                }),
        );
    }
    if notes.is_empty() {
        String::new()
    } else {
        format!("[Skill action context]\n{}", notes.join("\n"))
    }
}

pub(crate) fn parse_skill_doc(entry: MemoryEntry) -> Option<SkillDoc> {
    let content = entry.content.trim();
    if content.is_empty() {
        return None;
    }
    let name = skill_name_from_key(&entry.key);
    let description = first_non_heading_line(content).unwrap_or("No description");
    let usage = first_paragraph(content).unwrap_or_else(|| description.to_string());
    let tools = detect_tool_names(content);
    let actions = detect_actions(content, &tools);
    Some(SkillDoc {
        name,
        summary: truncate_chars(&format!("{description} Usage: {usage}"), 320),
        tools,
        actions,
    })
}

pub(crate) fn trim_memory_first(
    sections: &mut [String],
    memory_context: &mut String,
    max_chars: usize,
) {
    let total = rendered_sections_len(sections) + memory_context.chars().count();
    if total <= max_chars || memory_context.is_empty() {
        return;
    }
    let overflow = total - max_chars;
    *memory_context = truncate_chars(
        memory_context,
        memory_context.chars().count().saturating_sub(overflow),
    );
}

fn rendered_sections_len(sections: &[String]) -> usize {
    sections
        .iter()
        .filter(|section| !section.trim().is_empty())
        .enumerate()
        .map(|(index, section)| section.chars().count() + if index == 0 { 0 } else { 2 })
        .sum()
}

pub(crate) fn is_allowed_workspace_file(name: &str) -> bool {
    super::DEFAULT_WORKSPACE_FILES
        .iter()
        .any(|allowed| allowed == &name.trim())
}

fn is_assistant_autosave_key(key: &str) -> bool {
    let normalized = key.trim().to_ascii_lowercase();
    normalized == "assistant_resp"
        || normalized.starts_with("assistant_resp_")
        || normalized.contains("/assistant/")
}

fn skill_name_from_key(key: &str) -> String {
    key.trim_end_matches("/SKILL.md")
        .rsplit('/')
        .next()
        .unwrap_or("unknown")
        .to_string()
}

fn first_non_heading_line(content: &str) -> Option<&str> {
    content
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#'))
}

fn first_paragraph(content: &str) -> Option<String> {
    let mut paragraph = Vec::new();
    for line in content.lines().map(str::trim) {
        if line.is_empty() && !paragraph.is_empty() {
            break;
        }
        if !line.is_empty() && !line.starts_with('#') {
            paragraph.push(line);
        }
    }
    if paragraph.is_empty() {
        None
    } else {
        Some(paragraph.join(" "))
    }
}

fn detect_tool_names(content: &str) -> Vec<String> {
    [
        "memory_store",
        "memory_recall",
        "memory_forget",
        "http_request",
    ]
    .into_iter()
    .filter(|tool| content.contains(tool))
    .map(str::to_string)
    .collect()
}

fn detect_actions(content: &str, tools: &[String]) -> Vec<LightweightSkillAction> {
    let mut actions = BTreeSet::new();
    for line in content.lines().map(str::trim) {
        let lower = line.to_ascii_lowercase();
        if let Some(raw) = lower
            .strip_prefix("actions:")
            .or_else(|| lower.strip_prefix("action:"))
        {
            for token in raw.split([',', ' ']).filter(|token| !token.is_empty()) {
                match token {
                    "workspace_lookup" | "doc_lookup" => {
                        actions.insert(LightweightSkillAction::WorkspaceLookup);
                    }
                    "memory_recall" | "recall_memory" => {
                        actions.insert(LightweightSkillAction::MemoryRecall);
                    }
                    "http_request" | "http_request_guidance" => {
                        actions.insert(LightweightSkillAction::HttpRequestGuidance);
                    }
                    "skill_summary" | "skill_lookup" => {
                        actions.insert(LightweightSkillAction::SkillSummary);
                    }
                    _ => {}
                }
            }
        }
    }
    if tools.iter().any(|tool| tool == "memory_recall") {
        actions.insert(LightweightSkillAction::MemoryRecall);
    }
    if tools.iter().any(|tool| tool == "http_request") {
        actions.insert(LightweightSkillAction::HttpRequestGuidance);
    }
    actions.into_iter().collect()
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        value.to_string()
    } else {
        value.chars().take(max_chars).collect()
    }
}

use std::fs;

use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::editor;
use crate::models::{LlmConfig, NodeSummary, RejectedApproach};
use crate::store::GraphStore;

pub fn run(id: Option<&str>, transcript_path: &str) -> Result<()> {
    let store = GraphStore::open_from_cwd()?;
    let node_id = store.resolve_node_id(id)?;
    let config = store.load_config()?;

    let transcript = fs::read_to_string(transcript_path)
        .with_context(|| format!("Failed to read transcript: {}", transcript_path))?;

    eprintln!("Calling LLM ({}) to draft summary...", config.llm.model);
    let draft = call_llm(&config.llm, &transcript)?;
    eprintln!("Draft received. Opening editor for review...");

    let reviewed = editor::edit_node_summary(Some(&draft))?;

    let mut node = store.load_node(node_id)?;
    node.summary = reviewed;
    node.updated_at = Utc::now();
    store.save_node(&node)?;

    println!("Summary saved to node {}", node.short_id());
    Ok(())
}

// ─── OpenAI-compatible request/response types ────────────────────────────────

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
}

#[derive(Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ChatMessage,
}

// ─── LLM response JSON shape ──────────────────────────────────────────────────

#[derive(Deserialize)]
struct LlmSummary {
    goal: String,
    #[serde(default)]
    decisions: Vec<String>,
    #[serde(default)]
    rejected_approaches: Vec<LlmRejected>,
    #[serde(default)]
    open_threads: Vec<String>,
    #[serde(default)]
    key_artifacts: Vec<String>,
}

#[derive(Deserialize)]
struct LlmRejected {
    description: String,
    reason: String,
}

// ─────────────────────────────────────────────────────────────────────────────

fn call_llm(cfg: &LlmConfig, transcript: &str) -> Result<NodeSummary> {
    let endpoint = llm_endpoint(cfg);

    let system_prompt = r#"You are a technical documentation assistant helping a developer organize LLM-assisted work into structured conversation nodes.

Given a conversation transcript or description of work, extract a structured summary.

Return ONLY valid JSON — no markdown fences, no explanation, just the JSON object:
{
  "goal": "one sentence describing what this work was trying to accomplish",
  "decisions": ["specific technical decisions made"],
  "rejected_approaches": [
    { "description": "approach considered", "reason": "why it was not used" }
  ],
  "open_threads": ["unresolved questions or follow-up items"],
  "key_artifacts": ["relevant file paths, function names, or module names"]
}"#;

    let api_key = if cfg.api_key_env.is_empty() {
        String::new()
    } else {
        std::env::var(&cfg.api_key_env).unwrap_or_default()
    };

    let client = reqwest::blocking::Client::new();
    let mut req = client
        .post(&endpoint)
        .header("Content-Type", "application/json");

    if !api_key.is_empty() {
        req = req.header("Authorization", format!("Bearer {}", api_key));
    }

    let body = ChatRequest {
        model: cfg.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: format!("Transcript:\n\n{}", transcript),
            },
        ],
        temperature: 0.2,
    };

    let resp = req
        .json(&body)
        .send()
        .context("Failed to reach LLM endpoint")?;

    let status = resp.status();
    if !status.is_success() {
        let body_text = resp.text().unwrap_or_default();
        anyhow::bail!("LLM request failed ({}): {}", status, body_text);
    }

    let chat_resp: ChatResponse = resp.json().context("Failed to parse LLM response")?;
    let content = chat_resp
        .choices
        .into_iter()
        .next()
        .map(|c| c.message.content)
        .ok_or_else(|| anyhow::anyhow!("LLM returned no choices"))?;

    parse_summary(&content)
}

fn llm_endpoint(cfg: &LlmConfig) -> String {
    if !cfg.base_url.is_empty() {
        let base = cfg.base_url.trim_end_matches('/');
        return format!("{}/v1/chat/completions", base);
    }
    "https://api.openai.com/v1/chat/completions".to_string()
}

fn parse_summary(content: &str) -> Result<NodeSummary> {
    let json_str = strip_fences(content);
    let llm: LlmSummary = serde_json::from_str(json_str)
        .with_context(|| format!("Could not parse LLM response as JSON:\n{}", content))?;

    Ok(NodeSummary {
        goal: llm.goal,
        decisions: llm.decisions,
        rejected_approaches: llm
            .rejected_approaches
            .into_iter()
            .map(|r| RejectedApproach {
                description: r.description,
                reason: r.reason,
            })
            .collect(),
        open_threads: llm.open_threads,
        key_artifacts: llm.key_artifacts,
    })
}

fn strip_fences(s: &str) -> &str {
    let s = s.trim();
    if let Some(inner) = s.strip_prefix("```json") {
        return inner.trim_start_matches('\n').trim_end_matches("```").trim();
    }
    if let Some(inner) = s.strip_prefix("```") {
        return inner.trim_start_matches('\n').trim_end_matches("```").trim();
    }
    s
}

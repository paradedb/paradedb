use std::{collections::HashMap, time::Duration};

use clap::Parser;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

const DEFAULT_SLACK_MENTION: &str = "<@here>";

#[derive(Parser, Debug)]
#[command(about = "Enterprise sync helper for mapping conflicts to Slack mentions.")]
struct Args {
    /// GitHub repository in 'owner/name' format (required).
    #[arg(long)]
    repo: String,

    /// Commit SHA to inspect for author attribution.
    #[arg(long)]
    commit_sha: Option<String>,

    /// Output full Slack message payload instead of just metadata
    #[arg(long)]
    slack_payload: bool,
}

#[derive(Debug, Default)]
struct SlackMappings {
    entries: HashMap<String, String>,
}

impl SlackMappings {
    fn load() -> Self {
        // Load from environment variable - panic if missing since this is a CI config bug
        let env_mappings = std::env::var("USERNAME_MAPPING_GITHUB_TO_SLACK")
            .expect("USERNAME_MAPPING_GITHUB_TO_SLACK env var must be set");

        Self::parse_from_string(&env_mappings)
    }

    fn parse_from_string(content: &str) -> Self {
        let content = content.trim();

        // Try parsing as TOML first
        if let Ok(file) = toml::from_str::<MappingFile>(content) {
            let entries = file
                .github_to_slack
                .into_iter()
                .map(|(key, value)| (key.to_lowercase(), value))
                .collect();
            return Self { entries };
        }

        // Try parsing as newline-separated format: "username=<@slack>"
        let mut entries = HashMap::new();
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some((github_user, slack_mention)) = line.split_once('=') {
                let github_user = github_user.trim().to_lowercase();
                let slack_mention = slack_mention.trim().to_string();
                entries.insert(github_user, slack_mention);
            }
        }

        if !entries.is_empty() {
            return Self { entries };
        }

        // If we got here, parsing failed - this is a CI config bug
        panic!("USERNAME_MAPPING_GITHUB_TO_SLACK env var is empty or malformed");
    }

    fn find(&self, username: &str) -> Option<&String> {
        self.entries.get(&username.to_lowercase())
    }
}

#[derive(Deserialize, Debug)]
struct MappingFile {
    #[serde(default)]
    github_to_slack: HashMap<String, String>,
}

#[derive(Serialize, Clone)]
struct Output {
    slack_mention: String,
    commit_author: Option<String>,
    commit_message: Option<String>,
    used_fallback: bool,
}

#[derive(Serialize)]
struct SlackField {
    title: String,
    value: String,
    short: bool,
}

#[derive(Serialize)]
struct SlackAttachment {
    color: String,
    fields: Vec<SlackField>,
}

#[derive(Serialize)]
struct SlackPayload {
    text: String,
    attachments: Vec<SlackAttachment>,
}

fn main() {
    env_logger::init();
    let args = Args::parse();
    let output = run(&args);

    if args.slack_payload {
        let payload = build_slack_payload(&output, &args.repo);
        match serde_json::to_string_pretty(&payload) {
            Ok(json) => println!("{json}"),
            Err(err) => {
                eprintln!("enterprise-sync: failed to serialize Slack payload: {err}");
                std::process::exit(1);
            }
        }
    } else {
        match serde_json::to_string_pretty(&output) {
            Ok(json) => println!("{json}"),
            Err(err) => {
                eprintln!("enterprise-sync: failed to serialize output: {err}");
                println!(
                    "{{\"slack_mention\":\"{}\",\"used_fallback\":true}}",
                    output.slack_mention
                );
            }
        }
    }
}

fn run(args: &Args) -> Output {
    let default_mention = DEFAULT_SLACK_MENTION.to_string();
    let mappings = SlackMappings::load();

    let (github_username, commit_author, commit_message) = determine_github_details(args);

    let mut used_fallback = false;
    let mut slack_mention = default_mention.clone();

    if let Some(username) = github_username.as_ref() {
        if let Some(mapping) = mappings.find(username) {
            slack_mention = mapping.clone();
        } else {
            used_fallback = true;
            log::warn!(
                "Slack mapping missing for GitHub user '{}'. Defaulting to {}.",
                username,
                default_mention
            );
        }
    } else {
        used_fallback = true;
        log::warn!(
            "Unable to determine GitHub username for commit; defaulting to {}.",
            default_mention
        );
    }

    Output {
        slack_mention,
        commit_author,
        commit_message,
        used_fallback,
    }
}

fn build_slack_payload(output: &Output, repo: &str) -> SlackPayload {
    let commit_author = output.commit_author.as_deref().unwrap_or("Unknown");
    let message_text = format!(
        "🔧 Community Rebase Needs Resolution - {} ({})",
        output.slack_mention, commit_author
    );

    // Get current workflow/branch context
    let workflow = std::env::var("GITHUB_WORKFLOW")
        .ok()
        .unwrap_or_else(|| "Community Rebase".to_string());

    let mut fields = vec![
        SlackField {
            title: "Repository".to_string(),
            value: repo.to_string(),
            short: true,
        },
        SlackField {
            title: "Workflow".to_string(),
            value: workflow,
            short: true,
        },
        SlackField {
            title: "Commit Author".to_string(),
            value: commit_author.to_string(),
            short: true,
        },
    ];

    // Add View Logs link if we have run information
    if let Ok(run_url) = get_workflow_run_url() {
        fields.push(SlackField {
            title: "View Logs".to_string(),
            value: format!("<{}|Click here>", run_url),
            short: true,
        });
    }

    // Add commit message if available
    if let Some(commit_message) = output.commit_message.as_ref() {
        if !commit_message.is_empty() {
            fields.push(SlackField {
                title: "Commit Message".to_string(),
                value: commit_message.clone(),
                short: false,
            });
        }
    }

    SlackPayload {
        text: message_text,
        attachments: vec![SlackAttachment {
            color: "warning".to_string(),
            fields,
        }],
    }
}

fn get_workflow_run_url() -> Result<String, String> {
    // In GitHub Actions, these env vars are always set
    let server_url = std::env::var("GITHUB_SERVER_URL")
        .map_err(|e| format!("GITHUB_SERVER_URL not set: {e}"))?;
    let repository = std::env::var("GITHUB_REPOSITORY")
        .map_err(|e| format!("GITHUB_REPOSITORY not set: {e}"))?;
    let run_id =
        std::env::var("GITHUB_RUN_ID").map_err(|e| format!("GITHUB_RUN_ID not set: {e}"))?;

    Ok(format!(
        "{}/{}/actions/runs/{}",
        server_url, repository, run_id
    ))
}

fn determine_github_details(args: &Args) -> (Option<String>, Option<String>, Option<String>) {
    let commit_sha = match args.commit_sha.as_ref() {
        Some(sha) if !sha.is_empty() => sha,
        _ => {
            log::warn!("Commit SHA not provided. Supply --commit-sha.");
            return (None, None, None);
        }
    };

    let token = std::env::var("GH_TOKEN")
        .or_else(|_| std::env::var("GITHUB_TOKEN"))
        .ok();

    // Use the required --repo argument to fetch commit details
    match fetch_github_commit_details(&args.repo, commit_sha, token.as_deref()) {
        Ok((username, author, message)) => (username, Some(author), Some(message)),
        Err(err) => {
            log::warn!("Failed to query GitHub for commit {}: {}", commit_sha, err);
            (None, None, None)
        }
    }
}

fn fetch_github_commit_details(
    repo: &str,
    commit_sha: &str,
    token: Option<&str>,
) -> Result<(Option<String>, String, String), String> {
    let url = format!("https://api.github.com/repos/{repo}/commits/{commit_sha}");

    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent("enterprise-sync/0.1 (https://github.com/paradedb/paradedb-enterprise)")
        .build()
        .map_err(|err| format!("failed to build HTTP client: {err}"))?;

    let mut request = client.get(url);

    if let Some(token) = token {
        request = request.bearer_auth(token);
    }

    let response = request
        .send()
        .map_err(|err| format!("GitHub request error: {err}"))?;

    let status = response.status();
    let body = response
        .text()
        .map_err(|err| format!("failed to read GitHub response: {err}"))?;

    if !status.is_success() {
        return Err(format!(
            "GitHub request failed with status {status}: {body}"
        ));
    }

    let value: Value = serde_json::from_str(&body)
        .map_err(|err| format!("failed to decode GitHub response: {err}"))?;

    // Extract commit author name
    let commit_author = value
        .get("commit")
        .and_then(|c| c.get("author"))
        .and_then(|a| a.get("name"))
        .and_then(Value::as_str)
        .unwrap_or("Unknown")
        .to_string();

    // Extract commit message (subject line only)
    let commit_message = value
        .get("commit")
        .and_then(|c| c.get("message"))
        .and_then(Value::as_str)
        .and_then(|msg| msg.lines().next()) // Take only the first line (subject)
        .unwrap_or("No commit message")
        .to_string();

    // Extract GitHub username (for Slack mapping)
    let username = value
        .get("author")
        .and_then(|author| author.get("login"))
        .and_then(Value::as_str)
        .map(|s| s.to_string())
        .or_else(|| {
            value
                .get("committer")
                .and_then(|committer| committer.get("login"))
                .and_then(Value::as_str)
                .map(|s| s.to_string())
        });

    Ok((username, commit_author, commit_message))
}

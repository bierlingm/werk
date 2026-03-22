//! Run command handler.
//!
//! Two modes:
//! - **One-shot**: `werk run <id> "prompt"` — sends prompt with tension context to agent,
//!   parses response for suggested reality update.
//! - **Interactive**: `werk run <id> -- <command>` — launches agent subprocess with
//!   context piped to stdin.

use crate::commands::config::Config;
use crate::commands::context::ContextResult;
use crate::dynamics::{
    mutation_to_info, node_to_tension_info, tension_to_info,
    MutationInfo, TensionInfo,
};
use crate::error::WerkError;
use crate::output::Output;
use crate::prefix::PrefixResolver;
use crate::workspace::Workspace;
use werk_shared::truncate;
use chrono::Utc;
use sd_core::{compute_urgency, project_tension, DynamicsEngine, Mutation, ProjectionThresholds, Tension, TensionStatus};
use std::io::Write;
use std::process::Stdio;

use crate::agent_response::StructuredResponse;

pub fn cmd_run(
    output: &Output,
    id: Option<String>,
    prompt: Option<String>,
    no_suggest: bool,
    command: Vec<String>,
    system: bool,
    decompose: bool,
    dry_run: bool,
) -> Result<(), WerkError> {
    // Discover workspace
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;

    // Create DynamicsEngine from store
    let mut engine = DynamicsEngine::with_store(store);

    // Get all tensions for prefix resolution
    let all_tensions = engine
        .store()
        .list_tensions()
        .map_err(WerkError::StoreError)?;

    // --system mode: system-wide context, no specific tension required
    if system {
        let prompt_text = prompt.ok_or_else(|| {
            WerkError::InvalidInput("--system mode requires a prompt".to_string())
        })?;
        return run_system(output, &workspace, &mut engine, &all_tensions, &prompt_text, no_suggest, dry_run);
    }

    // All other modes require an ID
    let id = id.ok_or_else(|| {
        WerkError::InvalidInput("tension ID is required (use --system for system-wide mode)".to_string())
    })?;

    let resolver = PrefixResolver::new(all_tensions.clone());
    let tension = resolver.resolve(&id)?;

    // --decompose mode: auto-decompose tension into sub-tensions
    if decompose {
        return run_decompose(output, &workspace, &mut engine, tension, &all_tensions, no_suggest, dry_run);
    }

    // Route to one-shot or interactive mode
    if let Some(prompt_text) = prompt {
        run_one_shot(output, &workspace, &mut engine, tension, &all_tensions, &prompt_text, no_suggest, dry_run)
    } else {
        run_interactive(&workspace, &mut engine, tension, &all_tensions, &command)
    }
}

/// One-shot mode: send prompt with tension context to agent, parse response.
fn run_one_shot(
    output: &Output,
    workspace: &Workspace,
    engine: &mut DynamicsEngine,
    tension: &Tension,
    all_tensions: &[Tension],
    prompt: &str,
    no_suggest: bool,
    dry_run: bool,
) -> Result<(), WerkError> {
    // Build context
    let context_md = build_context_markdown(engine, tension, all_tensions);

    // Show tension info
    println!("\nTension: {}", tension.id);
    println!("Desired: {}", tension.desired);
    println!("Current: {}", tension.actual);
    println!();

    // Resolve agent command
    let config = Config::load(workspace)?;
    let agent_cmd = match config.get("agent.command") {
        Some(cmd) => cmd.clone(),
        None => {
            return Err(WerkError::InvalidInput(
                "no agent command configured. Set agent.command in config".to_string(),
            ));
        }
    };

    // Build the full prompt with context
    let full_prompt = format!(
        "You are helping manage a structural tension.\n\n\
         Context:\n{}\n\n\
         User message: {}\n\n\
         IMPORTANT: Respond in YAML format with two sections:\n\
         1. 'mutations' array: suggested changes to the tension forest\n\
         2. 'response' string: your advice in prose\n\n\
         Supported mutation actions:\n\
         - update_actual: {{tension_id, new_value, reasoning}}\n\
         - create_child: {{parent_id, desired, actual, reasoning}}\n\
         - add_note: {{tension_id, text}}\n\
         - update_status: {{tension_id, new_status, reasoning}}\n\
         - update_desired: {{tension_id, new_value, reasoning}}\n\n\
         Only suggest mutations you're confident about. \
         If nothing should change, return empty mutations: [].\n\n\
         Wrap your YAML in --- markers. Example:\n\
         ---\n\
         mutations:\n\
           - action: update_actual\n\
             tension_id: {tid}\n\
             new_value: \"Updated state\"\n\
             reasoning: \"Progress made\"\n\
         response: |\n\
           Your advice here.\n\
         ---\n\n\
         If you cannot produce YAML, respond in plain text. If suggesting a \
         reality update in plain text, use: SUGGESTED REALITY: <new value>",
        context_md, prompt, tid = tension.id
    );

    // Execute agent and capture response
    let response_text = execute_agent_capture(&agent_cmd, &full_prompt)?;

    // Record the one-shot session as a mutation
    engine
        .store()
        .record_mutation(&Mutation::new(
            tension.id.clone(),
            Utc::now(),
            "agent_one_shot".to_owned(),
            None,
            format!("prompt: {}", truncate(prompt, 100)),
        ))
        .map_err(WerkError::SdError)?;

    // Try structured YAML parsing first, fall back to simple text
    if let Some(structured) = StructuredResponse::from_response(&response_text) {
        handle_structured_response(output, engine, tension, structured, no_suggest, dry_run)?;
    } else {
        // Fallback: display as plain text
        println!("Agent Response:");
        println!("{}", "\u{2500}".repeat(60));
        println!("{}", response_text.trim());
        println!("{}", "\u{2500}".repeat(60));

        // Parse for simple suggested reality update
        if !no_suggest && !dry_run {
            if let Some(suggestion) = extract_update_suggestion(&response_text) {
                handle_update_suggestion(output, engine, tension, &suggestion)?;
            }
        }
    }

    Ok(())
}

/// System-wide mode: send all active tensions as context to agent.
fn run_system(
    output: &Output,
    workspace: &Workspace,
    engine: &mut DynamicsEngine,
    all_tensions: &[Tension],
    prompt: &str,
    no_suggest: bool,
    dry_run: bool,
) -> Result<(), WerkError> {
    let now = Utc::now();

    // Filter active tensions
    let active: Vec<_> = all_tensions
        .iter()
        .filter(|t| t.status != TensionStatus::Resolved && t.status != TensionStatus::Released)
        .collect();

    if active.is_empty() {
        return Err(WerkError::InvalidInput(
            "no active tensions found in workspace".to_string(),
        ));
    }

    println!("\nSystem-wide context: {} active tensions", active.len());
    println!();

    // Build system context with all active tensions
    let mut context_parts = vec!["System-wide tension context:\n".to_string()];
    for t in &active {
        let urgency = compute_urgency(t, now);
        let urgency_str = match urgency {
            Some(u) => format!("{:.2}", u.value),
            None => "none".to_string(),
        };
        let overdue = t.horizon.as_ref().map(|h| h.is_past(now)).unwrap_or(false);
        context_parts.push(format!(
            "- {} ({}): desired=\"{}\", actual=\"{}\", urgency={}{}\n",
            &t.id[..8.min(t.id.len())],
            t.status,
            t.desired,
            t.actual,
            urgency_str,
            if overdue { " OVERDUE" } else { "" },
        ));
    }

    // Resolve agent command
    let config = Config::load(workspace)?;
    let agent_cmd = match config.get("agent.command") {
        Some(cmd) => cmd.clone(),
        None => {
            return Err(WerkError::InvalidInput(
                "no agent command configured. Set agent.command in config".to_string(),
            ));
        }
    };

    let full_prompt = format!(
        "You are analyzing a system of structural tensions.\n\n\
         {}\n\n\
         User message: {}\n\n\
         IMPORTANT: Respond in YAML format with two sections:\n\
         1. 'mutations' array: suggested changes to the tension forest\n\
         2. 'response' string: your analysis in prose\n\n\
         Supported mutation actions:\n\
         - update_actual: {{tension_id, new_value, reasoning}}\n\
         - create_child: {{parent_id, desired, actual, reasoning}}\n\
         - add_note: {{tension_id, text}}\n\
         - update_status: {{tension_id, new_status, reasoning}}\n\
         - update_desired: {{tension_id, new_value, reasoning}}\n\n\
         Only suggest mutations you're confident about. \
         If nothing should change, return empty mutations: [].\n\n\
         Wrap your YAML in --- markers.\n\n\
         If you cannot produce YAML, respond in plain text.",
        context_parts.join(""),
        prompt,
    );

    // Execute agent and capture response
    let response_text = execute_agent_capture(&agent_cmd, &full_prompt)?;

    // For system mode, use first active tension as fallback for mutation recording
    let first_tension = active[0];

    // Record the system-wide session as a mutation
    engine
        .store()
        .record_mutation(&sd_core::Mutation::new(
            first_tension.id.clone(),
            Utc::now(),
            "agent_system".to_owned(),
            None,
            format!("system-wide prompt: {}", truncate(prompt, 100)),
        ))
        .map_err(WerkError::SdError)?;

    // Try structured YAML parsing first, fall back to simple text
    if let Some(structured) = StructuredResponse::from_response(&response_text) {
        handle_structured_response(output, engine, first_tension, structured, no_suggest, dry_run)?;
    } else {
        println!("Agent Response:");
        println!("{}", "\u{2500}".repeat(60));
        println!("{}", response_text.trim());
        println!("{}", "\u{2500}".repeat(60));
    }

    Ok(())
}

/// Decompose mode: ask agent to break tension into sub-tensions.
fn run_decompose(
    output: &Output,
    workspace: &Workspace,
    engine: &mut DynamicsEngine,
    tension: &Tension,
    all_tensions: &[Tension],
    no_suggest: bool,
    dry_run: bool,
) -> Result<(), WerkError> {
    // Build context
    let context_md = build_context_markdown(engine, tension, all_tensions);

    let horizon_str = match &tension.horizon {
        Some(h) => format!("{}", h),
        None => "none".to_string(),
    };

    println!("\nDecomposing tension: {}", tension.id);
    println!("Desired: {}", tension.desired);
    println!("Current: {}", tension.actual);
    println!();

    // Resolve agent command
    let config = Config::load(workspace)?;
    let agent_cmd = match config.get("agent.command") {
        Some(cmd) => cmd.clone(),
        None => {
            return Err(WerkError::InvalidInput(
                "no agent command configured. Set agent.command in config".to_string(),
            ));
        }
    };

    let decompose_prompt = format!(
        "You are helping decompose a structural tension into sub-tensions.\n\n\
         Context:\n{}\n\n\
         The parent tension is:\n  \
         ID: {}\n  \
         Desired: {}\n  \
         Actual: {}\n  \
         Horizon: {}\n\n\
         Break this into 3-7 concrete sub-tensions. Each should be:\n\
         - A specific gap between desired and actual state\n\
         - Small enough to make progress on within days\n\
         - Together they should cover the full scope of the parent\n\n\
         IMPORTANT: Respond in YAML format with two sections:\n\
         1. 'mutations' array: use create_child actions with parent_id: \"{tid}\"\n\
         2. 'response' string: your reasoning in prose\n\n\
         Supported mutation actions:\n\
         - create_child: {{parent_id, desired, actual, reasoning}}\n\
         - add_note: {{tension_id, text}}\n\n\
         Wrap your YAML in --- markers. Example:\n\
         ---\n\
         mutations:\n\
           - action: create_child\n\
             parent_id: {tid}\n\
             desired: \"Sub-goal achieved\"\n\
             actual: \"Not started\"\n\
             reasoning: \"First step toward parent goal\"\n\
         response: |\n\
           Decomposition rationale here.\n\
         ---",
        context_md,
        tension.id,
        tension.desired,
        tension.actual,
        horizon_str,
        tid = tension.id,
    );

    // Execute agent and capture response
    let response_text = execute_agent_capture(&agent_cmd, &decompose_prompt)?;

    // Record the decompose session as a mutation
    engine
        .store()
        .record_mutation(&sd_core::Mutation::new(
            tension.id.clone(),
            Utc::now(),
            "agent_decompose".to_owned(),
            None,
            format!("decompose: {}", truncate(&tension.desired, 80)),
        ))
        .map_err(WerkError::SdError)?;

    // Try structured YAML parsing first, fall back to simple text
    if let Some(structured) = StructuredResponse::from_response(&response_text) {
        handle_structured_response(output, engine, tension, structured, no_suggest, dry_run)?;
    } else {
        println!("Agent Response:");
        println!("{}", "\u{2500}".repeat(60));
        println!("{}", response_text.trim());
        println!("{}", "\u{2500}".repeat(60));
    }

    Ok(())
}

/// Interactive mode: launch agent subprocess with context piped to stdin.
fn run_interactive(
    workspace: &Workspace,
    engine: &mut DynamicsEngine,
    tension: &Tension,
    all_tensions: &[Tension],
    command: &[String],
) -> Result<(), WerkError> {
    let mutations = engine
        .store()
        .get_mutations(&tension.id)
        .map_err(WerkError::StoreError)?;

    let forest = sd_core::Forest::from_tensions(all_tensions.to_vec())
        .map_err(|e| WerkError::InvalidInput(e.to_string()))?;

    let now = Utc::now();
    let tension_info = tension_to_info(tension, &mutations, now);

    let ancestors: Vec<TensionInfo> = forest
        .ancestors(&tension.id)
        .unwrap_or_default()
        .into_iter()
        .map(|node| node_to_tension_info(node, now))
        .collect();

    let siblings: Vec<TensionInfo> = forest
        .siblings(&tension.id)
        .unwrap_or_default()
        .into_iter()
        .map(|node| node_to_tension_info(node, now))
        .collect();

    let children: Vec<TensionInfo> = forest
        .children(&tension.id)
        .unwrap_or_default()
        .into_iter()
        .map(|node| node_to_tension_info(node, now))
        .collect();

    let mutation_infos: Vec<MutationInfo> = mutations.iter().map(mutation_to_info).collect();

    let thresholds = ProjectionThresholds::default();
    let projections = project_tension(tension, &mutations, &thresholds, now);
    let projection_json = crate::commands::context::build_projection_json(&projections);

    let context = ContextResult {
        tension: tension_info,
        ancestors,
        siblings,
        children,
        mutations: mutation_infos,
        projection: projection_json,
    };

    let context_json = serde_json::to_string(&context)
        .map_err(|e| WerkError::IoError(format!("failed to serialize context JSON: {}", e)))?;

    // Determine command to run
    let (program, args, command_str_for_mutation): (String, Vec<String>, String) =
        if !command.is_empty() {
            let program = command[0].clone();
            let args: Vec<String> = command[1..].to_vec();
            let command_str = command.join(" ");
            (program, args, command_str)
        } else {
            let config = Config::load(workspace)?;
            match config.get("agent.command") {
                Some(cmd) => resolve_agent_command(cmd)?,
                None => {
                    return Err(WerkError::InvalidInput(
                        "no agent command configured. Use -- to specify a command or set agent.command in config".to_string(),
                    ));
                }
            }
        };

    let workspace_path = workspace.werk_dir();

    // Spawn subprocess
    let mut child = std::process::Command::new(&program)
        .args(&args)
        .env("WERK_TENSION_ID", &tension.id)
        .env("WERK_CONTEXT", &context_json)
        .env("WERK_WORKSPACE", workspace_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                WerkError::InvalidInput(format!("agent command not found: {}", program))
            } else {
                WerkError::IoError(format!("failed to spawn agent process: {}", e))
            }
        })?;

    // Write context to stdin
    {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or_else(|| WerkError::IoError("failed to open stdin for subprocess".to_string()))?;
        stdin
            .write_all(context_json.as_bytes())
            .map_err(|e| WerkError::IoError(format!("failed to write context to stdin: {}", e)))?;
    }

    let exit_status = child
        .wait()
        .map_err(|e| WerkError::IoError(format!("failed to wait for agent process: {}", e)))?;

    let exit_code = exit_status.code().unwrap_or(1);

    // Record session mutation
    engine
        .store()
        .record_mutation(&Mutation::new(
            tension.id.clone(),
            Utc::now(),
            "agent_session".to_owned(),
            None,
            command_str_for_mutation,
        ))
        .map_err(WerkError::SdError)?;

    if !exit_status.success() {
        std::process::exit(exit_code);
    }

    Ok(())
}

/// Build a markdown context string for agent consumption in one-shot mode.
pub fn build_context_markdown(
    _engine: &mut DynamicsEngine,
    tension: &Tension,
    all_tensions: &[Tension],
) -> String {
    let mut out = String::new();

    out.push_str(&format!("**ID:** {}\n", tension.id));
    out.push_str(&format!("**Desired:** {}\n", tension.desired));
    out.push_str(&format!("**Current:** {}\n", tension.actual));
    out.push_str(&format!("**Status:** {}\n", tension.status));

    if let Some(h) = &tension.horizon {
        out.push_str(&format!("**Horizon:** {}\n", h));
        let now = Utc::now();
        if let Some(u) = compute_urgency(tension, now) {
            out.push_str(&format!("**Urgency:** {:.0}%\n", u.value * 100.0));
        }
        if h.is_past(now) {
            out.push_str("**OVERDUE**\n");
        }
    }

    out.push_str(&format!("**Has gap:** {}\n", tension.desired != tension.actual));

    // Parent chain
    if let Some(parent_id) = &tension.parent_id {
        if let Some(parent) = all_tensions.iter().find(|t| &t.id == parent_id) {
            out.push_str(&format!(
                "\n**Parent:** {} ({})\n",
                parent.desired, parent.id
            ));
        }
    }

    out
}

/// Execute agent command and capture its stdout.
///
/// The prompt is passed as a CLI argument (appended to the command), which is
/// the standard pattern for one-shot agent invocations (e.g. `hermes chat -Q -q "prompt"`).
/// The prompt is also piped to stdin as a fallback for agents that read from stdin.
pub fn execute_agent_capture(agent_cmd: &str, prompt: &str) -> Result<String, WerkError> {
    let (program, args, _) = resolve_agent_command(agent_cmd)?;

    let mut child = if program == "sh" && args.len() == 2 && args[0] == "-c" {
        // Shell command — append prompt as a positional arg: sh -c '$cmd "$1"' -- "$prompt"
        let shell_cmd = format!("{} \"$1\"", args[1]);
        std::process::Command::new(&program)
            .args(["-c", &shell_cmd, "--", prompt])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| WerkError::IoError(format!("failed to spawn agent: {}", e)))?
    } else {
        // Direct command — append prompt as last argument
        std::process::Command::new(&program)
            .args(&args)
            .arg(prompt)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| WerkError::IoError(format!("failed to spawn agent: {}", e)))?
    };

    // Also pipe prompt to stdin as fallback
    if let Some(stdin) = child.stdin.as_mut() {
        let _ = stdin.write_all(prompt.as_bytes());
    }

    let output = child
        .wait_with_output()
        .map_err(|e| WerkError::IoError(format!("failed to read agent output: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(WerkError::IoError(format!(
            "agent command failed (exit {}): {}",
            output.status.code().unwrap_or(-1),
            stderr.trim()
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Extract a suggested reality update from agent response text.
///
/// Looks for the pattern: SUGGESTED REALITY: <text>
fn extract_update_suggestion(response: &str) -> Option<String> {
    for line in response.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("SUGGESTED REALITY:") {
            let suggestion = rest.trim().to_string();
            if !suggestion.is_empty() {
                return Some(suggestion);
            }
        }
    }
    None
}

/// Handle a suggested reality update — auto-apply in non-interactive CLI mode.
fn handle_update_suggestion(
    _output: &Output,
    engine: &mut DynamicsEngine,
    tension: &Tension,
    suggestion: &str,
) -> Result<(), WerkError> {
    println!("\nSuggested reality: \"{}\"", suggestion);
    println!("(Non-interactive mode: skipping suggestion prompt)");
    // In CLI mode without dialoguer, we skip interactive confirmation.
    // The TUI will handle interactive flows in the future.
    let _ = engine;
    let _ = tension;
    Ok(())
}

/// Handle a structured YAML response with mutations.
fn handle_structured_response(
    _output: &Output,
    engine: &mut DynamicsEngine,
    tension: &Tension,
    response: StructuredResponse,
    no_suggest: bool,
    dry_run: bool,
) -> Result<(), WerkError> {
    // Show the prose response
    println!("Agent Response:");
    println!("{}", "\u{2500}".repeat(60));
    println!("{}", response.response.trim());
    println!("{}", "\u{2500}".repeat(60));

    if response.mutations.is_empty() || no_suggest {
        if response.mutations.is_empty() {
            println!("\n(No structural changes suggested)");
        }
        return Ok(());
    }

    // Display suggested mutations
    println!("\nSuggested Changes:\n");
    for (i, mutation) in response.mutations.iter().enumerate() {
        print!("  {}. {}", i + 1, mutation.summary());
        if let Some(reason) = mutation.reasoning() {
            print!(" ({})", reason);
        }
        println!();
    }
    println!();

    // Dry run: show what would be applied without applying
    if dry_run {
        println!("Dry run -- showing what would be applied:");
        for m in &response.mutations {
            println!("  {}", m.summary());
        }
        return Ok(());
    }

    // Non-interactive CLI mode: auto-apply all suggested mutations
    println!("Applying all suggested changes...");
    apply_mutations(engine, tension, &response.mutations)?;

    Ok(())
}

/// Apply a list of mutations to the store.
fn apply_mutations(
    engine: &mut DynamicsEngine,
    _tension: &Tension,
    mutations: &[crate::agent_response::Mutation],
) -> Result<(), WerkError> {
    let mut applied = 0;
    for mutation in mutations {
        apply_single_mutation(engine, mutation)?;
        applied += 1;
    }
    println!("Applied {} change(s).", applied);
    Ok(())
}

/// Apply a single mutation to the store.
fn apply_single_mutation(
    engine: &mut DynamicsEngine,
    mutation: &crate::agent_response::Mutation,
) -> Result<(), WerkError> {
    use crate::agent_response::Mutation as AgentMutation;

    match mutation {
        AgentMutation::UpdateActual {
            tension_id,
            new_value,
            ..
        } => {
            engine
                .store()
                .update_actual(tension_id, new_value)
                .map_err(WerkError::SdError)?;
        }
        AgentMutation::CreateChild {
            parent_id,
            desired,
            actual,
            ..
        } => {
            engine
                .store()
                .create_tension_with_parent(desired, actual, Some(parent_id.clone()))
                .map_err(WerkError::SdError)?;
        }
        AgentMutation::AddNote {
            tension_id, text, ..
        } => {
            engine
                .store()
                .record_mutation(&sd_core::Mutation::new(
                    tension_id.clone(),
                    Utc::now(),
                    "note".to_owned(),
                    None,
                    text.clone(),
                ))
                .map_err(WerkError::SdError)?;
        }
        AgentMutation::UpdateStatus {
            tension_id,
            new_status,
            ..
        } => {
            let status = match new_status.to_lowercase().as_str() {
                "resolved" => TensionStatus::Resolved,
                "released" => TensionStatus::Released,
                "active" => TensionStatus::Active,
                other => {
                    return Err(WerkError::InvalidInput(format!(
                        "unknown status: '{}' (expected Active, Resolved, or Released)",
                        other
                    )));
                }
            };
            engine
                .store()
                .update_status(tension_id, status)
                .map_err(WerkError::SdError)?;
        }
        AgentMutation::UpdateDesired {
            tension_id,
            new_value,
            ..
        } => {
            engine
                .store()
                .update_desired(tension_id, new_value)
                .map_err(WerkError::SdError)?;
        }
        AgentMutation::SetHorizon {
            tension_id,
            horizon,
            ..
        } => {
            if let Ok(h) = sd_core::Horizon::parse(horizon) {
                engine.update_horizon(tension_id, Some(h))
                    .map_err(WerkError::SdError)?;
            }
        }
        AgentMutation::MoveTension {
            tension_id,
            new_parent_id,
            ..
        } => {
            engine.update_parent(tension_id, new_parent_id.as_deref())
                .map_err(WerkError::SdError)?;
        }
        AgentMutation::CreateParent {
            child_id,
            desired,
            actual,
            ..
        } => {
            let current_parent = engine.store().get_tension(child_id)
                .ok().flatten().and_then(|t| t.parent_id.clone());
            let parent = engine.create_tension_with_parent(desired, actual, current_parent)
                .map_err(WerkError::SdError)?;
            engine.update_parent(child_id, Some(&parent.id))
                .map_err(WerkError::SdError)?;
        }
    }

    Ok(())
}

/// Resolve an agent command string into (program, args, display_string).
///
/// Handles three cases:
/// 1. Absolute path — use directly
/// 2. Command with spaces — execute via shell (supports flags/args)
/// 3. Simple name — PATH lookup via `which`
fn resolve_agent_command(cmd: &str) -> Result<(String, Vec<String>, String), WerkError> {
    let cmd = cmd.trim();
    if cmd.is_empty() {
        return Err(WerkError::InvalidInput(
            "agent command in config is empty".to_string(),
        ));
    }

    if cmd.starts_with('/') {
        // Case 1: Absolute path
        if !std::path::Path::new(cmd).exists() {
            return Err(WerkError::InvalidInput(format!(
                "agent command not found at path: {}",
                cmd
            )));
        }
        Ok((cmd.to_string(), vec![], cmd.to_string()))
    } else if cmd.contains(' ') {
        // Case 2: Full command with flags — execute via shell
        Ok((
            "sh".to_string(),
            vec!["-c".to_string(), cmd.to_string()],
            cmd.to_string(),
        ))
    } else {
        // Case 3: Simple name — PATH lookup
        match which::which(cmd) {
            Ok(path) => Ok((
                path.to_string_lossy().to_string(),
                vec![],
                cmd.to_string(),
            )),
            Err(_) => Err(WerkError::InvalidInput(format!(
                "agent command not found: {}\n\nhint: Try one of these:\n  \
                 werk config set agent.command /absolute/path/to/command\n  \
                 werk config set agent.command \"command --with-flags\"\n  \
                 Ensure the command is in your PATH",
                cmd
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // === Design A: Agent Command Resolution ===

    #[test]
    fn test_resolve_agent_command_empty() {
        let result = resolve_agent_command("");
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_agent_command_whitespace_only() {
        let result = resolve_agent_command("   ");
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_agent_command_with_spaces_uses_shell() {
        let (program, args, _display) =
            resolve_agent_command("claude --dangerously-skip-permissions").unwrap();
        assert_eq!(program, "sh");
        assert_eq!(args, vec!["-c", "claude --dangerously-skip-permissions"]);
    }

    #[test]
    fn test_resolve_agent_command_absolute_path_nonexistent() {
        let result = resolve_agent_command("/nonexistent/path/to/agent");
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_agent_command_absolute_path_exists() {
        let (program, args, _) = resolve_agent_command("/bin/sh").unwrap();
        assert_eq!(program, "/bin/sh");
        assert!(args.is_empty());
    }

    #[test]
    fn test_resolve_agent_command_path_lookup() {
        let (program, args, display) = resolve_agent_command("sh").unwrap();
        assert!(!program.is_empty());
        assert!(args.is_empty());
        assert_eq!(display, "sh");
    }

    #[test]
    fn test_resolve_agent_command_not_in_path() {
        let result = resolve_agent_command("definitely_not_a_real_command_xyz123");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("not found"));
        assert!(err.contains("hint"));
    }

    // === Design D: Suggestion Extraction ===

    #[test]
    fn test_extract_update_suggestion_found() {
        let response = "Some advice here.\n\nSUGGESTED REALITY: Dylan agreed to record video";
        let result = extract_update_suggestion(response);
        assert_eq!(result, Some("Dylan agreed to record video".to_string()));
    }

    #[test]
    fn test_extract_update_suggestion_not_found() {
        let response = "Just some advice without a suggestion.";
        let result = extract_update_suggestion(response);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_update_suggestion_empty_value() {
        let response = "SUGGESTED REALITY:   ";
        let result = extract_update_suggestion(response);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_update_suggestion_with_leading_whitespace() {
        let response = "  SUGGESTED REALITY: Research phase complete, starting synthesis";
        let result = extract_update_suggestion(response);
        assert_eq!(
            result,
            Some("Research phase complete, starting synthesis".to_string())
        );
    }

    #[test]
    fn test_extract_update_suggestion_takes_first() {
        let response = "SUGGESTED REALITY: First suggestion\nSUGGESTED REALITY: Second suggestion";
        let result = extract_update_suggestion(response);
        assert_eq!(result, Some("First suggestion".to_string()));
    }

    #[test]
    fn test_truncate_short() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_long() {
        assert_eq!(truncate("hello world this is long", 10), "hello w...");
    }
}

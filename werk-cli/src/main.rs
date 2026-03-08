// werk: Operative instrument for structural dynamics
//
// The practitioner's workspace. Practice, presence, oracle.
// Built on sd-core. Maximally opinionated.
//
// Exit codes:
//   0 - Success
//   1 - User error (bad input, not found, invalid operation)
//   2 - Internal error (unexpected failure)

#![forbid(unsafe_code)]

use clap::Parser;
use werk::commands::Commands;
use werk::error::{ErrorCode, WerkError};
use werk::output::Output;

/// Operative instrument for structural dynamics.
#[derive(Parser, Debug)]
#[command(name = "werk")]
#[command(version, about, long_about = None)]
struct Cli {
    /// Output in JSON format.
    #[arg(short, long, global = true)]
    json: bool,

    /// Disable colored output.
    #[arg(long, global = true)]
    no_color: bool,

    /// Subcommand to execute.
    #[command(subcommand)]
    command: Commands,
}

fn main() {
    let args = Cli::parse();
    let output = Output::new(args.json, args.no_color);

    // Dispatch to subcommand handlers
    let result = match args.command {
        Commands::Init { global } => cmd_init(&output, global),
        Commands::Config { command } => cmd_config(&output, command),
        Commands::Add {
            desired,
            actual,
            parent,
            horizon,
        } => cmd_add(&output, desired, actual, parent, horizon),
        Commands::Horizon { id, value } => cmd_horizon(&output, id, value),
        Commands::Show { id, verbose } => cmd_show(&output, id, verbose),
        Commands::Reality { id, value } => cmd_reality(&output, id, value),
        Commands::Desire { id, value } => cmd_desire(&output, id, value),
        Commands::Resolve { id } => cmd_resolve(&output, id),
        Commands::Release { id, reason } => cmd_release(&output, id, reason),
        Commands::Rm { id } => cmd_rm(&output, id),
        Commands::Move { id, parent } => cmd_move(&output, id, parent),
        Commands::Note { arg1, arg2 } => cmd_note(&output, arg1, arg2),
        Commands::Notes => cmd_notes(&output),
        Commands::Tree {
            open,
            all,
            resolved,
            released,
        } => cmd_tree(&output, open, all, resolved, released),
        Commands::Context { id } => cmd_context(&output, id),
        Commands::Run { id, command } => cmd_run(&output, id, command),
        Commands::Nuke { confirm, global } => cmd_nuke(&output, confirm, global),
    };

    match result {
        Ok(()) => std::process::exit(0),
        Err(e) => {
            if output.is_json() {
                // Output structured JSON error when --json flag is set
                let code = match e.error_code() {
                    ErrorCode::NOT_FOUND => "NOT_FOUND",
                    ErrorCode::INVALID_INPUT => "INVALID_INPUT",
                    ErrorCode::AMBIGUOUS => "AMBIGUOUS",
                    ErrorCode::NO_WORKSPACE => "NO_WORKSPACE",
                    ErrorCode::PERMISSION_DENIED => "PERMISSION_DENIED",
                    ErrorCode::IO_ERROR => "IO_ERROR",
                    ErrorCode::CONFIG_ERROR => "CONFIG_ERROR",
                    ErrorCode::INTERNAL_ERROR => "INTERNAL_ERROR",
                };
                let _ = output.error_json(code, &e.to_string());
            } else {
                let _ = output.error(&e.to_string());
            }
            std::process::exit(e.exit_code());
        }
    }
}

// Stub implementations for subcommands.
// These will be implemented in future features.

fn cmd_init(output: &Output, global: bool) -> Result<(), WerkError> {
    use serde::Serialize;
    use std::path::PathBuf;

    /// JSON output structure for init command.
    #[derive(Serialize)]
    struct InitResult {
        path: String,
        created: bool,
    }

    let cwd = std::env::current_dir()
        .map_err(|e| WerkError::IoError(format!("failed to get current directory: {}", e)))?;

    // Determine target path
    let target_path: PathBuf = if global {
        dirs::home_dir()
            .ok_or_else(|| WerkError::IoError("cannot determine home directory".to_string()))?
    } else {
        cwd.clone()
    };

    // Check if workspace already exists
    let werk_dir = target_path.join(".werk");
    let db_path = werk_dir.join("sd.db");
    let already_exists = db_path.exists();

    // Initialize the store (this creates .werk/ and sd.db)
    // Store::init is idempotent - it won't overwrite existing data
    let _store = sd_core::Store::init(&target_path)?;

    let result = InitResult {
        path: werk_dir.to_string_lossy().to_string(),
        created: !already_exists,
    };

    if output.is_json() {
        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| WerkError::IoError(format!("failed to serialize JSON: {}", e)))?;
        println!("{}", json);
    } else {
        let message = if already_exists {
            format!("Workspace already initialized at {}", werk_dir.display())
        } else {
            format!("Workspace initialized at {}", werk_dir.display())
        };
        output
            .success(&message)
            .map_err(|e| WerkError::IoError(e.to_string()))?;
    }

    Ok(())
}

fn cmd_config(output: &Output, command: werk::commands::ConfigCommand) -> Result<(), WerkError> {
    use serde::Serialize;
    use werk::commands::config::Config;
    use werk::workspace::Workspace;

    /// JSON output structure for config set.
    #[derive(Serialize)]
    struct ConfigSetResult {
        key: String,
        value: String,
        path: String,
    }

    /// JSON output structure for config get.
    #[derive(Serialize)]
    struct ConfigGetResult {
        key: String,
        value: String,
    }

    match command {
        werk::commands::ConfigCommand::Set { key, value } => {
            // Validate key is not empty
            if key.is_empty() {
                return Err(WerkError::InvalidInput(
                    "config key cannot be empty".to_string(),
                ));
            }

            // Try to find a local workspace first, fall back to global
            let workspace_result = Workspace::discover();
            let mut config = match workspace_result {
                Ok(ws) => Config::load(&ws)?,
                Err(_) => {
                    // No local workspace - use global config
                    Config::load_global()?
                }
            };

            // Set the value
            config.set(&key, value.clone());

            // Save
            config.save()?;

            // Output
            let path = config
                .path()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "unknown".to_string());

            if output.is_json() {
                let result = ConfigSetResult { key, value, path };
                let json = serde_json::to_string_pretty(&result)
                    .map_err(|e| WerkError::IoError(format!("failed to serialize JSON: {}", e)))?;
                println!("{}", json);
            } else {
                output
                    .success(&format!(
                        "Set {} = {}",
                        key,
                        output.styled(&value, werk::output::ColorStyle::Highlight)
                    ))
                    .map_err(|e| WerkError::IoError(e.to_string()))?;
            }

            Ok(())
        }
        werk::commands::ConfigCommand::Get { key } => {
            // Validate key is not empty
            if key.is_empty() {
                return Err(WerkError::InvalidInput(
                    "config key cannot be empty".to_string(),
                ));
            }

            // Try to find a local workspace first, fall back to global
            let workspace_result = Workspace::discover();
            let config = match workspace_result {
                Ok(ws) => Config::load(&ws)?,
                Err(_) => {
                    // No local workspace - use global config
                    Config::load_global()?
                }
            };

            // Get the value
            match config.get(&key) {
                Some(value) => {
                    if output.is_json() {
                        let result = ConfigGetResult {
                            key,
                            value: value.clone(),
                        };
                        let json = serde_json::to_string_pretty(&result).map_err(|e| {
                            WerkError::IoError(format!("failed to serialize JSON: {}", e))
                        })?;
                        println!("{}", json);
                    } else {
                        println!(
                            "{} = {}",
                            output.styled(&key, werk::output::ColorStyle::Info),
                            output.styled(value, werk::output::ColorStyle::Highlight)
                        );
                    }
                    Ok(())
                }
                None => Err(WerkError::ConfigError(format!(
                    "config key '{}' not found",
                    key
                ))),
            }
        }
    }
}

fn cmd_add(
    output: &Output,
    desired: Option<String>,
    actual: Option<String>,
    parent: Option<String>,
    horizon: Option<String>,
) -> Result<(), WerkError> {
    use sd_core::Horizon;
    use serde::Serialize;
    use werk::workspace::Workspace;

    /// JSON output structure for add command.
    #[derive(Serialize)]
    struct AddResult {
        id: String,
        desired: String,
        actual: String,
        status: String,
        parent_id: Option<String>,
        horizon: Option<String>,
    }

    // Require both desired and actual as positional args
    let desired = desired.ok_or_else(|| {
        WerkError::InvalidInput(
            "desired state is required: werk add <desired> <actual>".to_string(),
        )
    })?;
    let actual = actual.ok_or_else(|| {
        WerkError::InvalidInput("actual state is required: werk add <desired> <actual>".to_string())
    })?;

    // Validate non-empty
    if desired.is_empty() {
        return Err(WerkError::InvalidInput(
            "desired state cannot be empty".to_string(),
        ));
    }
    if actual.is_empty() {
        return Err(WerkError::InvalidInput(
            "actual state cannot be empty".to_string(),
        ));
    }

    // Parse horizon if provided
    let horizon_parsed: Option<Horizon> = if let Some(h_str) = horizon {
        Some(Horizon::parse(&h_str).map_err(|e| {
            WerkError::InvalidInput(format!(
                "Invalid horizon '{}': {}. Examples: 2026, 2026-05, 2026-05-15, 2026-05-15T14:00:00Z",
                h_str, e
            ))
        })?)
    } else {
        None
    };

    // Discover workspace
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;

    // Resolve parent if provided
    let parent_id = if let Some(parent_prefix) = parent {
        let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
        let resolver = werk::prefix::PrefixResolver::new(tensions);
        let parent_tension = resolver.resolve(&parent_prefix)?;
        Some(parent_tension.id.clone())
    } else {
        None
    };

    // Create the tension with horizon
    let tension =
        store.create_tension_full(&desired, &actual, parent_id.clone(), horizon_parsed.clone())?;

    let result = AddResult {
        id: tension.id.clone(),
        desired: tension.desired.clone(),
        actual: tension.actual.clone(),
        status: tension.status.to_string(),
        parent_id,
        horizon: tension.horizon.as_ref().map(|h| h.to_string()),
    };

    if output.is_json() {
        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| WerkError::IoError(format!("failed to serialize JSON: {}", e)))?;
        println!("{}", json);
    } else {
        // Human-readable output
        let id_styled = output.styled(&tension.id, werk::output::ColorStyle::Id);
        let status_styled = output.styled(
            &tension.status.to_string(),
            werk::output::ColorStyle::Active,
        );
        output
            .success(&format!("Created tension {}", id_styled))
            .map_err(|e| WerkError::IoError(e.to_string()))?;
        println!(
            "  Desired: {}",
            output.styled(&tension.desired, werk::output::ColorStyle::Highlight)
        );
        println!(
            "  Actual:  {}",
            output.styled(&tension.actual, werk::output::ColorStyle::Muted)
        );
        println!("  Status:  {}", status_styled);
        if let Some(pid) = &tension.parent_id {
            println!(
                "  Parent:  {}",
                output.styled(pid, werk::output::ColorStyle::Id)
            );
        }
        if let Some(h) = &tension.horizon {
            println!(
                "  Horizon: {}",
                output.styled(&h.to_string(), werk::output::ColorStyle::Highlight)
            );
        }
    }

    Ok(())
}

fn cmd_horizon(output: &Output, id: String, value: Option<String>) -> Result<(), WerkError> {
    use chrono::Utc;
    use sd_core::{compute_urgency, Horizon, TensionStatus};
    use serde::Serialize;
    use werk::workspace::Workspace;

    /// JSON output structure for horizon command.
    #[derive(Serialize)]
    struct HorizonResult {
        id: String,
        horizon: Option<String>,
        old_horizon: Option<String>,
    }

    /// JSON output structure for horizon display (no value provided).
    #[derive(Serialize)]
    struct HorizonDisplayResult {
        id: String,
        horizon: Option<String>,
        urgency: Option<f64>,
        days_remaining: Option<i64>,
    }

    // Discover workspace
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;

    // Get all tensions for prefix resolution
    let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
    let resolver = werk::prefix::PrefixResolver::new(tensions);

    // Resolve the ID/prefix
    let tension = resolver.resolve(&id)?;

    match value {
        Some(new_value) => {
            // Set or clear horizon
            let horizon_parsed = if new_value.to_lowercase() == "none" {
                None
            } else {
                Some(Horizon::parse(&new_value).map_err(|e| {
                    WerkError::InvalidInput(format!(
                        "Invalid horizon '{}': {}. Examples: 2026, 2026-05, 2026-05-15, 2026-05-15T14:00:00Z",
                        new_value, e
                    ))
                })?)
            };

            // Check status - only Active tensions can have horizon updated
            if tension.status != TensionStatus::Active {
                return Err(WerkError::InvalidInput(format!(
                    "cannot update horizon on {} tension (must be Active)",
                    tension.status
                )));
            }

            // Record old horizon
            let old_horizon = tension.horizon.as_ref().map(|h| h.to_string());

            // Update horizon
            store
                .update_horizon(&tension.id, horizon_parsed.clone())
                .map_err(WerkError::SdError)?;

            let result = HorizonResult {
                id: tension.id.clone(),
                horizon: horizon_parsed.as_ref().map(|h| h.to_string()),
                old_horizon,
            };

            if output.is_json() {
                let json = serde_json::to_string_pretty(&result)
                    .map_err(|e| WerkError::IoError(format!("failed to serialize JSON: {}", e)))?;
                println!("{}", json);
            } else {
                let id_styled = output.styled(&tension.id, werk::output::ColorStyle::Id);
                match &horizon_parsed {
                    Some(h) => {
                        output
                            .success(&format!(
                                "Set horizon for tension {} to {}",
                                id_styled,
                                output.styled(&h.to_string(), werk::output::ColorStyle::Highlight)
                            ))
                            .map_err(|e| WerkError::IoError(e.to_string()))?;
                    }
                    None => {
                        output
                            .success(&format!("Cleared horizon for tension {}", id_styled))
                            .map_err(|e| WerkError::IoError(e.to_string()))?;
                    }
                }
            }

            Ok(())
        }
        None => {
            // Display current horizon with urgency
            let now = Utc::now();
            let urgency = compute_urgency(tension, now);

            let days_remaining = tension.horizon.as_ref().and_then(|h| {
                let remaining = h.range_end().signed_duration_since(now).num_days();
                if remaining >= 0 {
                    Some(remaining)
                } else {
                    None
                }
            });

            let result = HorizonDisplayResult {
                id: tension.id.clone(),
                horizon: tension.horizon.as_ref().map(|h| h.to_string()),
                urgency: urgency.as_ref().map(|u| u.value),
                days_remaining,
            };

            if output.is_json() {
                let json = serde_json::to_string_pretty(&result)
                    .map_err(|e| WerkError::IoError(format!("failed to serialize JSON: {}", e)))?;
                println!("{}", json);
            } else {
                let id_styled = output.styled(&tension.id, werk::output::ColorStyle::Id);
                println!("Tension {}", id_styled);

                match &tension.horizon {
                    Some(h) => {
                        println!(
                            "  Horizon: {}",
                            output.styled(&h.to_string(), werk::output::ColorStyle::Highlight)
                        );

                        // Human interpretation
                        let interpretation = match h.kind() {
                            sd_core::HorizonKind::Year(y) => format!("Year {}", y),
                            sd_core::HorizonKind::Month(y, m) => format!("{}-{:02}", y, m),
                            sd_core::HorizonKind::Day(d) => d.format("%Y-%m-%d").to_string(),
                            sd_core::HorizonKind::DateTime(_) => h.to_string(),
                        };
                        println!(
                            "  Interpreted: {}",
                            output.styled(&interpretation, werk::output::ColorStyle::Muted)
                        );

                        if let Some(urg) = &urgency {
                            let urgency_pct = (urg.value * 100.0).min(999.0);
                            println!("  Urgency:    {:.0}% of time window elapsed", urgency_pct);
                        }

                        if let Some(days) = days_remaining {
                            println!(
                                "  Days remaining: {}",
                                output.styled(
                                    &format!("{}", days),
                                    werk::output::ColorStyle::Highlight
                                )
                            );
                        }
                    }
                    None => {
                        println!(
                            "  Horizon:    {}",
                            output.styled("None", werk::output::ColorStyle::Muted)
                        );
                    }
                }
            }

            Ok(())
        }
    }
}

fn cmd_show(output: &Output, id: String, verbose: bool) -> Result<(), WerkError> {
    use chrono::Utc;
    use sd_core::{
        classify_creative_cycle_phase, classify_orientation, compute_structural_tension,
        compute_urgency, detect_compensating_strategy, detect_horizon_drift, detect_neglect,
        detect_oscillation, detect_resolution, detect_structural_conflict,
        measure_assimilation_depth, predict_structural_tendency, AssimilationDepthThresholds,
        CompensatingStrategyThresholds, ConflictThresholds, CreativeCyclePhase, Forest,
        LifecycleThresholds, NeglectThresholds, OrientationThresholds, OscillationThresholds,
        ResolutionThresholds, TensionStatus,
    };
    use serde::Serialize;
    use werk::workspace::Workspace;

    /// JSON output structure for show command.
    #[derive(Serialize)]
    struct ShowResult {
        id: String,
        desired: String,
        actual: String,
        status: String,
        parent_id: Option<String>,
        created_at: String,
        horizon: Option<String>,
        horizon_range: Option<HorizonRangeJson>,
        urgency: Option<f64>,
        pressure: Option<f64>,
        dynamics: DynamicsJson,
        mutations: Vec<MutationInfo>,
        children: Vec<ChildInfo>,
    }

    #[derive(Serialize)]
    struct HorizonRangeJson {
        start: String,
        end: String,
    }

    /// All 10 dynamics in JSON format.
    #[derive(Serialize)]
    struct DynamicsJson {
        structural_tension: Option<StructuralTensionJson>,
        structural_conflict: Option<ConflictJson>,
        oscillation: Option<OscillationJson>,
        resolution: Option<ResolutionJson>,
        phase: PhaseJson,
        orientation: Option<OrientationJson>,
        compensating_strategy: Option<CompensatingStrategyJson>,
        structural_tendency: TendencyJson,
        assimilation_depth: Option<AssimilationDepthJson>,
        neglect: Option<NeglectJson>,
    }

    #[derive(Serialize)]
    struct StructuralTensionJson {
        magnitude: f64,
        has_gap: bool,
    }

    #[derive(Serialize)]
    struct ConflictJson {
        pattern: String,
        tension_ids: Vec<String>,
    }

    #[derive(Serialize)]
    struct OscillationJson {
        reversals: usize,
        magnitude: f64,
        window_start: String,
        window_end: String,
    }

    #[derive(Serialize)]
    struct ResolutionJson {
        velocity: f64,
        trend: String,
        window_start: String,
        window_end: String,
    }

    #[derive(Serialize)]
    struct PhaseJson {
        phase: String,
        evidence: PhaseEvidenceJson,
    }

    #[derive(Serialize)]
    struct PhaseEvidenceJson {
        mutation_count: usize,
        gap_closing: bool,
        convergence_ratio: f64,
        age_seconds: i64,
    }

    #[derive(Serialize)]
    struct OrientationJson {
        orientation: String,
        creative_ratio: f64,
        problem_solving_ratio: f64,
        reactive_ratio: f64,
    }

    #[derive(Serialize)]
    struct CompensatingStrategyJson {
        strategy_type: String,
        persistence_seconds: i64,
    }

    #[derive(Serialize)]
    struct TendencyJson {
        tendency: String,
        has_conflict: bool,
    }

    #[derive(Serialize)]
    struct AssimilationDepthJson {
        depth: String,
        mutation_frequency: f64,
        frequency_trend: f64,
    }

    #[derive(Serialize)]
    struct NeglectJson {
        neglect_type: String,
        activity_ratio: f64,
    }

    /// Mutation information for display.
    #[derive(Serialize)]
    struct MutationInfo {
        timestamp: String,
        field: String,
        old_value: Option<String>,
        new_value: String,
    }

    /// Child information for display.
    #[derive(Serialize)]
    struct ChildInfo {
        id: String,
        id_prefix: String,
        desired: String,
        status: String,
    }

    // Discover workspace
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;

    // Get all tensions for prefix resolution
    let all_tensions = store.list_tensions().map_err(WerkError::StoreError)?;
    let resolver = werk::prefix::PrefixResolver::new(all_tensions.clone());

    // Resolve the ID/prefix
    let tension = resolver.resolve(&id)?;

    // Get mutations for this tension
    let mutations = store
        .get_mutations(&tension.id)
        .map_err(WerkError::StoreError)?;

    // Get all mutations for conflict and orientation detection
    let all_mutations = store.all_mutations().map_err(WerkError::StoreError)?;

    // Build forest for conflict/neglect detection and children finding
    let forest = Forest::from_tensions(all_tensions.clone())
        .map_err(|e| WerkError::InvalidInput(e.to_string()))?;

    // Get children
    let children: Vec<ChildInfo> = forest
        .children(&tension.id)
        .unwrap_or_default()
        .iter()
        .map(|child| ChildInfo {
            id: child.id().to_string(),
            id_prefix: child.id()[..8.min(child.id().len())].to_string(),
            desired: truncate(&child.tension.desired, 40),
            status: child.tension.status.to_string(),
        })
        .collect();

    // Get siblings for conflict detection (used implicitly by detect_structural_conflict)
    let _siblings: Vec<_> = forest
        .siblings(&tension.id)
        .unwrap_or_default()
        .iter()
        .map(|s| s.id().to_string())
        .collect();

    // Get resolved tensions for momentum phase detection
    let resolved_tensions: Vec<_> = all_tensions
        .iter()
        .filter(|t| t.status == TensionStatus::Resolved)
        .cloned()
        .collect();

    // Compute dynamics
    let now = Utc::now();
    let lifecycle_thresholds = LifecycleThresholds::default();
    let conflict_thresholds = ConflictThresholds::default();
    let oscillation_thresholds = OscillationThresholds::default();
    let resolution_thresholds = ResolutionThresholds::default();
    let orientation_thresholds = OrientationThresholds::default();
    let compensating_thresholds = CompensatingStrategyThresholds::default();
    let assimilation_thresholds = AssimilationDepthThresholds::default();
    let neglect_thresholds = NeglectThresholds::default();

    // 1. Structural Tension
    let structural_tension = compute_structural_tension(tension);

    // Horizon dynamics
    let urgency = compute_urgency(tension, now);
    let horizon_drift = detect_horizon_drift(&tension.id, &mutations);

    // 2. Structural Conflict
    let conflict = detect_structural_conflict(
        &forest,
        &tension.id,
        &all_mutations,
        &conflict_thresholds,
        now,
    );

    // 3. Oscillation
    let oscillation = detect_oscillation(
        &tension.id,
        &mutations,
        &oscillation_thresholds,
        now,
        tension.horizon.as_ref(),
    );

    // 4. Resolution
    let resolution = detect_resolution(tension, &mutations, &resolution_thresholds, now);

    // 5. Creative Cycle Phase
    let phase_result = classify_creative_cycle_phase(
        tension,
        &mutations,
        &resolved_tensions,
        &lifecycle_thresholds,
        now,
    );

    // 6. Orientation (requires multiple tensions)
    let orientation =
        classify_orientation(&all_tensions, &all_mutations, &orientation_thresholds, now);

    // 7. Compensating Strategy
    let compensating_strategy = detect_compensating_strategy(
        &tension.id,
        &mutations,
        oscillation.as_ref(),
        &compensating_thresholds,
        now,
        tension.horizon.as_ref(),
    );

    // 8. Structural Tendency
    let has_conflict = conflict.is_some();
    let tendency_result = predict_structural_tendency(tension, has_conflict, Some(now));

    // 9. Assimilation Depth
    let assimilation = measure_assimilation_depth(
        &tension.id,
        &mutations,
        tension,
        &assimilation_thresholds,
        now,
    );

    // 10. Neglect
    let neglect = detect_neglect(
        &forest,
        &tension.id,
        &all_mutations,
        &neglect_thresholds,
        now,
    );

    // Build mutation info (last 10, chronological order - oldest first)
    let mutation_infos: Vec<MutationInfo> = mutations
        .iter()
        .rev()
        .take(10)
        .rev()
        .map(|m| MutationInfo {
            timestamp: m.timestamp().to_rfc3339(),
            field: m.field().to_owned(),
            old_value: m.old_value().map(|s| s.to_owned()),
            new_value: m.new_value().to_owned(),
        })
        .collect();

    // Build dynamics JSON
    let dynamics_json = DynamicsJson {
        structural_tension: structural_tension.as_ref().map(|st| StructuralTensionJson {
            magnitude: st.magnitude,
            has_gap: st.has_gap,
        }),
        structural_conflict: conflict.as_ref().map(|c| ConflictJson {
            pattern: match c.pattern {
                sd_core::ConflictPattern::AsymmetricActivity => "AsymmetricActivity".to_string(),
                sd_core::ConflictPattern::CompetingTensions => "CompetingTensions".to_string(),
            },
            tension_ids: c.tension_ids.clone(),
        }),
        oscillation: oscillation.as_ref().map(|o| OscillationJson {
            reversals: o.reversals,
            magnitude: o.magnitude,
            window_start: o.window_start.to_rfc3339(),
            window_end: o.window_end.to_rfc3339(),
        }),
        resolution: resolution.as_ref().map(|r| ResolutionJson {
            velocity: r.velocity,
            trend: match r.trend {
                sd_core::ResolutionTrend::Accelerating => "Accelerating".to_string(),
                sd_core::ResolutionTrend::Steady => "Steady".to_string(),
                sd_core::ResolutionTrend::Decelerating => "Decelerating".to_string(),
            },
            window_start: r.window_start.to_rfc3339(),
            window_end: r.window_end.to_rfc3339(),
        }),
        phase: PhaseJson {
            phase: match phase_result.phase {
                CreativeCyclePhase::Germination => "Germination".to_string(),
                CreativeCyclePhase::Assimilation => "Assimilation".to_string(),
                CreativeCyclePhase::Completion => "Completion".to_string(),
                CreativeCyclePhase::Momentum => "Momentum".to_string(),
            },
            evidence: PhaseEvidenceJson {
                mutation_count: phase_result.evidence.mutation_count,
                gap_closing: phase_result.evidence.gap_closing,
                convergence_ratio: phase_result.evidence.convergence_ratio,
                age_seconds: phase_result.evidence.age_seconds,
            },
        },
        orientation: orientation.as_ref().map(|o| OrientationJson {
            orientation: match o.orientation {
                sd_core::Orientation::Creative => "Creative".to_string(),
                sd_core::Orientation::ProblemSolving => "ProblemSolving".to_string(),
                sd_core::Orientation::ReactiveResponsive => "ReactiveResponsive".to_string(),
            },
            creative_ratio: o.evidence.creative_ratio,
            problem_solving_ratio: o.evidence.problem_solving_ratio,
            reactive_ratio: o.evidence.reactive_ratio,
        }),
        compensating_strategy: compensating_strategy
            .as_ref()
            .map(|cs| CompensatingStrategyJson {
                strategy_type: match cs.strategy_type {
                    sd_core::CompensatingStrategyType::TolerableConflict => {
                        "TolerableConflict".to_string()
                    }
                    sd_core::CompensatingStrategyType::ConflictManipulation => {
                        "ConflictManipulation".to_string()
                    }
                    sd_core::CompensatingStrategyType::WillpowerManipulation => {
                        "WillpowerManipulation".to_string()
                    }
                },
                persistence_seconds: cs.persistence_seconds,
            }),
        structural_tendency: TendencyJson {
            tendency: match tendency_result.tendency {
                sd_core::StructuralTendency::Advancing => "Advancing".to_string(),
                sd_core::StructuralTendency::Oscillating => "Oscillating".to_string(),
                sd_core::StructuralTendency::Stagnant => "Stagnant".to_string(),
            },
            has_conflict: tendency_result.has_conflict,
        },
        assimilation_depth: if assimilation.depth == sd_core::AssimilationDepth::None
            && assimilation.evidence.total_mutations == 0
        {
            None
        } else {
            Some(AssimilationDepthJson {
                depth: match assimilation.depth {
                    sd_core::AssimilationDepth::Shallow => "Shallow".to_string(),
                    sd_core::AssimilationDepth::Deep => "Deep".to_string(),
                    sd_core::AssimilationDepth::None => "None".to_string(),
                },
                mutation_frequency: assimilation.mutation_frequency,
                frequency_trend: assimilation.frequency_trend,
            })
        },
        neglect: neglect.as_ref().map(|n| NeglectJson {
            neglect_type: match n.neglect_type {
                sd_core::NeglectType::ParentNeglectsChildren => {
                    "ParentNeglectsChildren".to_string()
                }
                sd_core::NeglectType::ChildrenNeglected => "ChildrenNeglected".to_string(),
            },
            activity_ratio: n.activity_ratio,
        }),
    };

    let result = ShowResult {
        id: tension.id.clone(),
        desired: tension.desired.clone(),
        actual: tension.actual.clone(),
        status: tension.status.to_string(),
        parent_id: tension.parent_id.clone(),
        created_at: tension.created_at.to_rfc3339(),
        horizon: tension.horizon.as_ref().map(|h| h.to_string()),
        horizon_range: tension.horizon.as_ref().map(|h| HorizonRangeJson {
            start: h.range_start().to_rfc3339(),
            end: h.range_end().to_rfc3339(),
        }),
        urgency: urgency.as_ref().map(|u| u.value),
        pressure: structural_tension.as_ref().and_then(|st| st.pressure),
        dynamics: dynamics_json,
        mutations: mutation_infos,
        children,
    };

    if output.is_json() {
        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| WerkError::IoError(format!("failed to serialize JSON: {}", e)))?;
        println!("{}", json);
    } else {
        // Human-readable output
        let id_styled = output.styled(&tension.id, werk::output::ColorStyle::Id);
        let status_style = match tension.status {
            TensionStatus::Active => werk::output::ColorStyle::Active,
            TensionStatus::Resolved => werk::output::ColorStyle::Resolved,
            TensionStatus::Released => werk::output::ColorStyle::Released,
        };
        let status_styled = output.styled(&tension.status.to_string(), status_style);

        println!("Tension {}", id_styled);
        println!(
            "  Desired:    {}",
            output.styled(&tension.desired, werk::output::ColorStyle::Highlight)
        );
        println!(
            "  Actual:     {}",
            output.styled(&tension.actual, werk::output::ColorStyle::Muted)
        );
        println!("  Status:     {}", status_styled);
        println!(
            "  Created:    {}",
            output.styled(
                &tension
                    .created_at
                    .format("%Y-%m-%d %H:%M:%S UTC")
                    .to_string(),
                werk::output::ColorStyle::Muted
            )
        );

        if let Some(pid) = &tension.parent_id {
            println!(
                "  Parent:     {}",
                output.styled(pid, werk::output::ColorStyle::Id)
            );
        }

        // Horizon display
        if let Some(h) = &tension.horizon {
            let horizon_str = h.to_string();
            // Human interpretation
            let interpretation = match h.kind() {
                sd_core::HorizonKind::Year(y) => format!("Year {}", y),
                sd_core::HorizonKind::Month(y, m) => {
                    let month_names = [
                        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct",
                        "Nov", "Dec",
                    ];
                    format!("{} {}", month_names[(m - 1) as usize], y)
                }
                sd_core::HorizonKind::Day(d) => d.format("%B %d, %Y").to_string(),
                sd_core::HorizonKind::DateTime(dt) => dt.format("%B %d, %Y %H:%M UTC").to_string(),
            };

            // Days remaining
            let days_remaining = h.range_end().signed_duration_since(now).num_days();
            let days_str = if days_remaining > 0 {
                format!(", {} days remaining", days_remaining)
            } else if days_remaining == 0 {
                ", today is the horizon".to_string()
            } else {
                format!(", {} days past horizon", -days_remaining)
            };

            println!(
                "  Horizon:    {} ({}{})",
                output.styled(&horizon_str, werk::output::ColorStyle::Highlight),
                output.styled(&interpretation, werk::output::ColorStyle::Muted),
                output.styled(&days_str, werk::output::ColorStyle::Muted)
            );
        }

        // Mutation count
        println!(
            "  Mutations:  {}",
            output.styled(
                &format!("{}", mutations.len()),
                werk::output::ColorStyle::Info
            )
        );

        // Children count
        if !result.children.is_empty() {
            println!(
                "  Children:   {}",
                output.styled(
                    &format!("{}", result.children.len()),
                    werk::output::ColorStyle::Info
                )
            );
        }

        // === Dynamics Summary (always shown) ===
        println!();
        println!("Dynamics:");

        // Phase (always shown)
        let phase_display =
            output.styled(&result.dynamics.phase.phase, werk::output::ColorStyle::Info);
        println!(
            "  Phase:      {} (mutations: {}, convergence: {:.0}%)",
            phase_display,
            result.dynamics.phase.evidence.mutation_count,
            (1.0 - result.dynamics.phase.evidence.convergence_ratio) * 100.0
        );

        // Structural Tension (show magnitude)
        match &result.dynamics.structural_tension {
            Some(st) => {
                println!(
                    "  Magnitude:  {}",
                    output.styled(
                        &format!("{:.2}", st.magnitude),
                        werk::output::ColorStyle::Highlight
                    )
                );
            }
            None => {
                println!(
                    "  Magnitude:  {}",
                    output.styled("None (no gap)", werk::output::ColorStyle::Muted)
                );
            }
        }

        // Conflict (show if present, else None)
        match &result.dynamics.structural_conflict {
            Some(c) => {
                println!(
                    "  Conflict:   {} with {} tensions",
                    output.styled(&c.pattern, werk::output::ColorStyle::Error),
                    c.tension_ids.len()
                );
            }
            None => {
                println!(
                    "  Conflict:   {}",
                    output.styled("None", werk::output::ColorStyle::Muted)
                );
            }
        }

        // Movement/Tendency
        let movement_symbol = match tendency_result.tendency {
            sd_core::StructuralTendency::Advancing => "→",
            sd_core::StructuralTendency::Oscillating => "↔",
            sd_core::StructuralTendency::Stagnant => "○",
        };
        println!(
            "  Movement:   {} {}",
            movement_symbol,
            output.styled(
                &result.dynamics.structural_tendency.tendency,
                werk::output::ColorStyle::Info
            )
        );

        // === Verbose: Show all 10 dynamics ===
        if verbose {
            println!();
            println!("All Dynamics:");

            // Horizon dynamics (if present)
            if tension.horizon.is_some() {
                // Urgency
                match &urgency {
                    Some(urg) => {
                        let pct = (urg.value * 100.0).min(999.0);
                        println!(
                            "  Urgency:      {:.0}% ({}s remaining of {}s window)",
                            pct, urg.time_remaining, urg.total_window
                        );
                    }
                    None => {
                        println!(
                            "  Urgency:      {}",
                            output.styled("None", werk::output::ColorStyle::Muted)
                        );
                    }
                }

                // Pressure
                match &structural_tension {
                    Some(st) if st.pressure.is_some() => {
                        println!(
                            "  Pressure:     {:.2} (magnitude * urgency)",
                            st.pressure.unwrap()
                        );
                    }
                    _ => {
                        println!(
                            "  Pressure:     {}",
                            output.styled("None", werk::output::ColorStyle::Muted)
                        );
                    }
                }

                // Horizon drift
                println!(
                    "  HorizonDrift: {} ({} changes, net shift {}s)",
                    match horizon_drift.drift_type {
                        sd_core::HorizonDriftType::Stable => "Stable",
                        sd_core::HorizonDriftType::Tightening => "Tightening",
                        sd_core::HorizonDriftType::Postponement => "Postponement",
                        sd_core::HorizonDriftType::RepeatedPostponement => "RepeatedPostponement",
                        sd_core::HorizonDriftType::Loosening => "Loosening",
                        sd_core::HorizonDriftType::Oscillating => "Oscillating",
                    },
                    horizon_drift.change_count,
                    horizon_drift.net_shift_seconds
                );

                println!();
            }

            // 1. Structural Tension
            match &result.dynamics.structural_tension {
                Some(st) => {
                    println!(
                        "  StructuralTension: magnitude={:.2}, has_gap={}",
                        st.magnitude, st.has_gap
                    );
                }
                None => {
                    println!(
                        "  StructuralTension: {}",
                        output.styled("None", werk::output::ColorStyle::Muted)
                    );
                }
            }

            // 2. Structural Conflict
            match &result.dynamics.structural_conflict {
                Some(c) => {
                    println!(
                        "  StructuralConflict: pattern={}, tensions={}",
                        c.pattern,
                        c.tension_ids.join(", ")
                    );
                }
                None => {
                    println!(
                        "  StructuralConflict: {}",
                        output.styled("None", werk::output::ColorStyle::Muted)
                    );
                }
            }

            // 3. Oscillation
            match &result.dynamics.oscillation {
                Some(o) => {
                    println!(
                        "  Oscillation: reversals={}, magnitude={:.2}",
                        o.reversals, o.magnitude
                    );
                }
                None => {
                    println!(
                        "  Oscillation: {}",
                        output.styled("None", werk::output::ColorStyle::Muted)
                    );
                }
            }

            // 4. Resolution
            match &result.dynamics.resolution {
                Some(r) => {
                    println!(
                        "  Resolution: velocity={:.2}, trend={}",
                        r.velocity, r.trend
                    );
                }
                None => {
                    println!(
                        "  Resolution: {}",
                        output.styled("None", werk::output::ColorStyle::Muted)
                    );
                }
            }

            // 5. Creative Cycle Phase (already in summary)
            println!(
                "  CreativeCyclePhase: phase={}, mutations={}, convergence={:.0}%",
                result.dynamics.phase.phase,
                result.dynamics.phase.evidence.mutation_count,
                (1.0 - result.dynamics.phase.evidence.convergence_ratio) * 100.0
            );

            // 6. Orientation
            match &result.dynamics.orientation {
                Some(o) => {
                    println!(
                        "  Orientation: {} (creative={:.0}%, problem={:.0}%, reactive={:.0}%)",
                        o.orientation,
                        o.creative_ratio * 100.0,
                        o.problem_solving_ratio * 100.0,
                        o.reactive_ratio * 100.0
                    );
                }
                None => {
                    println!(
                        "  Orientation: {}",
                        output.styled("None", werk::output::ColorStyle::Muted)
                    );
                }
            }

            // 7. Compensating Strategy
            match &result.dynamics.compensating_strategy {
                Some(cs) => {
                    println!(
                        "  CompensatingStrategy: type={}, persistence={}s",
                        cs.strategy_type, cs.persistence_seconds
                    );
                }
                None => {
                    println!(
                        "  CompensatingStrategy: {}",
                        output.styled("None", werk::output::ColorStyle::Muted)
                    );
                }
            }

            // 8. Structural Tendency (already in summary)
            println!(
                "  StructuralTendency: tendency={}, has_conflict={}",
                result.dynamics.structural_tendency.tendency,
                result.dynamics.structural_tendency.has_conflict
            );

            // 9. Assimilation Depth
            match &result.dynamics.assimilation_depth {
                Some(a) => {
                    println!(
                        "  AssimilationDepth: depth={}, frequency={:.2}, trend={:.2}",
                        a.depth, a.mutation_frequency, a.frequency_trend
                    );
                }
                None => {
                    println!(
                        "  AssimilationDepth: {}",
                        output.styled("None", werk::output::ColorStyle::Muted)
                    );
                }
            }

            // 10. Neglect
            match &result.dynamics.neglect {
                Some(n) => {
                    println!(
                        "  Neglect: type={}, ratio={:.2}",
                        n.neglect_type, n.activity_ratio
                    );
                }
                None => {
                    println!(
                        "  Neglect: {}",
                        output.styled("None", werk::output::ColorStyle::Muted)
                    );
                }
            }
        }

        // === Mutation History (last 10) ===
        println!();
        println!("Mutation History:");
        for m in &result.mutations {
            let old = m.old_value.as_deref().unwrap_or("(none)");
            println!(
                "  {} [{}] {} -> {}",
                output.styled(
                    &m.timestamp[..19].replace('T', " "),
                    werk::output::ColorStyle::Muted
                ),
                output.styled(&m.field, werk::output::ColorStyle::Info),
                output.styled(old, werk::output::ColorStyle::Muted),
                output.styled(&m.new_value, werk::output::ColorStyle::Highlight)
            );
        }

        // === Children List ===
        if !result.children.is_empty() {
            println!();
            println!("Children:");
            for child in &result.children {
                let status_style = match child.status.as_str() {
                    "Active" => werk::output::ColorStyle::Active,
                    "Resolved" => werk::output::ColorStyle::Resolved,
                    "Released" => werk::output::ColorStyle::Released,
                    _ => werk::output::ColorStyle::Muted,
                };
                println!(
                    "  {} {} [{}] {}",
                    output.styled(&child.id_prefix, werk::output::ColorStyle::Id),
                    output.styled(&child.status, status_style),
                    output.styled(&child.status, status_style),
                    output.styled(&child.desired, werk::output::ColorStyle::Muted)
                );
            }
        }
    }

    Ok(())
}

fn cmd_reality(output: &Output, id: String, value: Option<String>) -> Result<(), WerkError> {
    use serde::Serialize;
    use werk::workspace::Workspace;

    /// JSON output structure for reality command.
    #[derive(Serialize)]
    struct RealityResult {
        id: String,
        actual: String,
        old_actual: String,
    }

    // Discover workspace
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;

    // Get all tensions for prefix resolution
    let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
    let resolver = werk::prefix::PrefixResolver::new(tensions);

    // Resolve the ID/prefix
    let tension = resolver.resolve(&id)?;

    // Get the new value - either from argument or editor
    let new_value = match value {
        Some(v) => v,
        None => {
            // Open editor with current actual
            let edited = werk::edit_content(&tension.actual)?;
            match edited {
                Some(v) => v,
                None => {
                    // Editor returned no change - nothing to do
                    if output.is_json() {
                        let result = RealityResult {
                            id: tension.id.clone(),
                            actual: tension.actual.clone(),
                            old_actual: tension.actual.clone(),
                        };
                        let json = serde_json::to_string_pretty(&result).map_err(|e| {
                            WerkError::IoError(format!("failed to serialize JSON: {}", e))
                        })?;
                        println!("{}", json);
                    } else {
                        output
                            .info("No changes made (editor cancelled or content unchanged)")
                            .map_err(|e| WerkError::IoError(e.to_string()))?;
                    }
                    return Ok(());
                }
            }
        }
    };

    // Validate non-empty
    if new_value.is_empty() {
        return Err(WerkError::InvalidInput(
            "actual state cannot be empty".to_string(),
        ));
    }

    // Record old value for output
    let old_actual = tension.actual.clone();

    // Update via store (this handles status validation and mutation recording)
    store
        .update_actual(&tension.id, &new_value)
        .map_err(WerkError::SdError)?;

    let result = RealityResult {
        id: tension.id.clone(),
        actual: new_value,
        old_actual,
    };

    if output.is_json() {
        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| WerkError::IoError(format!("failed to serialize JSON: {}", e)))?;
        println!("{}", json);
    } else {
        // Human-readable output
        let id_styled = output.styled(&tension.id, werk::output::ColorStyle::Id);
        output
            .success(&format!("Updated actual for tension {}", id_styled))
            .map_err(|e| WerkError::IoError(e.to_string()))?;
        println!(
            "  Old:  {}",
            output.styled(&result.old_actual, werk::output::ColorStyle::Muted)
        );
        println!(
            "  New:  {}",
            output.styled(&result.actual, werk::output::ColorStyle::Highlight)
        );
    }

    Ok(())
}

fn cmd_desire(output: &Output, id: String, value: Option<String>) -> Result<(), WerkError> {
    use serde::Serialize;
    use werk::workspace::Workspace;

    /// JSON output structure for desire command.
    #[derive(Serialize)]
    struct DesireResult {
        id: String,
        desired: String,
        old_desired: String,
    }

    // Discover workspace
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;

    // Get all tensions for prefix resolution
    let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
    let resolver = werk::prefix::PrefixResolver::new(tensions);

    // Resolve the ID/prefix
    let tension = resolver.resolve(&id)?;

    // Get the new value - either from argument or editor
    let new_value = match value {
        Some(v) => v,
        None => {
            // Open editor with current desired
            let edited = werk::edit_content(&tension.desired)?;
            match edited {
                Some(v) => v,
                None => {
                    // Editor returned no change - nothing to do
                    if output.is_json() {
                        let result = DesireResult {
                            id: tension.id.clone(),
                            desired: tension.desired.clone(),
                            old_desired: tension.desired.clone(),
                        };
                        let json = serde_json::to_string_pretty(&result).map_err(|e| {
                            WerkError::IoError(format!("failed to serialize JSON: {}", e))
                        })?;
                        println!("{}", json);
                    } else {
                        output
                            .info("No changes made (editor cancelled or content unchanged)")
                            .map_err(|e| WerkError::IoError(e.to_string()))?;
                    }
                    return Ok(());
                }
            }
        }
    };

    // Validate non-empty
    if new_value.is_empty() {
        return Err(WerkError::InvalidInput(
            "desired state cannot be empty".to_string(),
        ));
    }

    // Record old value for output
    let old_desired = tension.desired.clone();

    // Update via store (this handles status validation and mutation recording)
    store
        .update_desired(&tension.id, &new_value)
        .map_err(WerkError::SdError)?;

    let result = DesireResult {
        id: tension.id.clone(),
        desired: new_value,
        old_desired,
    };

    if output.is_json() {
        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| WerkError::IoError(format!("failed to serialize JSON: {}", e)))?;
        println!("{}", json);
    } else {
        // Human-readable output
        let id_styled = output.styled(&tension.id, werk::output::ColorStyle::Id);
        output
            .success(&format!("Updated desired for tension {}", id_styled))
            .map_err(|e| WerkError::IoError(e.to_string()))?;
        println!(
            "  Old:  {}",
            output.styled(&result.old_desired, werk::output::ColorStyle::Muted)
        );
        println!(
            "  New:  {}",
            output.styled(&result.desired, werk::output::ColorStyle::Highlight)
        );
    }

    Ok(())
}

fn cmd_resolve(output: &Output, id: String) -> Result<(), WerkError> {
    use serde::Serialize;
    use werk::workspace::Workspace;

    /// JSON output structure for resolve command.
    #[derive(Serialize)]
    struct ResolveResult {
        id: String,
        status: String,
    }

    // Discover workspace
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;

    // Get all tensions for prefix resolution
    let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
    let resolver = werk::prefix::PrefixResolver::new(tensions);

    // Resolve the ID/prefix
    let tension = resolver.resolve(&id)?;

    // Record old status for output
    let old_status = tension.status;

    // Check if already resolved
    if old_status != sd_core::TensionStatus::Active {
        return Err(WerkError::InvalidInput(format!(
            "cannot resolve tension with status {} (must be Active)",
            old_status
        )));
    }

    // Update status via store (handles validation and mutation recording)
    store
        .update_status(&tension.id, sd_core::TensionStatus::Resolved)
        .map_err(WerkError::SdError)?;

    let result = ResolveResult {
        id: tension.id.clone(),
        status: "Resolved".to_string(),
    };

    if output.is_json() {
        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| WerkError::IoError(format!("failed to serialize JSON: {}", e)))?;
        println!("{}", json);
    } else {
        // Human-readable output
        let id_styled = output.styled(&tension.id, werk::output::ColorStyle::Id);
        output
            .success(&format!("Resolved tension {}", id_styled))
            .map_err(|e| WerkError::IoError(e.to_string()))?;
        println!(
            "  Status: {} -> {}",
            output.styled(&old_status.to_string(), werk::output::ColorStyle::Muted),
            output.styled("Resolved", werk::output::ColorStyle::Resolved)
        );
    }

    Ok(())
}

fn cmd_release(output: &Output, id: String, reason: String) -> Result<(), WerkError> {
    use serde::Serialize;
    use werk::workspace::Workspace;

    /// JSON output structure for release command.
    #[derive(Serialize)]
    struct ReleaseResult {
        id: String,
        status: String,
        reason: String,
    }

    // Discover workspace
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;

    // Get all tensions for prefix resolution
    let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
    let resolver = werk::prefix::PrefixResolver::new(tensions);

    // Resolve the ID/prefix
    let tension = resolver.resolve(&id)?;

    // Record old status for output
    let old_status = tension.status;

    // Check if already resolved/released
    if old_status != sd_core::TensionStatus::Active {
        return Err(WerkError::InvalidInput(format!(
            "cannot release tension with status {} (must be Active)",
            old_status
        )));
    }

    // Update status via store (handles validation and mutation recording)
    store
        .update_status(&tension.id, sd_core::TensionStatus::Released)
        .map_err(WerkError::SdError)?;

    // Record the release reason as a mutation
    use chrono::Utc;
    use sd_core::Mutation;
    store
        .record_mutation(&Mutation::new(
            tension.id.clone(),
            Utc::now(),
            "release_reason".to_owned(),
            None,
            reason.clone(),
        ))
        .map_err(WerkError::SdError)?;

    let result = ReleaseResult {
        id: tension.id.clone(),
        status: "Released".to_string(),
        reason: reason.clone(),
    };

    if output.is_json() {
        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| WerkError::IoError(format!("failed to serialize JSON: {}", e)))?;
        println!("{}", json);
    } else {
        // Human-readable output
        let id_styled = output.styled(&tension.id, werk::output::ColorStyle::Id);
        output
            .success(&format!("Released tension {}", id_styled))
            .map_err(|e| WerkError::IoError(e.to_string()))?;
        println!(
            "  Status: {} -> {}",
            output.styled(&old_status.to_string(), werk::output::ColorStyle::Muted),
            output.styled("Released", werk::output::ColorStyle::Released)
        );
        println!(
            "  Reason: {}",
            output.styled(&reason, werk::output::ColorStyle::Muted)
        );
    }

    Ok(())
}

fn cmd_rm(output: &Output, id: String) -> Result<(), WerkError> {
    use serde::Serialize;
    use werk::workspace::Workspace;

    /// JSON output structure for rm command.
    #[derive(Serialize)]
    struct RmResult {
        id: String,
        deleted: bool,
    }

    // Discover workspace
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;

    // Get all tensions for prefix resolution
    let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
    let resolver = werk::prefix::PrefixResolver::new(tensions);

    // Resolve the ID/prefix
    let tension = resolver.resolve(&id)?;

    // Record the tension ID before deletion
    let tension_id = tension.id.clone();
    let tension_desired = tension.desired.clone();

    // Delete via store (handles reparenting children to grandparent)
    store
        .delete_tension(&tension_id)
        .map_err(WerkError::SdError)?;

    let result = RmResult {
        id: tension_id.clone(),
        deleted: true,
    };

    if output.is_json() {
        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| WerkError::IoError(format!("failed to serialize JSON: {}", e)))?;
        println!("{}", json);
    } else {
        // Human-readable output
        let id_styled = output.styled(&tension_id, werk::output::ColorStyle::Id);
        output
            .success(&format!("Deleted tension {}", id_styled))
            .map_err(|e| WerkError::IoError(e.to_string()))?;
        println!(
            "  Desired: {}",
            output.styled(&tension_desired, werk::output::ColorStyle::Muted)
        );
    }

    Ok(())
}

fn cmd_move(output: &Output, id: String, parent: Option<String>) -> Result<(), WerkError> {
    use sd_core::Forest;
    use serde::Serialize;
    use werk::workspace::Workspace;

    /// JSON output structure for move command.
    #[derive(Serialize)]
    struct MoveResult {
        id: String,
        parent_id: Option<String>,
        old_parent_id: Option<String>,
    }

    // Discover workspace
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;

    // Get all tensions for prefix resolution and forest building
    let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
    let resolver = werk::prefix::PrefixResolver::new(tensions.clone());

    // Resolve the tension to move
    let tension = resolver.resolve(&id)?;
    let tension_id = tension.id.clone();
    let old_parent_id = tension.parent_id.clone();

    // Resolve the new parent if provided
    let new_parent_id = if let Some(parent_prefix) = parent {
        // Prevent moving to self
        let parent_tension = resolver.resolve(&parent_prefix)?;
        if parent_tension.id == tension_id {
            return Err(WerkError::InvalidInput(
                "cannot move tension to itself".to_string(),
            ));
        }

        // Check for cycles: new parent cannot be a descendant of the tension being moved
        let forest = Forest::from_tensions(tensions.clone())
            .map_err(|e| WerkError::InvalidInput(e.to_string()))?;

        if let Some(descendants) = forest.descendants(&tension_id) {
            let descendant_ids: std::collections::HashSet<_> =
                descendants.iter().map(|n| n.id()).collect();

            if descendant_ids.contains(parent_tension.id.as_str()) {
                return Err(WerkError::InvalidInput(
                    "cannot move tension under its descendant (would create cycle)".to_string(),
                ));
            }
        }

        Some(parent_tension.id.clone())
    } else {
        None
    };

    // Perform the move via store
    store
        .update_parent(&tension_id, new_parent_id.as_deref())
        .map_err(WerkError::SdError)?;

    let result = MoveResult {
        id: tension_id.clone(),
        parent_id: new_parent_id.clone(),
        old_parent_id,
    };

    if output.is_json() {
        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| WerkError::IoError(format!("failed to serialize JSON: {}", e)))?;
        println!("{}", json);
    } else {
        // Human-readable output
        let id_styled = output.styled(&tension_id, werk::output::ColorStyle::Id);
        match &new_parent_id {
            Some(pid) => {
                output
                    .success(&format!(
                        "Moved tension {} under {}",
                        id_styled,
                        output.styled(pid, werk::output::ColorStyle::Id)
                    ))
                    .map_err(|e| WerkError::IoError(e.to_string()))?;
            }
            None => {
                output
                    .success(&format!("Moved tension {} to root", id_styled))
                    .map_err(|e| WerkError::IoError(e.to_string()))?;
            }
        }
    }

    Ok(())
}

fn cmd_note(output: &Output, arg1: Option<String>, arg2: Option<String>) -> Result<(), WerkError> {
    use chrono::Utc;
    use sd_core::Mutation;
    use serde::Serialize;
    use werk::workspace::Workspace;

    /// JSON output structure for note command.
    #[derive(Serialize)]
    struct NoteResult {
        id: Option<String>,
        note: String,
    }

    // Parse arguments: determine ID and text
    let (id, text) = match (arg1, arg2) {
        (None, None) => {
            return Err(WerkError::InvalidInput(
                "note text is required: werk note <text> or werk note <id> <text>".to_string(),
            ));
        }
        (Some(text), None) => {
            // Single argument: treat as workspace note
            (None, text)
        }
        (Some(id), Some(text)) => {
            // Two arguments: first is ID, second is text
            (Some(id), text)
        }
        (None, Some(_)) => {
            // This shouldn't happen with clap, but handle it
            unreachable!("arg2 without arg1")
        }
    };

    // Discover workspace
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;

    let result = match id {
        Some(id_prefix) => {
            // Note on specific tension
            let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
            let resolver = werk::prefix::PrefixResolver::new(tensions);
            let tension = resolver.resolve(&id_prefix)?;

            // Record note mutation (notes work on any status, no validation needed)
            store
                .record_mutation(&Mutation::new(
                    tension.id.clone(),
                    Utc::now(),
                    "note".to_owned(),
                    None,
                    text.clone(),
                ))
                .map_err(WerkError::SdError)?;

            NoteResult {
                id: Some(tension.id.clone()),
                note: text.clone(),
            }
        }
        None => {
            // General workspace note - store as mutation on a sentinel ID
            // The sentinel is not a real tension but serves as an anchor for workspace-level notes
            const WORKSPACE_NOTE_TENSION_ID: &str = "WORKSPACE_NOTES";

            // Record note mutation on the sentinel
            store
                .record_mutation(&Mutation::new(
                    WORKSPACE_NOTE_TENSION_ID.to_owned(),
                    Utc::now(),
                    "note".to_owned(),
                    None,
                    text.clone(),
                ))
                .map_err(WerkError::SdError)?;

            NoteResult {
                id: None,
                note: text.clone(),
            }
        }
    };

    if output.is_json() {
        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| WerkError::IoError(format!("failed to serialize JSON: {}", e)))?;
        println!("{}", json);
    } else {
        // Human-readable output
        match &result.id {
            Some(tid) => {
                output
                    .success(&format!(
                        "Added note to tension {}",
                        output.styled(tid, werk::output::ColorStyle::Id)
                    ))
                    .map_err(|e| WerkError::IoError(e.to_string()))?;
            }
            None => {
                output
                    .success("Added workspace note")
                    .map_err(|e| WerkError::IoError(e.to_string()))?;
            }
        }
        println!(
            "  Note: {}",
            output.styled(&text, werk::output::ColorStyle::Muted)
        );
    }

    Ok(())
}

fn cmd_notes(output: &Output) -> Result<(), WerkError> {
    use serde::Serialize;
    use werk::workspace::Workspace;

    /// JSON output structure for notes command.
    #[derive(Serialize)]
    struct NotesResult {
        notes: Vec<NoteInfo>,
    }

    /// Note information for display.
    #[derive(Serialize)]
    struct NoteInfo {
        timestamp: String,
        text: String,
    }

    // Discover workspace
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;

    // Get workspace-level notes (mutations on the WORKSPACE_NOTES sentinel)
    const WORKSPACE_NOTE_TENSION_ID: &str = "WORKSPACE_NOTES";
    let mutations = store
        .get_mutations(WORKSPACE_NOTE_TENSION_ID)
        .map_err(WerkError::StoreError)?;

    // Filter for note mutations only
    let notes: Vec<NoteInfo> = mutations
        .into_iter()
        .filter(|m| m.field() == "note")
        .map(|m| NoteInfo {
            timestamp: m.timestamp().to_rfc3339(),
            text: m.new_value().to_owned(),
        })
        .collect();

    if output.is_json() {
        let result = NotesResult { notes };
        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| WerkError::IoError(format!("failed to serialize JSON: {}", e)))?;
        println!("{}", json);
    } else {
        // Human-readable output
        if notes.is_empty() {
            output
                .info("No workspace notes")
                .map_err(|e| WerkError::IoError(e.to_string()))?;
        } else {
            output
                .success(&format!("Workspace notes ({})", notes.len()))
                .map_err(|e| WerkError::IoError(e.to_string()))?;
            for (i, note) in notes.iter().enumerate() {
                println!(
                    "\n{}. {}",
                    i + 1,
                    output.styled(&note.text, werk::output::ColorStyle::Highlight)
                );
                println!(
                    "   {}",
                    output.styled(
                        &note.timestamp[..19].replace('T', " "),
                        werk::output::ColorStyle::Muted
                    )
                );
            }
        }
    }

    Ok(())
}

fn cmd_tree(
    output: &Output,
    _open: bool,
    all: bool,
    resolved: bool,
    released: bool,
) -> Result<(), WerkError> {
    use chrono::Utc;
    use sd_core::{
        classify_creative_cycle_phase, detect_structural_conflict, predict_structural_tendency,
        ConflictThresholds, Forest, LifecycleThresholds, TensionStatus,
    };
    use serde::Serialize;
    use werk::workspace::Workspace;

    /// JSON output structure for a tension in tree.
    #[derive(Serialize)]
    struct TensionJson {
        id: String,
        desired: String,
        actual: String,
        status: String,
        parent_id: Option<String>,
        created_at: String,
        horizon: Option<String>,
        phase: String,
        movement: String,
        has_conflict: bool,
    }

    /// JSON output structure for tree.
    #[derive(Serialize)]
    struct TreeJson {
        tensions: Vec<TensionJson>,
        summary: TreeSummary,
    }

    #[derive(Serialize)]
    struct TreeSummary {
        total: usize,
        active: usize,
        resolved: usize,
        released: usize,
    }

    // Discover workspace
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;

    // Get all tensions
    let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
    let all_mutations = store.all_mutations().map_err(WerkError::StoreError)?;

    // Build forest
    let forest = Forest::from_tensions(tensions.clone())
        .map_err(|e| WerkError::InvalidInput(e.to_string()))?;

    // Determine filter
    let filter = if all {
        Filter::All
    } else if resolved {
        Filter::Resolved
    } else if released {
        Filter::Released
    } else {
        // Default: --open (active only)
        Filter::Active
    };

    // Filter tensions
    let filtered_tensions: Vec<_> = tensions
        .iter()
        .filter(|t| match filter {
            Filter::All => true,
            Filter::Active => t.status == TensionStatus::Active,
            Filter::Resolved => t.status == TensionStatus::Resolved,
            Filter::Released => t.status == TensionStatus::Released,
        })
        .collect();

    // Handle empty forest
    if filtered_tensions.is_empty() {
        if output.is_json() {
            let result = TreeJson {
                tensions: vec![],
                summary: TreeSummary {
                    total: 0,
                    active: 0,
                    resolved: 0,
                    released: 0,
                },
            };
            let json = serde_json::to_string_pretty(&result)
                .map_err(|e| WerkError::IoError(format!("failed to serialize JSON: {}", e)))?;
            println!("{}", json);
        } else {
            output
                .info("No tensions found")
                .map_err(|e| WerkError::IoError(e.to_string()))?;
        }
        return Ok(());
    }

    // Compute dynamics for each tension
    let now = Utc::now();
    let thresholds = LifecycleThresholds::default();
    let conflict_thresholds = ConflictThresholds::default();

    // Get resolved tensions for momentum phase detection
    let resolved_tensions: Vec<_> = tensions
        .iter()
        .filter(|t| t.status == TensionStatus::Resolved)
        .cloned()
        .collect();

    // Build a map of tension ID to computed dynamics
    let mut dynamics_map: std::collections::HashMap<String, (String, String, bool)> =
        std::collections::HashMap::new();

    for tension in &filtered_tensions {
        // Get mutations for this tension
        let mutations: Vec<_> = all_mutations
            .iter()
            .filter(|m| m.tension_id() == tension.id)
            .cloned()
            .collect();

        // Classify phase
        let phase_result = classify_creative_cycle_phase(
            tension,
            &mutations,
            &resolved_tensions,
            &thresholds,
            now,
        );
        let phase_badge = match phase_result.phase {
            sd_core::CreativeCyclePhase::Germination => "[G]",
            sd_core::CreativeCyclePhase::Assimilation => "[A]",
            sd_core::CreativeCyclePhase::Completion => "[C]",
            sd_core::CreativeCyclePhase::Momentum => "[M]",
        };

        // Detect conflict with siblings
        let has_conflict = detect_structural_conflict(
            &forest,
            &tension.id,
            &all_mutations,
            &conflict_thresholds,
            now,
        )
        .is_some();

        // Predict movement tendency
        let tendency = predict_structural_tendency(tension, has_conflict, None);
        let movement_signal = match tendency.tendency {
            sd_core::StructuralTendency::Advancing => "→",
            sd_core::StructuralTendency::Oscillating => "↔",
            sd_core::StructuralTendency::Stagnant => "○",
        };

        dynamics_map.insert(
            tension.id.clone(),
            (
                phase_badge.to_string(),
                movement_signal.to_string(),
                has_conflict,
            ),
        );
    }

    // If JSON output, build JSON structure
    if output.is_json() {
        let json_tensions: Vec<TensionJson> = filtered_tensions
            .iter()
            .map(|t| {
                let (phase, movement, has_conflict) = dynamics_map.get(&t.id).cloned().unwrap_or((
                    "[G]".to_string(),
                    "○".to_string(),
                    false,
                ));
                TensionJson {
                    id: t.id.clone(),
                    desired: t.desired.clone(),
                    actual: t.actual.clone(),
                    status: t.status.to_string(),
                    parent_id: t.parent_id.clone(),
                    created_at: t.created_at.to_rfc3339(),
                    horizon: t.horizon.as_ref().map(|h| h.to_string()),
                    phase: phase.replace("[", "").replace("]", ""),
                    movement: movement.to_string(),
                    has_conflict,
                }
            })
            .collect();

        // Count by status
        let active_count = tensions
            .iter()
            .filter(|t| t.status == TensionStatus::Active)
            .count();
        let resolved_count = tensions
            .iter()
            .filter(|t| t.status == TensionStatus::Resolved)
            .count();
        let released_count = tensions
            .iter()
            .filter(|t| t.status == TensionStatus::Released)
            .count();

        let result = TreeJson {
            tensions: json_tensions,
            summary: TreeSummary {
                total: tensions.len(),
                active: active_count,
                resolved: resolved_count,
                released: released_count,
            },
        };

        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| WerkError::IoError(format!("failed to serialize JSON: {}", e)))?;
        println!("{}", json);
        return Ok(());
    }

    // Human-readable tree output
    // Build filtered forest for display
    let filtered_ids: std::collections::HashSet<_> =
        filtered_tensions.iter().map(|t| t.id.as_str()).collect();

    // Traverse and render the forest
    fn render_tree(
        forest: &Forest,
        root_ids: &[String],
        filtered_ids: &std::collections::HashSet<&str>,
        dynamics_map: &std::collections::HashMap<String, (String, String, bool)>,
        output: &Output,
        prefix: &str,
        lines: &mut Vec<String>,
    ) {
        let mut roots: Vec<_> = root_ids
            .iter()
            .filter(|id| filtered_ids.contains(id.as_str()))
            .filter_map(|id| forest.find(id))
            .collect();

        // Sort roots by horizon (earliest first, None last)
        roots.sort_by(|a, b| match (&a.tension.horizon, &b.tension.horizon) {
            (Some(ha), Some(hb)) => ha.cmp(hb),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        });

        for (i, node) in roots.iter().enumerate() {
            let is_last = i == roots.len() - 1;

            // Get dynamics
            let (phase, movement, has_conflict) = dynamics_map.get(node.id()).cloned().unwrap_or((
                "[G]".to_string(),
                "○".to_string(),
                false,
            ));

            // Status style
            let status_style = match node.tension.status {
                TensionStatus::Active => werk::output::ColorStyle::Active,
                TensionStatus::Resolved => werk::output::ColorStyle::Resolved,
                TensionStatus::Released => werk::output::ColorStyle::Released,
            };

            // Build the line
            let connector = if is_last { "└── " } else { "├── " };

            // Conflict marker
            let conflict_marker = if has_conflict { "!" } else { " " };

            // Horizon annotation
            let horizon_annotation = match &node.tension.horizon {
                Some(h) => format!("[{}]", h),
                None => "[—]".to_string(),
            };

            // Format: prefix + connector + [badge] status id horizon movement desired
            let id_short = &node.id()[..8.min(node.id().len())];
            let line = format!(
                "{}{}{}{} {} {} {}{} {}",
                prefix,
                connector,
                output.styled(&phase, werk::output::ColorStyle::Info),
                output.styled(&node.tension.status.to_string(), status_style),
                output.styled(id_short, werk::output::ColorStyle::Id),
                output.styled(&horizon_annotation, werk::output::ColorStyle::Muted),
                conflict_marker,
                movement,
                output.styled(
                    &truncate(&node.tension.desired, 50),
                    werk::output::ColorStyle::Highlight
                )
            );
            lines.push(line);

            // Recurse for children (only those that pass the filter)
            let mut children: Vec<_> = node
                .children
                .iter()
                .filter(|id| filtered_ids.contains(id.as_str()))
                .filter_map(|id| forest.find(id))
                .collect();

            // Sort children by horizon as well
            children.sort_by(|a, b| match (&a.tension.horizon, &b.tension.horizon) {
                (Some(ha), Some(hb)) => ha.cmp(hb),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            });

            if !children.is_empty() {
                let new_prefix = if is_last {
                    format!("{}    ", prefix)
                } else {
                    format!("{}│   ", prefix)
                };
                render_tree(
                    forest,
                    &node.children,
                    filtered_ids,
                    dynamics_map,
                    output,
                    &new_prefix,
                    lines,
                );
            }
        }
    }

    let mut lines = Vec::new();
    render_tree(
        &forest,
        forest.root_ids(),
        &filtered_ids,
        &dynamics_map,
        output,
        "",
        &mut lines,
    );

    // Print tree
    for line in &lines {
        println!("{}", line);
    }

    // Print summary footer
    let active_count = tensions
        .iter()
        .filter(|t| t.status == TensionStatus::Active)
        .count();
    let resolved_count = tensions
        .iter()
        .filter(|t| t.status == TensionStatus::Resolved)
        .count();
    let released_count = tensions
        .iter()
        .filter(|t| t.status == TensionStatus::Released)
        .count();

    println!();
    println!(
        "Total: {}  Active: {}  Resolved: {}  Released: {}",
        output.styled(
            &format!("{}", tensions.len()),
            werk::output::ColorStyle::Highlight
        ),
        output.styled(
            &format!("{}", active_count),
            werk::output::ColorStyle::Active
        ),
        output.styled(
            &format!("{}", resolved_count),
            werk::output::ColorStyle::Resolved
        ),
        output.styled(
            &format!("{}", released_count),
            werk::output::ColorStyle::Released
        )
    );

    Ok(())
}

/// Filter for tree display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Filter {
    All,
    Active,
    Resolved,
    Released,
}

/// Truncate a string to max length, adding ellipsis if needed.
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

fn cmd_context(_output: &Output, id: String) -> Result<(), WerkError> {
    use chrono::Utc;
    use sd_core::{
        classify_creative_cycle_phase, classify_orientation, compute_structural_tension,
        compute_urgency, detect_compensating_strategy, detect_neglect, detect_oscillation,
        detect_resolution, detect_structural_conflict, measure_assimilation_depth,
        predict_structural_tendency, AssimilationDepthThresholds, CompensatingStrategyThresholds,
        ConflictThresholds, CreativeCyclePhase, Forest, Horizon, LifecycleThresholds,
        NeglectThresholds, OrientationThresholds, OscillationThresholds, ResolutionThresholds,
        TensionStatus,
    };
    use serde::Serialize;
    use werk::workspace::Workspace;

    /// Context output structure - always JSON, designed for agent consumption.
    #[derive(Serialize)]
    struct ContextResult {
        tension: TensionInfo,
        ancestors: Vec<TensionInfo>,
        siblings: Vec<TensionInfo>,
        children: Vec<TensionInfo>,
        dynamics: DynamicsJson,
        mutations: Vec<MutationInfo>,
    }

    /// Tension information for context output.
    #[derive(Serialize)]
    struct TensionInfo {
        id: String,
        desired: String,
        actual: String,
        status: String,
        created_at: String,
        parent_id: Option<String>,
        horizon: Option<String>,
        horizon_range: Option<HorizonRangeJson>,
        urgency: Option<f64>,
        pressure: Option<f64>,
        staleness_ratio: Option<f64>,
    }

    #[derive(Serialize)]
    struct HorizonRangeJson {
        start: String,
        end: String,
    }

    /// All 10 dynamics in JSON format.
    #[derive(Serialize)]
    struct DynamicsJson {
        structural_tension: Option<StructuralTensionJson>,
        structural_conflict: Option<ConflictJson>,
        oscillation: Option<OscillationJson>,
        resolution: Option<ResolutionJson>,
        creative_cycle_phase: PhaseJson,
        orientation: Option<OrientationJson>,
        compensating_strategy: Option<CompensatingStrategyJson>,
        structural_tendency: TendencyJson,
        assimilation_depth: Option<AssimilationDepthJson>,
        neglect: Option<NeglectJson>,
    }

    #[derive(Serialize)]
    struct StructuralTensionJson {
        magnitude: f64,
        has_gap: bool,
    }

    #[derive(Serialize)]
    struct ConflictJson {
        pattern: String,
        tension_ids: Vec<String>,
    }

    #[derive(Serialize)]
    struct OscillationJson {
        reversals: usize,
        magnitude: f64,
        window_start: String,
        window_end: String,
    }

    #[derive(Serialize)]
    struct ResolutionJson {
        velocity: f64,
        trend: String,
        window_start: String,
        window_end: String,
    }

    #[derive(Serialize)]
    struct PhaseJson {
        phase: String,
        evidence: PhaseEvidenceJson,
    }

    #[derive(Serialize)]
    struct PhaseEvidenceJson {
        mutation_count: usize,
        gap_closing: bool,
        convergence_ratio: f64,
        age_seconds: i64,
    }

    #[derive(Serialize)]
    struct OrientationJson {
        orientation: String,
        creative_ratio: f64,
        problem_solving_ratio: f64,
        reactive_ratio: f64,
    }

    #[derive(Serialize)]
    struct CompensatingStrategyJson {
        strategy_type: String,
        persistence_seconds: i64,
    }

    #[derive(Serialize)]
    struct TendencyJson {
        tendency: String,
        has_conflict: bool,
    }

    #[derive(Serialize)]
    struct AssimilationDepthJson {
        depth: String,
        mutation_frequency: f64,
        frequency_trend: f64,
    }

    #[derive(Serialize)]
    struct NeglectJson {
        neglect_type: String,
        activity_ratio: f64,
    }

    /// Mutation information for context output.
    #[derive(Serialize)]
    struct MutationInfo {
        tension_id: String,
        timestamp: String,
        field: String,
        old_value: Option<String>,
        new_value: String,
    }

    // Helper function to convert a tension Node to TensionInfo with horizon data
    fn node_to_info(node: &sd_core::Node, now: chrono::DateTime<chrono::Utc>) -> TensionInfo {
        let horizon = node.tension.horizon.as_ref().map(|h| h.to_string());
        let horizon_range = node.tension.horizon.as_ref().map(|h| HorizonRangeJson {
            start: h.range_start().to_rfc3339(),
            end: h.range_end().to_rfc3339(),
        });
        let urgency = compute_urgency(&node.tension, now).map(|u| u.value);
        let structural_tension = compute_structural_tension(&node.tension);
        let pressure = structural_tension.as_ref().and_then(|st| st.pressure);

        TensionInfo {
            id: node.id().to_string(),
            desired: node.tension.desired.clone(),
            actual: node.tension.actual.clone(),
            status: node.tension.status.to_string(),
            created_at: node.tension.created_at.to_rfc3339(),
            parent_id: node.tension.parent_id.clone(),
            horizon,
            horizon_range,
            urgency,
            pressure,
            staleness_ratio: None, // Would need mutations to compute for siblings
        }
    }

    // Discover workspace
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;

    // Get all tensions for prefix resolution
    let all_tensions = store.list_tensions().map_err(WerkError::StoreError)?;
    let resolver = werk::prefix::PrefixResolver::new(all_tensions.clone());

    // Resolve the ID/prefix
    let tension = resolver.resolve(&id)?;

    // Get mutations for this tension
    let mutations = store
        .get_mutations(&tension.id)
        .map_err(WerkError::StoreError)?;

    // Get all mutations for conflict and orientation detection
    let all_mutations = store.all_mutations().map_err(WerkError::StoreError)?;

    // Build forest for ancestors, siblings, children, and conflict/neglect detection
    let forest = Forest::from_tensions(all_tensions.clone())
        .map_err(|e| WerkError::InvalidInput(e.to_string()))?;

    // === Compute time reference (needed for horizon dynamics) ===
    let now = Utc::now();

    // === Compute horizon dynamics for the main tension ===
    let horizon = tension.horizon.as_ref().map(|h| h.to_string());
    let horizon_range = tension.horizon.as_ref().map(|h| HorizonRangeJson {
        start: h.range_start().to_rfc3339(),
        end: h.range_end().to_rfc3339(),
    });
    let urgency = compute_urgency(tension, now).map(|u| u.value);
    let structural_tension_for_info = compute_structural_tension(tension);
    let pressure = structural_tension_for_info
        .as_ref()
        .and_then(|st| st.pressure);

    // Staleness ratio: need last mutation timestamp
    let last_mutation_time = mutations.last().map(|m| m.timestamp());
    let staleness_ratio = match (&tension.horizon, last_mutation_time) {
        (Some(h), Some(last_time)) => Some(h.staleness(last_time, now)),
        _ => None,
    };

    // === Tension Info ===
    let tension_info = TensionInfo {
        id: tension.id.clone(),
        desired: tension.desired.clone(),
        actual: tension.actual.clone(),
        status: tension.status.to_string(),
        created_at: tension.created_at.to_rfc3339(),
        parent_id: tension.parent_id.clone(),
        horizon,
        horizon_range,
        urgency,
        pressure,
        staleness_ratio,
    };

    // === Ancestors (root-first) ===
    let ancestors: Vec<TensionInfo> = forest
        .ancestors(&tension.id)
        .unwrap_or_default()
        .into_iter()
        .map(|node| node_to_info(node, now))
        .collect();

    // === Siblings (excluding self) ===
    let siblings: Vec<TensionInfo> = forest
        .siblings(&tension.id)
        .unwrap_or_default()
        .into_iter()
        .map(|node| node_to_info(node, now))
        .collect();

    // === Children ===
    let children: Vec<TensionInfo> = forest
        .children(&tension.id)
        .unwrap_or_default()
        .into_iter()
        .map(|node| node_to_info(node, now))
        .collect();

    // === Compute Dynamics Thresholds ===
    let lifecycle_thresholds = LifecycleThresholds::default();
    let conflict_thresholds = ConflictThresholds::default();
    let oscillation_thresholds = OscillationThresholds::default();
    let resolution_thresholds = ResolutionThresholds::default();
    let orientation_thresholds = OrientationThresholds::default();
    let compensating_thresholds = CompensatingStrategyThresholds::default();
    let assimilation_thresholds = AssimilationDepthThresholds::default();
    let neglect_thresholds = NeglectThresholds::default();

    // Get resolved tensions for momentum phase detection
    let resolved_tensions: Vec<_> = all_tensions
        .iter()
        .filter(|t| t.status == TensionStatus::Resolved)
        .cloned()
        .collect();

    // 1. Structural Tension
    let structural_tension = compute_structural_tension(tension);

    // 2. Structural Conflict
    let conflict = detect_structural_conflict(
        &forest,
        &tension.id,
        &all_mutations,
        &conflict_thresholds,
        now,
    );

    // 3. Oscillation
    let oscillation = detect_oscillation(
        &tension.id,
        &mutations,
        &oscillation_thresholds,
        now,
        None::<&Horizon>,
    );

    // 4. Resolution
    let resolution = detect_resolution(tension, &mutations, &resolution_thresholds, now);

    // 5. Creative Cycle Phase
    let phase_result = classify_creative_cycle_phase(
        tension,
        &mutations,
        &resolved_tensions,
        &lifecycle_thresholds,
        now,
    );

    // 6. Orientation (requires multiple tensions)
    let orientation =
        classify_orientation(&all_tensions, &all_mutations, &orientation_thresholds, now);

    // 7. Compensating Strategy
    let compensating_strategy = detect_compensating_strategy(
        &tension.id,
        &mutations,
        oscillation.as_ref(),
        &compensating_thresholds,
        now,
        None::<&Horizon>,
    );

    // 8. Structural Tendency
    let has_conflict = conflict.is_some();
    let tendency_result = predict_structural_tendency(tension, has_conflict, None);

    // 9. Assimilation Depth
    let assimilation = measure_assimilation_depth(
        &tension.id,
        &mutations,
        tension,
        &assimilation_thresholds,
        now,
    );

    // 10. Neglect
    let neglect = detect_neglect(
        &forest,
        &tension.id,
        &all_mutations,
        &neglect_thresholds,
        now,
    );

    // Build dynamics JSON
    let dynamics_json = DynamicsJson {
        structural_tension: structural_tension.as_ref().map(|st| StructuralTensionJson {
            magnitude: st.magnitude,
            has_gap: st.has_gap,
        }),
        structural_conflict: conflict.as_ref().map(|c| ConflictJson {
            pattern: match c.pattern {
                sd_core::ConflictPattern::AsymmetricActivity => "AsymmetricActivity".to_string(),
                sd_core::ConflictPattern::CompetingTensions => "CompetingTensions".to_string(),
            },
            tension_ids: c.tension_ids.clone(),
        }),
        oscillation: oscillation.as_ref().map(|o| OscillationJson {
            reversals: o.reversals,
            magnitude: o.magnitude,
            window_start: o.window_start.to_rfc3339(),
            window_end: o.window_end.to_rfc3339(),
        }),
        resolution: resolution.as_ref().map(|r| ResolutionJson {
            velocity: r.velocity,
            trend: match r.trend {
                sd_core::ResolutionTrend::Accelerating => "Accelerating".to_string(),
                sd_core::ResolutionTrend::Steady => "Steady".to_string(),
                sd_core::ResolutionTrend::Decelerating => "Decelerating".to_string(),
            },
            window_start: r.window_start.to_rfc3339(),
            window_end: r.window_end.to_rfc3339(),
        }),
        creative_cycle_phase: PhaseJson {
            phase: match phase_result.phase {
                CreativeCyclePhase::Germination => "Germination".to_string(),
                CreativeCyclePhase::Assimilation => "Assimilation".to_string(),
                CreativeCyclePhase::Completion => "Completion".to_string(),
                CreativeCyclePhase::Momentum => "Momentum".to_string(),
            },
            evidence: PhaseEvidenceJson {
                mutation_count: phase_result.evidence.mutation_count,
                gap_closing: phase_result.evidence.gap_closing,
                convergence_ratio: phase_result.evidence.convergence_ratio,
                age_seconds: phase_result.evidence.age_seconds,
            },
        },
        orientation: orientation.as_ref().map(|o| OrientationJson {
            orientation: match o.orientation {
                sd_core::Orientation::Creative => "Creative".to_string(),
                sd_core::Orientation::ProblemSolving => "ProblemSolving".to_string(),
                sd_core::Orientation::ReactiveResponsive => "ReactiveResponsive".to_string(),
            },
            creative_ratio: o.evidence.creative_ratio,
            problem_solving_ratio: o.evidence.problem_solving_ratio,
            reactive_ratio: o.evidence.reactive_ratio,
        }),
        compensating_strategy: compensating_strategy
            .as_ref()
            .map(|cs| CompensatingStrategyJson {
                strategy_type: match cs.strategy_type {
                    sd_core::CompensatingStrategyType::TolerableConflict => {
                        "TolerableConflict".to_string()
                    }
                    sd_core::CompensatingStrategyType::ConflictManipulation => {
                        "ConflictManipulation".to_string()
                    }
                    sd_core::CompensatingStrategyType::WillpowerManipulation => {
                        "WillpowerManipulation".to_string()
                    }
                },
                persistence_seconds: cs.persistence_seconds,
            }),
        structural_tendency: TendencyJson {
            tendency: match tendency_result.tendency {
                sd_core::StructuralTendency::Advancing => "Advancing".to_string(),
                sd_core::StructuralTendency::Oscillating => "Oscillating".to_string(),
                sd_core::StructuralTendency::Stagnant => "Stagnant".to_string(),
            },
            has_conflict: tendency_result.has_conflict,
        },
        assimilation_depth: if assimilation.depth == sd_core::AssimilationDepth::None
            && assimilation.evidence.total_mutations == 0
        {
            None
        } else {
            Some(AssimilationDepthJson {
                depth: match assimilation.depth {
                    sd_core::AssimilationDepth::Shallow => "Shallow".to_string(),
                    sd_core::AssimilationDepth::Deep => "Deep".to_string(),
                    sd_core::AssimilationDepth::None => "None".to_string(),
                },
                mutation_frequency: assimilation.mutation_frequency,
                frequency_trend: assimilation.frequency_trend,
            })
        },
        neglect: neglect.as_ref().map(|n| NeglectJson {
            neglect_type: match n.neglect_type {
                sd_core::NeglectType::ParentNeglectsChildren => {
                    "ParentNeglectsChildren".to_string()
                }
                sd_core::NeglectType::ChildrenNeglected => "ChildrenNeglected".to_string(),
            },
            activity_ratio: n.activity_ratio,
        }),
    };

    // === Mutations (chronological order - oldest first) ===
    // Store returns mutations in chronological order (oldest first)
    let mutation_infos: Vec<MutationInfo> = mutations
        .iter()
        .map(|m| MutationInfo {
            tension_id: m.tension_id().to_owned(),
            timestamp: m.timestamp().to_rfc3339(),
            field: m.field().to_owned(),
            old_value: m.old_value().map(|s| s.to_owned()),
            new_value: m.new_value().to_owned(),
        })
        .collect();

    // Build final result
    let result = ContextResult {
        tension: tension_info,
        ancestors,
        siblings,
        children,
        dynamics: dynamics_json,
        mutations: mutation_infos,
    };

    // Context always outputs JSON (designed for agent consumption)
    let json = serde_json::to_string_pretty(&result)
        .map_err(|e| WerkError::IoError(format!("failed to serialize JSON: {}", e)))?;
    println!("{}", json);

    Ok(())
}

fn cmd_run(_output: &Output, id: String, command: Vec<String>) -> Result<(), WerkError> {
    use chrono::Utc;
    use sd_core::{
        classify_creative_cycle_phase, classify_orientation, compute_structural_tension,
        compute_urgency, detect_compensating_strategy, detect_neglect, detect_oscillation,
        detect_resolution, detect_structural_conflict, measure_assimilation_depth,
        predict_structural_tendency, AssimilationDepthThresholds, CompensatingStrategyThresholds,
        ConflictThresholds, CreativeCyclePhase, Forest, Horizon, LifecycleThresholds, Mutation,
        NeglectThresholds, OrientationThresholds, OscillationThresholds, ResolutionThresholds,
        TensionStatus,
    };
    use serde::Serialize;
    use std::io::Write;
    use std::process::Stdio;
    use werk::commands::config::Config;
    use werk::workspace::Workspace;

    /// Context output structure - always JSON, designed for agent consumption.
    #[derive(Serialize)]
    struct ContextResult {
        tension: TensionInfo,
        ancestors: Vec<TensionInfo>,
        siblings: Vec<TensionInfo>,
        children: Vec<TensionInfo>,
        dynamics: DynamicsJson,
        mutations: Vec<MutationInfo>,
    }

    /// Tension information for context output.
    #[derive(Serialize)]
    struct TensionInfo {
        id: String,
        desired: String,
        actual: String,
        status: String,
        created_at: String,
        parent_id: Option<String>,
        horizon: Option<String>,
        horizon_range: Option<HorizonRangeJson>,
        urgency: Option<f64>,
        pressure: Option<f64>,
        staleness_ratio: Option<f64>,
    }

    #[derive(Serialize)]
    struct HorizonRangeJson {
        start: String,
        end: String,
    }

    /// All 10 dynamics in JSON format.
    #[derive(Serialize)]
    struct DynamicsJson {
        structural_tension: Option<StructuralTensionJson>,
        structural_conflict: Option<ConflictJson>,
        oscillation: Option<OscillationJson>,
        resolution: Option<ResolutionJson>,
        creative_cycle_phase: PhaseJson,
        orientation: Option<OrientationJson>,
        compensating_strategy: Option<CompensatingStrategyJson>,
        structural_tendency: TendencyJson,
        assimilation_depth: Option<AssimilationDepthJson>,
        neglect: Option<NeglectJson>,
    }

    #[derive(Serialize)]
    struct StructuralTensionJson {
        magnitude: f64,
        has_gap: bool,
    }

    #[derive(Serialize)]
    struct ConflictJson {
        pattern: String,
        tension_ids: Vec<String>,
    }

    #[derive(Serialize)]
    struct OscillationJson {
        reversals: usize,
        magnitude: f64,
        window_start: String,
        window_end: String,
    }

    #[derive(Serialize)]
    struct ResolutionJson {
        velocity: f64,
        trend: String,
        window_start: String,
        window_end: String,
    }

    #[derive(Serialize)]
    struct PhaseJson {
        phase: String,
        evidence: PhaseEvidenceJson,
    }

    #[derive(Serialize)]
    struct PhaseEvidenceJson {
        mutation_count: usize,
        gap_closing: bool,
        convergence_ratio: f64,
        age_seconds: i64,
    }

    #[derive(Serialize)]
    struct OrientationJson {
        orientation: String,
        creative_ratio: f64,
        problem_solving_ratio: f64,
        reactive_ratio: f64,
    }

    #[derive(Serialize)]
    struct CompensatingStrategyJson {
        strategy_type: String,
        persistence_seconds: i64,
    }

    #[derive(Serialize)]
    struct TendencyJson {
        tendency: String,
        has_conflict: bool,
    }

    #[derive(Serialize)]
    struct AssimilationDepthJson {
        depth: String,
        mutation_frequency: f64,
        frequency_trend: f64,
    }

    #[derive(Serialize)]
    struct NeglectJson {
        neglect_type: String,
        activity_ratio: f64,
    }

    /// Mutation information for context output.
    #[derive(Serialize)]
    struct MutationInfo {
        tension_id: String,
        timestamp: String,
        field: String,
        old_value: Option<String>,
        new_value: String,
    }

    // Helper function to convert a tension Node to TensionInfo with horizon data
    fn node_to_info(node: &sd_core::Node, now: chrono::DateTime<chrono::Utc>) -> TensionInfo {
        let horizon = node.tension.horizon.as_ref().map(|h| h.to_string());
        let horizon_range = node.tension.horizon.as_ref().map(|h| HorizonRangeJson {
            start: h.range_start().to_rfc3339(),
            end: h.range_end().to_rfc3339(),
        });
        let urgency = compute_urgency(&node.tension, now).map(|u| u.value);
        let structural_tension = compute_structural_tension(&node.tension);
        let pressure = structural_tension.as_ref().and_then(|st| st.pressure);

        TensionInfo {
            id: node.id().to_string(),
            desired: node.tension.desired.clone(),
            actual: node.tension.actual.clone(),
            status: node.tension.status.to_string(),
            created_at: node.tension.created_at.to_rfc3339(),
            parent_id: node.tension.parent_id.clone(),
            horizon,
            horizon_range,
            urgency,
            pressure,
            staleness_ratio: None, // Would need mutations to compute for siblings
        }
    }

    // Discover workspace
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;

    // Get all tensions for prefix resolution
    let all_tensions = store.list_tensions().map_err(WerkError::StoreError)?;
    let resolver = werk::prefix::PrefixResolver::new(all_tensions.clone());

    // Resolve the ID/prefix
    let tension = resolver.resolve(&id)?;

    // Get mutations for this tension
    let mutations = store
        .get_mutations(&tension.id)
        .map_err(WerkError::StoreError)?;

    // Get all mutations for conflict and orientation detection
    let all_mutations = store.all_mutations().map_err(WerkError::StoreError)?;

    // Build forest for ancestors, siblings, children, and conflict/neglect detection
    let forest = Forest::from_tensions(all_tensions.clone())
        .map_err(|e| WerkError::InvalidInput(e.to_string()))?;

    // === Compute time reference (needed for horizon dynamics) ===
    let now = Utc::now();

    // === Compute horizon dynamics for the main tension ===
    let horizon = tension.horizon.as_ref().map(|h| h.to_string());
    let horizon_range = tension.horizon.as_ref().map(|h| HorizonRangeJson {
        start: h.range_start().to_rfc3339(),
        end: h.range_end().to_rfc3339(),
    });
    let urgency = compute_urgency(tension, now).map(|u| u.value);
    let structural_tension_for_info = compute_structural_tension(tension);
    let pressure = structural_tension_for_info
        .as_ref()
        .and_then(|st| st.pressure);

    // Staleness ratio: need last mutation timestamp
    let last_mutation_time = mutations.last().map(|m| m.timestamp());
    let staleness_ratio = match (&tension.horizon, last_mutation_time) {
        (Some(h), Some(last_time)) => Some(h.staleness(last_time, now)),
        _ => None,
    };

    // === Tension Info ===
    let tension_info = TensionInfo {
        id: tension.id.clone(),
        desired: tension.desired.clone(),
        actual: tension.actual.clone(),
        status: tension.status.to_string(),
        created_at: tension.created_at.to_rfc3339(),
        parent_id: tension.parent_id.clone(),
        horizon,
        horizon_range,
        urgency,
        pressure,
        staleness_ratio,
    };

    // === Ancestors (root-first) ===
    let ancestors: Vec<TensionInfo> = forest
        .ancestors(&tension.id)
        .unwrap_or_default()
        .into_iter()
        .map(|node| node_to_info(node, now))
        .collect();

    // === Siblings (excluding self) ===
    let siblings: Vec<TensionInfo> = forest
        .siblings(&tension.id)
        .unwrap_or_default()
        .into_iter()
        .map(|node| node_to_info(node, now))
        .collect();

    // === Children ===
    let children: Vec<TensionInfo> = forest
        .children(&tension.id)
        .unwrap_or_default()
        .into_iter()
        .map(|node| node_to_info(node, now))
        .collect();

    // === Compute Dynamics Thresholds ===
    let lifecycle_thresholds = LifecycleThresholds::default();
    let conflict_thresholds = ConflictThresholds::default();
    let oscillation_thresholds = OscillationThresholds::default();
    let resolution_thresholds = ResolutionThresholds::default();
    let orientation_thresholds = OrientationThresholds::default();
    let compensating_thresholds = CompensatingStrategyThresholds::default();
    let assimilation_thresholds = AssimilationDepthThresholds::default();
    let neglect_thresholds = NeglectThresholds::default();

    // Get resolved tensions for momentum phase detection
    let resolved_tensions: Vec<_> = all_tensions
        .iter()
        .filter(|t| t.status == TensionStatus::Resolved)
        .cloned()
        .collect();

    // 1. Structural Tension
    let structural_tension = compute_structural_tension(tension);

    // 2. Structural Conflict
    let conflict = detect_structural_conflict(
        &forest,
        &tension.id,
        &all_mutations,
        &conflict_thresholds,
        now,
    );

    // 3. Oscillation
    let oscillation = detect_oscillation(
        &tension.id,
        &mutations,
        &oscillation_thresholds,
        now,
        None::<&Horizon>,
    );

    // 4. Resolution
    let resolution = detect_resolution(tension, &mutations, &resolution_thresholds, now);

    // 5. Creative Cycle Phase
    let phase_result = classify_creative_cycle_phase(
        tension,
        &mutations,
        &resolved_tensions,
        &lifecycle_thresholds,
        now,
    );

    // 6. Orientation (requires multiple tensions)
    let orientation =
        classify_orientation(&all_tensions, &all_mutations, &orientation_thresholds, now);

    // 7. Compensating Strategy
    let compensating_strategy = detect_compensating_strategy(
        &tension.id,
        &mutations,
        oscillation.as_ref(),
        &compensating_thresholds,
        now,
        None::<&Horizon>,
    );

    // 8. Structural Tendency
    let has_conflict = conflict.is_some();
    let tendency_result = predict_structural_tendency(tension, has_conflict, None);

    // 9. Assimilation Depth
    let assimilation = measure_assimilation_depth(
        &tension.id,
        &mutations,
        tension,
        &assimilation_thresholds,
        now,
    );

    // 10. Neglect
    let neglect = detect_neglect(
        &forest,
        &tension.id,
        &all_mutations,
        &neglect_thresholds,
        now,
    );

    // Build dynamics JSON
    let dynamics_json = DynamicsJson {
        structural_tension: structural_tension.as_ref().map(|st| StructuralTensionJson {
            magnitude: st.magnitude,
            has_gap: st.has_gap,
        }),
        structural_conflict: conflict.as_ref().map(|c| ConflictJson {
            pattern: match c.pattern {
                sd_core::ConflictPattern::AsymmetricActivity => "AsymmetricActivity".to_string(),
                sd_core::ConflictPattern::CompetingTensions => "CompetingTensions".to_string(),
            },
            tension_ids: c.tension_ids.clone(),
        }),
        oscillation: oscillation.as_ref().map(|o| OscillationJson {
            reversals: o.reversals,
            magnitude: o.magnitude,
            window_start: o.window_start.to_rfc3339(),
            window_end: o.window_end.to_rfc3339(),
        }),
        resolution: resolution.as_ref().map(|r| ResolutionJson {
            velocity: r.velocity,
            trend: match r.trend {
                sd_core::ResolutionTrend::Accelerating => "Accelerating".to_string(),
                sd_core::ResolutionTrend::Steady => "Steady".to_string(),
                sd_core::ResolutionTrend::Decelerating => "Decelerating".to_string(),
            },
            window_start: r.window_start.to_rfc3339(),
            window_end: r.window_end.to_rfc3339(),
        }),
        creative_cycle_phase: PhaseJson {
            phase: match phase_result.phase {
                CreativeCyclePhase::Germination => "Germination".to_string(),
                CreativeCyclePhase::Assimilation => "Assimilation".to_string(),
                CreativeCyclePhase::Completion => "Completion".to_string(),
                CreativeCyclePhase::Momentum => "Momentum".to_string(),
            },
            evidence: PhaseEvidenceJson {
                mutation_count: phase_result.evidence.mutation_count,
                gap_closing: phase_result.evidence.gap_closing,
                convergence_ratio: phase_result.evidence.convergence_ratio,
                age_seconds: phase_result.evidence.age_seconds,
            },
        },
        orientation: orientation.as_ref().map(|o| OrientationJson {
            orientation: match o.orientation {
                sd_core::Orientation::Creative => "Creative".to_string(),
                sd_core::Orientation::ProblemSolving => "ProblemSolving".to_string(),
                sd_core::Orientation::ReactiveResponsive => "ReactiveResponsive".to_string(),
            },
            creative_ratio: o.evidence.creative_ratio,
            problem_solving_ratio: o.evidence.problem_solving_ratio,
            reactive_ratio: o.evidence.reactive_ratio,
        }),
        compensating_strategy: compensating_strategy
            .as_ref()
            .map(|cs| CompensatingStrategyJson {
                strategy_type: match cs.strategy_type {
                    sd_core::CompensatingStrategyType::TolerableConflict => {
                        "TolerableConflict".to_string()
                    }
                    sd_core::CompensatingStrategyType::ConflictManipulation => {
                        "ConflictManipulation".to_string()
                    }
                    sd_core::CompensatingStrategyType::WillpowerManipulation => {
                        "WillpowerManipulation".to_string()
                    }
                },
                persistence_seconds: cs.persistence_seconds,
            }),
        structural_tendency: TendencyJson {
            tendency: match tendency_result.tendency {
                sd_core::StructuralTendency::Advancing => "Advancing".to_string(),
                sd_core::StructuralTendency::Oscillating => "Oscillating".to_string(),
                sd_core::StructuralTendency::Stagnant => "Stagnant".to_string(),
            },
            has_conflict: tendency_result.has_conflict,
        },
        assimilation_depth: if assimilation.depth == sd_core::AssimilationDepth::None
            && assimilation.evidence.total_mutations == 0
        {
            None
        } else {
            Some(AssimilationDepthJson {
                depth: match assimilation.depth {
                    sd_core::AssimilationDepth::Shallow => "Shallow".to_string(),
                    sd_core::AssimilationDepth::Deep => "Deep".to_string(),
                    sd_core::AssimilationDepth::None => "None".to_string(),
                },
                mutation_frequency: assimilation.mutation_frequency,
                frequency_trend: assimilation.frequency_trend,
            })
        },
        neglect: neglect.as_ref().map(|n| NeglectJson {
            neglect_type: match n.neglect_type {
                sd_core::NeglectType::ParentNeglectsChildren => {
                    "ParentNeglectsChildren".to_string()
                }
                sd_core::NeglectType::ChildrenNeglected => "ChildrenNeglected".to_string(),
            },
            activity_ratio: n.activity_ratio,
        }),
    };

    // === Mutations (chronological order - oldest first) ===
    let mutation_infos: Vec<MutationInfo> = mutations
        .iter()
        .map(|m| MutationInfo {
            tension_id: m.tension_id().to_owned(),
            timestamp: m.timestamp().to_rfc3339(),
            field: m.field().to_owned(),
            old_value: m.old_value().map(|s| s.to_owned()),
            new_value: m.new_value().to_owned(),
        })
        .collect();

    // Build context result
    let context = ContextResult {
        tension: tension_info,
        ancestors,
        siblings,
        children,
        dynamics: dynamics_json,
        mutations: mutation_infos,
    };

    // Serialize context to JSON
    let context_json = serde_json::to_string(&context)
        .map_err(|e| WerkError::IoError(format!("failed to serialize context JSON: {}", e)))?;

    // === Determine command to run ===
    let (program, args, command_str_for_mutation): (String, Vec<String>, String) = if !command
        .is_empty()
    {
        // Use -- override directly (already properly split by clap)
        let program = command[0].clone();
        let args: Vec<String> = command[1..].to_vec();
        let command_str = command.join(" ");
        (program, args, command_str)
    } else {
        // Try config default
        let config = Config::load(&workspace)?;
        match config.get("agent.command") {
            Some(cmd) => {
                // Parse config command - split on whitespace
                let cmd_parts: Vec<String> =
                    cmd.split_whitespace().map(|s| s.to_string()).collect();
                if cmd_parts.is_empty() {
                    return Err(WerkError::InvalidInput(
                        "agent command in config is empty".to_string(),
                    ));
                }
                let program = cmd_parts[0].clone();
                let args: Vec<String> = cmd_parts[1..].to_vec();
                (program, args, cmd.clone())
            }
            None => {
                return Err(WerkError::InvalidInput(
                    "no agent command configured. Use -- to specify a command or set agent.command in config".to_string(),
                ));
            }
        }
    };

    // Get workspace path
    let workspace_path = workspace.werk_dir();

    // === Spawn subprocess ===
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

    // Wait for subprocess to complete
    let exit_status = child
        .wait()
        .map_err(|e| WerkError::IoError(format!("failed to wait for agent process: {}", e)))?;

    // Get exit code
    let exit_code = exit_status.code().unwrap_or(1);

    // Record session mutation
    store
        .record_mutation(&Mutation::new(
            tension.id.clone(),
            Utc::now(),
            "agent_session".to_owned(),
            None,
            command_str_for_mutation,
        ))
        .map_err(WerkError::SdError)?;

    // If not successful, exit with the subprocess exit code
    if !exit_status.success() {
        std::process::exit(exit_code);
    }

    Ok(())
}

fn cmd_nuke(output: &Output, confirm: bool, global: bool) -> Result<(), WerkError> {
    use serde::Serialize;
    use std::path::PathBuf;
    use werk::workspace::Workspace;

    /// JSON output structure for nuke command.
    #[derive(Serialize)]
    struct NukeResult {
        path: String,
        deleted: bool,
    }

    // Determine target path
    let werk_dir: PathBuf = if global {
        let home = dirs::home_dir()
            .ok_or_else(|| WerkError::IoError("cannot determine home directory".to_string()))?;
        home.join(".werk")
    } else {
        // Try to discover workspace first
        match Workspace::discover() {
            Ok(ws) => ws.werk_dir().to_path_buf(),
            Err(_) => {
                // If no workspace found, use current directory's .werk
                let cwd = std::env::current_dir().map_err(|e| {
                    WerkError::IoError(format!("failed to get current directory: {}", e))
                })?;
                cwd.join(".werk")
            }
        }
    };

    // Check if the directory exists
    if !werk_dir.exists() {
        return Err(WerkError::InvalidInput(format!(
            "No .werk directory found at {}",
            werk_dir.display()
        )));
    }

    // If not confirmed, just show what would be deleted
    if !confirm {
        if output.is_json() {
            let result = NukeResult {
                path: werk_dir.to_string_lossy().to_string(),
                deleted: false,
            };
            let json = serde_json::to_string_pretty(&result)
                .map_err(|e| WerkError::IoError(format!("failed to serialize JSON: {}", e)))?;
            println!("{}", json);
        } else {
            output
                .info(&format!("Would delete: {}", werk_dir.display()))
                .map_err(|e| WerkError::IoError(e.to_string()))?;
            println!("\nPass --confirm to proceed with deletion.");
            println!("All data in this workspace will be permanently lost.");
        }
        return Ok(());
    }

    // Delete the entire .werk directory
    std::fs::remove_dir_all(&werk_dir).map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            WerkError::PermissionDenied(format!("{}", werk_dir.display()))
        } else {
            WerkError::IoError(format!("failed to delete {}: {}", werk_dir.display(), e))
        }
    })?;

    let result = NukeResult {
        path: werk_dir.to_string_lossy().to_string(),
        deleted: true,
    };

    if output.is_json() {
        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| WerkError::IoError(format!("failed to serialize JSON: {}", e)))?;
        println!("{}", json);
    } else {
        output
            .success(&format!("Deleted workspace: {}", werk_dir.display()))
            .map_err(|e| WerkError::IoError(e.to_string()))?;
    }

    Ok(())
}

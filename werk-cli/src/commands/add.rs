//! Add command handler.

use crate::error::WerkError;
use crate::output::Output;
use crate::prefix::PrefixResolver;
use crate::workspace::Workspace;
use sd_core::Horizon;
use serde::Serialize;
use werk_shared::{Config, HookEvent, HookRunner};

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

pub fn cmd_add(
    output: &Output,
    desired: Option<String>,
    actual: Option<String>,
    parent: Option<String>,
    horizon: Option<String>,
) -> Result<(), WerkError> {
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
    let mut store = workspace.open_store()?;

    // Resolve parent if provided
    let parent_id = if let Some(parent_prefix) = parent {
        let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
        let resolver = PrefixResolver::new(tensions);
        let parent_tension = resolver.resolve(&parent_prefix)?;
        Some(parent_tension.id.clone())
    } else {
        None
    };

    // Create the tension with horizon
    let _ = store.begin_gesture(Some("create tension"));
    let tension =
        store.create_tension_full(&desired, &actual, parent_id.clone(), horizon_parsed.clone())?;
    store.end_gesture();

    // Hook infrastructure (post_create — tension must exist first)
    let hooks = Config::load(&workspace)
        .map(|c| HookRunner::from_config(&c))
        .unwrap_or_else(|_| HookRunner::noop());
    let event = HookEvent::create(&tension.id, &tension.desired);
    hooks.post_create(&event);

    let result = AddResult {
        id: tension.id.clone(),
        desired: tension.desired.clone(),
        actual: tension.actual.clone(),
        status: tension.status.to_string(),
        parent_id,
        horizon: tension.horizon.as_ref().map(|h| h.to_string()),
    };

    if output.is_structured() {
        output
            .print_structured(&result)
            .map_err(WerkError::IoError)?;
    } else {
        // Human-readable output
        output
            .success(&format!(
                "Created tension {}",
                werk_shared::display_id(tension.short_code, &tension.id)
            ))
            .map_err(|e| WerkError::IoError(e.to_string()))?;
        println!("  Desired: {}", &tension.desired);
        println!("  Reality: {}", &tension.actual);
        println!("  Status:  {}", &tension.status);
        if let Some(pid) = &tension.parent_id {
            println!("  Parent:  {}", pid);
        }
        if let Some(h) = &tension.horizon {
            println!("  Deadline: {}", h);
        }
    }

    Ok(())
}

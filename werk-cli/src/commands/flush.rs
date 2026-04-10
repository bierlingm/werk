//! Flush command handler.
//!
//! Writes the tension tree state to a git-trackable JSON file at the workspace
//! root. Thin wrapper around `werk_shared::flush::flush_to_file`, which owns
//! the actual serialization, idempotency, and safety logic.

use crate::error::WerkError;
use crate::output::Output;
use crate::workspace::Workspace;
use serde::Serialize;
use werk_shared::flush as flush_impl;

pub fn cmd_flush(output: &Output) -> Result<(), WerkError> {
    let workspace = Workspace::discover()?;
    let outcome = flush_impl::flush_to_file(&workspace)
        .map_err(|e| match e {
            werk_shared::error::WerkError::IoError(s) => WerkError::IoError(s),
            werk_shared::error::WerkError::StoreError(s) => WerkError::StoreError(s),
            other => WerkError::IoError(format!("{:?}", other)),
        })?;

    if output.is_structured() {
        #[derive(Serialize)]
        struct FlushResult {
            path: String,
            tensions: usize,
            wrote: bool,
        }
        let result = FlushResult {
            path: outcome.path.to_string_lossy().to_string(),
            tensions: outcome.count,
            wrote: outcome.wrote,
        };
        output
            .print_structured(&result)
            .map_err(WerkError::IoError)?;
    } else {
        let msg = if outcome.wrote {
            format!(
                "Flushed {} tensions to {}",
                outcome.count,
                outcome.path.display()
            )
        } else {
            format!(
                "No changes — tensions.json already reflects current state ({} tensions)",
                outcome.count
            )
        };
        output
            .success(&msg)
            .map_err(|e| WerkError::IoError(e.to_string()))?;
    }

    Ok(())
}

/// Autoflush: silently write tensions.json if `flush.auto` config is set to true.
/// Errors are silently ignored — autoflush should never break a mutation command.
pub fn autoflush() {
    let Ok(workspace) = Workspace::discover() else {
        return;
    };
    let Ok(config) = werk_shared::config::Config::load(&workspace) else {
        return;
    };
    if config.get("flush.auto").map(|v| v.as_str()) != Some("true") {
        return;
    }
    let _ = flush_impl::flush_to_file(&workspace);
}

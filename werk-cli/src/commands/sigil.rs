use crate::error::WerkError;
use crate::output::Output;
use crate::prefix::PrefixResolver;
use crate::workspace::Workspace;
use chrono::Utc;
use serde::Serialize;
use std::io::Write;
use std::path::{Path, PathBuf};
use werk_core::store::SigilRecord;
use werk_core::{Address, parse_address};
use werk_sigil::{
    Ctx, Engine, Logic, Scope, ScopeKind, SigilError, archive_path, cleanup_cache, derive_seed,
    load_preset, scope_canonical,
};

#[derive(Serialize)]
struct SigilJson {
    scope: String,
    logic: String,
    logic_version: String,
    seed: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    svg: Option<String>,
    warnings: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dry_run: Option<bool>,
}

pub fn cmd_sigil(
    output: &Output,
    scopes: Vec<String>,
    logic_arg: Option<String>,
    seed: Option<u64>,
    out: Option<PathBuf>,
    save: bool,
    dry_run: bool,
) -> Result<(), WerkError> {
    let workspace = Workspace::discover()?;
    let store = workspace.open_store()?;
    let logic = load_logic(logic_arg)?;

    cleanup_cache(7).map_err(|e| WerkError::IoError(format!("cache cleanup failed: {e}")))?;

    if let Some(at) = logic.scope_at.as_deref()
        && at != "now"
    {
        return Err(WerkError::InvalidInput(
            "historical scope is not supported for sigils in v1".to_string(),
        ));
    }

    let scope = resolve_scope(&store, &logic, &scopes)?;
    let workspace_name = workspace_name(&workspace);
    let mut ctx = Ctx::new(Utc::now(), &store, workspace_name, 0);

    let mut compiled = Engine::compile(logic.clone()).map_err(map_sigil_error)?;
    let resolved = compiled
        .selector
        .select(scope.clone(), &mut ctx)
        .map_err(map_sigil_error)?;
    let scope_canonical = scope_canonical(&resolved);
    let seed_value = seed.unwrap_or_else(|| derive_seed(&compiled.logic, &scope_canonical));

    if dry_run {
        let result = SigilJson {
            scope: scope_canonical,
            logic: compiled.logic.meta.name.clone(),
            logic_version: compiled.logic.version_string(),
            seed: seed_value,
            path: None,
            svg: None,
            warnings: vec![],
            dry_run: Some(true),
        };
        output
            .print_structured(&result)
            .map_err(WerkError::IoError)?;
        return Ok(());
    }

    let sigil = Engine::render_with_compiled(scope, &mut compiled, &mut ctx, seed)
        .map_err(map_sigil_error)?;
    let warnings = ctx.diagnostics.warnings();

    let mut output_path: Option<PathBuf> = None;
    if let Some(path) = out.as_ref() {
        ensure_parent_dir(path)?;
        std::fs::write(path, &sigil.svg.0)
            .map_err(|e| WerkError::IoError(format!("failed to write {}: {e}", path.display())))?;
        output_path = Some(path.clone());
    }

    if save {
        let archive = archive_path(
            &scope_canonical,
            &compiled.logic.meta.name,
            seed_value,
            ctx.now,
        );
        ensure_parent_dir(&archive)?;
        std::fs::write(&archive, &sigil.svg.0).map_err(|e| {
            WerkError::IoError(format!("failed to write {}: {e}", archive.display()))
        })?;
        record_sigil(
            &store,
            &scope_canonical,
            &compiled.logic,
            seed_value,
            ctx.now,
            archive.clone(),
        )?;
        if output_path.is_none() {
            output_path = Some(archive);
        }
    }

    if output.is_structured() {
        let svg = if output_path.is_none() {
            Some(
                String::from_utf8(sigil.svg.0)
                    .map_err(|e| WerkError::IoError(format!("invalid svg bytes: {e}")))?,
            )
        } else {
            None
        };
        let result = SigilJson {
            scope: scope_canonical,
            logic: compiled.logic.meta.name.clone(),
            logic_version: compiled.logic.version_string(),
            seed: seed_value,
            path: output_path.as_ref().map(|p| p.display().to_string()),
            svg,
            warnings,
            dry_run: None,
        };
        output
            .print_structured(&result)
            .map_err(WerkError::IoError)?;
    } else if output_path.is_none() {
        std::io::stdout()
            .write_all(&sigil.svg.0)
            .map_err(|e| WerkError::IoError(format!("failed to write svg: {e}")))?;
    } else {
        let message = if save && out.is_some() {
            format!(
                "Saved sigil to {} and archive",
                output_path.as_ref().unwrap().display()
            )
        } else {
            format!("Saved sigil to {}", output_path.as_ref().unwrap().display())
        };
        output
            .success(&message)
            .map_err(|e| WerkError::IoError(e.to_string()))?;
    }

    Ok(())
}

fn resolve_scope(
    store: &werk_core::Store,
    logic: &Logic,
    scopes: &[String],
) -> Result<Scope, WerkError> {
    if scopes.is_empty() {
        return Ok(logic.scope_fallback.clone().into_scope(None, None));
    }
    let resolved: Result<Vec<Scope>, WerkError> = scopes
        .iter()
        .map(|input| resolve_single_scope(store, logic, input))
        .collect();
    let resolved = resolved?;
    if resolved.len() == 1 {
        return Ok(resolved[0].clone());
    }
    Ok(Scope {
        kind: ScopeKind::Union,
        root: None,
        depth: None,
        name: None,
        status: None,
        members: resolved,
        at: None,
    })
}

fn resolve_single_scope(
    store: &werk_core::Store,
    logic: &Logic,
    input: &str,
) -> Result<Scope, WerkError> {
    if let Ok(addr) = parse_address(input) {
        match addr {
            Address::Tension(n) => return resolve_by_short_code(store, logic, &n.to_string()),
            Address::Epoch { .. } | Address::Note { .. } | Address::TensionAt { .. } => {
                return Err(WerkError::InvalidInput(
                    "historical or sub-address scopes are not supported for sigils".to_string(),
                ));
            }
            Address::Sigil(_) => {
                return Err(WerkError::InvalidInput(
                    "sigil short codes cannot be used as render scopes".to_string(),
                ));
            }
            Address::Gesture(_) | Address::Session(_) | Address::CrossSpace { .. } => {
                return Err(WerkError::InvalidInput(
                    "unsupported scope address for sigil rendering".to_string(),
                ));
            }
        }
    }
    resolve_by_prefix(store, logic, input)
}

fn resolve_by_short_code(
    store: &werk_core::Store,
    logic: &Logic,
    code: &str,
) -> Result<Scope, WerkError> {
    resolve_by_prefix(store, logic, code)
}

fn resolve_by_prefix(
    store: &werk_core::Store,
    logic: &Logic,
    input: &str,
) -> Result<Scope, WerkError> {
    let tensions = store.list_tensions().map_err(WerkError::StoreError)?;
    let resolver = PrefixResolver::new(tensions);
    let tension = resolver.resolve(input)?;
    Ok(logic
        .scope_default
        .clone()
        .into_scope(Some(tension.id.clone()), None))
}

fn load_logic(arg: Option<String>) -> Result<Logic, WerkError> {
    let logic_name = arg.unwrap_or_else(|| "contemplative".to_string());
    let path = logic_path(&logic_name)?;
    load_preset(path)
        .map(|preset| preset.logic)
        .map_err(|e| WerkError::InvalidInput(format!("failed to load logic: {e}")))
}

fn logic_path(logic_name: &str) -> Result<PathBuf, WerkError> {
    let candidate = PathBuf::from(logic_name);
    if candidate.extension().is_some() || logic_name.contains('/') || logic_name.contains('\\') {
        if candidate.exists() {
            return Ok(candidate);
        }
        return Err(WerkError::InvalidInput(format!(
            "logic file not found: {}",
            candidate.display()
        )));
    }
    let preset_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../werk-sigil/presets");
    Ok(preset_dir.join(format!("{logic_name}.toml")))
}

fn ensure_parent_dir(path: &Path) -> Result<(), WerkError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            WerkError::IoError(format!("failed to create {}: {e}", parent.display()))
        })?;
    }
    Ok(())
}

fn record_sigil(
    store: &werk_core::Store,
    scope_canonical: &str,
    logic: &Logic,
    seed: u64,
    rendered_at: chrono::DateTime<chrono::Utc>,
    archive_path: PathBuf,
) -> Result<(), WerkError> {
    let existing = store.list_sigils().map_err(WerkError::StoreError)?;
    let next = existing
        .iter()
        .map(|s| s.short_code)
        .max()
        .unwrap_or(0)
        .saturating_add(1);
    let record = SigilRecord {
        id: 0,
        short_code: next,
        scope_canonical: scope_canonical.to_string(),
        logic_id: logic.meta.name.clone(),
        logic_version: logic.version_string(),
        seed: seed as i64,
        rendered_at,
        file_path: archive_path.display().to_string(),
        label: None,
    };
    store.record_sigil(&record).map_err(WerkError::StoreError)?;
    Ok(())
}

fn workspace_name(workspace: &Workspace) -> String {
    if workspace.is_global() {
        "global".to_string()
    } else {
        workspace
            .root()
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "werk".to_string())
    }
}

fn map_sigil_error(err: SigilError) -> WerkError {
    match err {
        SigilError::Construction { message, .. } => WerkError::InvalidInput(message),
        SigilError::Unsupported { feature } => {
            WerkError::InvalidInput(format!("unsupported feature: {feature}"))
        }
        SigilError::UnknownChannel { name } => {
            WerkError::InvalidInput(format!("unknown channel: {name}"))
        }
        SigilError::IrIncompatible {
            stage,
            expected,
            actual,
        } => WerkError::InvalidInput(format!(
            "pipeline incompatible at {stage}: expected {expected:?}, got {actual:?}"
        )),
        SigilError::Render { message } => WerkError::InvalidInput(message),
        SigilError::Internal { message } => WerkError::IoError(message),
        SigilError::Io { message } => WerkError::IoError(message),
        SigilError::RecursionLimit { depth } => WerkError::InvalidInput(format!(
            "recursion limit exceeded at depth {depth}"
        )),
    }
}

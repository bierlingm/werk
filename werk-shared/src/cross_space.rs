//! Cross-space address resolution.
//!
//! Resolves a cross-space address (`werk:42`, `journal:7~e3`) by:
//! 1. Looking up the space name in the registry
//! 2. Opening the target workspace
//! 3. Opening its store
//! 4. Resolving the inner address (tension + sub-entity)
//!
//! Returns a `CrossSpaceResult` containing the resolved tension, the open
//! store (for further queries like mutations/epochs/notes), and the workspace
//! metadata. The caller owns everything — no borrowed lifetimes across stores.

use werk_core::{Address, Store, Tension};

use crate::error::{Result, WerkError};
use crate::prefix::PrefixResolver;
use crate::registry::Registry;
use crate::workspace::Workspace;

/// The result of resolving a cross-space address.
///
/// Contains everything the caller needs to work with the remote tension
/// as if it were local: the open store, the resolved tension, all tensions
/// in that space (for forest construction), and the workspace itself.
pub struct CrossSpaceResult {
    /// The open store for the target space.
    pub store: Store,
    /// The resolved base tension.
    pub tension: Tension,
    /// All tensions in the target space (for forest construction, signals, etc.).
    pub all_tensions: Vec<Tension>,
    /// The workspace that was opened.
    pub workspace: Workspace,
    /// The space name used for resolution.
    pub space_name: String,
}

/// Resolve a cross-space address.
///
/// `space` is the registered space name (e.g. "werk", "journal").
/// `inner` is the inner address (Tension, Epoch, Note, or TensionAt).
///
/// The inner address is resolved down to the base tension — sub-entity
/// resolution (epoch number, note number, temporal lookup) is left to the
/// caller, which has the store and can query mutations/epochs/notes as needed.
/// The inner address is returned alongside the result for this purpose.
///
/// # Errors
///
/// - `WerkError::IoError` if the space name is not in the registry
/// - `WerkError::StoreError` if the target store can't be opened
/// - `WerkError::TensionNotFound` if the short code doesn't exist in the target space
pub fn resolve_cross_space(space: &str, inner: &Address) -> Result<CrossSpaceResult> {
    // Look up space in registry
    let registry = Registry::load()?;
    let entry = registry.get(space).ok_or_else(|| {
        let available: Vec<String> = registry.list().into_iter().map(|w| w.name).collect();
        let hint = if available.is_empty() {
            "no spaces registered. Use `werk spaces register <name> <path>`.".to_string()
        } else {
            format!("registered spaces: {}", available.join(", "))
        };
        WerkError::IoError(format!("unknown space '{space}'. {hint}"))
    })?;

    // Open the workspace and store
    let workspace = Workspace::discover_from(&entry.path)?;
    let store = workspace.open_store()?;

    // List tensions for prefix resolution
    let all_tensions = store.list_tensions().map_err(WerkError::StoreError)?;
    let resolver = PrefixResolver::new(all_tensions.clone());

    // Extract the short code from the inner address
    let short_code = tension_short_code(inner)?;
    let tension = resolver.resolve(&short_code)?.clone();

    Ok(CrossSpaceResult {
        store,
        tension,
        all_tensions,
        workspace,
        space_name: space.to_owned(),
    })
}

/// Extract the tension short code string from an inner address.
/// All tension-based address variants have a tension i32 as their root.
fn tension_short_code(addr: &Address) -> Result<String> {
    match addr {
        Address::Tension(n)
        | Address::Epoch { tension: n, .. }
        | Address::Note { tension: n, .. }
        | Address::TensionAt { tension: n, .. } => Ok(n.to_string()),
        Address::Gesture(_) | Address::Session(_) | Address::Sigil(_) => Err(WerkError::InvalidInput(
            "cross-space addresses only support tension-based inner addresses, not gestures, sessions, or sigils"
                .to_string(),
        )),
        Address::CrossSpace { .. } => Err(WerkError::InvalidInput(
            "nested cross-space addresses are not supported".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tension_short_code_extraction() {
        assert_eq!(tension_short_code(&Address::Tension(42)).unwrap(), "42");
        assert_eq!(
            tension_short_code(&Address::Epoch {
                tension: 7,
                epoch_num: 3
            })
            .unwrap(),
            "7"
        );
        assert_eq!(
            tension_short_code(&Address::Note {
                tension: 10,
                note_num: 2
            })
            .unwrap(),
            "10"
        );
        assert_eq!(
            tension_short_code(&Address::TensionAt {
                tension: 5,
                timespec: "2026-03".to_owned()
            })
            .unwrap(),
            "5"
        );
    }

    #[test]
    fn test_tension_short_code_rejects_gesture() {
        assert!(tension_short_code(&Address::Gesture("abc".to_owned())).is_err());
    }

    #[test]
    fn test_tension_short_code_rejects_session() {
        assert!(tension_short_code(&Address::Session("abc".to_owned())).is_err());
    }

    #[test]
    fn test_tension_short_code_rejects_sigil() {
        assert!(tension_short_code(&Address::Sigil(7)).is_err());
    }

    #[test]
    fn test_tension_short_code_rejects_nested_cross_space() {
        let nested = Address::CrossSpace {
            space: "outer".to_owned(),
            inner: Box::new(Address::Tension(1)),
        };
        assert!(tension_short_code(&nested).is_err());
    }

    #[test]
    fn test_resolve_unknown_space() {
        // Resolving a space that doesn't exist should give a clear error
        let result = resolve_cross_space("nonexistent", &Address::Tension(1));
        match result {
            Err(e) => {
                let msg = e.to_string();
                assert!(
                    msg.contains("nonexistent"),
                    "error should name the space: {msg}"
                );
            }
            Ok(_) => panic!("expected error for unknown space"),
        }
    }
}

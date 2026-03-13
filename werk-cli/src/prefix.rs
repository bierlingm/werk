//! ID prefix resolution for werk-cli.
//!
//! Tensions use ULID identifiers which are long and unwieldy. Users can reference
//! tensions by a unique prefix instead of the full ID.
//!
//! Rules:
//! - Minimum 4 characters required
//! - Case-insensitive matching
//! - Must resolve to exactly one tension
//! - If ambiguous, offer interactive selection (TTY) or error (non-TTY)

use crate::error::{Result, WerkError};
use sd_core::Tension;

/// The minimum prefix length required.
pub const MIN_PREFIX_LEN: usize = 4;

/// Resolver for ID prefixes.
#[derive(Debug, Clone)]
pub struct PrefixResolver {
    tensions: Vec<Tension>,
}

impl PrefixResolver {
    /// Create a new resolver with the given tensions.
    pub fn new(tensions: Vec<Tension>) -> Self {
        Self { tensions }
    }

    /// Resolve a prefix to a single tension (non-interactive).
    ///
    /// Returns an error if:
    /// - Prefix is shorter than 4 characters
    /// - No tension matches the prefix
    /// - Multiple tensions match the prefix (ambiguous)
    pub fn resolve(&self, prefix: &str) -> Result<&Tension> {
        // Check minimum length
        if prefix.len() < MIN_PREFIX_LEN {
            return Err(WerkError::PrefixTooShort {
                prefix: prefix.to_string(),
                len: prefix.len(),
            });
        }

        let matches = self.find_matches(prefix);

        match matches.len() {
            0 => Err(WerkError::TensionNotFound(prefix.to_string())),
            1 => Ok(matches[0]),
            _ => {
                let match_list = matches
                    .iter()
                    .map(|t| format!("  {} - {}", t.id, truncate(&t.desired, 40)))
                    .collect::<Vec<_>>()
                    .join("\n");
                Err(WerkError::AmbiguousPrefix {
                    prefix: prefix.to_string(),
                    matches: match_list,
                })
            }
        }
    }

    /// Resolve a prefix with interactive disambiguation.
    ///
    /// If the prefix is ambiguous and stdin is a TTY, shows an interactive
    /// selection menu. Otherwise falls back to the non-interactive error.
    pub fn resolve_interactive(&self, prefix: &str) -> Result<&Tension> {
        if prefix.len() < MIN_PREFIX_LEN {
            return Err(WerkError::PrefixTooShort {
                prefix: prefix.to_string(),
                len: prefix.len(),
            });
        }

        let matches = self.find_matches(prefix);

        match matches.len() {
            0 => Err(WerkError::TensionNotFound(prefix.to_string())),
            1 => Ok(matches[0]),
            _ => {
                // Check if we're in an interactive terminal
                if !atty_is_stdin() {
                    let match_list = matches
                        .iter()
                        .map(|t| format!("  {} - {}", t.id, truncate(&t.desired, 40)))
                        .collect::<Vec<_>>()
                        .join("\n");
                    return Err(WerkError::AmbiguousPrefix {
                        prefix: prefix.to_string(),
                        matches: match_list,
                    });
                }

                interactive_select_tension(prefix, &matches)
            }
        }
    }

    /// Find all tensions matching a prefix (case-insensitive).
    fn find_matches(&self, prefix: &str) -> Vec<&Tension> {
        let prefix_lower = prefix.to_lowercase();
        self.tensions
            .iter()
            .filter(|t| t.id.to_lowercase().starts_with(&prefix_lower))
            .collect()
    }

    /// Check if a prefix is valid (meets minimum length requirement).
    pub fn is_valid_prefix(prefix: &str) -> bool {
        prefix.len() >= MIN_PREFIX_LEN
    }

    /// Find all tensions matching a prefix.
    ///
    /// Unlike `resolve`, this returns all matches without requiring uniqueness.
    pub fn find_all(&self, prefix: &str) -> Vec<&Tension> {
        if prefix.len() < MIN_PREFIX_LEN {
            return Vec::new();
        }
        self.find_matches(prefix)
    }
}

/// Check if stdin is a TTY (interactive terminal).
fn atty_is_stdin() -> bool {
    use std::io::IsTerminal;
    std::io::stdin().is_terminal()
}

/// Show an interactive selection menu for ambiguous tension prefixes.
fn interactive_select_tension<'a>(
    prefix: &str,
    matches: &[&'a Tension],
) -> Result<&'a Tension> {
    use dialoguer::Select;

    eprintln!(
        "\nAmbiguous ID '{}' matches {} tensions. Select one:\n",
        prefix,
        matches.len()
    );

    let items: Vec<String> = matches
        .iter()
        .map(|t| {
            format!(
                "{} - {}",
                &t.id[..std::cmp::min(t.id.len(), 12)],
                truncate(&t.desired, 50)
            )
        })
        .collect();

    let selection = Select::new()
        .items(&items)
        .default(0)
        .interact_opt()
        .map_err(|e| WerkError::IoError(format!("selection failed: {}", e)))?
        .ok_or_else(|| {
            WerkError::InvalidInput("selection cancelled".to_string())
        })?;

    Ok(matches[selection])
}

/// Truncate a string to a maximum length, adding ellipsis if needed (Unicode-safe).
fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len).collect();
        format!("{}...", truncated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use sd_core::TensionStatus;

    fn make_tension(id: &str, desired: &str) -> Tension {
        Tension {
            id: id.to_string(),
            desired: desired.to_string(),
            actual: "actual".to_string(),
            parent_id: None,
            created_at: Utc::now(),
            status: TensionStatus::Active,
            horizon: None,
        }
    }

    #[test]
    fn test_resolve_exact_id() {
        let tensions = vec![make_tension("01ARZ3N4K5B6C7D8E9F0G1H2I3", "goal")];
        let resolver = PrefixResolver::new(tensions);
        let result = resolver.resolve("01ARZ3N4K5B6C7D8E9F0G1H2I3").unwrap();
        assert_eq!(result.desired, "goal");
    }

    #[test]
    fn test_resolve_prefix() {
        let tensions = vec![make_tension("01ARZ3N4K5B6C7D8E9F0G1H2I3", "goal")];
        let resolver = PrefixResolver::new(tensions);
        let result = resolver.resolve("01ARZ3").unwrap();
        assert_eq!(result.desired, "goal");
    }

    #[test]
    fn test_resolve_case_insensitive() {
        let tensions = vec![make_tension("01ARZ3N4K5B6C7D8E9F0G1H2I3", "goal")];
        let resolver = PrefixResolver::new(tensions);
        let result = resolver.resolve("01arz3").unwrap();
        assert_eq!(result.desired, "goal");
    }

    #[test]
    fn test_resolve_too_short() {
        let tensions = vec![make_tension("01ARZ3N4K5B6C7D8E9F0G1H2I3", "goal")];
        let resolver = PrefixResolver::new(tensions);
        let result = resolver.resolve("01A");
        assert!(matches!(result, Err(WerkError::PrefixTooShort { .. })));
    }

    #[test]
    fn test_resolve_not_found() {
        let tensions = vec![make_tension("01ARZ3N4K5B6C7D8E9F0G1H2I3", "goal")];
        let resolver = PrefixResolver::new(tensions);
        let result = resolver.resolve("ZZZZZZZ");
        assert!(matches!(result, Err(WerkError::TensionNotFound(_))));
    }

    #[test]
    fn test_resolve_ambiguous() {
        let tensions = vec![
            make_tension("01ARZ3N4K5B6C7D8E9F0G1H2I3", "first goal"),
            make_tension("01ARZ3N4K5B6C7D8E9F0G1H2J4", "second goal"),
        ];
        let resolver = PrefixResolver::new(tensions);
        let result = resolver.resolve("01ARZ3");
        assert!(matches!(result, Err(WerkError::AmbiguousPrefix { .. })));
    }

    #[test]
    fn test_resolve_interactive_unique_prefix() {
        let tensions = vec![make_tension("01ARZ3N4K5B6C7D8E9F0G1H2I3", "goal")];
        let resolver = PrefixResolver::new(tensions);
        // Unique prefix should resolve without prompting
        let result = resolver.resolve_interactive("01ARZ3").unwrap();
        assert_eq!(result.desired, "goal");
    }

    #[test]
    fn test_resolve_interactive_not_found() {
        let tensions = vec![make_tension("01ARZ3N4K5B6C7D8E9F0G1H2I3", "goal")];
        let resolver = PrefixResolver::new(tensions);
        let result = resolver.resolve_interactive("ZZZZZZZ");
        assert!(matches!(result, Err(WerkError::TensionNotFound(_))));
    }

    #[test]
    fn test_resolve_interactive_too_short() {
        let tensions = vec![make_tension("01ARZ3N4K5B6C7D8E9F0G1H2I3", "goal")];
        let resolver = PrefixResolver::new(tensions);
        let result = resolver.resolve_interactive("01A");
        assert!(matches!(result, Err(WerkError::PrefixTooShort { .. })));
    }

    #[test]
    fn test_resolve_interactive_ambiguous_non_tty() {
        // In test environment, stdin is not a TTY, so this should error
        let tensions = vec![
            make_tension("01ARZ3N4K5B6C7D8E9F0G1H2I3", "first goal"),
            make_tension("01ARZ3N4K5B6C7D8E9F0G1H2J4", "second goal"),
        ];
        let resolver = PrefixResolver::new(tensions);
        let result = resolver.resolve_interactive("01ARZ3");
        assert!(matches!(result, Err(WerkError::AmbiguousPrefix { .. })));
    }

    #[test]
    fn test_is_valid_prefix() {
        assert!(!PrefixResolver::is_valid_prefix("abc"));
        assert!(PrefixResolver::is_valid_prefix("abcd"));
        assert!(PrefixResolver::is_valid_prefix("abcde"));
    }

    #[test]
    fn test_find_all_returns_all_matches() {
        let tensions = vec![
            make_tension("01ARZ3N4K5B6C7D8E9F0G1H2I3", "first goal"),
            make_tension("01ARZ3N4K5B6C7D8E9F0G1H2J4", "second goal"),
            make_tension("ZZZZZZZZZZZZZZZZZZZZZZZZZZ", "other"),
        ];
        let resolver = PrefixResolver::new(tensions);
        let matches = resolver.find_all("01ARZ3");
        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn test_find_all_short_prefix_returns_empty() {
        let tensions = vec![make_tension("01ARZ3N4K5B6C7D8E9F0G1H2I3", "goal")];
        let resolver = PrefixResolver::new(tensions);
        let matches = resolver.find_all("01A");
        assert!(matches.is_empty());
    }

    #[test]
    fn test_truncate_short_string() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_long_string() {
        assert_eq!(truncate("hello world this is long", 10), "hello worl...");
    }
}

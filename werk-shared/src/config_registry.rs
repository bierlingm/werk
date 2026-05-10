//! Registry of known config keys — single source of truth for every key that
//! werk reads, its default value, its typed shape, and a one-line gloss.
//!
//! Keys are grouped by their top-level namespace (`flush`, `signals`,
//! `agent`, …) — the prefix is the group, which means `reset flush` maps
//! 1:1 to the display. No interpretive layer above the key's own name.

/// Typed shape of a config value. Drives validation on write and
/// canonicalization (e.g. "YES" → "true" for Bool). Not used for display
/// grouping — the namespace prefix is.
///
/// `IntLevels` / `FloatLevels` are named-level tunables. They accept either
/// a label (stored as-is, resolved to the underlying value at read time) or
/// a raw numeric value. The labels array maps each name to its default
/// underlying value and drives the display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Bool,
    Int,
    Float,
    String,
    /// Named integer levels. e.g. `&[("a week", "7"), ("two weeks", "14")]`.
    IntLevels(&'static [(&'static str, &'static str)]),
    /// Named float levels. e.g. `&[("patient", "0.3"), ("balanced", "0.5")]`.
    FloatLevels(&'static [(&'static str, &'static str)]),
    /// Synthetic string levels — no underlying numeric. Used for bundled
    /// keys like `analysis.sensitivity` that cascade to other config keys
    /// when set. The key itself is not stored in config.toml; its displayed
    /// value is inferred by matching the cascade map against what *is* stored.
    StringEnum(&'static [&'static str]),
}

impl Kind {
    /// The underlying numeric kind for a levels variant (or the primitive itself).
    pub fn underlying(&self) -> Kind {
        match self {
            Kind::IntLevels(_) => Kind::Int,
            Kind::FloatLevels(_) => Kind::Float,
            other => *other,
        }
    }

    /// Labels for a levels variant, empty for primitives. For StringEnum,
    /// returns tuples of (name, name) since there's no backing numeric value.
    pub fn labels(&self) -> &'static [(&'static str, &'static str)] {
        match self {
            Kind::IntLevels(labels) | Kind::FloatLevels(labels) => labels,
            Kind::StringEnum(_) => &[], // names-only; use `enum_names()` instead
            _ => &[],
        }
    }

    /// Names for a StringEnum, empty otherwise.
    pub fn enum_names(&self) -> &'static [&'static str] {
        match self {
            Kind::StringEnum(names) => names,
            _ => &[],
        }
    }

    /// Does this Kind support named levels?
    pub fn has_levels(&self) -> bool {
        matches!(
            self,
            Kind::IntLevels(_) | Kind::FloatLevels(_) | Kind::StringEnum(_)
        )
    }

    /// Is this a synthetic key (not stored in config.toml, value inferred from others)?
    pub fn is_synthetic(&self) -> bool {
        matches!(self, Kind::StringEnum(_))
    }
}

/// One entry in the registry.
#[derive(Debug, Clone, Copy)]
pub struct ConfigKey {
    pub key: &'static str,
    pub default: &'static str,
    pub kind: Kind,
    pub gloss: &'static str,
}

/// The full registry. Grouped by top-level namespace; within a namespace,
/// keys are ordered from "most user-facing" to "most tunable-by-experts".
pub const REGISTRY: &[ConfigKey] = &[
    // ── agent ───────────────────────────────────────────────────
    ConfigKey {
        key: "agent.command",
        default: "claude",
        kind: Kind::String,
        gloss: "command invoked by `werk mcp` agent tooling",
    },
    ConfigKey {
        key: "agent.timeout",
        default: "default",
        kind: Kind::IntLevels(&[("quick", "10"), ("default", "30"), ("patient", "120")]),
        gloss: "agent command timeout in seconds",
    },
    // ── analysis ────────────────────────────────────────────────
    ConfigKey {
        key: "analysis.sensitivity",
        default: "balanced",
        kind: Kind::StringEnum(&["relaxed", "balanced", "sharp"]),
        gloss: "pattern recognition bundle (synthesizes the four projection keys)",
    },
    ConfigKey {
        key: "analysis.projection.pattern_window_days",
        default: "30",
        kind: Kind::Int,
        gloss: "window for mutation pattern recognition",
    },
    ConfigKey {
        key: "analysis.projection.neglect_frequency",
        default: "0.1",
        kind: Kind::Float,
        gloss: "mutations/day below this = neglect risk",
    },
    ConfigKey {
        key: "analysis.projection.oscillation_variance",
        default: "0.02",
        kind: Kind::Float,
        gloss: "gap variance above this = oscillation risk",
    },
    ConfigKey {
        key: "analysis.projection.resolution_gap",
        default: "0.05",
        kind: Kind::Float,
        gloss: "desire-reality gap below this = resolved",
    },
    // ── display ─────────────────────────────────────────────────
    ConfigKey {
        key: "display.theme",
        default: "auto",
        kind: Kind::String,
        gloss: "color theme: auto | light | dark",
    },
    // ── editor ──────────────────────────────────────────────────
    ConfigKey {
        key: "editor.command",
        default: "",
        kind: Kind::String,
        gloss: "override $EDITOR for werk (empty = use $EDITOR)",
    },
    // ── flush ───────────────────────────────────────────────────
    ConfigKey {
        key: "flush.auto",
        default: "false",
        kind: Kind::Bool,
        gloss: "auto-flush tensions.json after every mutation",
    },
    ConfigKey {
        key: "flush.include_released",
        default: "false",
        kind: Kind::Bool,
        gloss: "include released tensions in tensions.json",
    },
    // ── hooks ───────────────────────────────────────────────────
    ConfigKey {
        key: "hooks.log_tail",
        default: "20",
        kind: Kind::Int,
        gloss: "default --tail for `werk hooks log`",
    },
    // ── list ────────────────────────────────────────────────────
    ConfigKey {
        key: "list.default_sort",
        default: "urgency",
        kind: Kind::String,
        gloss: "default sort: urgency | name | deadline | created | updated | position",
    },
    // ── serve ───────────────────────────────────────────────────
    ConfigKey {
        key: "serve.port",
        default: "3749",
        kind: Kind::Int,
        gloss: "default port for `werk serve`",
    },
    ConfigKey {
        key: "serve.host",
        default: "127.0.0.1",
        kind: Kind::String,
        gloss: "default bind host for `werk serve`",
    },
    // ── signals ─────────────────────────────────────────────────
    ConfigKey {
        key: "signals.approaching.days",
        default: "two weeks",
        kind: Kind::IntLevels(&[("a week", "7"), ("two weeks", "14"), ("a month", "30")]),
        gloss: "days-to-deadline considered \"approaching\"",
    },
    ConfigKey {
        key: "signals.approaching.urgency",
        default: "balanced",
        kind: Kind::FloatLevels(&[("patient", "0.3"), ("balanced", "0.5"), ("alert", "0.7")]),
        gloss: "urgency threshold for \"approaching\"",
    },
    ConfigKey {
        key: "signals.stale.days",
        default: "two weeks",
        kind: Kind::IntLevels(&[
            ("a few days", "3"),
            ("a week", "7"),
            ("two weeks", "14"),
            ("a month", "30"),
        ]),
        gloss: "days without mutation before a tension is stale",
    },
    ConfigKey {
        key: "signals.drift.threshold",
        default: "noticeable",
        kind: Kind::FloatLevels(&[("subtle", "0.1"), ("noticeable", "0.3"), ("large", "0.5")]),
        gloss: "desire-reality gap above which DRIFT fires",
    },
    ConfigKey {
        key: "signals.hub.centrality",
        default: "sensitive",
        kind: Kind::FloatLevels(&[
            ("wide", "0.00001"),
            ("sensitive", "0.0001"),
            ("strict", "0.001"),
        ]),
        gloss: "betweenness threshold for HUB glyph",
    },
    ConfigKey {
        key: "signals.reach.descendants",
        default: "medium",
        kind: Kind::IntLevels(&[("small", "3"), ("medium", "5"), ("large", "10")]),
        gloss: "descendant count for REACH glyph",
    },
    // ── stats ───────────────────────────────────────────────────
    ConfigKey {
        key: "stats.default_window_days",
        default: "a week",
        kind: Kind::IntLevels(&[
            ("today", "1"),
            ("a week", "7"),
            ("two weeks", "14"),
            ("a month", "30"),
        ]),
        gloss: "default --days for `werk stats`",
    },
];

/// Find a registry entry by key name.
pub fn lookup(key: &str) -> Option<&'static ConfigKey> {
    REGISTRY.iter().find(|k| k.key == key)
}

/// Extract the top-level namespace of a dotted key. `signals.stale.days` → `signals`.
/// A key without a dot returns the whole string.
pub fn group_of(key: &str) -> &str {
    key.split('.').next().unwrap_or(key)
}

/// Unique top-level namespaces in the registry, alphabetically sorted.
/// Drives the grouped display in `werk config`.
pub fn groups() -> Vec<&'static str> {
    let mut seen: Vec<&'static str> = REGISTRY.iter().map(|k| group_of(k.key)).collect();
    seen.sort_unstable();
    seen.dedup();
    seen
}

/// All registry keys whose top-level namespace matches `group`, preserving
/// registry order.
pub fn keys_in_group(group: &str) -> impl Iterator<Item = &'static ConfigKey> + '_ {
    REGISTRY.iter().filter(move |k| group_of(k.key) == group)
}

/// All registry keys whose name starts with `prefix`. Used by `reset <prefix>`.
/// A trailing dot is implied: `prefix("signals")` matches `signals.stale.days`
/// but not `signalsomething`.
pub fn keys_with_prefix(prefix: &str) -> impl Iterator<Item = &'static ConfigKey> + '_ {
    REGISTRY
        .iter()
        .filter(move |k| k.key == prefix || k.key.starts_with(&format!("{prefix}.")))
}

/// Does this key look like a hook key? Hooks are user-defined (any `post_*` or
/// `pre_*`) so they never appear in the registry, but the display groups them
/// under their own heading.
pub fn is_hook_key(key: &str) -> bool {
    key.starts_with("post_") || key.starts_with("pre_")
}

/// Validate that a raw string value parses as the declared Kind. Returns the
/// canonical string form on success (e.g. "TRUE" → "true"). For levels
/// kinds, a matching label is stored as-is; a raw value must parse as the
/// underlying primitive.
pub fn validate(kind: Kind, raw: &str) -> Result<String, String> {
    match kind {
        Kind::Bool => match raw.trim().to_ascii_lowercase().as_str() {
            "true" | "yes" | "1" | "on" => Ok("true".into()),
            "false" | "no" | "0" | "off" => Ok("false".into()),
            other => Err(format!("expected bool (true/false), got '{other}'")),
        },
        Kind::Int => raw
            .trim()
            .parse::<i64>()
            .map(|n| n.to_string())
            .map_err(|e| format!("expected integer: {e}")),
        Kind::Float => raw
            .trim()
            .parse::<f64>()
            .map(|n| n.to_string())
            .map_err(|e| format!("expected number: {e}")),
        Kind::String => Ok(raw.to_string()),
        Kind::StringEnum(names) => {
            let trimmed = raw.trim();
            if names.iter().any(|n| *n == trimmed) {
                Ok(trimmed.to_string())
            } else {
                Err(format!("expected one of [{}]", names.join(", ")))
            }
        }
        Kind::IntLevels(labels) | Kind::FloatLevels(labels) => {
            let trimmed = raw.trim();
            // Exact label match — store as-is.
            if labels.iter().any(|(name, _)| *name == trimmed) {
                return Ok(trimmed.to_string());
            }
            // Otherwise must parse as the underlying numeric type.
            let result = if matches!(kind, Kind::IntLevels(_)) {
                trimmed.parse::<i64>().map(|n| n.to_string()).map_err(|e| {
                    format!("expected integer or one of [{}]: {e}", label_list(labels))
                })
            } else {
                trimmed
                    .parse::<f64>()
                    .map(|n| n.to_string())
                    .map_err(|e| format!("expected number or one of [{}]: {e}", label_list(labels)))
            };
            result
        }
    }
}

fn label_list(labels: &[(&str, &str)]) -> String {
    labels
        .iter()
        .map(|(n, _)| *n)
        .collect::<Vec<_>>()
        .join(", ")
}

/// Resolve a stored config value to its underlying raw form. If the stored
/// value is a level label, returns the backing numeric string. Otherwise
/// returns the value unchanged. This is what consumers call before parsing.
pub fn resolve_value(key: &str, stored: &str) -> String {
    let Some(entry) = lookup(key) else {
        return stored.to_string();
    };
    let labels = entry.kind.labels();
    for (name, value) in labels {
        if *name == stored {
            return (*value).to_string();
        }
    }
    stored.to_string()
}

/// A named bundle of key-value pairs users can snap to. Shipped presets
/// embody practice stances (focus / patient / quiet) rather than single
/// tunables. Values can be labels — the Set handler resolves them and
/// handles synthetic cascades (e.g. `analysis.sensitivity = "sharp"`).
#[derive(Debug, Clone, Copy)]
pub struct Preset {
    pub name: &'static str,
    pub description: &'static str,
    pub values: &'static [(&'static str, &'static str)],
}

pub const PRESETS: &[Preset] = &[
    Preset {
        name: "focus",
        description: "aggressive signals, short windows — for sprinting",
        values: &[
            ("signals.approaching.days", "a week"),
            ("signals.approaching.urgency", "alert"),
            ("signals.stale.days", "a few days"),
            ("signals.drift.threshold", "subtle"),
            ("analysis.sensitivity", "sharp"),
        ],
    },
    Preset {
        name: "patient",
        description: "relaxed signals, long windows — for strategy",
        values: &[
            ("signals.approaching.days", "a month"),
            ("signals.approaching.urgency", "patient"),
            ("signals.stale.days", "a month"),
            ("signals.drift.threshold", "large"),
            ("analysis.sensitivity", "relaxed"),
        ],
    },
    Preset {
        name: "quiet",
        description: "minimal signals — for review mode",
        values: &[
            ("signals.approaching.days", "a month"),
            ("signals.approaching.urgency", "patient"),
            ("signals.stale.days", "a month"),
            ("signals.hub.centrality", "strict"),
            ("signals.reach.descendants", "large"),
            ("signals.drift.threshold", "large"),
        ],
    },
    Preset {
        name: "default",
        description: "werk's shipped defaults",
        values: &[
            ("signals.approaching.days", "two weeks"),
            ("signals.approaching.urgency", "balanced"),
            ("signals.stale.days", "two weeks"),
            ("signals.drift.threshold", "noticeable"),
            ("signals.hub.centrality", "sensitive"),
            ("signals.reach.descendants", "medium"),
            ("analysis.sensitivity", "balanced"),
        ],
    },
];

/// Look up a preset by name.
pub fn preset(name: &str) -> Option<&'static Preset> {
    PRESETS.iter().find(|p| p.name == name)
}

/// The cascade definitions for synthetic keys. `analysis.sensitivity = "sharp"`
/// expands to four writes against `analysis.projection.*`. Setting the
/// synthetic key itself does NOT persist — only the cascaded writes do.
pub fn cascade_for(
    synthetic_key: &str,
    label: &str,
) -> Option<&'static [(&'static str, &'static str)]> {
    match (synthetic_key, label) {
        ("analysis.sensitivity", "relaxed") => Some(&[
            ("analysis.projection.pattern_window_days", "60"),
            ("analysis.projection.neglect_frequency", "0.05"),
            ("analysis.projection.oscillation_variance", "0.04"),
            ("analysis.projection.resolution_gap", "0.08"),
        ]),
        ("analysis.sensitivity", "balanced") => Some(&[
            ("analysis.projection.pattern_window_days", "30"),
            ("analysis.projection.neglect_frequency", "0.1"),
            ("analysis.projection.oscillation_variance", "0.02"),
            ("analysis.projection.resolution_gap", "0.05"),
        ]),
        ("analysis.sensitivity", "sharp") => Some(&[
            ("analysis.projection.pattern_window_days", "14"),
            ("analysis.projection.neglect_frequency", "0.15"),
            ("analysis.projection.oscillation_variance", "0.01"),
            ("analysis.projection.resolution_gap", "0.03"),
        ]),
        _ => None,
    }
}

/// Infer the current value of a synthetic key from the live config map.
/// Returns the label whose cascade matches every underlying value; `None`
/// if no bundle matches (== "custom").
pub fn infer_synthetic(
    synthetic_key: &str,
    config: &std::collections::BTreeMap<String, String>,
) -> Option<&'static str> {
    let entry = lookup(synthetic_key)?;
    for name in entry.kind.enum_names() {
        let Some(bundle) = cascade_for(synthetic_key, name) else {
            continue;
        };
        let matches_all = bundle.iter().all(|(k, v)| {
            let stored = config.get(*k).map(String::as_str);
            let resolved = stored.map(|s| resolve_value(k, s));
            resolved.as_deref() == Some(*v)
                || (stored.is_none() && lookup(k).map(|e| e.default) == Some(*v))
        });
        if matches_all {
            return Some(name);
        }
    }
    None
}

/// The label matching a stored value (or backing value), if any. Used by
/// display to render `balanced (0.5)` when the user set `0.5` directly.
pub fn label_for(key: &str, stored: &str) -> Option<&'static str> {
    let entry = lookup(key)?;
    let labels = entry.kind.labels();
    for (name, value) in labels {
        if *name == stored || *value == stored {
            return Some(*name);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_registry_key_has_unique_name() {
        let mut seen = std::collections::HashSet::new();
        for k in REGISTRY {
            assert!(seen.insert(k.key), "duplicate registry key: {}", k.key);
        }
    }

    #[test]
    fn groups_derive_from_key_prefixes() {
        let gs = groups();
        // Spot-check: every known prefix should show up exactly once.
        for want in &[
            "agent", "analysis", "display", "editor", "flush", "hooks", "list", "serve", "signals",
            "stats",
        ] {
            assert!(gs.contains(want), "missing group: {want}");
        }
        // And the list is sorted + deduped.
        let mut sorted = gs.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(gs, sorted);
    }

    #[test]
    fn keys_in_group_returns_only_that_prefix() {
        let signals: Vec<_> = keys_in_group("signals").map(|k| k.key).collect();
        assert!(signals.contains(&"signals.stale.days"));
        assert!(signals.iter().all(|k| k.starts_with("signals.")));
        let flush: Vec<_> = keys_in_group("flush").map(|k| k.key).collect();
        assert_eq!(flush, vec!["flush.auto", "flush.include_released"]);
    }

    #[test]
    fn defaults_parse_as_their_declared_kind() {
        for k in REGISTRY {
            validate(k.kind, k.default)
                .unwrap_or_else(|e| panic!("default for {} fails validation: {}", k.key, e));
        }
    }

    #[test]
    fn hook_key_detection() {
        assert!(is_hook_key("post_mutation"));
        assert!(is_hook_key("pre_delete"));
        assert!(!is_hook_key("agent.command"));
        assert!(!is_hook_key("postmortem"));
    }

    #[test]
    fn bool_validation_is_lenient_but_canonicalizes() {
        assert_eq!(validate(Kind::Bool, "TRUE").unwrap(), "true");
        assert_eq!(validate(Kind::Bool, "yes").unwrap(), "true");
        assert_eq!(validate(Kind::Bool, "off").unwrap(), "false");
        assert!(validate(Kind::Bool, "maybe").is_err());
    }

    #[test]
    fn prefix_lookup_matches_dotted_children_only() {
        let signals: Vec<_> = keys_with_prefix("signals").map(|k| k.key).collect();
        assert!(signals.contains(&"signals.stale.days"));
        assert!(signals.contains(&"signals.drift.threshold"));
        assert!(signals.iter().all(|k| k.starts_with("signals.")));
        // Empty prefix is not a wildcard — guard against accidental "match all".
        // Actual "reset all" uses the None branch in cmd_config, not prefix.
        let exact = keys_with_prefix("signals.stale.days").count();
        assert_eq!(exact, 1);
    }
}

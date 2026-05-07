//! Address parser for the deep addressability scheme.
//!
//! Addresses are the universal referencing syntax for all structural entities:
//!
//! - `#42` or `42` — tension by short code
//! - `#42~e3` — epoch 3 of tension 42
//! - `#42.n3` — note 3 of tension 42
//! - `#42@2026-03` — tension 42 as of March 2026
//! - `g:01JQXYZ...` — gesture by ULID
//! - `s:20260328-1` — session by date and sequence
//! - `*7` — sigil by short code
//! - `werk:42` — tension 42 in space "werk" (cross-space)
//! - `journal:7~e3` — epoch 3 of tension 7 in space "journal"
//!
//! Cross-space disambiguation: single-char prefixes before `:` are sigils
//! (`g:`, `s:`). Prefixes of ≥2 characters matching `[a-z0-9][a-z0-9_-]*`
//! are space names. This is deterministic — no registry lookup at parse time.
//!
//! All syntax is shell-safe (no expansion in unquoted contexts).

use std::fmt;

/// A parsed address referencing a structural entity.
#[derive(Debug, Clone, PartialEq)]
pub enum Address {
    /// A tension by short code: `#42` or `42`
    Tension(i32),
    /// An epoch within a tension: `#42~e3` (epoch number 3)
    Epoch { tension: i32, epoch_num: usize },
    /// A note within a tension: `#42.n3` (note number 3)
    Note { tension: i32, note_num: usize },
    /// A tension at a point in time: `#42@2026-03`
    TensionAt { tension: i32, timespec: String },
    /// A gesture by ID: `g:01JQXYZ...`
    Gesture(String),
    /// A session by date and sequence: `s:20260328-1`
    Session(String),
    /// A sigil short code: `*7`
    Sigil(i32),
    /// A cross-space address: `werk:42`, `journal:7~e3`
    ///
    /// The space name is ≥2 chars matching `[a-z0-9][a-z0-9_-]*`.
    /// The inner address is any non-CrossSpace variant.
    CrossSpace { space: String, inner: Box<Address> },
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Address::Tension(n) => write!(f, "#{}", n),
            Address::Epoch { tension, epoch_num } => write!(f, "#{}~e{}", tension, epoch_num),
            Address::Note { tension, note_num } => write!(f, "#{}.n{}", tension, note_num),
            Address::TensionAt { tension, timespec } => write!(f, "#{}@{}", tension, timespec),
            Address::Gesture(id) => write!(f, "g:{}", id),
            Address::Session(id) => write!(f, "s:{}", id),
            Address::Sigil(n) => write!(f, "*{}", n),
            Address::CrossSpace { space, inner } => write!(f, "{}:{}", space, inner),
        }
    }
}

impl Address {
    /// Returns true if this is a cross-space address.
    pub fn is_cross_space(&self) -> bool {
        matches!(self, Address::CrossSpace { .. })
    }

    /// If this is a CrossSpace address, returns (space_name, inner_address).
    pub fn as_cross_space(&self) -> Option<(&str, &Address)> {
        match self {
            Address::CrossSpace { space, inner } => Some((space, inner)),
            _ => None,
        }
    }
}

/// Error when parsing an address.
#[derive(Debug, Clone, PartialEq)]
pub struct AddressParseError {
    pub input: String,
    pub reason: String,
}

impl fmt::Display for AddressParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid address '{}': {}", self.input, self.reason)
    }
}

impl std::error::Error for AddressParseError {}

/// Parse an address string into a structured Address.
///
/// Accepts:
/// - `42` or `#42` — tension
/// - `#42~e3` — epoch
/// - `#42.n3` — note
/// - `#42@2026-03` — temporal
/// - `g:ULID` — gesture
/// - `s:DATE-N` — session
/// - `*7` — sigil
/// - `werk:42` — cross-space tension
/// - `journal:7~e3` — cross-space epoch (all inner variants compose)
pub fn parse_address(input: &str) -> Result<Address, AddressParseError> {
    let input = input.trim();

    if input.is_empty() {
        return Err(AddressParseError {
            input: input.to_owned(),
            reason: "empty address".to_owned(),
        });
    }

    // Gesture: g:...  (single-char sigil, checked before cross-space)
    if let Some(rest) = input.strip_prefix("g:") {
        if rest.is_empty() {
            return Err(AddressParseError {
                input: input.to_owned(),
                reason: "gesture ID is empty".to_owned(),
            });
        }
        return Ok(Address::Gesture(rest.to_owned()));
    }

    // Session: s:...  (single-char sigil, checked before cross-space)
    if let Some(rest) = input.strip_prefix("s:") {
        if rest.is_empty() {
            return Err(AddressParseError {
                input: input.to_owned(),
                reason: "session ID is empty".to_owned(),
            });
        }
        return Ok(Address::Session(rest.to_owned()));
    }

    // Cross-space: <name>:<inner> where name is ≥2 chars, [a-z0-9][a-z0-9_-]*
    // Must be checked before # stripping — `werk:42` has no # prefix.
    if let Some(colon_pos) = input.find(':') {
        let candidate = &input[..colon_pos];
        if is_space_name(candidate) {
            let rest = &input[colon_pos + 1..];
            if rest.is_empty() {
                return Err(AddressParseError {
                    input: input.to_owned(),
                    reason: format!("empty address after space name '{}'", candidate),
                });
            }
            let inner = parse_address_inner(rest, input)?;
            return Ok(Address::CrossSpace {
                space: candidate.to_owned(),
                inner: Box::new(inner),
            });
        }
    }

    // Tension-based (local): strip optional # prefix, parse inner
    parse_address_inner(input, input)
}

/// Parse the inner (non-cross-space) portion of an address.
/// `full_input` is carried for error messages.
fn parse_address_inner(input: &str, full_input: &str) -> Result<Address, AddressParseError> {
    let had_hash = input.starts_with('#');
    let body = input.strip_prefix('#').unwrap_or(input);

    if let Some(rest) = body.strip_prefix('*') {
        if had_hash {
            return Err(AddressParseError {
                input: full_input.to_owned(),
                reason: "sigil short code cannot be prefixed with #".to_owned(),
            });
        }
        if rest.is_empty() {
            return Err(AddressParseError {
                input: full_input.to_owned(),
                reason: "sigil short code is empty".to_owned(),
            });
        }
        let sigil = rest.parse().map_err(|_| AddressParseError {
            input: full_input.to_owned(),
            reason: format!("invalid sigil short code: '{}'", rest),
        })?;
        return Ok(Address::Sigil(sigil));
    }

    // Find sub-addressing sigil: ~ (epoch), . (note/sub), @ (temporal)
    // Try epoch: ~e<N>
    if let Some(pos) = body.find('~') {
        let (num_part, rest) = body.split_at(pos);
        let tension = parse_short_code(num_part, full_input)?;
        let rest = &rest[1..]; // skip ~
        if let Some(epoch_str) = rest.strip_prefix('e') {
            let epoch_num: usize = epoch_str.parse().map_err(|_| AddressParseError {
                input: full_input.to_owned(),
                reason: format!("invalid epoch number: '{}'", epoch_str),
            })?;
            return Ok(Address::Epoch { tension, epoch_num });
        }
        return Err(AddressParseError {
            input: full_input.to_owned(),
            reason: format!("unknown sub-address after ~: '{}'", rest),
        });
    }

    // Try note: .n<N>
    if let Some(pos) = body.find(".n") {
        let (num_part, rest) = body.split_at(pos);
        let tension = parse_short_code(num_part, full_input)?;
        let note_str = &rest[2..]; // skip .n
        let note_num: usize = note_str.parse().map_err(|_| AddressParseError {
            input: full_input.to_owned(),
            reason: format!("invalid note number: '{}'", note_str),
        })?;
        return Ok(Address::Note { tension, note_num });
    }

    // Try temporal: @<timespec>
    if let Some(pos) = body.find('@') {
        let (num_part, rest) = body.split_at(pos);
        let tension = parse_short_code(num_part, full_input)?;
        let timespec = &rest[1..]; // skip @
        if timespec.is_empty() {
            return Err(AddressParseError {
                input: full_input.to_owned(),
                reason: "timespec is empty after @".to_owned(),
            });
        }
        return Ok(Address::TensionAt {
            tension,
            timespec: timespec.to_owned(),
        });
    }

    // Plain tension
    let tension = parse_short_code(body, full_input)?;
    Ok(Address::Tension(tension))
}

/// Check if a string is a valid space name for cross-space addressing.
/// Must be ≥2 characters, start with [a-z0-9], rest [a-z0-9_-].
/// This mirrors registry::validate_name() constraints but is a pure
/// syntactic check — no I/O, no registry lookup.
fn is_space_name(s: &str) -> bool {
    if s.len() < 2 {
        return false;
    }
    let mut chars = s.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_alphanumeric() {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn parse_short_code(s: &str, full_input: &str) -> Result<i32, AddressParseError> {
    s.parse().map_err(|_| AddressParseError {
        input: full_input.to_owned(),
        reason: format!("invalid short code: '{}'", s),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tension() {
        assert_eq!(parse_address("42").unwrap(), Address::Tension(42));
        assert_eq!(parse_address("#42").unwrap(), Address::Tension(42));
        assert_eq!(parse_address(" #42 ").unwrap(), Address::Tension(42));
    }

    #[test]
    fn test_epoch() {
        assert_eq!(
            parse_address("#42~e3").unwrap(),
            Address::Epoch {
                tension: 42,
                epoch_num: 3
            }
        );
        assert_eq!(
            parse_address("42~e0").unwrap(),
            Address::Epoch {
                tension: 42,
                epoch_num: 0
            }
        );
    }

    #[test]
    fn test_note() {
        assert_eq!(
            parse_address("#42.n3").unwrap(),
            Address::Note {
                tension: 42,
                note_num: 3
            }
        );
    }

    #[test]
    fn test_temporal() {
        assert_eq!(
            parse_address("#42@2026-03").unwrap(),
            Address::TensionAt {
                tension: 42,
                timespec: "2026-03".to_owned()
            }
        );
        assert_eq!(
            parse_address("#42@last-week").unwrap(),
            Address::TensionAt {
                tension: 42,
                timespec: "last-week".to_owned()
            }
        );
    }

    #[test]
    fn test_gesture() {
        assert_eq!(
            parse_address("g:01JQXYZ").unwrap(),
            Address::Gesture("01JQXYZ".to_owned())
        );
    }

    #[test]
    fn test_session() {
        assert_eq!(
            parse_address("s:20260328-1").unwrap(),
            Address::Session("20260328-1".to_owned())
        );
    }

    #[test]
    fn test_cross_space_tension() {
        assert_eq!(
            parse_address("werk:42").unwrap(),
            Address::CrossSpace {
                space: "werk".to_owned(),
                inner: Box::new(Address::Tension(42)),
            }
        );
        assert_eq!(
            parse_address("journal:3").unwrap(),
            Address::CrossSpace {
                space: "journal".to_owned(),
                inner: Box::new(Address::Tension(3)),
            }
        );
    }

    #[test]
    fn test_cross_space_with_hash() {
        // `werk:#42` — hash in inner part
        assert_eq!(
            parse_address("werk:#42").unwrap(),
            Address::CrossSpace {
                space: "werk".to_owned(),
                inner: Box::new(Address::Tension(42)),
            }
        );
    }

    #[test]
    fn test_cross_space_epoch() {
        assert_eq!(
            parse_address("werk:42~e3").unwrap(),
            Address::CrossSpace {
                space: "werk".to_owned(),
                inner: Box::new(Address::Epoch {
                    tension: 42,
                    epoch_num: 3,
                }),
            }
        );
    }

    #[test]
    fn test_cross_space_note() {
        assert_eq!(
            parse_address("journal:7.n3").unwrap(),
            Address::CrossSpace {
                space: "journal".to_owned(),
                inner: Box::new(Address::Note {
                    tension: 7,
                    note_num: 3,
                }),
            }
        );
    }

    #[test]
    fn test_cross_space_temporal() {
        assert_eq!(
            parse_address("werk:42@2026-03").unwrap(),
            Address::CrossSpace {
                space: "werk".to_owned(),
                inner: Box::new(Address::TensionAt {
                    tension: 42,
                    timespec: "2026-03".to_owned(),
                }),
            }
        );
    }

    #[test]
    fn test_cross_space_with_hyphens_underscores() {
        assert_eq!(
            parse_address("desk-werk:10").unwrap(),
            Address::CrossSpace {
                space: "desk-werk".to_owned(),
                inner: Box::new(Address::Tension(10)),
            }
        );
        assert_eq!(
            parse_address("my_journal:5").unwrap(),
            Address::CrossSpace {
                space: "my_journal".to_owned(),
                inner: Box::new(Address::Tension(5)),
            }
        );
    }

    #[test]
    fn test_sigil_not_cross_space() {
        // g: and s: are sigils, not space names (single char)
        assert_eq!(
            parse_address("g:01JQXYZ").unwrap(),
            Address::Gesture("01JQXYZ".to_owned())
        );
        assert_eq!(
            parse_address("s:20260328-1").unwrap(),
            Address::Session("20260328-1".to_owned())
        );
    }

    #[test]
    fn test_cross_space_empty_inner() {
        assert!(parse_address("werk:").is_err());
    }

    #[test]
    fn test_single_char_not_space_name() {
        // Single char before colon is not a space name — falls through
        // to tension parsing which will fail on the colon
        assert!(parse_address("x:42").is_err());
    }

    #[test]
    fn test_cross_space_accessors() {
        let addr = parse_address("werk:42").unwrap();
        assert!(addr.is_cross_space());
        let (space, inner) = addr.as_cross_space().unwrap();
        assert_eq!(space, "werk");
        assert_eq!(*inner, Address::Tension(42));

        let local = parse_address("42").unwrap();
        assert!(!local.is_cross_space());
        assert!(local.as_cross_space().is_none());
    }

    #[test]
    fn test_errors() {
        assert!(parse_address("").is_err());
        assert!(parse_address("g:").is_err());
        assert!(parse_address("s:").is_err());
        assert!(parse_address("#abc").is_err());
        assert!(parse_address("#42~x3").is_err());
        assert!(parse_address("#42@").is_err());
    }

    #[test]
    fn test_display_roundtrip() {
        let cases = vec![
            "#42",
            "#42~e3",
            "#42.n3",
            "#42@2026-03",
            "g:01JQXYZ",
            "s:20260328-1",
            "*7",
            "werk:#42",
            "werk:#42~e3",
            "journal:#7.n3",
            "werk:#42@2026-03",
            "werk:*7",
        ];
        for case in cases {
            let addr = parse_address(case).unwrap();
            let displayed = addr.to_string();
            let reparsed = parse_address(&displayed).unwrap();
            assert_eq!(addr, reparsed, "roundtrip failed for {}", case);
        }
    }

    #[test]
    fn test_sigil() {
        assert_eq!(parse_address("*7").unwrap(), Address::Sigil(7));
    }

    #[test]
    fn test_cross_space_sigil() {
        assert_eq!(
            parse_address("werk:*7").unwrap(),
            Address::CrossSpace {
                space: "werk".to_owned(),
                inner: Box::new(Address::Sigil(7)),
            }
        );
    }

    #[test]
    fn sigil_prefix_collision_rejected() {
        assert!(parse_address("#*7").is_err());
        assert_eq!(
            parse_address("g:*7").unwrap(),
            Address::Gesture("*7".to_owned())
        );
        assert!(parse_address("*").is_err());
    }
}

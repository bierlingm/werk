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
pub fn parse_address(input: &str) -> Result<Address, AddressParseError> {
    let input = input.trim();

    if input.is_empty() {
        return Err(AddressParseError {
            input: input.to_owned(),
            reason: "empty address".to_owned(),
        });
    }

    // Gesture: g:...
    if let Some(rest) = input.strip_prefix("g:") {
        if rest.is_empty() {
            return Err(AddressParseError {
                input: input.to_owned(),
                reason: "gesture ID is empty".to_owned(),
            });
        }
        return Ok(Address::Gesture(rest.to_owned()));
    }

    // Session: s:...
    if let Some(rest) = input.strip_prefix("s:") {
        if rest.is_empty() {
            return Err(AddressParseError {
                input: input.to_owned(),
                reason: "session ID is empty".to_owned(),
            });
        }
        return Ok(Address::Session(rest.to_owned()));
    }

    // Tension-based: strip optional # prefix
    let body = input.strip_prefix('#').unwrap_or(input);

    // Find sub-addressing sigil: ~ (epoch), . (note/sub), @ (temporal)
    // Try epoch: ~e<N>
    if let Some(pos) = body.find('~') {
        let (num_part, rest) = body.split_at(pos);
        let tension = parse_short_code(num_part, input)?;
        let rest = &rest[1..]; // skip ~
        if let Some(epoch_str) = rest.strip_prefix('e') {
            let epoch_num: usize = epoch_str.parse().map_err(|_| AddressParseError {
                input: input.to_owned(),
                reason: format!("invalid epoch number: '{}'", epoch_str),
            })?;
            return Ok(Address::Epoch { tension, epoch_num });
        }
        return Err(AddressParseError {
            input: input.to_owned(),
            reason: format!("unknown sub-address after ~: '{}'", rest),
        });
    }

    // Try note: .n<N>
    if let Some(pos) = body.find(".n") {
        let (num_part, rest) = body.split_at(pos);
        let tension = parse_short_code(num_part, input)?;
        let note_str = &rest[2..]; // skip .n
        let note_num: usize = note_str.parse().map_err(|_| AddressParseError {
            input: input.to_owned(),
            reason: format!("invalid note number: '{}'", note_str),
        })?;
        return Ok(Address::Note { tension, note_num });
    }

    // Try temporal: @<timespec>
    if let Some(pos) = body.find('@') {
        let (num_part, rest) = body.split_at(pos);
        let tension = parse_short_code(num_part, input)?;
        let timespec = &rest[1..]; // skip @
        if timespec.is_empty() {
            return Err(AddressParseError {
                input: input.to_owned(),
                reason: "timespec is empty after @".to_owned(),
            });
        }
        return Ok(Address::TensionAt {
            tension,
            timespec: timespec.to_owned(),
        });
    }

    // Plain tension
    let tension = parse_short_code(body, input)?;
    Ok(Address::Tension(tension))
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
        ];
        for case in cases {
            let addr = parse_address(case).unwrap();
            let displayed = addr.to_string();
            let reparsed = parse_address(&displayed).unwrap();
            assert_eq!(addr, reparsed, "roundtrip failed for {}", case);
        }
    }
}

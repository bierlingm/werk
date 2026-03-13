//! Structured agent response parsing and mutation application.
//!
//! Agents in one-shot mode can return structured YAML with mutation suggestions
//! that the user can review and apply in bulk.

use serde::{Deserialize, Serialize};

/// A structured response from an agent, containing prose advice and
/// optional mutation suggestions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredResponse {
    /// Suggested mutations to the tension forest.
    #[serde(default)]
    pub mutations: Vec<Mutation>,
    /// Human-readable advice/response text.
    #[serde(default)]
    pub response: String,
}

/// A single suggested mutation to the tension forest.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Mutation {
    /// Update the actual state of a tension.
    UpdateActual {
        tension_id: String,
        new_value: String,
        #[serde(default)]
        reasoning: String,
    },
    /// Create a child tension under a parent.
    CreateChild {
        parent_id: String,
        desired: String,
        actual: String,
        #[serde(default)]
        reasoning: String,
    },
    /// Add a note to a tension.
    AddNote {
        tension_id: String,
        text: String,
    },
    /// Update the status of a tension.
    UpdateStatus {
        tension_id: String,
        new_status: String,
        #[serde(default)]
        reasoning: String,
    },
    /// Update the desired state of a tension.
    UpdateDesired {
        tension_id: String,
        new_value: String,
        #[serde(default)]
        reasoning: String,
    },
}

impl Mutation {
    /// Return a human-readable summary of this mutation.
    pub fn summary(&self) -> String {
        match self {
            Mutation::UpdateActual { new_value, .. } => {
                format!("Update actual: \"{}\"", truncate(new_value, 60))
            }
            Mutation::CreateChild { desired, .. } => {
                format!("Create child: \"{}\"", truncate(desired, 60))
            }
            Mutation::AddNote { text, .. } => {
                format!("Add note: \"{}\"", truncate(text, 60))
            }
            Mutation::UpdateStatus { new_status, .. } => {
                format!("Set status: {}", new_status)
            }
            Mutation::UpdateDesired { new_value, .. } => {
                format!("Update desired: \"{}\"", truncate(new_value, 60))
            }
        }
    }

    /// Return the reasoning for this mutation, if any.
    pub fn reasoning(&self) -> Option<&str> {
        match self {
            Mutation::UpdateActual { reasoning, .. }
            | Mutation::CreateChild { reasoning, .. }
            | Mutation::UpdateStatus { reasoning, .. }
            | Mutation::UpdateDesired { reasoning, .. } => {
                if reasoning.is_empty() {
                    None
                } else {
                    Some(reasoning)
                }
            }
            Mutation::AddNote { .. } => None,
        }
    }
}

impl StructuredResponse {
    /// Try to parse a structured response from YAML text.
    ///
    /// The agent response must contain YAML between `---` markers with
    /// both `mutations` and `response` keys. Returns None if the text
    /// doesn't contain valid structured YAML.
    pub fn from_response(text: &str) -> Option<Self> {
        // Only try to parse YAML from between --- markers
        let yaml_text = extract_yaml_block(text)?;

        // Must contain both expected keys to be considered structured
        if !yaml_text.contains("mutations") || !yaml_text.contains("response") {
            return None;
        }

        // Try parsing as StructuredResponse
        let parsed: Self = serde_yaml::from_str(yaml_text).ok()?;

        // Require non-empty response text
        if parsed.response.trim().is_empty() {
            return None;
        }

        Some(parsed)
    }
}

/// Extract YAML content between `---` markers.
fn extract_yaml_block(text: &str) -> Option<&str> {
    let trimmed = text.trim();

    // Look for content between --- markers
    if let Some(start) = trimmed.find("---") {
        let after_start = &trimmed[start + 3..];
        if let Some(end) = after_start.find("---") {
            let yaml = after_start[..end].trim();
            if !yaml.is_empty() {
                return Some(yaml);
            }
        }
        // If no closing ---, treat the rest as YAML
        let yaml = after_start.trim();
        if !yaml.is_empty() {
            return Some(yaml);
        }
    }

    None
}

/// Truncate a string for display (Unicode-safe).
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

    #[test]
    fn test_parse_simple_yaml() {
        let yaml = r#"
---
mutations:
  - action: update_actual
    tension_id: "ABC123"
    new_value: "Research complete"
    reasoning: "Progress made"
response: |
  Good work on the research.
---
"#;
        let parsed = StructuredResponse::from_response(yaml).unwrap();
        assert_eq!(parsed.mutations.len(), 1);
        assert!(parsed.response.contains("Good work"));

        match &parsed.mutations[0] {
            Mutation::UpdateActual {
                tension_id,
                new_value,
                reasoning,
            } => {
                assert_eq!(tension_id, "ABC123");
                assert_eq!(new_value, "Research complete");
                assert_eq!(reasoning, "Progress made");
            }
            other => panic!("Expected UpdateActual, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_with_yaml_markers() {
        let text = r#"Here's my analysis:
---
mutations:
  - action: create_child
    parent_id: "PARENT1"
    desired: "Sub-task complete"
    actual: "Not started"
    reasoning: "Break it down"
response: "Created a sub-task for tracking."
---
"#;
        let parsed = StructuredResponse::from_response(text).unwrap();
        assert_eq!(parsed.mutations.len(), 1);
        match &parsed.mutations[0] {
            Mutation::CreateChild {
                parent_id, desired, ..
            } => {
                assert_eq!(parent_id, "PARENT1");
                assert_eq!(desired, "Sub-task complete");
            }
            other => panic!("Expected CreateChild, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_multiple_mutations() {
        let yaml = r#"
---
mutations:
  - action: update_actual
    tension_id: "T1"
    new_value: "Delegated"
    reasoning: "Handed off"
  - action: create_child
    parent_id: "T1"
    desired: "Quality check"
    actual: "Pending"
    reasoning: "Track separately"
  - action: add_note
    tension_id: "T1"
    text: "Email sent Tuesday"
response: "All set."
---
"#;
        let parsed = StructuredResponse::from_response(yaml).unwrap();
        assert_eq!(parsed.mutations.len(), 3);
        assert_eq!(parsed.response, "All set.");
    }

    #[test]
    fn test_parse_empty_mutations() {
        let yaml = r#"
---
mutations: []
response: "Nothing to change."
---
"#;
        let parsed = StructuredResponse::from_response(yaml).unwrap();
        assert!(parsed.mutations.is_empty());
    }

    #[test]
    fn test_parse_invalid_yaml_returns_none() {
        let text = "This is just plain text, not YAML at all.";
        let parsed = StructuredResponse::from_response(text);
        assert!(parsed.is_none());
    }

    #[test]
    fn test_mutation_summary() {
        let m = Mutation::UpdateActual {
            tension_id: "T1".to_string(),
            new_value: "Done".to_string(),
            reasoning: "Completed".to_string(),
        };
        assert!(m.summary().contains("Update actual"));
        assert!(m.summary().contains("Done"));
    }

    #[test]
    fn test_mutation_reasoning() {
        let m = Mutation::UpdateActual {
            tension_id: "T1".to_string(),
            new_value: "Done".to_string(),
            reasoning: "Progress made".to_string(),
        };
        assert_eq!(m.reasoning(), Some("Progress made"));

        let m2 = Mutation::AddNote {
            tension_id: "T1".to_string(),
            text: "A note".to_string(),
        };
        assert_eq!(m2.reasoning(), None);
    }

    #[test]
    fn test_mutation_empty_reasoning() {
        let m = Mutation::UpdateActual {
            tension_id: "T1".to_string(),
            new_value: "Done".to_string(),
            reasoning: String::new(),
        };
        assert_eq!(m.reasoning(), None);
    }

    #[test]
    fn test_parse_update_status() {
        let yaml = r#"
---
mutations:
  - action: update_status
    tension_id: "T1"
    new_status: "Resolved"
    reasoning: "Task completed"
response: "Done."
---
"#;
        let parsed = StructuredResponse::from_response(yaml).unwrap();
        match &parsed.mutations[0] {
            Mutation::UpdateStatus {
                new_status,
                reasoning,
                ..
            } => {
                assert_eq!(new_status, "Resolved");
                assert_eq!(reasoning, "Task completed");
            }
            other => panic!("Expected UpdateStatus, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_update_desired() {
        let yaml = r#"
---
mutations:
  - action: update_desired
    tension_id: "T1"
    new_value: "Better goal"
    reasoning: "Refined"
response: "Refined the goal."
---
"#;
        let parsed = StructuredResponse::from_response(yaml).unwrap();
        match &parsed.mutations[0] {
            Mutation::UpdateDesired {
                new_value,
                reasoning,
                ..
            } => {
                assert_eq!(new_value, "Better goal");
                assert_eq!(reasoning, "Refined");
            }
            other => panic!("Expected UpdateDesired, got {:?}", other),
        }
    }

    #[test]
    fn test_extract_yaml_block_between_markers() {
        let text = "Preamble\n---\nmutations: []\nresponse: hi\n---\nPostamble";
        let yaml = extract_yaml_block(text).unwrap();
        assert!(yaml.starts_with("mutations"));
    }

    #[test]
    fn test_extract_yaml_block_single_marker() {
        let text = "Preamble\n---\nmutations: []\nresponse: hi";
        let yaml = extract_yaml_block(text).unwrap();
        assert!(yaml.starts_with("mutations"));
    }

    #[test]
    fn test_extract_yaml_block_no_markers() {
        let text = "Just plain text";
        assert!(extract_yaml_block(text).is_none());
    }
}

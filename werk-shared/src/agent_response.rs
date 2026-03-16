//! Structured agent response parsing and mutation application.
//!
//! Agents in one-shot mode can return structured YAML with mutation suggestions
//! that the user can review and apply in bulk.
//!
//! This module lives in werk-shared so both werk-cli and werk-tui can reuse it.

use serde::{Deserialize, Serialize};
use crate::util::truncate;

/// A structured response from an agent, containing prose advice and
/// optional mutation suggestions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredResponse {
    /// Suggested mutations to the tension forest.
    #[serde(default)]
    pub mutations: Vec<AgentMutation>,
    /// Human-readable advice/response text.
    #[serde(default)]
    pub response: String,
}

/// A single suggested mutation to the tension forest.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum AgentMutation {
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
    /// Set or update a tension's horizon.
    SetHorizon {
        tension_id: String,
        horizon: String,
        #[serde(default)]
        reasoning: String,
    },
    /// Move a tension to a new parent (reparent).
    MoveTension {
        tension_id: String,
        #[serde(default)]
        new_parent_id: Option<String>,
        #[serde(default)]
        reasoning: String,
    },
    /// Create a parent tension and reparent this tension under it.
    CreateParent {
        child_id: String,
        desired: String,
        actual: String,
        #[serde(default)]
        reasoning: String,
    },
}

impl AgentMutation {
    /// Return a human-readable summary of this mutation.
    pub fn summary(&self) -> String {
        match self {
            AgentMutation::UpdateActual { new_value, .. } => {
                format!("Update actual: \"{}\"", truncate(new_value, 60))
            }
            AgentMutation::CreateChild { desired, .. } => {
                format!("Create child: \"{}\"", truncate(desired, 60))
            }
            AgentMutation::AddNote { text, .. } => {
                format!("Add note: \"{}\"", truncate(text, 60))
            }
            AgentMutation::UpdateStatus { new_status, .. } => {
                format!("Set status: {}", new_status)
            }
            AgentMutation::UpdateDesired { new_value, .. } => {
                format!("Update desired: \"{}\"", truncate(new_value, 60))
            }
            AgentMutation::SetHorizon { horizon, .. } => {
                format!("Set horizon: {}", horizon)
            }
            AgentMutation::MoveTension { new_parent_id, .. } => {
                match new_parent_id {
                    Some(pid) => format!("Move to parent: {}", &pid[..12.min(pid.len())]),
                    None => "Move to root".to_string(),
                }
            }
            AgentMutation::CreateParent { desired, .. } => {
                format!("Create parent: \"{}\"", truncate(desired, 60))
            }
        }
    }

    /// Return the reasoning for this mutation, if any.
    pub fn reasoning(&self) -> Option<&str> {
        match self {
            AgentMutation::UpdateActual { reasoning, .. }
            | AgentMutation::CreateChild { reasoning, .. }
            | AgentMutation::UpdateStatus { reasoning, .. }
            | AgentMutation::UpdateDesired { reasoning, .. }
            | AgentMutation::SetHorizon { reasoning, .. }
            | AgentMutation::MoveTension { reasoning, .. }
            | AgentMutation::CreateParent { reasoning, .. } => {
                if reasoning.is_empty() {
                    None
                } else {
                    Some(reasoning)
                }
            }
            AgentMutation::AddNote { .. } => None,
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

        // Must contain mutations key to be considered structured
        if !yaml_text.contains("mutations") {
            return None;
        }

        // Try parsing as StructuredResponse (response field defaults to empty string)
        if let Ok(parsed) = serde_yaml::from_str::<Self>(yaml_text) {
            return Some(parsed);
        }

        // Fallback: try parsing just the mutations array
        #[derive(serde::Deserialize)]
        struct MutationsOnly {
            #[serde(default)]
            mutations: Vec<AgentMutation>,
        }
        if let Ok(parsed) = serde_yaml::from_str::<MutationsOnly>(yaml_text) {
            if !parsed.mutations.is_empty() {
                return Some(Self {
                    mutations: parsed.mutations,
                    response: String::new(),
                });
            }
        }

        None
    }
}

/// Extract YAML content between `---` markers.
///
/// Searches for the LAST `---` delimited block that contains `mutations:`
/// to avoid being tripped up by markdown horizontal rules (`---`) in the
/// agent's prose response.
fn extract_yaml_block(text: &str) -> Option<&str> {
    let trimmed = text.trim();

    // Collect all --- delimited blocks, then find the one with mutations:
    let mut blocks: Vec<&str> = Vec::new();
    let mut search_from = 0;

    while let Some(start) = trimmed[search_from..].find("---") {
        let abs_start = search_from + start + 3;
        if abs_start >= trimmed.len() {
            break;
        }
        let after_start = &trimmed[abs_start..];
        if let Some(end) = after_start.find("---") {
            let block = after_start[..end].trim();
            if !block.is_empty() {
                blocks.push(block);
            }
            search_from = abs_start + end + 3;
        } else {
            // No closing --- — treat rest as potential block
            let block = after_start.trim();
            if !block.is_empty() {
                blocks.push(block);
            }
            break;
        }
    }

    // Prefer the block that contains both 'mutations' and 'response' keys
    for block in blocks.iter().rev() {
        if block.contains("mutations") && block.contains("response") {
            return Some(block);
        }
    }

    // Fallback: any block with 'mutations'
    for block in blocks.iter().rev() {
        if block.contains("mutations") {
            return Some(block);
        }
    }

    // Last resort: first non-empty block
    blocks.into_iter().next()
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
            AgentMutation::UpdateActual {
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
        let m = AgentMutation::UpdateActual {
            tension_id: "T1".to_string(),
            new_value: "Done".to_string(),
            reasoning: "Completed".to_string(),
        };
        assert!(m.summary().contains("Update actual"));
        assert!(m.summary().contains("Done"));
    }

    #[test]
    fn test_mutation_reasoning() {
        let m = AgentMutation::UpdateActual {
            tension_id: "T1".to_string(),
            new_value: "Done".to_string(),
            reasoning: "Progress made".to_string(),
        };
        assert_eq!(m.reasoning(), Some("Progress made"));

        let m2 = AgentMutation::AddNote {
            tension_id: "T1".to_string(),
            text: "A note".to_string(),
        };
        assert_eq!(m2.reasoning(), None);
    }
}

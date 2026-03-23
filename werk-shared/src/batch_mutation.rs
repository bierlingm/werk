//! Batch mutation types for YAML-driven bulk operations.
//!
//! Used by `werk batch apply` to parse and apply mutations from YAML files.

use serde::{Deserialize, Serialize};
use crate::util::truncate;

/// A single mutation to the tension forest, parsed from YAML.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum BatchMutation {
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

impl BatchMutation {
    /// Return a human-readable summary of this mutation.
    pub fn summary(&self) -> String {
        match self {
            BatchMutation::UpdateActual { new_value, .. } => {
                format!("Update actual: \"{}\"", truncate(new_value, 60))
            }
            BatchMutation::CreateChild { desired, .. } => {
                format!("Create child: \"{}\"", truncate(desired, 60))
            }
            BatchMutation::AddNote { text, .. } => {
                format!("Add note: \"{}\"", truncate(text, 60))
            }
            BatchMutation::UpdateStatus { new_status, .. } => {
                format!("Set status: {}", new_status)
            }
            BatchMutation::UpdateDesired { new_value, .. } => {
                format!("Update desired: \"{}\"", truncate(new_value, 60))
            }
            BatchMutation::SetHorizon { horizon, .. } => {
                format!("Set horizon: {}", horizon)
            }
            BatchMutation::MoveTension { new_parent_id, .. } => {
                match new_parent_id {
                    Some(pid) => format!("Move to parent: {}", &pid[..12.min(pid.len())]),
                    None => "Move to root".to_string(),
                }
            }
            BatchMutation::CreateParent { desired, .. } => {
                format!("Create parent: \"{}\"", truncate(desired, 60))
            }
        }
    }

    /// Return the reasoning for this mutation, if any.
    pub fn reasoning(&self) -> Option<&str> {
        match self {
            BatchMutation::UpdateActual { reasoning, .. }
            | BatchMutation::CreateChild { reasoning, .. }
            | BatchMutation::UpdateStatus { reasoning, .. }
            | BatchMutation::UpdateDesired { reasoning, .. }
            | BatchMutation::SetHorizon { reasoning, .. }
            | BatchMutation::MoveTension { reasoning, .. }
            | BatchMutation::CreateParent { reasoning, .. } => {
                if reasoning.is_empty() {
                    None
                } else {
                    Some(reasoning)
                }
            }
            BatchMutation::AddNote { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_yaml_list() {
        let yaml = r#"
- action: update_actual
  tension_id: "ABC123"
  new_value: "Research complete"
  reasoning: "Progress made"
- action: create_child
  parent_id: "PARENT1"
  desired: "Sub-task complete"
  actual: "Not started"
  reasoning: "Break it down"
"#;
        let mutations: Vec<BatchMutation> = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(mutations.len(), 2);

        match &mutations[0] {
            BatchMutation::UpdateActual { tension_id, new_value, reasoning } => {
                assert_eq!(tension_id, "ABC123");
                assert_eq!(new_value, "Research complete");
                assert_eq!(reasoning, "Progress made");
            }
            other => panic!("Expected UpdateActual, got {:?}", other),
        }
    }

    #[test]
    fn test_mutation_summary() {
        let m = BatchMutation::UpdateActual {
            tension_id: "T1".to_string(),
            new_value: "Done".to_string(),
            reasoning: "Completed".to_string(),
        };
        assert!(m.summary().contains("Update actual"));
        assert!(m.summary().contains("Done"));
    }

    #[test]
    fn test_mutation_reasoning() {
        let m = BatchMutation::UpdateActual {
            tension_id: "T1".to_string(),
            new_value: "Done".to_string(),
            reasoning: "Progress made".to_string(),
        };
        assert_eq!(m.reasoning(), Some("Progress made"));

        let m2 = BatchMutation::AddNote {
            tension_id: "T1".to_string(),
            text: "A note".to_string(),
        };
        assert_eq!(m2.reasoning(), None);
    }

    #[test]
    fn test_mutation_empty_reasoning() {
        let m = BatchMutation::UpdateActual {
            tension_id: "T1".to_string(),
            new_value: "Done".to_string(),
            reasoning: String::new(),
        };
        assert_eq!(m.reasoning(), None);
    }
}

//! Structured agent response parsing and mutation application.
//!
//! This module re-exports types from werk-shared for backward compatibility.
//! The actual implementation now lives in werk-shared::agent_response.

pub use werk_shared::agent_response::{AgentMutation as Mutation, StructuredResponse};

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
    fn test_extract_yaml_block_no_markers() {
        let text = "Just plain text";
        assert!(StructuredResponse::from_response(text).is_none());
    }
}

//! Import tensions from tensions.json into a fresh SD database.
//! Usage: cargo run --example import_json

use std::collections::HashMap;
use werk_core::{Horizon, Store, TensionStatus};

fn main() {
    let json_str = std::fs::read_to_string("tensions.json").expect("read tensions.json");
    let data: serde_json::Value = serde_json::from_str(&json_str).expect("parse JSON");

    // Init store at workspace root (creates .werk/werk.db)
    let store = Store::init(std::path::Path::new(".")).expect("create store");

    let tensions = data["tensions"].as_array().expect("tensions array");

    // First pass: create all tensions as roots (we'll set parents after)
    let mut old_to_new: HashMap<String, String> = HashMap::new();

    // Sort by short_code to ensure parents are created before children
    let mut sorted: Vec<&serde_json::Value> = tensions.iter().collect();
    sorted.sort_by_key(|t| t["short_code"].as_i64().unwrap_or(0));

    for t in &sorted {
        let desired = t["desired"].as_str().unwrap();
        let actual = t["actual"].as_str().unwrap();
        let old_id = t["id"].as_str().unwrap().to_string();
        let parent_id = t["parent_id"].as_str();

        let mapped_parent = parent_id.and_then(|pid| old_to_new.get(pid));

        let new_tension = if let Some(new_pid) = mapped_parent {
            store
                .create_tension_with_parent(desired, actual, Some(new_pid.clone()))
                .expect("create child")
        } else {
            store
                .create_tension(desired, actual)
                .expect("create tension")
        };

        let new_id = new_tension.id.clone();

        // Set horizon
        if let Some(h_str) = t["horizon"].as_str()
            && let Ok(h) = Horizon::parse(h_str)
        {
            let _ = store.update_horizon(&new_id, Some(h));
        }

        // Set position
        if let Some(pos) = t["position"].as_i64() {
            let _ = store.update_position(&new_id, Some(pos as i32));
        }

        // Set status
        match t["status"].as_str().unwrap() {
            "Resolved" => {
                let _ = store.update_status(&new_id, TensionStatus::Resolved);
            }
            "Released" => {
                let _ = store.update_status(&new_id, TensionStatus::Released);
            }
            _ => {}
        }

        let sc = t["short_code"].as_i64().unwrap_or(-1);
        println!(
            "#{} -> #{} ({})",
            sc,
            new_tension.short_code.unwrap_or(-1),
            &new_id[..8]
        );

        old_to_new.insert(old_id, new_id);
    }

    println!("\nDone: {} tensions imported", old_to_new.len());
}

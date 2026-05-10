use chrono::{TimeZone, Utc};
use werk_core::horizon::Horizon;
use werk_core::store::Store;
use werk_core::tension::TensionStatus;

pub struct Fixture {
    pub store: Store,
    pub root_id: String,
}

pub fn small_tree() -> Fixture {
    let store = Store::new_in_memory().unwrap();
    let root = store.create_tension("root", "root actual").unwrap();
    let horizon = Horizon::new_month(2026, 4).unwrap();
    store.update_horizon(&root.id, Some(horizon)).unwrap();
    let c1 = store
        .create_tension_with_parent("child 1", "child 1 actual", Some(root.id.clone()))
        .unwrap();
    let c2 = store
        .create_tension_with_parent("child 2", "child 2 actual", Some(root.id.clone()))
        .unwrap();
    let _g1 = store
        .create_tension_with_parent("grand 1", "grand 1 actual", Some(c1.id.clone()))
        .unwrap();
    let g2 = store
        .create_tension_with_parent("grand 2", "grand 2 actual", Some(c1.id.clone()))
        .unwrap();
    let _g3 = store
        .create_tension_with_parent("grand 3", "grand 3 actual", Some(c2.id.clone()))
        .unwrap();
    let _leaf = store
        .create_tension_with_parent("leaf", "leaf actual", Some(g2.id.clone()))
        .unwrap();
    store
        .update_status(&c2.id, TensionStatus::Resolved)
        .unwrap();
    store
        .update_status(&g2.id, TensionStatus::Released)
        .unwrap();
    Fixture {
        store,
        root_id: root.id,
    }
}

pub fn fixed_now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 5, 8, 10, 0, 0).unwrap()
}

use chrono::{DateTime, Utc};
use serde_json::Value;

use crate::Store;
use crate::ir::{Ir, IrError, IrKind};

#[derive(Debug, Clone, PartialEq)]
pub struct EpochSnapshot {
    pub desire: String,
    pub reality: String,
    pub children: Option<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EpochPoint {
    pub timestamp: DateTime<Utc>,
    pub snapshot: EpochSnapshot,
}

#[derive(Debug, Clone)]
pub struct EpochSeries {
    pub tension_id: String,
    pub points: Vec<EpochPoint>,
}

impl Ir for EpochSeries {
    fn kind(&self) -> IrKind {
        IrKind::EpochSeries
    }
}

#[derive(Debug, Clone)]
pub enum EpochSeriesScope {
    Tension { id: String },
    Subtree { root: String, depth: usize },
}

impl EpochSeries {
    pub fn for_tension(store: &Store, tension_id: &str) -> Result<Self, IrError> {
        let epochs = store.get_epochs(tension_id)?;
        let mut points = Vec::with_capacity(epochs.len());

        for epoch in epochs {
            let children = match epoch.children_snapshot_json.as_deref() {
                Some(json) => {
                    Some(serde_json::from_str(json).map_err(IrError::invalid_epoch_snapshot)?)
                }
                None => None,
            };

            points.push(EpochPoint {
                timestamp: epoch.timestamp,
                snapshot: EpochSnapshot {
                    desire: epoch.desire_snapshot,
                    reality: epoch.reality_snapshot,
                    children,
                },
            });
        }

        Ok(Self {
            tension_id: tension_id.to_string(),
            points,
        })
    }

    pub fn for_scope(store: &Store, scope: EpochSeriesScope) -> Result<Self, IrError> {
        match scope {
            EpochSeriesScope::Tension { id } => Self::for_tension(store, &id),
            EpochSeriesScope::Subtree { .. } => {
                Err(IrError::unsupported_epoch_series_scope("subtree"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{EpochSeries, EpochSeriesScope};
    use crate::Store;

    #[test]
    fn matches_get_epochs_chronologically() {
        let store = Store::new_in_memory().unwrap();
        let parent = store
            .create_tension("build the thing", "nothing built yet")
            .unwrap();
        let child = store
            .create_tension_with_parent("step one", "not started", Some(parent.id.clone()))
            .unwrap();

        let children_json = serde_json::json!({"children": [
            {"id": child.id, "desired": "step one", "status": "Active"}
        ]})
        .to_string();

        let _e1 = store
            .create_epoch(
                &parent.id,
                "build the thing",
                "nothing built yet",
                Some(&children_json),
                None,
            )
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let _e2 = store
            .create_epoch(
                &parent.id,
                "build the thing v2",
                "still nothing",
                None,
                None,
            )
            .unwrap();

        let epochs = store.get_epochs(&parent.id).unwrap();
        let series = EpochSeries::for_tension(&store, &parent.id).unwrap();

        assert_eq!(series.points.len(), epochs.len());
        for (point, record) in series.points.iter().zip(epochs.iter()) {
            assert_eq!(point.timestamp, record.timestamp);
            assert_eq!(point.snapshot.desire, record.desire_snapshot);
            assert_eq!(point.snapshot.reality, record.reality_snapshot);
            let expected_children = record
                .children_snapshot_json
                .as_ref()
                .map(|json| serde_json::from_str::<serde_json::Value>(json).unwrap());
            assert_eq!(point.snapshot.children, expected_children);
        }
    }

    #[test]
    fn rejects_non_tension_scope() {
        let store = Store::new_in_memory().unwrap();
        let scope = EpochSeriesScope::Subtree {
            root: "root".to_string(),
            depth: 1,
        };
        let err = EpochSeries::for_scope(&store, scope).unwrap_err();
        assert!(err.to_string().contains("unsupported"));
    }
}

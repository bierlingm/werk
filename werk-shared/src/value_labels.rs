//! Value labels — named bands for continuous metrics. Unlike the registry's
//! level tables (which map labels to *configuration thresholds*), these map
//! *observed values* to descriptive terms. Used in `show`, `list --long`,
//! `log --compare` to give numeric readings a narrative anchor.
//!
//! Bands are chosen to be intuitive, not statistical — the cutoffs are
//! deliberately round and reader-friendly.

/// Urgency on [0.0, 1.0]. 0 = fresh, 1 = deadline now. Banded into four
/// readings: calm / stirring / pressing / critical.
pub fn urgency_label(u: f64) -> &'static str {
    if u < 0.25 {
        "calm"
    } else if u < 0.5 {
        "stirring"
    } else if u < 0.75 {
        "pressing"
    } else {
        "critical"
    }
}

/// Drift on [0.0, 1.0]. 0 = tight coupling of desire and reality,
/// 1 = totally orthogonal. Banded into four: tight / slight / noticeable / wide.
pub fn drift_label(d: f64) -> &'static str {
    if d < 0.1 {
        "tight"
    } else if d < 0.3 {
        "slight"
    } else if d < 0.6 {
        "noticeable"
    } else {
        "wide"
    }
}

/// Staleness in days since last mutation. 0 days = moving, 7+ slowing,
/// 14+ stale, 30+ dormant.
pub fn staleness_label(days: i64) -> &'static str {
    if days < 3 {
        "moving"
    } else if days < 14 {
        "slowing"
    } else if days < 30 {
        "stale"
    } else {
        "dormant"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urgency_bands() {
        assert_eq!(urgency_label(0.0), "calm");
        assert_eq!(urgency_label(0.24), "calm");
        assert_eq!(urgency_label(0.25), "stirring");
        assert_eq!(urgency_label(0.49), "stirring");
        assert_eq!(urgency_label(0.5), "pressing");
        assert_eq!(urgency_label(0.74), "pressing");
        assert_eq!(urgency_label(0.75), "critical");
        assert_eq!(urgency_label(1.0), "critical");
    }

    #[test]
    fn drift_bands() {
        assert_eq!(drift_label(0.0), "tight");
        assert_eq!(drift_label(0.1), "slight");
        assert_eq!(drift_label(0.3), "noticeable");
        assert_eq!(drift_label(0.6), "wide");
        assert_eq!(drift_label(1.0), "wide");
    }

    #[test]
    fn staleness_bands() {
        assert_eq!(staleness_label(0), "moving");
        assert_eq!(staleness_label(3), "slowing");
        assert_eq!(staleness_label(14), "stale");
        assert_eq!(staleness_label(30), "dormant");
        assert_eq!(staleness_label(365), "dormant");
    }
}

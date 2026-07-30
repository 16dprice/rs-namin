use crate::scene::value::AnimValue;

use super::track::TrackTarget;

/// Locks a property of one object (or the camera) to a property of another,
/// re-evaluated every frame after keyframe tracks are applied. The optional
/// offset is added component-wise (Float/Vec2/Vec3/Vec4 only).
///
/// A binding may be limited to a time window (`start`/`end`, either side
/// open): inside the window it overwrites the target property (winning over
/// any keyframe track); outside it does nothing, so a track on the same
/// property takes over. An unwindowed binding owns its property outright —
/// combining one with a track is a build error. Outside every window, an
/// untracked property simply keeps its last-evaluated value, which is
/// scrub-order dependent — add a track (or initial override) if determinism
/// matters there.
#[derive(Debug, Clone)]
pub struct Binding {
    pub target: TrackTarget,
    pub target_property: String,
    pub source: TrackTarget,
    pub source_property: String,
    pub offset: Option<AnimValue>,
    /// Window start (inclusive); `None` = from the beginning.
    pub start: Option<f32>,
    /// Window end (exclusive); `None` = forever.
    pub end: Option<f32>,
}

impl Binding {
    /// Whether the binding drives its target at `time`.
    pub fn active_at(&self, time: f32) -> bool {
        self.start.is_none_or(|s| time >= s) && self.end.is_none_or(|e| time < e)
    }

    /// Whether this binding's window overlaps another's (unbounded sides
    /// extend to infinity). Used to reject ambiguous same-property bindings.
    pub fn window_overlaps(&self, other: &Binding) -> bool {
        let starts_before_other_ends = match (self.start, other.end) {
            (Some(s), Some(e)) => s < e,
            _ => true,
        };
        let other_starts_before_self_ends = match (other.start, self.end) {
            (Some(s), Some(e)) => s < e,
            _ => true,
        };
        starts_before_other_ends && other_starts_before_self_ends
    }

    pub fn is_windowed(&self) -> bool {
        self.start.is_some() || self.end.is_some()
    }

    /// Sort bindings so every binding runs after any binding that writes its
    /// source. Granularity is the whole target (object or camera), not
    /// (target, property): setters may have side effects on sibling
    /// properties (e.g. Turtle's `progress` updates `position`), so all
    /// writes to the source must land before it is read.
    ///
    /// On a cycle, returns the targets of the bindings that could not be
    /// ordered.
    pub fn sort(mut bindings: Vec<Binding>) -> Result<Vec<Binding>, Vec<TrackTarget>> {
        let mut sorted = Vec::with_capacity(bindings.len());
        while !bindings.is_empty() {
            let ready: Vec<usize> = (0..bindings.len())
                .filter(|&i| !bindings.iter().any(|b| b.target == bindings[i].source))
                .collect();
            if ready.is_empty() {
                let mut targets: Vec<TrackTarget> = Vec::new();
                for b in &bindings {
                    if !targets.contains(&b.target) {
                        targets.push(b.target);
                    }
                }
                return Err(targets);
            }
            // Descending order keeps swap_remove indices valid.
            for &i in ready.iter().rev() {
                sorted.push(bindings.swap_remove(i));
            }
        }
        Ok(sorted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::ObjectId;

    fn bind(target: ObjectId, source: ObjectId) -> Binding {
        Binding {
            target: TrackTarget::Object(target),
            target_property: "position".to_string(),
            source: TrackTarget::Object(source),
            source_property: "position".to_string(),
            offset: None,
            start: None,
            end: None,
        }
    }

    fn windowed(start: Option<f32>, end: Option<f32>) -> Binding {
        let mut b = bind(id(0), id(1));
        b.start = start;
        b.end = end;
        b
    }

    #[test]
    fn active_at_respects_window() {
        let b = windowed(Some(2.0), Some(10.0));
        assert!(!b.active_at(1.9));
        assert!(b.active_at(2.0));
        assert!(b.active_at(9.9));
        assert!(!b.active_at(10.0));
        assert!(windowed(None, None).active_at(1e9));
        assert!(windowed(None, Some(5.0)).active_at(0.0));
        assert!(!windowed(Some(5.0), None).active_at(4.9));
    }

    #[test]
    fn window_overlap_detection() {
        let a = windowed(None, Some(10.0));
        assert!(a.window_overlaps(&windowed(Some(5.0), None)));
        assert!(!a.window_overlaps(&windowed(Some(10.0), None)));
        assert!(a.window_overlaps(&windowed(None, None)));
        assert!(!windowed(Some(0.0), Some(2.0)).window_overlaps(&windowed(Some(2.0), Some(4.0))));
        assert!(windowed(Some(0.0), Some(3.0)).window_overlaps(&windowed(Some(2.0), Some(4.0))));
    }

    fn id(n: usize) -> ObjectId {
        ObjectId::test_id(n)
    }

    #[test]
    fn sort_orders_chain() {
        // c ← b ← a, added backwards.
        let bindings = vec![bind(id(2), id(1)), bind(id(1), id(0))];
        let sorted = Binding::sort(bindings).unwrap();
        assert_eq!(sorted[0].target, TrackTarget::Object(id(1)));
        assert_eq!(sorted[1].target, TrackTarget::Object(id(2)));
    }

    #[test]
    fn sort_detects_cycle() {
        let bindings = vec![bind(id(0), id(1)), bind(id(1), id(0))];
        let cycle = Binding::sort(bindings).unwrap_err();
        assert_eq!(cycle.len(), 2);
    }

    #[test]
    fn sort_self_binding_is_a_cycle() {
        let bindings = vec![bind(id(0), id(0))];
        assert!(Binding::sort(bindings).is_err());
    }

    #[test]
    fn sort_independent_bindings_pass_through() {
        let bindings = vec![bind(id(1), id(0)), bind(id(3), id(2))];
        assert_eq!(Binding::sort(bindings).unwrap().len(), 2);
    }

    #[test]
    fn sort_orders_writes_before_reads_at_object_granularity() {
        // Two bindings write different properties of object 1; a third reads
        // object 1. Both writers must come first.
        let mut writer_a = bind(id(1), id(0));
        writer_a.target_property = "progress".to_string();
        let writer_b = bind(id(1), id(0));
        let reader = bind(id(2), id(1));
        let sorted = Binding::sort(vec![reader, writer_a, writer_b]).unwrap();
        assert_eq!(sorted[2].target, TrackTarget::Object(id(2)));
    }
}

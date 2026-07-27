use crate::scene::value::AnimValue;

use super::track::TrackTarget;

/// Locks a property of one object (or the camera) to a property of another,
/// re-evaluated every frame after keyframe tracks are applied. The optional
/// offset is added component-wise (Float/Vec2/Vec3/Vec4 only).
///
/// Bindings replace keyframes for the target property — a property is driven
/// by a track or a binding, never both (validated at build time).
#[derive(Debug, Clone)]
pub struct Binding {
    pub target: TrackTarget,
    pub target_property: String,
    pub source: TrackTarget,
    pub source_property: String,
    pub offset: Option<AnimValue>,
}

impl Binding {
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
        }
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

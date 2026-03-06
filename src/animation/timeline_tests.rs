#[cfg(test)]
mod tests {
    use macroquad::prelude::*;

    use crate::animation::timeline::Timeline;
    use crate::animation::track::{Keyframe, Track};
    use crate::scene::objects::Circle;
    use crate::scene::value::AnimValue;
    use crate::scene::Scene;

    #[test]
    fn apply_sets_property_at_time() {
        let mut scene = Scene::new();
        let id = scene.add(Circle::new(vec3(0.0, 0.0, 0.0), 10.0, WHITE));

        let mut track = Track::new(id, "radius");
        track.add_keyframe(Keyframe::new(0.0, AnimValue::Float(10.0)));
        track.add_keyframe(Keyframe::new(2.0, AnimValue::Float(50.0)));

        let mut timeline = Timeline::new();
        timeline.add_track(track);

        timeline.apply(1.0, &mut scene);
        let obj = scene.get(id).unwrap();
        assert_eq!(obj.get("radius"), Some(AnimValue::Float(30.0)));
    }

    #[test]
    fn apply_multiple_tracks_same_object() {
        let mut scene = Scene::new();
        let id = scene.add(Circle::new(vec3(0.0, 0.0, 0.0), 10.0, WHITE));

        let mut radius_track = Track::new(id, "radius");
        radius_track.add_keyframe(Keyframe::new(0.0, AnimValue::Float(10.0)));
        radius_track.add_keyframe(Keyframe::new(1.0, AnimValue::Float(50.0)));

        let mut pos_track = Track::new(id, "position");
        pos_track.add_keyframe(Keyframe::new(0.0, AnimValue::Vec3(vec3(0.0, 0.0, 0.0))));
        pos_track.add_keyframe(Keyframe::new(1.0, AnimValue::Vec3(vec3(100.0, 0.0, 0.0))));

        let mut timeline = Timeline::new();
        timeline.add_track(radius_track);
        timeline.add_track(pos_track);

        timeline.apply(0.5, &mut scene);
        let obj = scene.get(id).unwrap();
        assert_eq!(obj.get("radius"), Some(AnimValue::Float(30.0)));
        assert_eq!(
            obj.get("position"),
            Some(AnimValue::Vec3(vec3(50.0, 0.0, 0.0)))
        );
    }

    #[test]
    fn duration_returns_max_across_tracks() {
        let mut t1 = Track::new(
            crate::scene::ObjectId::test_id(0),
            "a",
        );
        t1.add_keyframe(Keyframe::new(0.0, AnimValue::Float(0.0)));
        t1.add_keyframe(Keyframe::new(3.0, AnimValue::Float(0.0)));

        let mut t2 = Track::new(
            crate::scene::ObjectId::test_id(0),
            "b",
        );
        t2.add_keyframe(Keyframe::new(0.0, AnimValue::Float(0.0)));
        t2.add_keyframe(Keyframe::new(5.0, AnimValue::Float(0.0)));

        let mut timeline = Timeline::new();
        timeline.add_track(t1);
        timeline.add_track(t2);

        assert!((timeline.duration() - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn empty_timeline_duration_is_zero() {
        let timeline = Timeline::new();
        assert_eq!(timeline.duration(), 0.0);
    }
}

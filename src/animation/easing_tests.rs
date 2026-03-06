#[cfg(test)]
mod tests {
    use crate::animation::easing::*;

    const EASINGS: &[(&str, EasingFn)] = &[
        ("linear", linear),
        ("quad_in", quad_in),
        ("quad_out", quad_out),
        ("quad_in_out", quad_in_out),
        ("cubic_in", cubic_in),
        ("cubic_out", cubic_out),
        ("cubic_in_out", cubic_in_out),
    ];

    #[test]
    fn all_easings_return_zero_at_zero() {
        for (name, easing) in EASINGS {
            assert!(
                (easing(0.0)).abs() < f32::EPSILON,
                "{name} failed at t=0.0"
            );
        }
    }

    #[test]
    fn all_easings_return_one_at_one() {
        for (name, easing) in EASINGS {
            assert!(
                (easing(1.0) - 1.0).abs() < f32::EPSILON,
                "{name} failed at t=1.0"
            );
        }
    }

    #[test]
    fn linear_is_identity() {
        assert_eq!(linear(0.25), 0.25);
        assert_eq!(linear(0.5), 0.5);
        assert_eq!(linear(0.75), 0.75);
    }

    #[test]
    fn quad_in_is_slower_than_linear_at_midpoint() {
        assert!(quad_in(0.5) < 0.5);
    }

    #[test]
    fn quad_out_is_faster_than_linear_at_midpoint() {
        assert!(quad_out(0.5) > 0.5);
    }

    #[test]
    fn in_out_passes_through_midpoint() {
        assert!((quad_in_out(0.5) - 0.5).abs() < f32::EPSILON);
        assert!((cubic_in_out(0.5) - 0.5).abs() < f32::EPSILON);
    }
}

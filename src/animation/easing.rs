/// TECH DEBT: This is a bare function pointer, which prevents parameterized easings
/// (e.g. a dolly_zoom factory that takes fov_start/fov_end). Changing to
/// `Box<dyn Fn(f32) -> f32>` or `Arc<dyn Fn(f32) -> f32>` would enable closures
/// that capture state, at the cost of a heap allocation per keyframe.
pub type EasingFn = fn(f32) -> f32;

pub fn linear(t: f32) -> f32 {
    t
}

pub fn quad_in(t: f32) -> f32 {
    t * t
}

pub fn quad_out(t: f32) -> f32 {
    1.0 - (1.0 - t) * (1.0 - t)
}

pub fn quad_in_out(t: f32) -> f32 {
    if t < 0.5 {
        2.0 * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
    }
}

pub fn cubic_in(t: f32) -> f32 {
    t * t * t
}

pub fn cubic_out(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

pub fn cubic_in_out(t: f32) -> f32 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

pub fn quart_in(t: f32) -> f32 {
    t * t * t * t
}

pub fn quart_out(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(4)
}

pub fn quart_in_out(t: f32) -> f32 {
    if t < 0.5 {
        8.0 * t * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(4) / 2.0
    }
}

pub fn quint_in(t: f32) -> f32 {
    t * t * t * t * t
}

pub fn quint_out(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(5)
}

pub fn quint_in_out(t: f32) -> f32 {
    if t < 0.5 {
        16.0 * t * t * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(5) / 2.0
    }
}

pub fn sine_in(t: f32) -> f32 {
    1.0 - (t * std::f32::consts::FRAC_PI_2).cos()
}

pub fn sine_out(t: f32) -> f32 {
    (t * std::f32::consts::FRAC_PI_2).sin()
}

pub fn sine_in_out(t: f32) -> f32 {
    -(std::f32::consts::PI * t).cos() / 2.0 + 0.5
}

pub fn expo_in(t: f32) -> f32 {
    if t == 0.0 { 0.0 } else { (2.0_f32).powf(10.0 * t - 10.0) }
}

pub fn expo_out(t: f32) -> f32 {
    if t == 1.0 { 1.0 } else { 1.0 - (2.0_f32).powf(-10.0 * t) }
}

pub fn expo_in_out(t: f32) -> f32 {
    if t == 0.0 {
        0.0
    } else if t == 1.0 {
        1.0
    } else if t < 0.5 {
        (2.0_f32).powf(20.0 * t - 10.0) / 2.0
    } else {
        (2.0 - (2.0_f32).powf(-20.0 * t + 10.0)) / 2.0
    }
}

pub fn back_in(t: f32) -> f32 {
    const C1: f32 = 1.70158;
    const C3: f32 = C1 + 1.0;
    C3 * t * t * t - C1 * t * t
}

pub fn back_out(t: f32) -> f32 {
    const C1: f32 = 1.70158;
    const C3: f32 = C1 + 1.0;
    1.0 + C3 * (t - 1.0).powi(3) + C1 * (t - 1.0).powi(2)
}

pub fn back_in_out(t: f32) -> f32 {
    const C1: f32 = 1.70158;
    const C2: f32 = C1 * 1.525;
    if t < 0.5 {
        ((2.0 * t).powi(2) * ((C2 + 1.0) * 2.0 * t - C2)) / 2.0
    } else {
        ((2.0 * t - 2.0).powi(2) * ((C2 + 1.0) * (2.0 * t - 2.0) + C2) + 2.0) / 2.0
    }
}

pub fn elastic_in(t: f32) -> f32 {
    if t == 0.0 {
        0.0
    } else if t == 1.0 {
        1.0
    } else {
        let c4 = std::f32::consts::TAU / 3.0;
        -(2.0_f32).powf(10.0 * t - 10.0) * ((10.0 * t - 10.75) * c4).sin()
    }
}

pub fn elastic_out(t: f32) -> f32 {
    if t == 0.0 {
        0.0
    } else if t == 1.0 {
        1.0
    } else {
        let c4 = std::f32::consts::TAU / 3.0;
        (2.0_f32).powf(-10.0 * t) * ((10.0 * t - 0.75) * c4).sin() + 1.0
    }
}

pub fn elastic_in_out(t: f32) -> f32 {
    if t == 0.0 {
        0.0
    } else if t == 1.0 {
        1.0
    } else {
        let c5 = std::f32::consts::TAU / 4.5;
        if t < 0.5 {
            -(2.0_f32).powf(20.0 * t - 10.0) * ((20.0 * t - 11.125) * c5).sin() / 2.0
        } else {
            (2.0_f32).powf(-20.0 * t + 10.0) * ((20.0 * t - 11.125) * c5).sin() / 2.0 + 1.0
        }
    }
}

pub fn bounce_out(t: f32) -> f32 {
    const N1: f32 = 7.5625;
    const D1: f32 = 2.75;
    if t < 1.0 / D1 {
        N1 * t * t
    } else if t < 2.0 / D1 {
        let t = t - 1.5 / D1;
        N1 * t * t + 0.75
    } else if t < 2.5 / D1 {
        let t = t - 2.25 / D1;
        N1 * t * t + 0.9375
    } else {
        let t = t - 2.625 / D1;
        N1 * t * t + 0.984375
    }
}

pub fn bounce_in(t: f32) -> f32 {
    1.0 - bounce_out(1.0 - t)
}

pub fn bounce_in_out(t: f32) -> f32 {
    if t < 0.5 {
        (1.0 - bounce_out(1.0 - 2.0 * t)) / 2.0
    } else {
        (1.0 + bounce_out(2.0 * t - 1.0)) / 2.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EASINGS: &[(&str, EasingFn)] = &[
        ("linear", linear),
        ("quad_in", quad_in),
        ("quad_out", quad_out),
        ("quad_in_out", quad_in_out),
        ("cubic_in", cubic_in),
        ("cubic_out", cubic_out),
        ("cubic_in_out", cubic_in_out),
        ("quart_in", quart_in),
        ("quart_out", quart_out),
        ("quart_in_out", quart_in_out),
        ("quint_in", quint_in),
        ("quint_out", quint_out),
        ("quint_in_out", quint_in_out),
        ("sine_in", sine_in),
        ("sine_out", sine_out),
        ("sine_in_out", sine_in_out),
        ("expo_in", expo_in),
        ("expo_out", expo_out),
        ("expo_in_out", expo_in_out),
        ("back_in", back_in),
        ("back_out", back_out),
        ("back_in_out", back_in_out),
        ("elastic_in", elastic_in),
        ("elastic_out", elastic_out),
        ("elastic_in_out", elastic_in_out),
        ("bounce_in", bounce_in),
        ("bounce_out", bounce_out),
        ("bounce_in_out", bounce_in_out),
    ];

    #[test]
    fn all_easings_return_zero_at_zero() {
        for (name, easing) in EASINGS {
            assert!((easing(0.0)).abs() < f32::EPSILON, "{name} failed at t=0.0");
        }
    }

    #[test]
    fn all_easings_return_one_at_one() {
        for (name, easing) in EASINGS {
            assert!((easing(1.0) - 1.0).abs() < f32::EPSILON, "{name} failed at t=1.0");
        }
    }

    #[test]
    fn linear_is_identity() {
        assert_eq!(linear(0.25), 0.25);
        assert_eq!(linear(0.5), 0.5);
        assert_eq!(linear(0.75), 0.75);
    }

    #[test]
    fn ease_in_is_slower_than_linear_at_midpoint() {
        assert!(quad_in(0.5) < 0.5);
        assert!(cubic_in(0.5) < 0.5);
        assert!(quart_in(0.5) < 0.5);
        assert!(quint_in(0.5) < 0.5);
        assert!(sine_in(0.5) < 0.5);
        assert!(expo_in(0.5) < 0.5);
    }

    #[test]
    fn ease_out_is_faster_than_linear_at_midpoint() {
        assert!(quad_out(0.5) > 0.5);
        assert!(cubic_out(0.5) > 0.5);
        assert!(quart_out(0.5) > 0.5);
        assert!(quint_out(0.5) > 0.5);
        assert!(sine_out(0.5) > 0.5);
        assert!(expo_out(0.5) > 0.5);
    }

    #[test]
    fn in_out_passes_through_midpoint() {
        assert!((quad_in_out(0.5) - 0.5).abs() < f32::EPSILON);
        assert!((cubic_in_out(0.5) - 0.5).abs() < f32::EPSILON);
        assert!((quart_in_out(0.5) - 0.5).abs() < f32::EPSILON);
        assert!((quint_in_out(0.5) - 0.5).abs() < f32::EPSILON);
        assert!((sine_in_out(0.5) - 0.5).abs() < 1e-6);
        assert!((expo_in_out(0.5) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn back_overshoots() {
        // back_in goes negative near start
        assert!(back_in(0.1) < 0.0);
        // back_out goes above 1.0 near end
        assert!(back_out(0.9) > 1.0);
    }

    #[test]
    fn bounce_out_monotonically_reaches_one() {
        assert!((bounce_out(1.0) - 1.0).abs() < f32::EPSILON);
        // First bounce peak
        assert!(bounce_out(0.5) > 0.0);
    }

    #[test]
    fn elastic_oscillates() {
        // elastic_in goes negative in early range
        let has_negative = (1..10).any(|i| elastic_in(i as f32 * 0.1) < 0.0);
        assert!(has_negative, "elastic_in should go negative");
    }
}

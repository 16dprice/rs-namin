use macroquad::prelude::Vec2;

use crate::scene::polyline::LineSegment;

#[derive(Debug, Clone)]
pub struct ReplacementRule {
    pub from: char,
    pub to: String,
}

#[derive(Debug, Clone)]
pub struct LSystemConfig {
    pub axiom: String,
    pub rules: Vec<ReplacementRule>,
}

/// Iteratively apply replacement rules to the axiom string.
pub fn apply_rules(config: &LSystemConfig, iterations: usize) -> String {
    let mut current = config.axiom.clone();
    for _ in 0..iterations {
        let mut next = String::with_capacity(current.len() * 2);
        for ch in current.chars() {
            if let Some(rule) = config.rules.iter().find(|r| r.from == ch) {
                next.push_str(&rule.to);
            } else {
                next.push(ch);
            }
        }
        current = next;
    }
    current
}

/// Interpret an L-system string as turtle graphics commands, producing line segments.
///
/// Commands:
/// - `F`, `G`: move forward, drawing a line
/// - `+`: turn left (CCW) by theta
/// - `-`: turn right (CW) by theta
/// - `[`: push position and angle onto stack
/// - `]`: pop position and angle from stack
/// - Anything else (X, A, B, etc.): ignored (rewriting-only variables)
///
/// Initial direction is up (π/2), y-up coordinate system.
pub fn get_lines(l_string: &str, theta: f32, step_distance: f32) -> Vec<LineSegment> {
    let mut segments = Vec::new();
    let mut x: f32 = 0.0;
    let mut y: f32 = 0.0;
    let mut angle: f32 = std::f32::consts::FRAC_PI_2; // facing up
    let mut stack: Vec<(f32, f32, f32)> = Vec::new();

    for ch in l_string.chars() {
        match ch {
            'F' | 'G' => {
                let nx = x + step_distance * angle.cos();
                let ny = y + step_distance * angle.sin();
                segments.push(LineSegment {
                    start: Vec2::new(x, y),
                    end: Vec2::new(nx, ny),
                });
                x = nx;
                y = ny;
            }
            '+' => angle += theta,
            '-' => angle -= theta,
            '[' => stack.push((x, y, angle)),
            ']' => {
                let (sx, sy, sa) = stack.pop().expect("unmatched ']' in L-system string");
                x = sx;
                y = sy;
                angle = sa;
            }
            _ => {} // ignore other characters
        }
    }
    segments
}

// --- Presets ---

pub fn dragon_curve() -> (LSystemConfig, f32) {
    (
        LSystemConfig {
            axiom: "F".into(),
            rules: vec![
                ReplacementRule {
                    from: 'F',
                    to: "F+G".into(),
                },
                ReplacementRule {
                    from: 'G',
                    to: "F-G".into(),
                },
            ],
        },
        std::f32::consts::FRAC_PI_2,
    )
}

pub fn sierpinski() -> (LSystemConfig, f32) {
    (
        LSystemConfig {
            axiom: "F".into(),
            rules: vec![
                ReplacementRule {
                    from: 'F',
                    to: "G-F-G".into(),
                },
                ReplacementRule {
                    from: 'G',
                    to: "F+G+F".into(),
                },
            ],
        },
        std::f32::consts::FRAC_PI_3,
    )
}

pub fn koch() -> (LSystemConfig, f32) {
    (
        LSystemConfig {
            axiom: "F".into(),
            rules: vec![ReplacementRule {
                from: 'F',
                to: "F-F+F+F-F".into(),
            }],
        },
        std::f32::consts::FRAC_PI_2,
    )
}

pub fn fractal_plant() -> (LSystemConfig, f32) {
    (
        LSystemConfig {
            axiom: "-X".into(),
            rules: vec![
                ReplacementRule {
                    from: 'X',
                    to: "F+[[X]-X]-F[-FX]+X".into(),
                },
                ReplacementRule {
                    from: 'F',
                    to: "FF".into(),
                },
            ],
        },
        std::f32::consts::PI / 7.0,
    )
}

pub fn crystal() -> (LSystemConfig, f32) {
    (
        LSystemConfig {
            axiom: "X".into(),
            rules: vec![
                ReplacementRule {
                    from: 'X',
                    to: "F[+X][-X]FX".into(),
                },
                ReplacementRule {
                    from: 'F',
                    to: "FF".into(),
                },
            ],
        },
        std::f32::consts::PI / 7.0,
    )
}

pub fn binary_tree() -> (LSystemConfig, f32) {
    (
        LSystemConfig {
            axiom: "F".into(),
            rules: vec![
                ReplacementRule {
                    from: 'F',
                    to: "G[-F]+F".into(),
                },
                ReplacementRule {
                    from: 'G',
                    to: "GG".into(),
                },
            ],
        },
        std::f32::consts::FRAC_PI_4,
    )
}

pub fn hilbert() -> (LSystemConfig, f32) {
    (
        LSystemConfig {
            axiom: "A".into(),
            rules: vec![
                ReplacementRule {
                    from: 'A',
                    to: "+BF-AFA-FB+".into(),
                },
                ReplacementRule {
                    from: 'B',
                    to: "-AF+BFB+FA-".into(),
                },
            ],
        },
        std::f32::consts::FRAC_PI_2,
    )
}

pub fn tree() -> (LSystemConfig, f32) {
    (
        LSystemConfig {
            axiom: "F".into(),
            rules: vec![ReplacementRule {
                from: 'F',
                to: "F[+F]F[-F]F".into(),
            }],
        },
        std::f32::consts::PI / 7.0,
    )
}

pub fn my1() -> (LSystemConfig, f32) {
    (
        LSystemConfig {
            axiom: "F".into(),
            rules: vec![ReplacementRule {
                from: 'F',
                to: "F[+F[-F[+F[-F]]]]".into(),
            }],
        },
        std::f32::consts::FRAC_PI_4,
    )
}

pub fn my2() -> (LSystemConfig, f32) {
    (
        LSystemConfig {
            axiom: "F".into(),
            rules: vec![ReplacementRule {
                from: 'F',
                to: "F[+F-F][-F+F]F".into(),
            }],
        },
        std::f32::consts::FRAC_PI_4,
    )
}

pub fn my3() -> (LSystemConfig, f32) {
    (
        LSystemConfig {
            axiom: "F".into(),
            rules: vec![ReplacementRule {
                from: 'F',
                to: "F[-F-F][+F+F]F".into(),
            }],
        },
        std::f32::consts::FRAC_PI_4,
    )
}

pub fn my4() -> (LSystemConfig, f32) {
    (
        LSystemConfig {
            axiom: "F".into(),
            rules: vec![ReplacementRule {
                from: 'F',
                to: "F-F+F".into(),
            }],
        },
        2.09,
    )
}

pub fn my5() -> (LSystemConfig, f32) {
    (
        LSystemConfig {
            axiom: "F".into(),
            rules: vec![ReplacementRule {
                from: 'F',
                to: "F[F-F+F+F-F]F[F+F-F-F+F]F".into(),
            }],
        },
        std::f32::consts::FRAC_PI_4,
    )
}

pub fn my6() -> (LSystemConfig, f32) {
    (
        LSystemConfig {
            axiom: "F".into(),
            rules: vec![ReplacementRule {
                from: 'F',
                to: "F[-F+F+F-]F".into(),
            }],
        },
        std::f32::consts::FRAC_PI_4,
    )
}

pub fn my7() -> (LSystemConfig, f32) {
    (
        LSystemConfig {
            axiom: "F--F--F".into(),
            rules: vec![ReplacementRule {
                from: 'F',
                to: "F+F--F+F".into(),
            }],
        },
        std::f32::consts::FRAC_PI_3,
    )
}

pub fn my8() -> (LSystemConfig, f32) {
    (
        LSystemConfig {
            axiom: "F".into(),
            rules: vec![ReplacementRule {
                from: 'F',
                to: "FF[+++F][---F]".into(),
            }],
        },
        std::f32::consts::FRAC_PI_4,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dragon_curve_iteration_1() {
        let (config, _) = dragon_curve();
        assert_eq!(apply_rules(&config, 1), "F+G");
    }

    #[test]
    fn dragon_curve_iteration_2() {
        let (config, _) = dragon_curve();
        assert_eq!(apply_rules(&config, 2), "F+G+F-G");
    }

    #[test]
    fn dragon_curve_iteration_3() {
        let (config, _) = dragon_curve();
        assert_eq!(apply_rules(&config, 3), "F+G+F-G+F+G-F-G");
    }

    #[test]
    fn dragon_curve_line_count_iter3() {
        let (config, theta) = dragon_curve();
        let s = apply_rules(&config, 3);
        let lines = get_lines(&s, theta, 1.0);
        // F+G+F-G+F+G-F-G → draw chars: F,G,F,G,F,G,F,G = 8
        assert_eq!(lines.len(), 8);
    }

    #[test]
    fn empty_axiom_returns_empty() {
        let config = LSystemConfig {
            axiom: String::new(),
            rules: vec![],
        };
        assert_eq!(apply_rules(&config, 5), "");
        let lines = get_lines("", std::f32::consts::FRAC_PI_2, 1.0);
        assert!(lines.is_empty());
    }

    #[test]
    fn zero_iterations_returns_axiom() {
        let (config, _) = dragon_curve();
        assert_eq!(apply_rules(&config, 0), "F");
    }

    #[test]
    fn fractal_plant_uses_stack() {
        let (config, theta) = fractal_plant();
        let s = apply_rules(&config, 2);
        let lines = get_lines(&s, theta, 1.0);
        assert!(!lines.is_empty());
    }

    #[test]
    #[should_panic(expected = "unmatched ']'")]
    fn unmatched_bracket_panics() {
        get_lines("]", std::f32::consts::FRAC_PI_2, 1.0);
    }

    #[test]
    fn hilbert_curve_lines() {
        let (config, theta) = hilbert();
        let s = apply_rules(&config, 2);
        let lines = get_lines(&s, theta, 1.0);
        assert!(!lines.is_empty());
    }

    #[test]
    fn g_character_draws() {
        let lines = get_lines("G", std::f32::consts::FRAC_PI_2, 1.0);
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn dragon_curve_segments_connected() {
        let (config, theta) = dragon_curve();
        let s = apply_rules(&config, 4);
        let lines = get_lines(&s, theta, 1.0);
        // Each segment's end should be the next segment's start
        for i in 0..lines.len() - 1 {
            let gap = (lines[i].end - lines[i + 1].start).length();
            assert!(
                gap < 1e-4,
                "gap between segments {} and {}: {}",
                i,
                i + 1,
                gap
            );
        }
    }
}

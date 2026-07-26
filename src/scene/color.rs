use macroquad::prelude::*;

/// Sample a color from a gradient for item `index` of `total`, interpolating
/// linearly between entries in `colors`. If `colors` has fewer than 2 entries,
/// `fallback` is used. Returns RGBA bytes suitable for vertex color attributes.
pub fn gradient_sample(colors: &[Vec4], fallback: Vec4, index: usize, total: usize) -> [u8; 4] {
    let c = if colors.len() >= 2 {
        let t = if total <= 1 { 0.0 } else { index as f32 / (total - 1) as f32 };
        let span = (colors.len() - 1) as f32;
        let pos = t * span;
        let lo = (pos.floor() as usize).min(colors.len() - 2);
        let frac = pos - lo as f32;
        colors[lo].lerp(colors[lo + 1], frac)
    } else {
        fallback
    };
    Color::new(c.x, c.y, c.z, c.w).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(c: Color) -> Vec4 {
        vec4(c.r, c.g, c.b, c.a)
    }

    #[test]
    fn gradient_interpolates_between_entries() {
        let colors = vec![v(RED), v(BLUE)];
        let total = 10;

        let first = gradient_sample(&colors, v(WHITE), 0, total);
        assert!(first[0] > first[2], "first item should have more red than blue");

        let last = gradient_sample(&colors, v(WHITE), total - 1, total);
        assert!(last[2] > last[0], "last item should have more blue than red");

        let mid = gradient_sample(&colors, v(WHITE), total / 2, total);
        assert!(mid[0] > 0 && mid[2] > 0, "mid item should have both red and blue");
    }

    #[test]
    fn empty_gradient_uses_fallback() {
        let colors: Vec<Vec4> = Vec::new();
        let fallback = v(GREEN);
        let c0 = gradient_sample(&colors, fallback, 0, 10);
        let c1 = gradient_sample(&colors, fallback, 9, 10);
        assert_eq!(c0, c1);
        assert_eq!(
            c0,
            <Color as Into<[u8; 4]>>::into(Color::new(fallback.x, fallback.y, fallback.z, fallback.w))
        );
    }

    #[test]
    fn single_color_uses_fallback() {
        let colors = vec![v(RED)];
        let fallback = v(GREEN);
        let c = gradient_sample(&colors, fallback, 0, 10);
        assert_eq!(
            c,
            <Color as Into<[u8; 4]>>::into(Color::new(fallback.x, fallback.y, fallback.z, fallback.w))
        );
    }

    #[test]
    fn total_one_returns_first_entry() {
        let colors = vec![v(RED), v(BLUE)];
        let c = gradient_sample(&colors, v(WHITE), 0, 1);
        // t=0 → first entry (red)
        assert!(c[0] > c[2]);
    }
}

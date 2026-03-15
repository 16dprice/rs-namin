/// Convert RGBA pixel data to RGB with Y-flip (OpenGL render targets are upside-down).
pub fn rgba_to_rgb_flipped(rgba: &[[u8; 4]], width: usize, height: usize, out: &mut Vec<u8>) {
    out.clear();
    for y in (0..height).rev() {
        for x in 0..width {
            let pixel = rgba[y * width + x];
            out.push(pixel[0]);
            out.push(pixel[1]);
            out.push(pixel[2]);
        }
    }
}

/// Convert RGBA pixel data to RGBA with Y-flip (OpenGL render targets are upside-down).
pub fn rgba_flipped(rgba: &[[u8; 4]], width: usize, height: usize, out: &mut Vec<u8>) {
    out.clear();
    for y in (0..height).rev() {
        for x in 0..width {
            let pixel = rgba[y * width + x];
            out.push(pixel[0]);
            out.push(pixel[1]);
            out.push(pixel[2]);
            out.push(pixel[3]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb_flipped_strips_alpha_and_flips() {
        let rgba: Vec<[u8; 4]> = vec![
            [255, 0, 0, 255],
            [0, 255, 0, 255],
            [0, 0, 255, 255],
            [255, 255, 255, 128],
        ];
        let mut out = Vec::new();
        rgba_to_rgb_flipped(&rgba, 2, 2, &mut out);
        assert_eq!(out, vec![0, 0, 255, 255, 255, 255, 255, 0, 0, 0, 255, 0,]);
    }

    #[test]
    fn rgba_flipped_preserves_alpha_and_flips() {
        let rgba: Vec<[u8; 4]> = vec![
            [255, 0, 0, 255],
            [0, 255, 0, 128],
            [0, 0, 255, 64],
            [255, 255, 255, 0],
        ];
        let mut out = Vec::new();
        rgba_flipped(&rgba, 2, 2, &mut out);
        assert_eq!(
            out,
            vec![
                0, 0, 255, 64, 255, 255, 255, 0, 255, 0, 0, 255, 0, 255, 0, 128,
            ]
        );
    }

    #[test]
    fn single_pixel() {
        let rgba: Vec<[u8; 4]> = vec![[42, 99, 200, 128]];
        let mut out = Vec::new();
        rgba_to_rgb_flipped(&rgba, 1, 1, &mut out);
        assert_eq!(out, vec![42, 99, 200]);
    }

    #[test]
    fn reuses_buffer() {
        let rgba: Vec<[u8; 4]> = vec![[1, 2, 3, 4]];
        let mut out = vec![99, 99, 99, 99, 99];
        rgba_to_rgb_flipped(&rgba, 1, 1, &mut out);
        assert_eq!(out, vec![1, 2, 3]);
    }
}

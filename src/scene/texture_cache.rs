//! Lazy path → `Texture2D` cache for scene-document sprites.
//!
//! Texture creation needs the GL context and can fail (missing file, bad
//! image data), but `ObjectSpec::spawn()` must stay GL-free and infallible —
//! so doc sprites store only a path and resolve it HERE, at draw time (draw
//! always runs inside the window). Failures cache a magenta/black
//! placeholder instead of erroring; since results are cached by path string,
//! editing the path in the inspector is what retries a fixed file.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use macroquad::prelude::*;

fn cache() -> &'static Mutex<HashMap<String, Texture2D>> {
    static CACHE: OnceLock<Mutex<HashMap<String, Texture2D>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Get the texture at `path` (relative to the working directory), loading
/// and caching it on first use. Never fails: unreadable or undecodable
/// files yield a cached placeholder.
pub fn get(path: &str) -> Texture2D {
    if let Some(texture) = cache().lock().unwrap().get(path) {
        return texture.clone();
    }
    let texture = load(path).unwrap_or_else(placeholder);
    texture.set_filter(FilterMode::Nearest);
    cache().lock().unwrap().insert(path.to_string(), texture.clone());
    texture
}

fn load(path: &str) -> Option<Texture2D> {
    let bytes = std::fs::read(path).ok()?;
    let image = Image::from_file_with_format(&bytes, None).ok()?;
    Some(Texture2D::from_image(&image))
}

/// 2x2 magenta/black checker — the classic "texture missing" marker.
fn placeholder() -> Texture2D {
    const M: [u8; 4] = [255, 0, 255, 255];
    const B: [u8; 4] = [0, 0, 0, 255];
    let pixels: Vec<u8> = [M, B, B, M].concat();
    Texture2D::from_rgba8(2, 2, &pixels)
}

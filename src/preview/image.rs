//! Native image preview (PRD V1 §4.7): PNG/JPG/GIF/WebP + SVG.
//!
//! Images are never treated as text. Large images are downscaled on load
//! so the texture cache stays bounded.

use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;
use std::sync::mpsc::{channel, Receiver, Sender};

pub const MAX_PREVIEW_DIM: u32 = 2048;

/// Shrink-to-fit ("contain") size for an image of `image` points inside
/// `available` points: large images scale down to fit, small ones keep
/// their native size (never upscaled), aspect ratio always preserved.
/// Degenerate inputs yield a zero size (renders nothing, never NaN).
pub fn fit_contain(image: egui::Vec2, available: egui::Vec2) -> egui::Vec2 {
    if image.x <= 0.0 || image.y <= 0.0 || available.x <= 0.0 || available.y <= 0.0 {
        return egui::Vec2::ZERO;
    }
    let ratio = (available.x / image.x).min(available.y / image.y).min(1.0);
    if !ratio.is_finite() {
        return egui::Vec2::ZERO;
    }
    image * ratio
}

pub fn is_image(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .as_deref(),
        Some("png" | "jpg" | "jpeg" | "gif" | "webp" | "svg")
    )
}

/// True when an image of `w×h` exceeds the preview cap and must be
/// downscaled on load. Pure decision logic, unit-tested.
pub fn needs_downscale(w: u32, h: u32) -> bool {
    w > MAX_PREVIEW_DIM || h > MAX_PREVIEW_DIM
}

/// Take an RGBA buffer out of a decoded image without copying when it is
/// already RGBA8 (the common PNG case); convert otherwise.
fn into_rgba8_fast(img: image::DynamicImage) -> image::RgbaImage {
    match img {
        image::DynamicImage::ImageRgba8(buf) => buf,
        other => other.to_rgba8(),
    }
}

fn load_raster(path: &Path) -> Result<egui::ColorImage> {
    // Header-only read first: skip the resample pass when the image
    // already fits the preview cap.
    let (w0, h0) =
        image::image_dimensions(path).with_context(|| format!("probe {}", path.display()))?;
    let img = image::open(path).with_context(|| format!("decode {}", path.display()))?;
    let img = if needs_downscale(w0, h0) {
        img.thumbnail(MAX_PREVIEW_DIM, MAX_PREVIEW_DIM)
    } else {
        img
    };
    let buf = into_rgba8_fast(img);
    let (w, h) = (buf.width() as usize, buf.height() as usize);
    Ok(egui::ColorImage::from_rgba_unmultiplied(
        [w, h],
        &buf.into_raw(),
    ))
}

fn load_svg(path: &Path, scale: f32) -> Result<egui::ColorImage> {
    let data = std::fs::read(path)?;
    let tree = resvg::usvg::Tree::from_data(&data, &resvg::usvg::Options::default())
        .map_err(|e| anyhow::anyhow!("svg parse: {e}"))?;
    let size = tree.size();
    let (w, h) = (
        ((size.width() * scale) as u32).clamp(1, MAX_PREVIEW_DIM),
        ((size.height() * scale) as u32).clamp(1, MAX_PREVIEW_DIM),
    );
    let mut pixmap = resvg::tiny_skia::Pixmap::new(w, h).context("pixmap alloc")?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(w as f32 / size.width(), h as f32 / size.height()),
        &mut pixmap.as_mut(),
    );
    Ok(egui::ColorImage::from_rgba_unmultiplied(
        [w as usize, h as usize],
        pixmap.data(),
    ))
}

/// Decode on any thread: picks raster vs SVG by extension.
fn decode_image(path: &Path) -> Result<egui::ColorImage> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if ext == "svg" {
        load_svg(path, 2.0)
    } else {
        load_raster(path)
    }
}

pub struct ImageCache {
    textures: HashMap<String, egui::TextureHandle>,
    failed: HashMap<String, String>,
    pending: HashSet<String>,
    order: VecDeque<String>,
    tx: Sender<(String, Result<egui::ColorImage, String>)>,
    rx: Receiver<(String, Result<egui::ColorImage, String>)>,
}

impl Default for ImageCache {
    fn default() -> Self {
        let (tx, rx) = channel();
        Self {
            textures: HashMap::new(),
            failed: HashMap::new(),
            pending: HashSet::new(),
            order: VecDeque::new(),
            tx,
            rx,
        }
    }
}

/// At most this many decoded previews stay resident; older ones are
/// forgotten (GPU texture freed) on insert.
pub const MAX_CACHED_IMAGES: usize = 8;

impl ImageCache {
    /// Non-blocking lookup: `Ok(Some)` ready, `Ok(None)` still decoding
    /// (caller shows a spinner), `Err` decoded-and-failed.
    pub fn texture_for(
        &mut self,
        ctx: &egui::Context,
        path: &Path,
    ) -> Result<Option<egui::TextureHandle>> {
        self.pump(ctx);
        let key = path.display().to_string();
        if let Some(t) = self.textures.get(&key) {
            return Ok(Some(t.clone()));
        }
        if let Some(e) = self.failed.get(&key) {
            anyhow::bail!("{e}");
        }
        if !self.pending.contains(&key) {
            self.pending.insert(key.clone());
            let tx = self.tx.clone();
            let ctx = ctx.clone();
            let path = path.to_path_buf();
            // shortcut: one short-lived thread per image; upgrade to a
            // shared pool if gallery-style browsing ever spawns many.
            std::thread::spawn(move || {
                let out = decode_image(&path).map_err(|e| format!("{e:#}"));
                let _ = tx.send((key, out));
                ctx.request_repaint();
            });
        }
        Ok(None)
    }

    /// Move finished background decodes onto GPU textures (UI thread only).
    fn pump(&mut self, ctx: &egui::Context) {
        while let Ok((key, out)) = self.rx.try_recv() {
            self.pending.remove(&key);
            match out {
                Ok(img) => {
                    let tex = ctx.load_texture(key.clone(), img, egui::TextureOptions::LINEAR);
                    self.textures.insert(key.clone(), tex);
                    self.order.push_back(key);
                    // shortcut: plain FIFO at 8 entries, not a true LRU;
                    // upgrade if multi-image workflows ever thrash.
                    while self.order.len() > MAX_CACHED_IMAGES {
                        if let Some(old) = self.order.pop_front() {
                            if self.textures.remove(&old).is_some() {
                                ctx.forget_image(&old);
                            }
                        } else {
                            break;
                        }
                    }
                }
                Err(e) => {
                    self.failed.insert(key, e);
                }
            }
        }
    }

    pub fn invalidate(&mut self, path: &Path) {
        let key = path.display().to_string();
        if self.textures.remove(&key).is_some() {
            self.order.retain(|k| k != &key);
        }
        self.failed.remove(&key);
        self.pending.remove(&key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: egui::Vec2, b: egui::Vec2) -> bool {
        (a - b).length() < 0.01
    }

    #[test]
    fn large_landscape_shrinks_to_fit_width() {
        let out = fit_contain(egui::vec2(2000.0, 1000.0), egui::vec2(800.0, 600.0));
        assert!(approx(out, egui::vec2(800.0, 400.0)));
    }

    #[test]
    fn large_portrait_shrinks_to_fit_height() {
        let out = fit_contain(egui::vec2(1000.0, 2000.0), egui::vec2(800.0, 600.0));
        assert!(approx(out, egui::vec2(300.0, 600.0)));
    }

    #[test]
    fn small_image_keeps_native_size() {
        let out = fit_contain(egui::vec2(64.0, 32.0), egui::vec2(800.0, 600.0));
        assert!(approx(out, egui::vec2(64.0, 32.0)));
    }

    #[test]
    fn exact_fit_is_unchanged() {
        let out = fit_contain(egui::vec2(800.0, 600.0), egui::vec2(800.0, 600.0));
        assert!(approx(out, egui::vec2(800.0, 600.0)));
    }
    #[test]
    fn degenerate_inputs_yield_zero() {
        assert_eq!(
            fit_contain(egui::Vec2::ZERO, egui::vec2(800.0, 600.0)),
            egui::Vec2::ZERO
        );
        assert_eq!(
            fit_contain(egui::vec2(100.0, 100.0), egui::Vec2::ZERO),
            egui::Vec2::ZERO
        );
    }

    #[test]
    fn downscale_only_past_the_cap() {
        assert!(!needs_downscale(1920, 1080));
        assert!(!needs_downscale(MAX_PREVIEW_DIM, MAX_PREVIEW_DIM));
        assert!(needs_downscale(MAX_PREVIEW_DIM + 1, 100));
        assert!(needs_downscale(100, MAX_PREVIEW_DIM + 1));
    }

    #[test]
    fn rgba_fast_path_avoids_copy() {
        let buf = image::RgbaImage::from_pixel(4, 4, image::Rgba([1, 2, 3, 4]));
        let out = into_rgba8_fast(image::DynamicImage::ImageRgba8(buf));
        assert_eq!(out.get_pixel(0, 0), &image::Rgba([1, 2, 3, 4]));
    }

    #[test]
    fn background_decode_path_reads_generated_png() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("t.png");
        image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
            4,
            3,
            image::Rgba([1, 2, 3, 255]),
        ))
        .save(&p)
        .unwrap();
        let out = decode_image(&p).unwrap();
        assert_eq!(out.size, [4, 3]);
    }
}

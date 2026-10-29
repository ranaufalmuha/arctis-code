//! App icon rasterized once from the bundled SVG.
use std::sync::{Arc, OnceLock};

pub fn icon() -> Option<Arc<egui::IconData>> {
    static ICON: OnceLock<Option<Arc<egui::IconData>>> = OnceLock::new();
    ICON.get_or_init(|| {
        let tree = resvg::usvg::Tree::from_data(include_bytes!("../public/icons/logo.svg"), &resvg::usvg::Options::default()).ok()?;
        let size = tree.size();
        let mut pixmap = resvg::tiny_skia::Pixmap::new(256, 256)?;
        resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(256.0 / size.width(), 256.0 / size.height()), &mut pixmap.as_mut());
        Some(Arc::new(egui::IconData { rgba: pixmap.data().to_vec(), width: 256, height: 256 }))
    }).clone()
}

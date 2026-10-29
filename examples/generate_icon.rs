fn main() -> anyhow::Result<()> {
    let output = std::env::args_os().nth(1).ok_or_else(|| anyhow::anyhow!("Expected output PNG path"))?;
    let tree = resvg::usvg::Tree::from_data(include_bytes!("../public/icons/logo.svg"), &resvg::usvg::Options::default()).map_err(|e| anyhow::anyhow!("{e}"))?;
    let size = tree.size();
    let mut image = resvg::tiny_skia::Pixmap::new(1024, 1024).ok_or_else(|| anyhow::anyhow!("Cannot allocate image"))?;
    resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(1024.0 / size.width(), 1024.0 / size.height()), &mut image.as_mut());
    image.save_png(output)?;
    Ok(())
}

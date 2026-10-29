use arctis_code::app::ArctisCodeApp;

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let initial_path = args.get(1).map(|s| s.to_owned());

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([800.0, 500.0])
            .with_title("ArctisCode")
            // Transparent window, sharp (no OS blur: vibrancy blanks GL
            // compositing on this stack). Only sidebar/status paints are
            // translucent; every other surface stays opaque.
            .with_transparent(true)
            .with_drag_and_drop(true)
            // macOS: content draws behind the transparent titlebar so our
            // custom topbar (search + tabs) sits in the same visual row
            // as the native traffic lights. Buttons stay visible; only
            // the title text + bar fill are hidden.
            .with_fullsize_content_view(true)
            .with_titlebar_shown(false)
            .with_title_shown(false),
        ..Default::default()
    };

    eframe::run_native(
        "ArctisCode",
        options,
        Box::new(move |cc| {
            Ok(Box::new(ArctisCodeApp::new(cc, initial_path)))
        }),
    )
}

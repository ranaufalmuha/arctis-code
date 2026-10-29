//! Design-token consistency: `public/styles/tokens.json` is the source of
//! truth, `VscodeTheme::default()` is the runtime mapping. This test fails
//! loudly if they drift apart.

use arctis_code::theme::VscodeTheme;
use egui::Color32;

fn hex(hex: &str) -> Color32 {
    let h = hex.strip_prefix('#').unwrap();
    let v = u32::from_str_radix(h, 16).unwrap();
    Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

#[test]
fn tokens_match_runtime_theme() {
    let text = std::fs::read_to_string("public/styles/tokens.json").unwrap();
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    let c = &v["colors"];
    let t = VscodeTheme::default();
    assert_eq!(t.editor_bg, hex(c["editorBg"].as_str().unwrap()));
    assert_eq!(t.editor_fg, hex(c["editorFg"].as_str().unwrap()));
    assert_eq!(t.card_bg, hex(c["cardBg"].as_str().unwrap()));
    assert_eq!(t.text, hex(c["text"].as_str().unwrap()));
    assert_eq!(t.muted, hex(c["muted"].as_str().unwrap()));
    assert_eq!(t.accent, hex(c["accent"].as_str().unwrap()));
    assert_eq!(t.selection, hex(c["selection"].as_str().unwrap()));
    assert_eq!(t.danger, hex(c["danger"].as_str().unwrap()));
    let m = &v["metrics"];
    assert_eq!(t.mono_size, v["font"]["monoSize"].as_f64().unwrap() as f32);
    assert_eq!(
        m["activityBarWidth"].as_u64().unwrap() as f32,
        arctis_code::theme::ACTIVITY_BAR_WIDTH
    );
    assert_eq!(
        m["tabStripHeight"].as_u64().unwrap() as f32,
        arctis_code::theme::TAB_STRIP_HEIGHT
    );
    assert_eq!(
        m["statusBarHeight"].as_u64().unwrap() as f32,
        arctis_code::theme::STATUS_BAR_HEIGHT
    );
    assert_eq!(
        m["treeRowHeight"].as_u64().unwrap() as f32,
        arctis_code::theme::TREE_ROW_HEIGHT
    );
}

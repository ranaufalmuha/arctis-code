fn main() {
    println!("cargo:rerun-if-changed=packaging/macos/dock.m");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") { return; }
    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let object = out.join("dock.o");
    let archive = out.join("libarctis_dock.a");
    assert!(std::process::Command::new("clang").args(["-c", "packaging/macos/dock.m", "-o"]).arg(&object).status().expect("clang").success());
    assert!(std::process::Command::new("ar").arg("crs").arg(&archive).arg(&object).status().expect("ar").success());
    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=static=arctis_dock");
    println!("cargo:rustc-link-lib=framework=AppKit");
}

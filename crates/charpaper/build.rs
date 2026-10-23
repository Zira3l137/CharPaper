use std::env;

fn main() {
    println!("cargo:rerun-if-changed=icon");

    // A build script runs on the machine doing the build, so `cfg!(windows)` would describe
    // that machine. The target is only known from Cargo's environment.
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    // Other systems don't read icons from the binary: Linux takes them from a .desktop file,
    // macOS from the app bundle. Those belong to packaging, not here.
    if target_os == "windows" {
        embed_windows_icon();
    }
}

fn embed_windows_icon() {
    // Absolute, so the .ico is found whichever directory the resource compiler runs in.
    let icon_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/icon");
    let params = embed_resource::ParamsIncludeDirs(&[icon_dir]);
    embed_resource::compile("icon/charpaper.rc", params).manifest_optional().unwrap();
}

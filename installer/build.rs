//! Makes sure a freshly built `ytb_dl_tui` binary for the target platform
//! is sitting in `embed/` before compiling, and exposes its path via an
//! env var so `main.rs` can `include_bytes!` it. This is a build-time
//! guard, not a builder: run the app's own build (for the same target)
//! first -- see `../build-installer.sh`, which does exactly that.

use std::path::Path;

fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();

    let filename = if target_os == "windows" {
        "ytb_dl_tui.exe"
    } else {
        "ytb_dl_tui"
    };
    let path = Path::new(&manifest_dir).join("embed").join(filename);

    if !path.is_file() {
        panic!(
            "\n\n\
             installer build.rs: missing {path}\n\n\
             The installer embeds a real ytb_dl_tui binary at compile time.\n\
             Build the main app for this same target first, e.g.:\n\
             \n    cargo build --release --target <target-triple>\n\
             \n\
             ... then copy target/<target-triple>/release/{filename} to\n\
             installer/embed/{filename} before building the installer.\n\
             (build-installer.sh at the repo root does both steps in order.)\n\n",
            path = path.display(),
            filename = filename,
        );
    }

    println!("cargo:rustc-env=YTB_DL_TUI_BINARY_PATH={}", path.display());
    println!("cargo:rerun-if-changed={}", path.display());
}

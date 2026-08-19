//! Windows only: embeds the app icon (`../assets/icon.ico`) as a resource
//! on the built `.exe`, so it shows a real icon in Explorer, the taskbar,
//! and Alt+Tab instead of the generic default one. No-op on every other
//! platform -- Linux picks its icon up from the `.desktop` entry the
//! installer writes instead (see `../installer/src/shortcut.rs`).

#[cfg(windows)]
fn main() {
    let mut res = winresource::WindowsResource::new();
    res.set_icon("../assets/icon.ico");
    if let Err(e) = res.compile() {
        panic!("\n\nfailed to embed the Windows .exe icon: {e:#}\n\n");
    }
}

#[cfg(not(windows))]
fn main() {}

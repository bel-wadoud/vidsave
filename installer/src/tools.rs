//! Everything that differs between ffmpeg and deno: where to download each
//! from, what shape the download comes in, and how to tell a working copy
//! from a broken one.
//!
//! yt-dlp itself isn't here: we ship our own vendored copy of its source
//! (see `../vendor/`) plus a bundled Python runtime (see `python_runtime.rs`)
//! instead of downloading yt-dlp's own release binary -- `main.rs` installs
//! both directly rather than through this generic per-tool path.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Ffmpeg,
    Deno,
}

impl Tool {
    /// Just deno -- ffmpeg is installed separately (`ensure_ffmpeg` in
    /// `main.rs`) since, unlike every other tool here, it needs a second
    /// file (`ffprobe`) pulled from the same downloaded archive.
    pub const ALL: [Tool; 1] = [Tool::Deno];

    pub fn display_name(self) -> &'static str {
        match self {
            Tool::Ffmpeg => "ffmpeg",
            Tool::Deno => "deno (JS runtime)",
        }
    }

    /// Base executable name, no platform extension -- also what we search
    /// PATH for.
    pub fn exe_stem(self) -> &'static str {
        match self {
            Tool::Ffmpeg => "ffmpeg",
            Tool::Deno => "deno",
        }
    }

    pub fn exe_filename(self) -> String {
        if cfg!(windows) {
            format!("{}.exe", self.exe_stem())
        } else {
            self.exe_stem().to_string()
        }
    }

    /// `ffprobe`, bundled in the same archive as `ffmpeg` -- yt-dlp expects
    /// to find it in the same directory as whatever `--ffmpeg-location`
    /// points at, so `main.rs`'s `ensure_ffmpeg` installs both from one
    /// download rather than fetching the archive twice.
    pub fn companion_exe_filename(self) -> Option<String> {
        match self {
            Tool::Ffmpeg => Some(if cfg!(windows) {
                "ffprobe.exe".to_string()
            } else {
                "ffprobe".to_string()
            }),
            Tool::Deno => None,
        }
    }

    /// Arguments that make the tool print its version and exit cleanly --
    /// used both to sanity-check a pre-existing binary and to confirm a
    /// freshly downloaded one actually runs.
    pub fn version_args(self) -> &'static [&'static str] {
        match self {
            Tool::Ffmpeg => &["-version"],
            Tool::Deno => &["--version"],
        }
    }

    /// Neither is strictly required -- they degrade specific features
    /// (merging/embedding, some videos failing to extract) but the app
    /// still runs without them. The Python runtime + vendored yt-dlp
    /// (installed separately in `main.rs`) *are* required.
    pub fn required(self) -> bool {
        false
    }

    /// Direct-download standalone builds only: a static ffmpeg and deno's
    /// single-binary release -- matching the "drop it next to the app, no
    /// installer, no PATH edits" story the main app's own binary resolution
    /// expects.
    pub fn download_url(self) -> &'static str {
        match self {
            Tool::Ffmpeg => {
                if cfg!(windows) {
                    "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip"
                } else {
                    "https://johnvansickle.com/ffmpeg/releases/ffmpeg-release-amd64-static.tar.xz"
                }
            }
            Tool::Deno => {
                if cfg!(windows) {
                    "https://github.com/denoland/deno/releases/latest/download/deno-x86_64-pc-windows-msvc.zip"
                } else {
                    "https://github.com/denoland/deno/releases/latest/download/deno-x86_64-unknown-linux-gnu.zip"
                }
            }
        }
    }
}

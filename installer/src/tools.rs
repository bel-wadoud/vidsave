//! Everything that differs between yt-dlp, ffmpeg, and deno: where to
//! download each from, what shape the download comes in, and how to tell a
//! working copy from a broken one.

/// How a downloaded payload needs to be turned into a usable executable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Payload {
    /// The download *is* the executable; just save it as-is.
    RawExecutable,
    /// The download is an archive (.zip or .tar.xz, inferred from the URL)
    /// containing the executable somewhere inside.
    Archive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    YtDlp,
    Ffmpeg,
    Deno,
}

impl Tool {
    pub const ALL: [Tool; 3] = [Tool::YtDlp, Tool::Ffmpeg, Tool::Deno];

    pub fn display_name(self) -> &'static str {
        match self {
            Tool::YtDlp => "yt-dlp",
            Tool::Ffmpeg => "ffmpeg",
            Tool::Deno => "deno (JS runtime)",
        }
    }

    /// Base executable name, no platform extension -- also what we search
    /// PATH for.
    pub fn exe_stem(self) -> &'static str {
        match self {
            Tool::YtDlp => "yt-dlp",
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

    /// Arguments that make the tool print its version and exit cleanly --
    /// used both to sanity-check a pre-existing binary and to confirm a
    /// freshly downloaded one actually runs.
    pub fn version_args(self) -> &'static [&'static str] {
        match self {
            Tool::YtDlp => &["--version"],
            Tool::Ffmpeg => &["-version"],
            Tool::Deno => &["--version"],
        }
    }

    /// Only yt-dlp is strictly required; ffmpeg/deno degrade specific
    /// features (merging/embedding, some videos failing to extract) but
    /// the app still runs without them.
    pub fn required(self) -> bool {
        matches!(self, Tool::YtDlp)
    }

    pub fn payload(self) -> Payload {
        match self {
            Tool::YtDlp => Payload::RawExecutable,
            Tool::Ffmpeg | Tool::Deno => Payload::Archive,
        }
    }

    /// Direct-download standalone builds only: yt-dlp's PyInstaller binary
    /// (no Python needed), a static ffmpeg, and deno's single-binary
    /// release -- matching the "drop it next to the app, no installer,
    /// no PATH edits" story the main app's own binary resolution expects.
    pub fn download_url(self) -> &'static str {
        match self {
            Tool::YtDlp => {
                if cfg!(windows) {
                    "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe"
                } else {
                    "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_linux"
                }
            }
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

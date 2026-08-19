//! Shared logic behind every vidsave-tui frontend (the terminal UI, the
//! desktop GUI, and the installer's own use of `Settings`' defaults): config
//! persistence, download-option types, yt-dlp/ffmpeg/JS-runtime resolution
//! and invocation, and the concurrent download queue. Nothing in this crate
//! knows about ratatui, iced, or any other UI toolkit -- each frontend is
//! free to render this however fits its own toolkit.

pub mod config;
pub mod downloader;
pub mod models;
pub mod settings_fields;
pub mod ytdlp;

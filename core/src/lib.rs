//! Shared logic behind every VidSave frontend (the terminal UI, the
//! desktop GUI, and the installer's own use of `Settings`' defaults): config
//! persistence, download-option types, yt-dlp/ffmpeg/JS-runtime resolution
//! and invocation, the concurrent download queue, and download history.
//! Nothing in this crate knows about ratatui, iced, or any other UI toolkit
//! -- each frontend is free to render this however fits its own toolkit.

pub mod config;
pub mod downloader;
pub mod history;
pub mod models;
pub mod settings_fields;
pub mod update_check;
pub mod ytdlp;

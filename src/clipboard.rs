//! Clipboard write: OSC 52 first, then native OS clipboard fallback.

use std::io::{self, Write};

use crossterm::clipboard::CopyToClipboard;
use crossterm::QueueableCommand;

/// Copy `text` to the host clipboard.
///
/// Tries OSC 52 (terminal / SSH-friendly) first. If that write fails, falls
/// back to the native OS clipboard. Returns `Err` only when both fail.
pub fn copy_text(text: &str) -> Result<(), ()> {
    if copy_osc52(text).is_ok() {
        return Ok(());
    }
    copy_native(text)
}

fn copy_osc52(text: &str) -> io::Result<()> {
    let mut out = io::stdout();
    out.queue(CopyToClipboard::to_clipboard_from(text))?;
    out.flush()
}

fn copy_native(text: &str) -> Result<(), ()> {
    let mut clipboard = arboard::Clipboard::new().map_err(|_| ())?;
    clipboard.set_text(text.to_owned()).map_err(|_| ())
}

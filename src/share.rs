//! Share string encode/decode: Difficulty + Seed + generating first click.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;

use crate::app::Difficulty;
use crate::game::Pos;

/// First shipped Share string format version.
pub const SHARE_VERSION: u8 = 1;

const TAG_BEGINNER: u8 = 0;
const TAG_INTERMEDIATE: u8 = 1;
const TAG_EXPERT: u8 = 2;
const TAG_CUSTOM: u8 = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharePayload {
    pub difficulty: Difficulty,
    pub seed: u64,
    pub click: Pos,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShareError {
    /// Encode requested before a generating first click exists.
    NotOpened,
    /// Truncated, corrupt, or non-base64url input.
    Malformed,
    /// Version byte the client does not understand.
    UnknownVersion(u8),
    /// Generating first click outside the encoded Difficulty board.
    OutOfRangeClick,
    /// Width, height, mines, or click coordinate does not fit the `u16` wire field.
    Oversized,
}

impl std::fmt::Display for ShareError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShareError::NotOpened => {
                write!(f, "Share string needs a generating first click first")
            }
            ShareError::Malformed => write!(f, "malformed Share string"),
            ShareError::UnknownVersion(v) => {
                write!(f, "unsupported Share string version {v}")
            }
            ShareError::OutOfRangeClick => {
                write!(f, "Share string click is out of range for that board")
            }
            ShareError::Oversized => {
                write!(f, "Share string field exceeds u16 wire limit")
            }
        }
    }
}

impl std::error::Error for ShareError {}

/// Encode a Share payload to an opaque unpadded base64url string.
///
/// Fails closed if any `usize` field does not fit the `u16` wire encoding.
pub fn encode(payload: &SharePayload) -> Result<String, ShareError> {
    Ok(encode_raw(&encode_bytes(payload)?))
}

/// Decode a Share string. Does not validate click bounds (caller does, with dims).
pub fn decode(s: &str) -> Result<SharePayload, ShareError> {
    if s.is_empty() {
        return Err(ShareError::Malformed);
    }
    let bytes = URL_SAFE_NO_PAD
        .decode(s.as_bytes())
        .map_err(|_| ShareError::Malformed)?;
    decode_bytes(&bytes)
}

/// Base64url-encode arbitrary bytes (test helper for unknown-version cases).
#[cfg(test)]
pub fn encode_raw(bytes: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(bytes)
}

#[cfg(not(test))]
fn encode_raw(bytes: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(bytes)
}

fn encode_bytes(payload: &SharePayload) -> Result<Vec<u8>, ShareError> {
    let mut out = Vec::with_capacity(20);
    out.push(SHARE_VERSION);
    match payload.difficulty {
        Difficulty::Beginner => out.push(TAG_BEGINNER),
        Difficulty::Intermediate => out.push(TAG_INTERMEDIATE),
        Difficulty::Expert => out.push(TAG_EXPERT),
        Difficulty::Custom {
            width,
            height,
            mines,
        } => {
            out.push(TAG_CUSTOM);
            out.extend_from_slice(&u16_bytes(width)?);
            out.extend_from_slice(&u16_bytes(height)?);
            out.extend_from_slice(&u16_bytes(mines)?);
        }
    }
    out.extend_from_slice(&payload.seed.to_le_bytes());
    out.extend_from_slice(&u16_bytes(payload.click.x)?);
    out.extend_from_slice(&u16_bytes(payload.click.y)?);
    Ok(out)
}

fn decode_bytes(bytes: &[u8]) -> Result<SharePayload, ShareError> {
    if bytes.is_empty() {
        return Err(ShareError::Malformed);
    }
    let version = bytes[0];
    if version != SHARE_VERSION {
        return Err(ShareError::UnknownVersion(version));
    }
    let mut i = 1;
    let tag = *bytes.get(i).ok_or(ShareError::Malformed)?;
    i += 1;
    let difficulty = match tag {
        TAG_BEGINNER => Difficulty::Beginner,
        TAG_INTERMEDIATE => Difficulty::Intermediate,
        TAG_EXPERT => Difficulty::Expert,
        TAG_CUSTOM => {
            let width = read_u16(bytes, &mut i)?;
            let height = read_u16(bytes, &mut i)?;
            let mines = read_u16(bytes, &mut i)?;
            if width == 0 || height == 0 {
                return Err(ShareError::Malformed);
            }
            Difficulty::Custom {
                width: width as usize,
                height: height as usize,
                mines: mines as usize,
            }
        }
        _ => return Err(ShareError::Malformed),
    };
    let seed = read_u64(bytes, &mut i)?;
    let click_x = read_u16(bytes, &mut i)?;
    let click_y = read_u16(bytes, &mut i)?;
    if i != bytes.len() {
        return Err(ShareError::Malformed);
    }
    Ok(SharePayload {
        difficulty,
        seed,
        click: Pos {
            x: click_x as usize,
            y: click_y as usize,
        },
    })
}

fn u16_bytes(v: usize) -> Result<[u8; 2], ShareError> {
    let n = u16::try_from(v).map_err(|_| ShareError::Oversized)?;
    Ok(n.to_le_bytes())
}

fn read_u16(bytes: &[u8], i: &mut usize) -> Result<u16, ShareError> {
    let start = *i;
    let end = start.checked_add(2).ok_or(ShareError::Malformed)?;
    let slice = bytes.get(start..end).ok_or(ShareError::Malformed)?;
    *i = end;
    Ok(u16::from_le_bytes([slice[0], slice[1]]))
}

fn read_u64(bytes: &[u8], i: &mut usize) -> Result<u64, ShareError> {
    let start = *i;
    let end = start.checked_add(8).ok_or(ShareError::Malformed)?;
    let slice = bytes.get(start..end).ok_or(ShareError::Malformed)?;
    *i = end;
    let mut buf = [0u8; 8];
    buf.copy_from_slice(slice);
    Ok(u64::from_le_bytes(buf))
}

/// Reject clicks outside the Difficulty board (launch fail-closed).
pub fn validate_click(difficulty: Difficulty, click: Pos) -> Result<(), ShareError> {
    let (w, h, _) = difficulty.dims();
    if click.x >= w || click.y >= h {
        return Err(ShareError::OutOfRangeClick);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_rejects_oversized_custom_dims_fail_closed() {
        let oversized = SharePayload {
            difficulty: Difficulty::Custom {
                width: usize::from(u16::MAX) + 1,
                height: 9,
                mines: 10,
            },
            seed: 1,
            click: Pos { x: 0, y: 0 },
        };
        assert_eq!(encode(&oversized), Err(ShareError::Oversized));
    }

    #[test]
    fn encode_rejects_oversized_click_fail_closed() {
        let oversized = SharePayload {
            difficulty: Difficulty::Beginner,
            seed: 1,
            click: Pos {
                x: usize::from(u16::MAX) + 1,
                y: 0,
            },
        };
        assert_eq!(encode(&oversized), Err(ShareError::Oversized));
    }
}

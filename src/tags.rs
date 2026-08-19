//! Tag (metadata) reading and WAV `LIST INFO` writing.
//!
//! [`read_tags`] extracts Vorbis comments from FLAC files (with the `flac`
//! feature) and `LIST INFO` chunks from WAV files, normalized to uppercase
//! Vorbis-style names (`TITLE`, `ARTIST`, ...). [`write_wav_with_tags`] writes
//! an [`AudioBuffer`] and attaches a `LIST INFO` chunk so metadata survives
//! FLAC → WAV and WAV → WAV conversions.
//!
//! Reading is best-effort: files without parseable metadata yield an empty
//! [`Tags`] rather than an error, so tag extraction can never fail a resample.

use crate::ResampleError;
use crate::io::{AudioBuffer, write_wav};
use std::path::Path;

/// Ordered `(NAME, value)` pairs. Names are uppercase; order is preserved.
pub type Tags = Vec<(String, String)>;

/// Vorbis comment name ↔ RIFF `LIST INFO` fourcc. Tags outside this table are
/// dropped on write (e.g. `ALBUMARTIST`, which has no INFO fourcc).
const MAP: [(&str, &[u8; 4]); 8] = [
    ("TITLE", b"INAM"),
    ("ARTIST", b"IART"),
    ("ALBUM", b"IPRD"),
    ("DATE", b"ICRD"),
    ("TRACKNUMBER", b"ITRK"),
    ("GENRE", b"IGNR"),
    ("COMMENT", b"ICMT"),
    ("COPYRIGHT", b"ICOP"),
];

fn fourcc_to_name(fourcc: &[u8; 4]) -> Option<&'static str> {
    MAP.iter()
        .find_map(|&(name, code)| (code == fourcc).then_some(name))
}

fn name_to_fourcc(name: &str) -> Option<&'static [u8; 4]> {
    MAP.iter()
        .find_map(|&(tag, code)| (tag == name).then_some(code))
}

/// Read tags from a `.flac` or `.wav` file. Unknown extensions and unparseable
/// metadata yield an empty [`Tags`].
pub fn read_tags<P: AsRef<Path>>(path: P) -> Tags {
    let path = path.as_ref();
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("wav") => read_wav_info(path).unwrap_or_default(),
        #[cfg(feature = "flac")]
        Some("flac") => read_flac_tags(path).unwrap_or_default(),
        _ => Vec::new(),
    }
}

#[cfg(feature = "flac")]
fn read_flac_tags(path: &Path) -> Option<Tags> {
    let reader = claxon::FlacReader::open(path).ok()?;
    Some(
        reader
            .tags()
            .map(|(name, value)| (name.to_ascii_uppercase(), value.to_string()))
            .collect(),
    )
}

/// Walk the RIFF chunk list of a WAV file and parse the first `LIST INFO`.
fn read_wav_info(path: &Path) -> Option<Tags> {
    let bytes = std::fs::read(path).ok()?;
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return None;
    }

    let mut offset = 12;
    while offset + 8 <= bytes.len() {
        let id = &bytes[offset..offset + 4];
        let size = u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().ok()?) as usize;
        let body = offset + 8;
        if body + size > bytes.len() {
            return None;
        }

        if id == b"LIST" && size >= 4 && &bytes[body..body + 4] == b"INFO" {
            return parse_info_subchunks(&bytes[body + 4..body + size]);
        }
        offset = body + size + (size & 1);
    }
    None
}

fn parse_info_subchunks(info: &[u8]) -> Option<Tags> {
    let mut tags = Vec::new();
    let mut offset = 0;
    while offset + 8 <= info.len() {
        let fourcc: &[u8; 4] = info[offset..offset + 4].try_into().ok()?;
        let size = u32::from_le_bytes(info[offset + 4..offset + 8].try_into().ok()?) as usize;
        let body = offset + 8;
        if body + size > info.len() {
            break;
        }
        if let Some(name) = fourcc_to_name(fourcc) {
            let value = String::from_utf8_lossy(&info[body..body + size]).into_owned();
            tags.push((name.into(), value));
        }
        offset = body + size + (size & 1);
    }
    Some(tags)
}

/// Write `buffer` as a 32-bit float WAV file, then attach `tags` as a
/// `LIST INFO` chunk between `fmt ` and `data`.
pub fn write_wav_with_tags<P: AsRef<Path>>(
    path: P,
    buffer: &AudioBuffer,
    tags: &Tags,
) -> Result<(), ResampleError> {
    write_wav(&path, buffer)?;
    if tags.is_empty() {
        return Ok(());
    }
    attach_wav_info(path, tags)
}

/// Insert a `LIST INFO` chunk into an existing WAV file, before its `data`
/// chunk, and patch the RIFF size.
pub fn attach_wav_info<P: AsRef<Path>>(path: P, tags: &Tags) -> Result<(), ResampleError> {
    let mut bytes = std::fs::read(&path)?;

    // Locate the `data` chunk; the LIST chunk is inserted right before it.
    let mut offset = 12;
    let insert_at = loop {
        if offset + 8 > bytes.len() {
            return Err(ResampleError::BufferError("no data chunk in WAV".into()));
        }
        if &bytes[offset..offset + 4] == b"data" {
            break offset;
        }
        let size = u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap_or_default())
            as usize;
        offset = offset + 8 + size + (size & 1);
    };

    let mut payload: Vec<u8> = b"INFO".to_vec();
    for (name, value) in tags {
        let Some(fourcc) = name_to_fourcc(name) else {
            continue;
        };
        let value = value.as_bytes();
        payload.extend_from_slice(fourcc);
        payload.extend_from_slice(&(value.len() as u32).to_le_bytes());
        payload.extend_from_slice(value);
        if value.len() & 1 == 1 {
            payload.push(0);
        }
    }

    let mut chunk = Vec::with_capacity(8 + payload.len());
    chunk.extend_from_slice(b"LIST");
    chunk.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    chunk.extend_from_slice(&payload);
    if payload.len() & 1 == 1 {
        chunk.push(0);
    }

    bytes.splice(insert_at..insert_at, chunk);
    let riff_size = (bytes.len() - 8) as u32;
    bytes[4..8].copy_from_slice(&riff_size.to_le_bytes());

    std::fs::write(path, bytes)?;
    Ok(())
}

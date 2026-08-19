//! Tag reading and WAV `LIST INFO` writing tests.

use br41ndmg::io::{AudioBuffer, read_wav, write_wav};
use br41ndmg::tags::{Tags, attach_wav_info, read_tags, write_wav_with_tags};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_path(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("br41ndmg_tags_{name}_{unique}.wav"))
}

fn tiny_buffer() -> AudioBuffer {
    AudioBuffer::new(8000, 1, vec![0.25, -0.25, 0.5, -0.5]).unwrap()
}

fn sample_tags() -> Tags {
    vec![
        ("TITLE".into(), "more than words".into()),
        ("ARTIST".into(), "Hitsujibungaku".into()),
        // Odd-length value exercises subchunk zero-padding.
        ("ALBUM".into(), "abc".into()),
        // No INFO fourcc exists for this one; must be dropped on write.
        ("ALBUMARTIST".into(), "Dropped".into()),
    ]
}

#[test]
fn tagged_wav_round_trips() {
    let path = temp_path("roundtrip");
    write_wav_with_tags(&path, &tiny_buffer(), &sample_tags()).unwrap();

    let tags = read_tags(&path);
    assert_eq!(
        tags,
        vec![
            ("TITLE".to_string(), "more than words".to_string()),
            ("ARTIST".to_string(), "Hitsujibungaku".to_string()),
            ("ALBUM".to_string(), "abc".to_string()),
        ]
    );

    // The patched RIFF size must still satisfy a real WAV reader.
    let buf = read_wav(&path).unwrap();
    assert_eq!(buf.frame_count(), 4);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn plain_wav_has_no_tags() {
    let path = temp_path("plain");
    write_wav(&path, &tiny_buffer()).unwrap();
    assert!(read_tags(&path).is_empty());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn attach_then_read_matches() {
    let path = temp_path("attach");
    write_wav(&path, &tiny_buffer()).unwrap();
    attach_wav_info(&path, &vec![("TITLE".into(), "une vie à peindre".into())]).unwrap();
    assert_eq!(
        read_tags(&path),
        vec![("TITLE".to_string(), "une vie à peindre".to_string())]
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn read_tags_unknown_or_missing_file_is_empty() {
    assert!(read_tags(Path::new("/nonexistent/whatever.mp3")).is_empty());
}

#[cfg(feature = "flac")]
#[test]
fn flac_fixture_tags_survive_resample() {
    let dir = Path::new("test_subjects");
    let fixture = match std::fs::read_dir(dir) {
        Ok(entries) => entries
            .flatten()
            .map(|e| e.path())
            .find(|p| p.extension().and_then(|e| e.to_str()) == Some("flac")),
        Err(_) => return, // fixtures absent: auto-skip
    };
    let Some(fixture) = fixture else { return };

    let source_tags = read_tags(&fixture);
    assert!(
        source_tags.iter().any(|(name, _)| name == "TITLE"),
        "fixture should carry a TITLE tag"
    );

    let out = temp_path("flac_meta");
    let buf = br41ndmg::io::read_audio(&fixture).unwrap();
    let result = buf.resample_to(48000).unwrap();
    write_wav_with_tags(&out, &result, &source_tags).unwrap();

    // Only tags with an INFO fourcc survive; they must keep name and value.
    let written = read_tags(&out);
    let expected: Tags = source_tags
        .iter()
        .filter(|(name, _)| {
            matches!(
                name.as_str(),
                "TITLE"
                    | "ARTIST"
                    | "ALBUM"
                    | "DATE"
                    | "TRACKNUMBER"
                    | "GENRE"
                    | "COMMENT"
                    | "COPYRIGHT"
            )
        })
        .cloned()
        .collect();
    assert_eq!(written, expected);
    let _ = std::fs::remove_file(&out);
}

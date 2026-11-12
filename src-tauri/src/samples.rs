//! Thu bộ câu giọng thật cho gate Phase 2 (SPEC §19.2) — **chỉ bản dev** (`debug_assertions`).
//! Đây là ngoại lệ có chủ đích của "không ghi audio ra đĩa": người dùng tự bật, ghi vào
//! `tests/fixtures/audio/` của repo (gitignore `*.wav`), tên file chỉ được lấy từ manifest.
use serde::Serialize;
use std::path::PathBuf;

const MANIFEST: &str = include_str!("../../tests/fixtures/stt_manifest.csv");

#[derive(Serialize)]
pub struct Sample {
    pub file: String,
    pub category: String,
    pub text: String,
    pub recorded: bool,
}

pub fn dir() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../tests/fixtures/audio"
    ))
}

/// (file, category, ref). Cột file/category không có dấu phẩy → tách 2 lần, phần còn lại là câu (có thể có ngoặc kép CSV).
fn rows() -> impl Iterator<Item = (&'static str, &'static str, String)> {
    MANIFEST
        .lines()
        .skip(1)
        .filter(|l| !l.starts_with('#'))
        .filter_map(|l| {
            let mut it = l.splitn(3, ',');
            let (file, cat, text) = (it.next()?, it.next()?, it.next()?);
            let text = match text.strip_prefix('"').and_then(|t| t.strip_suffix('"')) {
                Some(q) => q.replace("\"\"", "\""),
                None => text.to_string(),
            };
            Some((file, cat, text))
        })
}

pub fn list() -> Vec<Sample> {
    let dir = dir();
    rows()
        .map(|(file, cat, text)| Sample {
            recorded: dir.join(file).exists(),
            file: file.into(),
            category: cat.into(),
            text,
        })
        .collect()
}

/// Chỉ nhận đúng tên file có trong manifest → không có đường đi tùy ý từ IPC.
pub fn path_for(file: &str) -> Option<PathBuf> {
    rows().any(|(f, _, _)| f == file).then(|| dir().join(file))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_has_100_rows_and_rejects_unknown_files() {
        let all = list();
        assert_eq!(all.len(), 100);
        assert_eq!(all[0].text, "Mori ơi, hôm nay mình hơi mệt.");
        assert!(path_for("d01.wav").is_some());
        for bad in ["../x.wav", "d01.wav/../../a", "C:\\x.wav", "", "zz.wav"] {
            assert!(path_for(bad).is_none(), "{bad}");
        }
    }
}

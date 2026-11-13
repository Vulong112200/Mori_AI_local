//! Kiểm SHA-256 model/chương trình theo `config/manifest.sha256` (SPEC §14.2–14.3, audit Phase 6 M1).
//! Băm bằng BCrypt của Windows (windows-sys đã có, không thêm crate). File lớn (GGUF 4.7 GB ≈ 10 s) chỉ băm
//! lại khi kích thước/thời điểm sửa đổi đổi — kết quả nhớ trong settings.
use crate::db::Db;
use std::path::{Path, PathBuf};

const MANIFEST: &str = include_str!("../../config/manifest.sha256");

pub struct Entry {
    pub rel: &'static str,
    pub sha256: &'static str,
}

pub fn manifest() -> Vec<Entry> {
    MANIFEST
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .filter_map(|l| {
            let (sha256, rel) = l.split_once("  ")?;
            Some(Entry {
                rel: rel.trim(),
                sha256: sha256.trim(),
            })
        })
        .collect()
}

pub fn path(data: &Path, rel: &str) -> PathBuf {
    rel.split('/')
        .fold(data.to_path_buf(), |p, part| p.join(part))
}

/// `Ok(())` nếu file khớp manifest; `Err(lý do)` nếu thiếu/sai.
pub fn check(db: &Db, data: &Path, e: &Entry) -> Result<(), String> {
    let p = path(data, e.rel);
    let meta = std::fs::metadata(&p).map_err(|_| "thiếu file".to_string())?;
    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_secs());
    let stamp = format!("{}:{mtime}", meta.len());
    let key = format!("sha256:{}", e.rel);
    let cached = db
        .setting(&key)
        .ok()
        .flatten()
        .and_then(|v| v.strip_prefix(&format!("{stamp}:")).map(str::to_string));
    let hash = match cached {
        Some(h) => h,
        None => {
            let h = sha256_file(&p).map_err(|err| format!("không đọc được: {err}"))?;
            let _ = db.set_setting(&key, &format!("{stamp}:{h}"));
            h
        }
    };
    if hash.eq_ignore_ascii_case(e.sha256) {
        Ok(())
    } else {
        Err("SHA-256 không khớp manifest (file bị đổi/hỏng)".into())
    }
}

#[cfg(windows)]
pub fn sha256_file(p: &Path) -> std::io::Result<String> {
    use std::io::Read;
    use windows_sys::Win32::Security::Cryptography::{
        BCryptCreateHash, BCryptDestroyHash, BCryptFinishHash, BCryptHashData,
        BCRYPT_SHA256_ALG_HANDLE,
    };
    let fail = |what: &str, st: i32| std::io::Error::other(format!("{what}: NTSTATUS {st:#x}"));
    let mut file = std::fs::File::open(p)?;
    let mut hash = std::ptr::null_mut();
    // SAFETY: pseudo-handle SHA256 (Windows 10+); BCrypt tự cấp bộ nhớ đối tượng hash (pbHashObject = null).
    let st = unsafe {
        BCryptCreateHash(
            BCRYPT_SHA256_ALG_HANDLE,
            &mut hash,
            std::ptr::null_mut(),
            0,
            std::ptr::null(),
            0,
            0,
        )
    };
    if st < 0 {
        return Err(fail("BCryptCreateHash", st));
    }
    let mut buf = vec![0u8; 4 << 20];
    let result = (|| {
        loop {
            let n = file.read(&mut buf)?;
            if n == 0 {
                break;
            }
            // SAFETY: buf có ít nhất n byte hợp lệ; hash còn sống tới DestroyHash bên dưới.
            let st = unsafe { BCryptHashData(hash, buf.as_ptr(), n as u32, 0) };
            if st < 0 {
                return Err(fail("BCryptHashData", st));
            }
        }
        let mut out = [0u8; 32];
        // SAFETY: out đủ 32 byte cho SHA-256.
        let st = unsafe { BCryptFinishHash(hash, out.as_mut_ptr(), 32, 0) };
        if st < 0 {
            return Err(fail("BCryptFinishHash", st));
        }
        Ok(out.iter().map(|b| format!("{b:02x}")).collect())
    })();
    // SAFETY: hash tạo bởi BCryptCreateHash ở trên, hủy đúng một lần.
    unsafe { BCryptDestroyHash(hash) };
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_parses_and_hash_is_correct() {
        let m = manifest();
        assert!(
            m.len() >= 6
                && m.iter()
                    .all(|e| e.sha256.len() == 64 && !e.rel.contains('\\'))
        );
        let dir = std::env::temp_dir().join(format!("mori_sha_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("abc.txt");
        std::fs::write(&f, "abc").unwrap();
        assert_eq!(
            sha256_file(&f).unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        let db = Db::in_memory().unwrap();
        let ok = Entry {
            rel: "abc.txt",
            sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        };
        assert!(check(&db, &dir, &ok).is_ok());
        assert!(check(&db, &dir, &ok).is_ok(), "lần 2 dùng kết quả đã nhớ");
        std::fs::write(&f, "abcd").unwrap(); // file bị đổi → kích thước đổi → băm lại → phát hiện
        assert!(
            check(&db, &dir, &ok).is_err(),
            "file bị đổi phải bị phát hiện"
        );
        assert!(check(
            &db,
            &dir,
            &Entry {
                rel: "missing.txt",
                sha256: "00"
            }
        )
        .is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}

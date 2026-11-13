//! SQLite: hội thoại, tin nhắn, settings, memory (SPEC §11). Migration theo `PRAGMA user_version`.
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use std::{path::Path, sync::Mutex};

const MIGRATIONS: &[&str] = &[
    // v1 — Phase 1.
    "CREATE TABLE conversations (
         id TEXT PRIMARY KEY,
         title TEXT,
         created_at TEXT NOT NULL,
         updated_at TEXT NOT NULL
     );
     CREATE TABLE messages (
         id TEXT PRIMARY KEY,
         conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
         role TEXT NOT NULL CHECK (role IN ('user','assistant','system')),
         content TEXT NOT NULL,
         created_at TEXT NOT NULL
     );
     CREATE INDEX idx_messages_conv ON messages(conversation_id, created_at);
     CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
    // v2 — Phase 4: memory + FTS5 (tìm không dấu), tóm tắt hội thoại.
    // `summary_upto` = số tin đầu tiên của hội thoại đã nằm trong `summary` (tin nhắn không bị xóa lẻ).
    "CREATE TABLE memories (
         id TEXT PRIMARY KEY,
         category TEXT NOT NULL,
         content TEXT NOT NULL,
         source_message_id TEXT REFERENCES messages(id) ON DELETE SET NULL,
         status TEXT NOT NULL DEFAULT 'proposed' CHECK (status IN ('proposed','approved','rejected')),
         created_at TEXT NOT NULL,
         updated_at TEXT NOT NULL
     );
     CREATE VIRTUAL TABLE memories_fts USING fts5(
         content, content='memories', content_rowid='rowid', tokenize='unicode61 remove_diacritics 2'
     );
     CREATE TRIGGER memories_ai AFTER INSERT ON memories BEGIN
         INSERT INTO memories_fts(rowid, content) VALUES (new.rowid, new.content);
     END;
     CREATE TRIGGER memories_ad AFTER DELETE ON memories BEGIN
         INSERT INTO memories_fts(memories_fts, rowid, content) VALUES ('delete', old.rowid, old.content);
     END;
     CREATE TRIGGER memories_au AFTER UPDATE OF content ON memories BEGIN
         INSERT INTO memories_fts(memories_fts, rowid, content) VALUES ('delete', old.rowid, old.content);
         INSERT INTO memories_fts(rowid, content) VALUES (new.rowid, new.content);
     END;
     ALTER TABLE conversations ADD COLUMN summary TEXT;
     ALTER TABLE conversations ADD COLUMN summary_upto INTEGER NOT NULL DEFAULT 0;",
];

// id và thời gian sinh trong SQL → không cần crate uuid/chrono.
const NEW_ID: &str = "lower(hex(randomblob(16)))";
const NOW: &str = "strftime('%Y-%m-%dT%H:%M:%fZ','now')";

/// Nhóm memory (UI hiển thị tên tiếng Việt).
pub const CATEGORIES: &[&str] = &["so_thich", "du_an", "ca_nhan", "khac"];
/// Giới hạn một memory (ranh giới tin cậy + giữ prompt gọn).
pub const MAX_MEMORY_CHARS: usize = 500;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Message {
    pub id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Memory {
    pub id: String,
    pub category: String,
    pub content: String,
    pub status: String,
    pub created_at: String,
}

pub struct Db(Mutex<Connection>);

fn migrate(conn: &Connection) -> rusqlite::Result<()> {
    let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(version as usize) {
        conn.execute_batch(&format!(
            "BEGIN; {sql}; PRAGMA user_version = {}; COMMIT;",
            i + 1
        ))?;
    }
    Ok(())
}

/// Gộp khoảng trắng/xuống dòng (memory nằm trên một dòng trong prompt) và cắt độ dài.
pub fn one_line(s: &str, max: usize) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max)
        .collect()
}

/// Truy vấn FTS5 từ câu tự do: chỉ giữ từ chữ/số (không lọt cú pháp FTS), nối OR.
fn fts_query(text: &str) -> Option<String> {
    let terms: Vec<String> = text
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.chars().count() >= 2)
        .take(32)
        .map(|w| format!("\"{w}\""))
        .collect();
    (!terms.is_empty()).then(|| terms.join(" OR "))
}

fn fail(msg: &str) -> rusqlite::Error {
    rusqlite::Error::SqliteFailure(
        rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_ERROR),
        Some(msg.into()),
    )
}

/// Dồn WAL vào file DB và cắt WAL về 0; có kết nối khác đang đọc → báo lỗi thay vì báo "đã xóa" sai.
fn checkpoint(conn: &Connection) -> rusqlite::Result<()> {
    let busy: i64 = conn.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |r| r.get(0))?;
    if busy != 0 {
        return Err(fail(
            "chưa dọn được WAL (DB đang được mở ở nơi khác), thử lại sau",
        ));
    }
    Ok(())
}

fn memory_row(r: &rusqlite::Row) -> rusqlite::Result<Memory> {
    Ok(Memory {
        id: r.get(0)?,
        category: r.get(1)?,
        content: r.get(2)?,
        status: r.get(3)?,
        created_at: r.get(4)?,
    })
}

const MEMORY_COLS: &str = "id, category, content, status, created_at";

impl Db {
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        Self::init(Connection::open(path)?)
    }

    #[cfg(test)]
    pub fn in_memory() -> rusqlite::Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> rusqlite::Result<Self> {
        conn.execute_batch(
            "PRAGMA foreign_keys = ON; PRAGMA secure_delete = ON; PRAGMA journal_mode = WAL;",
        )?;
        migrate(&conn)?;
        Ok(Self(Mutex::new(conn)))
    }

    fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn create_conversation(&self) -> rusqlite::Result<String> {
        self.conn().query_row(
            &format!("INSERT INTO conversations (id, created_at, updated_at) VALUES ({NEW_ID}, {NOW}, {NOW}) RETURNING id"),
            [],
            |r| r.get(0),
        )
    }

    pub fn latest_conversation(&self) -> rusqlite::Result<Option<String>> {
        self.conn()
            .query_row(
                "SELECT id FROM conversations ORDER BY updated_at DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .optional()
    }

    pub fn messages(&self, conversation_id: &str) -> rusqlite::Result<Vec<Message>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, role, content, created_at FROM messages WHERE conversation_id = ?1 ORDER BY created_at, rowid",
        )?;
        let rows = stmt.query_map([conversation_id], |r| {
            Ok(Message {
                id: r.get(0)?,
                role: r.get(1)?,
                content: r.get(2)?,
                created_at: r.get(3)?,
            })
        })?;
        rows.collect()
    }

    /// Nội dung tin nhắn của người dùng (nút ⭐ chỉ dành cho tin người dùng).
    pub fn user_message_content(&self, id: &str) -> rusqlite::Result<Option<String>> {
        self.conn()
            .query_row(
                "SELECT content FROM messages WHERE id = ?1 AND role = 'user'",
                [id],
                |r| r.get(0),
            )
            .optional()
    }

    pub fn add_message(
        &self,
        conversation_id: &str,
        role: &str,
        content: &str,
    ) -> rusqlite::Result<Message> {
        let conn = self.conn();
        conn.execute(
            &format!("UPDATE conversations SET updated_at = {NOW} WHERE id = ?1"),
            [conversation_id],
        )?;
        conn.query_row(
            &format!(
                "INSERT INTO messages (id, conversation_id, role, content, created_at)
                 VALUES ({NEW_ID}, ?1, ?2, ?3, {NOW}) RETURNING id, role, content, created_at"
            ),
            params![conversation_id, role, content],
            |r| {
                Ok(Message {
                    id: r.get(0)?,
                    role: r.get(1)?,
                    content: r.get(2)?,
                    created_at: r.get(3)?,
                })
            },
        )
    }

    /// (tóm tắt, số tin đầu đã tóm tắt).
    pub fn summary(&self, conversation_id: &str) -> rusqlite::Result<(Option<String>, usize)> {
        self.conn()
            .query_row(
                "SELECT summary, summary_upto FROM conversations WHERE id = ?1",
                [conversation_id],
                |r| Ok((r.get(0)?, r.get::<_, i64>(1)?.max(0) as usize)),
            )
            .optional()
            .map(|o| o.unwrap_or((None, 0)))
    }

    /// Ghi tóm tắt chỉ khi trạng thái còn như lúc đọc (`prev_upto`) và hội thoại đủ `upto` tin
    /// → tóm tắt chạy chồng hoặc chạy sau khi khôi phục DB không ghi đè sai. Trả `false` nếu bỏ qua.
    pub fn set_summary(
        &self,
        conversation_id: &str,
        summary: &str,
        prev_upto: usize,
        upto: usize,
    ) -> rusqlite::Result<bool> {
        self.conn()
            .execute(
                "UPDATE conversations SET summary = ?2, summary_upto = ?3
                 WHERE id = ?1 AND summary_upto = ?4
                   AND (SELECT count(*) FROM messages WHERE conversation_id = ?1) >= ?3",
                params![conversation_id, summary, upto as i64, prev_upto as i64],
            )
            .map(|n| n > 0)
    }

    pub fn setting(&self, key: &str) -> rusqlite::Result<Option<String>> {
        self.conn()
            .query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| {
                r.get(0)
            })
            .optional()
    }

    pub fn set_setting(&self, key: &str, value: &str) -> rusqlite::Result<()> {
        self.conn()
            .execute(
                "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![key, value],
            )
            .map(|_| ())
    }

    // ---- Memory (SPEC §11.4): AI/lệnh chỉ đề xuất; chỉ `approved` vào prompt. ----

    /// Lưu ghi nhớ dùng ngay (ADR-0012: tự nhớ, người dùng xem/sửa/xóa/hoàn tác sau).
    pub fn remember(
        &self,
        content: &str,
        category: &str,
        source: Option<&str>,
    ) -> rusqlite::Result<Memory> {
        self.conn().query_row(
            &format!(
                "INSERT INTO memories (id, category, content, source_message_id, status, created_at, updated_at)
                 VALUES ({NEW_ID}, ?1, ?2, ?3, 'approved', {NOW}, {NOW}) RETURNING {MEMORY_COLS}"
            ),
            params![category, one_line(content, MAX_MEMORY_CHARS), source],
            memory_row,
        )
    }

    #[cfg(test)]
    pub fn propose_memory(&self, content: &str, source: Option<&str>) -> rusqlite::Result<Memory> {
        self.conn().query_row(
            &format!(
                "INSERT INTO memories (id, category, content, source_message_id, status, created_at, updated_at)
                 VALUES ({NEW_ID}, 'khac', ?1, ?2, 'proposed', {NOW}, {NOW}) RETURNING {MEMORY_COLS}"
            ),
            params![one_line(content, MAX_MEMORY_CHARS), source],
            memory_row,
        )
    }

    /// Đề xuất trước (mới nhất trên cùng), rồi đã duyệt.
    pub fn memories(&self) -> rusqlite::Result<Vec<Memory>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(&format!(
            "SELECT {MEMORY_COLS} FROM memories WHERE status != 'rejected'
             ORDER BY status = 'approved', created_at DESC, rowid DESC"
        ))?;
        let rows = stmt.query_map([], memory_row)?;
        rows.collect()
    }

    /// Trả `false` nếu không có memory `id`.
    pub fn update_memory(
        &self,
        id: &str,
        content: &str,
        category: &str,
        status: &str,
    ) -> rusqlite::Result<bool> {
        let n = self.conn().execute(
            &format!(
                "UPDATE memories SET content = ?2, category = ?3, status = ?4, updated_at = {NOW} WHERE id = ?1"
            ),
            params![id, one_line(content, MAX_MEMORY_CHARS), category, status],
        )?;
        if n > 0 {
            self.scrub()?; // nội dung cũ không còn trong FTS/WAL
        }
        Ok(n > 0)
    }

    /// Xóa có kiểm chứng (SPEC §11.4): `secure_delete` ghi đè trang, rebuild FTS bỏ token cũ, checkpoint WAL.
    pub fn delete_memory(&self, id: &str) -> rusqlite::Result<()> {
        self.conn()
            .execute("DELETE FROM memories WHERE id = ?1", [id])?;
        self.scrub()
    }

    pub fn delete_all_memories(&self) -> rusqlite::Result<()> {
        self.conn().execute("DELETE FROM memories", [])?;
        self.scrub()?;
        let conn = self.conn();
        conn.execute_batch("VACUUM;")?;
        checkpoint(&conn)
    }

    fn scrub(&self) -> rusqlite::Result<()> {
        let conn = self.conn();
        conn.execute_batch("INSERT INTO memories_fts(memories_fts) VALUES ('rebuild');")?;
        checkpoint(&conn)
    }

    /// Memory đã duyệt cho prompt. Đủ ngân sách → tất cả theo thứ tự tạo (prefix ổn định → KV cache tái dùng).
    /// ponytail: vượt ngân sách → chỉ các memory khớp FTS5 với tin mới (đổi theo lượt → mất cache phần sau);
    /// thêm ghim/ưu tiên khi số memory thật sự lớn.
    pub fn approved_memories(&self, query: &str, budget: usize) -> rusqlite::Result<Vec<String>> {
        let conn = self.conn();
        let all: Vec<String> = conn
            .prepare(
                "SELECT content FROM memories WHERE status = 'approved' ORDER BY created_at, rowid",
            )?
            .query_map([], |r| r.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        if all.iter().map(|m| m.chars().count()).sum::<usize>() <= budget {
            return Ok(all);
        }
        let Some(q) = fts_query(query) else {
            return Ok(Vec::new());
        };
        let hits: Vec<String> = conn
            .prepare(
                "SELECT m.content FROM memories_fts JOIN memories m ON m.rowid = memories_fts.rowid
                 WHERE memories_fts MATCH ?1 AND m.status = 'approved' ORDER BY rank LIMIT 20",
            )?
            .query_map([q], |r| r.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        let mut used = 0;
        Ok(hits
            .into_iter()
            .take_while(|m| {
                used += m.chars().count();
                used <= budget
            })
            .collect())
    }

    // ---- Sao lưu / khôi phục (SPEC §11.4) ----

    /// Ghi bản sao nhất quán ra `dir/mori-<thời gian>.db`, trả tên file.
    pub fn backup(&self, dir: &Path) -> rusqlite::Result<String> {
        std::fs::create_dir_all(dir)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(e.into()))?;
        let conn = self.conn();
        let stamp: String =
            conn.query_row("SELECT strftime('%Y%m%d-%H%M%f','now')", [], |r| r.get(0))?;
        let name = format!("mori-{}.db", stamp.replace('.', ""));
        let path = dir.join(&name);
        conn.execute("VACUUM INTO ?1", [path.to_string_lossy()])?;
        Ok(name)
    }

    /// Thay toàn bộ DB bằng bản sao lưu `path` (đã kiểm tên ở tầng trên), rồi nâng schema nếu bản cũ.
    /// Bản sao lưu phải nguyên vẹn và không mới hơn schema của app.
    pub fn restore(&self, path: &Path) -> rusqlite::Result<()> {
        let src = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        let ok: String = src.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
        let version: i64 = src.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        drop(src);
        if ok != "ok" || version < 1 || version as usize > MIGRATIONS.len() {
            return Err(fail(
                "bản sao lưu hỏng hoặc không phải của phiên bản Mori này",
            ));
        }
        let mut conn = self.conn();
        conn.restore(
            rusqlite::MAIN_DB,
            path,
            None::<fn(rusqlite::backup::Progress)>,
        )?;
        migrate(&conn)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_and_stores_messages_in_order() {
        let db = Db::in_memory().unwrap();
        let v: i64 = db
            .conn()
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(v as usize, MIGRATIONS.len());

        let c = db.create_conversation().unwrap();
        assert_eq!(
            db.latest_conversation().unwrap().as_deref(),
            Some(c.as_str())
        );
        db.add_message(&c, "user", "Mori ơi").unwrap();
        db.add_message(&c, "assistant", "Mình đây!").unwrap();
        let msgs = db.messages(&c).unwrap();
        assert_eq!(
            msgs.iter().map(|m| m.content.as_str()).collect::<Vec<_>>(),
            ["Mori ơi", "Mình đây!"]
        );
        assert!(
            db.add_message(&c, "hacker", "x").is_err(),
            "role ngoài danh sách phải bị CHECK chặn"
        );
        assert_eq!(db.summary(&c).unwrap(), (None, 0));
        assert!(db.set_summary(&c, "Đã chào nhau.", 0, 2).unwrap());
        assert_eq!(db.summary(&c).unwrap(), (Some("Đã chào nhau.".into()), 2));
        assert!(
            !db.set_summary(&c, "chồng", 0, 2).unwrap(),
            "đọc cũ → không ghi đè"
        );
        assert!(
            !db.set_summary(&c, "quá", 2, 5).unwrap(),
            "upto > số tin → bỏ"
        );
    }

    #[test]
    fn settings_upsert() {
        let db = Db::in_memory().unwrap();
        assert_eq!(db.setting("fast_mode").unwrap(), None);
        db.set_setting("fast_mode", "1").unwrap();
        db.set_setting("fast_mode", "0").unwrap();
        assert_eq!(db.setting("fast_mode").unwrap().as_deref(), Some("0"));
    }

    #[test]
    fn memory_lifecycle_and_prompt_selection() {
        let db = Db::in_memory().unwrap();
        let m = db
            .propose_memory("Người dùng   thích\ncà phê đen", None)
            .unwrap();
        assert_eq!(
            (m.content.as_str(), m.status.as_str()),
            ("Người dùng thích cà phê đen", "proposed")
        );
        assert!(
            db.approved_memories("", 2000).unwrap().is_empty(),
            "đề xuất chưa duyệt không vào prompt"
        );
        assert!(db
            .update_memory(&m.id, "Người dùng thích cà phê đen", "so_thich", "approved")
            .unwrap());
        assert!(
            db.update_memory(&m.id, "x", "khac", "bad").is_err(),
            "status ngoài CHECK"
        );
        let b = db
            .propose_memory("Dự án Mori viết bằng Rust", None)
            .unwrap();
        db.update_memory(&b.id, &b.content, "du_an", "approved")
            .unwrap();
        assert_eq!(
            db.approved_memories("", 2000).unwrap(),
            ["Người dùng thích cà phê đen", "Dự án Mori viết bằng Rust"]
        );
        // Vượt ngân sách → chỉ memory khớp FTS (không dấu cũng khớp).
        assert_eq!(
            db.approved_memories("uong ca phe khong?", 30).unwrap(),
            ["Người dùng thích cà phê đen"]
        );
        assert!(db.approved_memories("\"*) OR (", 10).unwrap().is_empty());
        db.delete_memory(&m.id).unwrap();
        assert_eq!(db.memories().unwrap().len(), 1);
        db.delete_all_memories().unwrap();
        assert!(db.memories().unwrap().is_empty());
    }

    /// Gate Phase 4: đọc thô file DB (+ WAL) sau khi xóa → không còn chuỗi đã xóa (kể cả token FTS).
    #[test]
    fn deleted_memory_not_in_raw_file() {
        let dir = std::env::temp_dir().join(format!("mori_rawdel_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("t.db");
        let raw = |needle: &str| -> bool {
            ["", "-wal"].iter().any(|ext| {
                std::fs::read(format!("{}{ext}", path.display()))
                    .map(|b| b.windows(needle.len()).any(|w| w == needle.as_bytes()))
                    .unwrap_or(false)
            })
        };
        {
            let db = Db::open(&path).unwrap();
            let keep = db.propose_memory("giữ lại qwertyuiop", None).unwrap();
            let a = db
                .propose_memory("Mật khẩu zqxjkwvbn của nhà", None)
                .unwrap();
            db.update_memory(&a.id, &a.content, "ca_nhan", "approved")
                .unwrap();
            let b = db.propose_memory("bí mật plmoknijb", None).unwrap();
            db.update_memory(&b.id, "bí mật đã sửa xcvbnmasd", "khac", "approved")
                .unwrap();
            assert!(
                raw("zqxjkwvbn") && raw("xcvbnmasd"),
                "phải thấy trước khi xóa"
            );
            assert!(!raw("plmoknijb"), "nội dung cũ trước khi sửa phải bị xóa");
            db.delete_memory(&a.id).unwrap();
            assert!(!raw("zqxjkwvbn"), "xóa từng memory: còn trong file");
            assert!(raw("qwertyuiop"), "memory khác phải còn");
            db.delete_all_memories().unwrap();
            assert!(
                !raw("xcvbnmasd") && !raw("qwertyuiop"),
                "xóa toàn bộ: còn trong file"
            );
            assert!(db.memories().unwrap().is_empty());
            let _ = keep;
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn backup_then_restore() {
        let dir = std::env::temp_dir().join(format!("mori_backup_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        {
            let db = Db::open(&dir.join("live.db")).unwrap();
            let c = db.create_conversation().unwrap();
            db.add_message(&c, "user", "trước sao lưu").unwrap();
            let name = db.backup(&dir.join("backups")).unwrap();
            assert!(name.starts_with("mori-") && name.ends_with(".db"));
            db.add_message(&c, "user", "sau sao lưu").unwrap();
            db.propose_memory("sẽ mất khi khôi phục", None).unwrap();
            db.restore(&dir.join("backups").join(&name)).unwrap();
            assert_eq!(db.messages(&c).unwrap().len(), 1);
            assert!(db.memories().unwrap().is_empty());
            std::fs::write(dir.join("backups").join("mori-x.db"), b"khong phai sqlite").unwrap();
            assert!(db.restore(&dir.join("backups").join("mori-x.db")).is_err());
            assert_eq!(
                db.messages(&c).unwrap().len(),
                1,
                "khôi phục hỏng không đụng DB"
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}

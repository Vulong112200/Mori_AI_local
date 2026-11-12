//! SQLite: hội thoại, tin nhắn, settings. Migration theo `PRAGMA user_version` (SPEC §11.3).
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use std::{path::Path, sync::Mutex};

const MIGRATIONS: &[&str] = &[
    // v1 — Phase 1. Bảng memories đến ở Phase 4 (v2).
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
];

// id và thời gian sinh trong SQL → không cần crate uuid/chrono.
const NEW_ID: &str = "lower(hex(randomblob(16)))";
const NOW: &str = "strftime('%Y-%m-%dT%H:%M:%fZ','now')";

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Message {
    pub id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
}

pub struct Db(Mutex<Connection>);

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
        let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        for (i, sql) in MIGRATIONS.iter().enumerate().skip(version as usize) {
            conn.execute_batch(&format!(
                "BEGIN; {sql}; PRAGMA user_version = {}; COMMIT;",
                i + 1
            ))?;
        }
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
    }

    #[test]
    fn settings_upsert() {
        let db = Db::in_memory().unwrap();
        assert_eq!(db.setting("fast_mode").unwrap(), None);
        db.set_setting("fast_mode", "1").unwrap();
        db.set_setting("fast_mode", "0").unwrap();
        assert_eq!(db.setting("fast_mode").unwrap().as_deref(), Some("0"));
    }
}

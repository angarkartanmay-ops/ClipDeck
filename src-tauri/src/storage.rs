// Copyright (c) 2026 Tanmay Angarkar. Licensed under PolyForm Noncommercial 1.0.0.

//! History storage: SQLite for metadata, one file per item for its raw data,
//! one PNG per image item for its thumbnail. Payloads are never kept in RAM.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

use crate::clip::ClipData;
use crate::crypto;

pub struct Store {
    conn: Mutex<Connection>,
    blobs: PathBuf,
    thumbs: PathBuf,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ItemDto {
    pub id: i64,
    pub kind: String,
    pub created_at: i64,
    pub source_app: String,
    pub source_exe: String,
    pub pinned: bool,
    pub size: i64,
    pub text: String,
    pub meta: serde_json::Value,
    pub has_thumb: bool,
}

pub struct NewItem<'a> {
    pub kind: &'a str,
    pub created_at: i64,
    pub source_app: &'a str,
    pub source_exe: &'a str,
    pub pinned: bool,
    pub hash: &'a str,
    pub preview: &'a str,
    pub meta: &'a serde_json::Value,
}

type Result<T> = std::result::Result<T, String>;

fn db_err(e: rusqlite::Error) -> String {
    format!("database: {e}")
}

fn seal(bytes: &[u8], encrypt: bool) -> Result<Vec<u8>> {
    if encrypt {
        crypto::protect(bytes)
    } else {
        Ok(bytes.to_vec())
    }
}

fn open_sealed(bytes: Vec<u8>, encrypted: bool) -> Result<Vec<u8>> {
    if encrypted {
        crypto::unprotect(&bytes)
    } else {
        Ok(bytes)
    }
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes).map_err(|e| format!("write {}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("rename {}: {e}", path.display()))
}

impl Store {
    pub fn open(dir: &Path) -> Result<Store> {
        let blobs = dir.join("blobs");
        let thumbs = dir.join("thumbs");
        for d in [&blobs, &thumbs] {
            std::fs::create_dir_all(d).map_err(|e| format!("create {}: {e}", d.display()))?;
        }
        let conn = Connection::open(dir.join("clipdeck.db")).map_err(db_err)?;
        conn.execute_batch(
            "PRAGMA cache_size = -512;
             PRAGMA secure_delete = ON;
             PRAGMA foreign_keys = ON;
             CREATE TABLE IF NOT EXISTS items (
                 id          INTEGER PRIMARY KEY AUTOINCREMENT,
                 kind        TEXT    NOT NULL,
                 created_at  INTEGER NOT NULL,
                 source_app  TEXT    NOT NULL DEFAULT '',
                 source_exe  TEXT    NOT NULL DEFAULT '',
                 pinned      INTEGER NOT NULL DEFAULT 0,
                 size        INTEGER NOT NULL,
                 hash        TEXT    NOT NULL,
                 preview     BLOB,
                 meta        TEXT    NOT NULL DEFAULT '{}',
                 has_thumb   INTEGER NOT NULL DEFAULT 0,
                 encrypted   INTEGER NOT NULL DEFAULT 0
             );
             CREATE INDEX IF NOT EXISTS items_created ON items(created_at DESC);
             CREATE INDEX IF NOT EXISTS items_hash ON items(hash);",
        )
        .map_err(db_err)?;
        let store = Store {
            conn: Mutex::new(conn),
            blobs,
            thumbs,
        };
        store.remove_orphan_files();
        Ok(store)
    }

    fn blob_path(&self, id: i64) -> PathBuf {
        self.blobs.join(format!("{id}.bin"))
    }

    fn thumb_path(&self, id: i64) -> PathBuf {
        self.thumbs.join(format!("{id}.png"))
    }

    fn remove_files(&self, id: i64) {
        let _ = std::fs::remove_file(self.blob_path(id));
        let _ = std::fs::remove_file(self.thumb_path(id));
    }

    /// Deletes payload files left behind by a crash between file and row writes.
    fn remove_orphan_files(&self) {
        let ids: std::collections::HashSet<i64> = {
            let conn = self.conn.lock().unwrap();
            let mut stmt = match conn.prepare("SELECT id FROM items") {
                Ok(s) => s,
                Err(_) => return,
            };
            let rows = stmt.query_map([], |r| r.get(0));
            match rows {
                Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
                Err(_) => return,
            }
        };
        for dir in [&self.blobs, &self.thumbs] {
            let Ok(entries) = std::fs::read_dir(dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                let id = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .and_then(|s| s.parse::<i64>().ok());
                if id.map_or(true, |id| !ids.contains(&id)) {
                    let _ = std::fs::remove_file(path);
                }
            }
        }
    }

    pub fn latest_hash(&self) -> Option<String> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT hash FROM items ORDER BY created_at DESC, id DESC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .optional()
        .ok()
        .flatten()
    }

    pub fn find_by_hash(&self, hash: &str) -> Option<(i64, bool)> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, pinned FROM items WHERE hash = ?1 ORDER BY created_at DESC LIMIT 1",
            params![hash],
            |r| Ok((r.get(0)?, r.get::<_, i64>(1)? != 0)),
        )
        .optional()
        .ok()
        .flatten()
    }

    pub fn insert(&self, item: &NewItem, data: &ClipData, encrypt: bool) -> Result<i64> {
        let payload = seal(&data.serialize(), encrypt)?;
        let preview = seal(item.preview.as_bytes(), encrypt)?;
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO items (kind, created_at, source_app, source_exe, pinned, size, hash, preview, meta, encrypted)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                item.kind,
                item.created_at,
                item.source_app,
                item.source_exe,
                item.pinned as i64,
                payload.len() as i64,
                item.hash,
                preview,
                item.meta.to_string(),
                encrypt as i64,
            ],
        )
        .map_err(db_err)?;
        let id = conn.last_insert_rowid();
        if let Err(err) = write_atomic(&self.blob_path(id), &payload) {
            let _ = conn.execute("DELETE FROM items WHERE id = ?1", params![id]);
            return Err(err);
        }
        Ok(id)
    }

    pub fn save_thumb(&self, id: i64, png: &[u8]) -> Result<()> {
        let encrypted = self.is_encrypted(id)?;
        let bytes = seal(png, encrypted)?;
        write_atomic(&self.thumb_path(id), &bytes)?;
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE items SET has_thumb = 1, size = size + ?2 WHERE id = ?1",
            params![id, bytes.len() as i64],
        )
        .map_err(db_err)?;
        Ok(())
    }

    fn is_encrypted(&self, id: i64) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT encrypted FROM items WHERE id = ?1",
            params![id],
            |r| r.get::<_, i64>(0),
        )
        .map(|v| v != 0)
        .map_err(db_err)
    }

    pub fn list(&self) -> Result<Vec<ItemDto>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, kind, created_at, source_app, source_exe, pinned, size, preview, meta, has_thumb, encrypted
                 FROM items ORDER BY created_at DESC, id DESC",
            )
            .map_err(db_err)?;
        let rows = stmt
            .query_map([], |r| {
                let preview: Option<Vec<u8>> = r.get(7)?;
                let encrypted = r.get::<_, i64>(10)? != 0;
                let meta: String = r.get(8)?;
                Ok((
                    ItemDto {
                        id: r.get(0)?,
                        kind: r.get(1)?,
                        created_at: r.get(2)?,
                        source_app: r.get(3)?,
                        source_exe: r.get(4)?,
                        pinned: r.get::<_, i64>(5)? != 0,
                        size: r.get(6)?,
                        text: String::new(),
                        meta: serde_json::from_str(&meta).unwrap_or_default(),
                        has_thumb: r.get::<_, i64>(9)? != 0,
                    },
                    preview.unwrap_or_default(),
                    encrypted,
                ))
            })
            .map_err(db_err)?;
        let mut items = Vec::new();
        for row in rows {
            let (mut item, preview, encrypted) = row.map_err(db_err)?;
            if let Ok(bytes) = open_sealed(preview, encrypted) {
                item.text = String::from_utf8_lossy(&bytes).into_owned();
            }
            items.push(item);
        }
        Ok(items)
    }

    /// Ids in dashboard order (newest first).
    pub fn ids(&self) -> Result<Vec<i64>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id FROM items ORDER BY created_at DESC, id DESC")
            .map_err(db_err)?;
        let ids = stmt.query_map([], |r| r.get(0)).map_err(db_err)?;
        ids.collect::<std::result::Result<Vec<i64>, _>>()
            .map_err(db_err)
    }

    pub fn load_data(&self, id: i64) -> Result<ClipData> {
        let encrypted = self.is_encrypted(id)?;
        let bytes =
            std::fs::read(self.blob_path(id)).map_err(|e| format!("read item {id}: {e}"))?;
        ClipData::deserialize(&open_sealed(bytes, encrypted)?)
    }

    pub fn load_thumb(&self, id: i64) -> Result<Option<Vec<u8>>> {
        let encrypted = self.is_encrypted(id)?;
        match std::fs::read(self.thumb_path(id)) {
            Ok(bytes) => open_sealed(bytes, encrypted).map(Some),
            Err(_) => Ok(None),
        }
    }

    pub fn set_pinned(&self, id: i64, pinned: bool) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE items SET pinned = ?2 WHERE id = ?1",
            params![id, pinned as i64],
        )
        .map_err(db_err)?;
        Ok(())
    }

    pub fn delete(&self, id: i64) -> Result<()> {
        {
            let conn = self.conn.lock().unwrap();
            conn.execute("DELETE FROM items WHERE id = ?1", params![id])
                .map_err(db_err)?;
        }
        self.remove_files(id);
        Ok(())
    }

    /// Removes every item (or every unpinned item). Returns the removed ids.
    pub fn clear(&self, keep_pinned: bool) -> Result<Vec<i64>> {
        let ids: Vec<i64> = {
            let conn = self.conn.lock().unwrap();
            let sql = if keep_pinned {
                "SELECT id FROM items WHERE pinned = 0"
            } else {
                "SELECT id FROM items"
            };
            let mut stmt = conn.prepare(sql).map_err(db_err)?;
            let ids = stmt.query_map([], |r| r.get(0)).map_err(db_err)?;
            ids.collect::<std::result::Result<_, _>>().map_err(db_err)?
        };
        for id in &ids {
            self.delete(*id)?;
        }
        Ok(ids)
    }

    /// Evicts the oldest unpinned items until both caps hold. Pinned items
    /// are never evicted. Returns the removed ids.
    pub fn enforce_limits(&self, max_items: u32, max_bytes: u64) -> Result<Vec<i64>> {
        let mut removed = Vec::new();
        let over_count: Vec<i64> = {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn
                .prepare(
                    "SELECT id FROM items WHERE pinned = 0
                     ORDER BY created_at DESC, id DESC LIMIT -1 OFFSET ?1",
                )
                .map_err(db_err)?;
            let ids = stmt
                .query_map(params![max_items], |r| r.get(0))
                .map_err(db_err)?;
            ids.collect::<std::result::Result<_, _>>().map_err(db_err)?
        };
        for id in over_count {
            self.delete(id)?;
            removed.push(id);
        }
        loop {
            let (total, oldest): (i64, Option<i64>) = {
                let conn = self.conn.lock().unwrap();
                let total = conn
                    .query_row("SELECT COALESCE(SUM(size), 0) FROM items", [], |r| r.get(0))
                    .map_err(db_err)?;
                let oldest = conn
                    .query_row(
                        "SELECT id FROM items WHERE pinned = 0 ORDER BY created_at ASC, id ASC LIMIT 1",
                        [],
                        |r| r.get(0),
                    )
                    .optional()
                    .map_err(db_err)?;
                (total, oldest)
            };
            match oldest {
                Some(id) if total as u64 > max_bytes => {
                    self.delete(id)?;
                    removed.push(id);
                }
                _ => break,
            }
        }
        Ok(removed)
    }

    pub fn stats(&self) -> (i64, i64) {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT COUNT(*), COALESCE(SUM(size), 0) FROM items",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap_or((0, 0))
    }

    /// Re-writes every item whose encryption state differs from `encrypt`.
    pub fn set_encryption(&self, encrypt: bool) -> Result<()> {
        let rows: Vec<(i64, Option<Vec<u8>>)> = {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn
                .prepare("SELECT id, preview FROM items WHERE encrypted != ?1")
                .map_err(db_err)?;
            let rows = stmt
                .query_map(params![encrypt as i64], |r| Ok((r.get(0)?, r.get(1)?)))
                .map_err(db_err)?;
            rows.collect::<std::result::Result<_, _>>()
                .map_err(db_err)?
        };
        for (id, preview) in rows {
            let was = !encrypt;
            let convert =
                |bytes: Vec<u8>| -> Result<Vec<u8>> { seal(&open_sealed(bytes, was)?, encrypt) };
            let preview = convert(preview.unwrap_or_default())?;
            let blob = convert(std::fs::read(self.blob_path(id)).map_err(|e| e.to_string())?)?;
            let thumb = match std::fs::read(self.thumb_path(id)) {
                Ok(bytes) => Some(convert(bytes)?),
                Err(_) => None,
            };
            write_atomic(&self.blob_path(id), &blob)?;
            let mut size = blob.len() as i64;
            if let Some(thumb) = &thumb {
                write_atomic(&self.thumb_path(id), thumb)?;
                size += thumb.len() as i64;
            }
            let conn = self.conn.lock().unwrap();
            conn.execute(
                "UPDATE items SET preview = ?2, encrypted = ?3, size = ?4 WHERE id = ?1",
                params![id, preview, encrypt as i64, size],
            )
            .map_err(db_err)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clip::{ClipFormat, TEXT};

    fn temp_store(name: &str) -> (Store, PathBuf) {
        let dir = std::env::temp_dir().join(format!("clipdeck-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        (Store::open(&dir).unwrap(), dir)
    }

    fn text_item(s: &str) -> ClipData {
        ClipData {
            formats: vec![ClipFormat {
                name: TEXT.into(),
                data: s.encode_utf16().flat_map(u16::to_le_bytes).collect(),
            }],
        }
    }

    fn add(store: &Store, s: &str, ts: i64, pinned: bool, encrypt: bool) -> i64 {
        let data = text_item(s);
        let meta = serde_json::json!({});
        let item = NewItem {
            kind: "text",
            created_at: ts,
            source_app: "Test",
            source_exe: "test.exe",
            pinned,
            hash: &data.hash(),
            preview: s,
            meta: &meta,
        };
        store.insert(&item, &data, encrypt).unwrap()
    }

    #[test]
    fn insert_list_load_delete() {
        let (store, dir) = temp_store("basic");
        let id = add(&store, "hello", 1, false, false);
        let items = store.list().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].text, "hello");
        assert_eq!(
            store.load_data(id).unwrap().text().as_deref(),
            Some("hello")
        );
        assert!(store.find_by_hash(&text_item("hello").hash()).is_some());
        store.delete(id).unwrap();
        assert!(store.list().unwrap().is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn eviction_keeps_pinned() {
        let (store, dir) = temp_store("evict");
        let pinned = add(&store, "keep me", 0, true, false);
        for i in 1..=15 {
            add(&store, &format!("item {i}"), i, false, false);
        }
        let removed = store.enforce_limits(10, u64::MAX).unwrap();
        assert_eq!(removed.len(), 5);
        let ids = store.ids().unwrap();
        assert_eq!(ids.len(), 11);
        assert!(ids.contains(&pinned));
        // A tiny disk cap evicts every unpinned item but never the pinned one.
        store.enforce_limits(10, 1).unwrap();
        assert_eq!(store.ids().unwrap(), vec![pinned]);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn encryption_roundtrip() {
        let (store, dir) = temp_store("crypto");
        let id = add(&store, "secret", 1, false, true);
        let raw = std::fs::read(store.blob_path(id)).unwrap();
        assert!(!raw.windows(4).any(|w| w == b"CDK1"));
        assert_eq!(store.list().unwrap()[0].text, "secret");
        store.set_encryption(false).unwrap();
        let raw = std::fs::read(store.blob_path(id)).unwrap();
        assert!(raw.starts_with(b"CDK1"));
        assert_eq!(
            store.load_data(id).unwrap().text().as_deref(),
            Some("secret")
        );
        let _ = std::fs::remove_dir_all(dir);
    }
}

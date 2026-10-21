use super::{OpenCodeDatabase, SessionMetadata};
use crate::CaptureResult;
use rusqlite::params;
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};

pub struct OpenCodePage {
    pub messages: Vec<Value>,
    pub complete: bool,
    pub read_messages: usize,
    pub total_messages: usize,
    pub sha256: [u8; 32],
}
pub struct OpenCodeSnapshot {
    database: OpenCodeDatabase,
    pub metadata: SessionMetadata,
    after: (i64, String),
    read: usize,
    bytes: usize,
    hash: Sha256,
    failed: bool,
}
impl OpenCodeSnapshot {
    pub(super) fn new(database: OpenCodeDatabase, metadata: SessionMetadata) -> Self {
        Self {
            database,
            metadata,
            after: (i64::MIN, String::new()),
            read: 0,
            bytes: 0,
            hash: Sha256::new(),
            failed: false,
        }
    }
    pub fn next(
        &mut self,
        maximum: usize,
        check: &dyn Fn() -> CaptureResult<()>,
    ) -> CaptureResult<OpenCodePage> {
        let result = self.read(maximum, check);
        if result.is_err() {
            self.failed = true;
        }
        result
    }
    fn read(
        &mut self,
        maximum: usize,
        check: &dyn Fn() -> CaptureResult<()>,
    ) -> CaptureResult<OpenCodePage> {
        if self.failed || !(1..=32).contains(&maximum) {
            return Err("OpenCode snapshot page unavailable".into());
        }
        check()?;
        self.database.check()?;
        let connection = &self.database.connection;
        let total: usize = connection.query_row(
            "SELECT count(*) FROM message WHERE session_id=?1",
            [&self.metadata.id],
            |row| row.get(0),
        )?;
        if total > 8192 {
            return Err("OpenCode message bound exceeded".into());
        }
        let mut statement = connection.prepare("SELECT id,time_created,CASE WHEN length(CAST(data AS BLOB))<=1048576 THEN data ELSE NULL END FROM message WHERE session_id=?1 AND (time_created,id)>(?2,?3) ORDER BY time_created,id LIMIT ?4")?;
        let rows = statement.query_map(
            params![self.metadata.id, self.after.0, self.after.1, maximum as i64],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )?;
        let mut messages = Vec::new();
        let mut page_bytes = 0usize;
        for row in rows {
            check()?;
            let (id, time, raw) = row?;
            page_bytes = page_bytes
                .checked_add(raw.len())
                .ok_or("OpenCode page byte bound")?;
            if page_bytes > 4 * 1024 * 1024 {
                return Err("OpenCode page byte bound exceeded".into());
            }
            let mut info: Value =
                serde_json::from_str(&raw).map_err(|_| "OpenCode message JSON unavailable")?;
            let object = info
                .as_object_mut()
                .ok_or("OpenCode message shape unavailable")?;
            object.insert("id".into(), json!(id));
            object.insert("sessionID".into(), json!(self.metadata.id));
            let mut parts_statement = connection.prepare("SELECT id,CASE WHEN length(CAST(data AS BLOB))<=1048576 THEN data ELSE NULL END FROM part WHERE session_id=?1 AND message_id=?2 ORDER BY time_created,id LIMIT 257")?;
            let parts_rows = parts_statement.query_map(params![self.metadata.id, id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            let mut parts = Vec::new();
            for part in parts_rows {
                check()?;
                let (part_id, raw) = part?;
                page_bytes = page_bytes
                    .checked_add(raw.len())
                    .ok_or("OpenCode page byte bound")?;
                if page_bytes > 4 * 1024 * 1024 {
                    return Err("OpenCode page byte bound exceeded".into());
                }
                self.bytes = self
                    .bytes
                    .checked_add(raw.len())
                    .ok_or("OpenCode byte bound")?;
                if parts.len() == 256 || self.bytes > 64 * 1024 * 1024 {
                    return Err("OpenCode part/byte bound exceeded".into());
                }
                let mut value: Value =
                    serde_json::from_str(&raw).map_err(|_| "OpenCode part JSON unavailable")?;
                let object = value
                    .as_object_mut()
                    .ok_or("OpenCode part shape unavailable")?;
                object.insert("id".into(), json!(part_id));
                object.insert("messageID".into(), json!(id));
                object.insert("sessionID".into(), json!(self.metadata.id));
                parts.push(value);
            }
            self.bytes = self
                .bytes
                .checked_add(raw.len())
                .ok_or("OpenCode byte bound")?;
            if self.bytes > 64 * 1024 * 1024 {
                return Err("OpenCode byte bound exceeded".into());
            }
            let value = json!({"info":info,"parts":parts,"createdAtMillis":time});
            self.hash.update(serde_json::to_vec(&value)?);
            messages.push(value);
            self.after = (time, id);
            self.read += 1;
        }
        self.database.check()?;
        check()?;
        Ok(OpenCodePage {
            messages,
            complete: self.read == total,
            read_messages: self.read,
            total_messages: total,
            sha256: self.hash.clone().finalize().into(),
        })
    }
}

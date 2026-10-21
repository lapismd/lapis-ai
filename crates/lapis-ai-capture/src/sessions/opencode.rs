use crate::{CaptureResult, source_file};
use rusqlite::{Connection, OpenFlags, params};
use std::path::{Path, PathBuf};

/// Native-only metadata. These fields have not passed application privacy policy.
#[derive(Clone)]
pub struct SessionMetadata {
    pub id: String,
    pub directory: PathBuf,
    pub title: String,
    pub version: String,
    pub created_at: i64,
    pub updated_at: i64,
}
pub struct OpenCodeDatabase {
    pub(super) connection: Connection,
    path: PathBuf,
    stamp: source_file::FileStamp,
}
impl OpenCodeDatabase {
    pub fn open(path: &Path) -> CaptureResult<Self> {
        let (_, stamp) = source_file::open_regular(path, None)?;
        let connection = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        connection.busy_timeout(std::time::Duration::from_millis(250))?;
        connection.execute_batch("PRAGMA query_only=ON; PRAGMA trusted_schema=OFF;")?;
        connection.prepare(
            "SELECT id,directory,title,version,time_created,time_updated FROM session LIMIT 0",
        )?;
        connection.prepare("SELECT id,session_id,time_created,data FROM message LIMIT 0")?;
        connection
            .prepare("SELECT id,message_id,session_id,time_created,data FROM part LIMIT 0")?;
        let database = Self {
            connection,
            path: path.to_owned(),
            stamp,
        };
        database.check()?;
        Ok(database)
    }
    pub(super) fn check(&self) -> CaptureResult<()> {
        let (_, stamp) = source_file::open_regular(&self.path, None)?;
        if stamp.device != self.stamp.device || stamp.inode != self.stamp.inode {
            return Err("OpenCode database identity changed".into());
        }
        Ok(())
    }
    pub fn sessions(&self, after: &str, maximum: usize) -> CaptureResult<Vec<SessionMetadata>> {
        if maximum == 0 || maximum > 256 || after.len() > 128 {
            return Err("OpenCode metadata page bound".into());
        }
        self.check()?;
        let mut statement = self.connection.prepare("SELECT CASE WHEN length(CAST(id AS BLOB))<=128 THEN id ELSE NULL END,CASE WHEN length(CAST(directory AS BLOB))<=4096 THEN directory ELSE NULL END,CASE WHEN length(CAST(title AS BLOB))<=4096 THEN title ELSE NULL END,CASE WHEN length(CAST(version AS BLOB))<=80 THEN version ELSE NULL END,time_created,time_updated FROM session WHERE id > ?1 ORDER BY id LIMIT ?2")?;
        let rows = statement.query_map(params![after, maximum as i64], |row| {
            Ok(SessionMetadata {
                id: row.get(0)?,
                directory: PathBuf::from(row.get::<_, String>(1)?),
                title: row.get(2)?,
                version: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?;
        let mut result = Vec::new();
        for row in rows {
            let row = row?;
            if !valid_id(&row.id)
                || !row.directory.is_absolute()
                || row.directory.as_os_str().len() > 4096
                || row.title.len() > 4096
                || row.version.len() > 80
            {
                return Err("OpenCode metadata exceeds its supported bounds".into());
            }
            result.push(row);
        }
        self.check()?;
        Ok(result)
    }
    pub fn snapshot(&self, session: &str) -> CaptureResult<super::OpenCodeSnapshot> {
        if !valid_id(session) {
            return Err("OpenCode session selector unavailable".into());
        }
        self.check()?;
        let database = Self::open(&self.path)?;
        database.connection.execute_batch("BEGIN")?;
        let metadata = database.sessions_before_snapshot(session)?;
        Ok(super::OpenCodeSnapshot::new(database, metadata))
    }
    fn sessions_before_snapshot(&self, session: &str) -> CaptureResult<SessionMetadata> {
        let row = self.connection.query_row(
            "SELECT CASE WHEN length(CAST(id AS BLOB))<=128 THEN id ELSE NULL END,CASE WHEN length(CAST(directory AS BLOB))<=4096 THEN directory ELSE NULL END,CASE WHEN length(CAST(title AS BLOB))<=4096 THEN title ELSE NULL END,CASE WHEN length(CAST(version AS BLOB))<=80 THEN version ELSE NULL END,time_created,time_updated FROM session WHERE id=?1",
            [session],
            |row| {
                Ok(SessionMetadata {
                    id: row.get(0)?,
                    directory: PathBuf::from(row.get::<_, String>(1)?),
                    title: row.get(2)?,
                    version: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            },
        )?;
        if !row.directory.is_absolute() || row.directory.as_os_str().len() > 4096 {
            return Err("OpenCode context unavailable".into());
        }
        Ok(row)
    }
}
pub(super) fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 128 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

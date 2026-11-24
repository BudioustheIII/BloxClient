//! SQLite access. A single connection is shared behind a mutex; queries here
//! are short and local, so a pool would add cost without benefit.

mod migrations;

use std::path::Path;
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use rusqlite::Connection;

use crate::error::AppResult;

pub struct Database {
    connection: Mutex<Connection>,
}

impl Database {
    pub fn open(path: &Path) -> AppResult<Self> {
        let connection = Connection::open(path)?;
        connection.busy_timeout(Duration::from_secs(3))?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "synchronous", "NORMAL")?;
        Self::prepare(connection)
    }

    #[cfg(test)]
    pub fn in_memory() -> AppResult<Self> {
        Self::prepare(Connection::open_in_memory()?)
    }

    fn prepare(mut connection: Connection) -> AppResult<Self> {
        connection.pragma_update(None, "foreign_keys", "ON")?;
        // Overwrite deleted content instead of leaving it in free pages.
        connection.pragma_update(None, "secure_delete", "ON")?;
        migrations::run(&mut connection)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    /// A panic mid-query leaves no partial state: SQLite rolls back the open
    /// transaction, so a poisoned lock is safe to reuse.
    pub fn connection(&self) -> MutexGuard<'_, Connection> {
        self.connection
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    pub fn schema_version(&self) -> AppResult<i64> {
        migrations::current_version(&self.connection())
    }
}

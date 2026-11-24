//! Forward-only schema migrations tracked with `PRAGMA user_version`.
//! Append new files to `MIGRATIONS`; never edit one that has shipped.

use rusqlite::Connection;

use crate::error::{AppError, AppResult};

const MIGRATIONS: &[&str] = &[
    include_str!("../../migrations/0001_settings.sql"),
    include_str!("../../migrations/0002_deployments.sql"),
    include_str!("../../migrations/0003_recent_experiences.sql"),
    include_str!("../../migrations/0004_vault.sql"),
    include_str!("../../migrations/0005_fflags.sql"),
];

pub fn current_version(connection: &Connection) -> AppResult<i64> {
    Ok(connection.pragma_query_value(None, "user_version", |row| row.get(0))?)
}

pub fn run(connection: &mut Connection) -> AppResult<()> {
    apply(connection, MIGRATIONS)
}

fn apply(connection: &mut Connection, migrations: &[&str]) -> AppResult<()> {
    let current = current_version(connection)?;
    let supported = migrations.len() as i64;
    if current > supported {
        return Err(AppError::DatabaseTooNew {
            found: current,
            supported,
        });
    }

    for (index, sql) in migrations.iter().enumerate().skip(current as usize) {
        let transaction = connection.transaction()?;
        transaction.execute_batch(sql)?;
        transaction.pragma_update(None, "user_version", (index + 1) as i64)?;
        transaction.commit()?;
        log::info!("applied database migration {}", index + 1);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_apply_once() {
        let mut connection = Connection::open_in_memory().unwrap();
        run(&mut connection).unwrap();
        run(&mut connection).unwrap();
        assert_eq!(
            current_version(&connection).unwrap(),
            MIGRATIONS.len() as i64
        );
    }

    #[test]
    fn newer_database_is_rejected_and_untouched() {
        let mut connection = Connection::open_in_memory().unwrap();
        connection
            .pragma_update(None, "user_version", MIGRATIONS.len() as i64 + 1)
            .unwrap();
        assert!(matches!(
            run(&mut connection),
            Err(AppError::DatabaseTooNew { .. })
        ));
        assert_eq!(
            current_version(&connection).unwrap(),
            MIGRATIONS.len() as i64 + 1
        );
    }

    #[test]
    fn failed_migration_rolls_back() {
        let mut connection = Connection::open_in_memory().unwrap();
        let broken = ["CREATE TABLE good (id INTEGER); THIS IS NOT SQL;"];
        assert!(apply(&mut connection, &broken).is_err());
        assert_eq!(current_version(&connection).unwrap(), 0);
        let tables: i64 = connection
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE name = 'good'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(tables, 0);
    }
}

//! Encrypted storage for personal data, using Windows DPAPI. Values can only
//! be decrypted by this Windows user on this PC, so a copied database is
//! useless. DPAPI can't stop malware already running as the same user, which
//! is why sign-in tokens are never stored at all.

use std::ptr::{null, null_mut};

use rusqlite::{params, Connection, OptionalExtension};
use serde::de::DeserializeOwned;
use serde::Serialize;
use windows_sys::Win32::Foundation::LocalFree;
use windows_sys::Win32::Security::Cryptography::{
    CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
};

use crate::error::{AppError, AppResult};

const ENTROPY: &[u8] = b"BloxClient.vault.v1";

fn blob(bytes: &[u8]) -> CRYPT_INTEGER_BLOB {
    CRYPT_INTEGER_BLOB {
        cbData: bytes.len() as u32,
        pbData: bytes.as_ptr().cast_mut(),
    }
}

/// Copies a DPAPI output buffer and releases it.
///
/// # Safety
/// `output` must have been filled by a successful DPAPI call.
unsafe fn take_output(output: CRYPT_INTEGER_BLOB) -> Vec<u8> {
    let bytes = unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) }.to_vec();
    unsafe { LocalFree(output.pbData.cast()) };
    bytes
}

fn protect(plain: &[u8]) -> AppResult<Vec<u8>> {
    let (input, entropy) = (blob(plain), blob(ENTROPY));
    let mut output = CRYPT_INTEGER_BLOB { cbData: 0, pbData: null_mut() };
    // SAFETY: the input blobs borrow live slices that DPAPI only reads, and
    // `output` is released by `take_output` on success.
    unsafe {
        if CryptProtectData(&input, null(), &entropy, null(), null(), CRYPTPROTECT_UI_FORBIDDEN, &mut output) == 0 {
            return Err(AppError::Vault(std::io::Error::last_os_error().to_string()));
        }
        Ok(take_output(output))
    }
}

fn unprotect(encrypted: &[u8]) -> AppResult<Vec<u8>> {
    let (input, entropy) = (blob(encrypted), blob(ENTROPY));
    let mut output = CRYPT_INTEGER_BLOB { cbData: 0, pbData: null_mut() };
    // SAFETY: as in `protect`.
    unsafe {
        if CryptUnprotectData(&input, null_mut(), &entropy, null(), null(), CRYPTPROTECT_UI_FORBIDDEN, &mut output) == 0 {
            return Err(AppError::Vault(std::io::Error::last_os_error().to_string()));
        }
        Ok(take_output(output))
    }
}

/// Unreadable values, such as a database moved from another PC, read as empty.
pub fn read<T: DeserializeOwned>(connection: &Connection, key: &str) -> AppResult<Option<T>> {
    let stored: Option<Vec<u8>> = connection
        .query_row("SELECT value FROM vault WHERE key = ?1", [key], |row| row.get(0))
        .optional()?;
    let Some(stored) = stored else {
        return Ok(None);
    };
    let value = unprotect(&stored).and_then(|plain| Ok(serde_json::from_slice(&plain)?));
    match value {
        Ok(value) => Ok(Some(value)),
        Err(error) => {
            log::warn!("encrypted value '{key}' could not be read: {error}");
            Ok(None)
        }
    }
}

pub fn write<T: Serialize>(connection: &Connection, key: &str, value: &T) -> AppResult<()> {
    let encrypted = protect(&serde_json::to_vec(value)?)?;
    connection.execute(
        "INSERT INTO vault (key, value) VALUES (?1, ?2)
         ON CONFLICT (key) DO UPDATE SET value = excluded.value",
        params![key, encrypted],
    )?;
    Ok(())
}

pub fn remove(connection: &Connection, key: &str) -> AppResult<()> {
    connection.execute("DELETE FROM vault WHERE key = ?1", [key])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    #[test]
    fn values_round_trip() {
        let db = Database::in_memory().unwrap();
        let connection = db.connection();
        write(&connection, "item", &"private value".to_string()).unwrap();
        assert_eq!(read::<String>(&connection, "item").unwrap().as_deref(), Some("private value"));
        remove(&connection, "item").unwrap();
        assert_eq!(read::<String>(&connection, "item").unwrap(), None);
    }

    #[test]
    fn stored_bytes_do_not_contain_the_value() {
        let db = Database::in_memory().unwrap();
        let connection = db.connection();
        write(&connection, "item", &"private value".to_string()).unwrap();
        let raw: Vec<u8> = connection
            .query_row("SELECT value FROM vault WHERE key = 'item'", [], |row| row.get(0))
            .unwrap();
        assert!(!raw.windows(7).any(|window| window == b"private"));
    }

    #[test]
    fn tampered_values_read_as_empty() {
        let db = Database::in_memory().unwrap();
        let connection = db.connection();
        write(&connection, "item", &"private value".to_string()).unwrap();
        connection
            .execute("UPDATE vault SET value = x'00112233' WHERE key = 'item'", [])
            .unwrap();
        assert_eq!(read::<String>(&connection, "item").unwrap(), None);
    }

    #[test]
    fn a_different_entropy_cannot_decrypt() {
        let encrypted = protect(b"private value").unwrap();
        let (input, wrong) = (blob(&encrypted), blob(b"another app"));
        let mut output = CRYPT_INTEGER_BLOB { cbData: 0, pbData: null_mut() };
        // SAFETY: the blobs borrow live slices; failure leaves nothing to free.
        let ok = unsafe {
            CryptUnprotectData(&input, null_mut(), &wrong, null(), null(), CRYPTPROTECT_UI_FORBIDDEN, &mut output)
        };
        assert_eq!(ok, 0);
    }
}

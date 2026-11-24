//! The per-user `bloxclient://` link that returns the browser to the app. It
//! exists only while a sign-in is in progress.

use std::io::{self, ErrorKind};

use winreg::enums::HKEY_CURRENT_USER;
use winreg::RegKey;

const PROTOCOL_KEY: &str = r"Software\Classes\bloxclient";

fn write_handler(key_path: &str, executable: &str) -> io::Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hkcu.create_subkey(key_path)?;
    key.set_value("", &"URL:BloxClient sign-in")?;
    key.set_value("URL Protocol", &"")?;
    let (command, _) = key.create_subkey(r"shell\open\command")?;
    command.set_value("", &format!("\"{executable}\" \"%1\""))
}

fn remove_handler(key_path: &str) -> io::Result<()> {
    match RegKey::predef(HKEY_CURRENT_USER).delete_subkey_all(key_path) {
        Err(error) if error.kind() != ErrorKind::NotFound => Err(error),
        _ => Ok(()),
    }
}

pub fn register() -> io::Result<()> {
    let executable = std::env::current_exe()?;
    write_handler(PROTOCOL_KEY, &executable.display().to_string())
}

pub fn unregister() {
    if let Err(error) = remove_handler(PROTOCOL_KEY) {
        log::warn!("could not remove sign-in link: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A private key, so tests never register a real protocol handler.
    const TEST_PARENT: &str = r"Software\BloxClientProtocolTests";
    const TEST_KEY: &str = r"Software\BloxClientProtocolTests\Protocol";

    #[test]
    fn handler_round_trip() {
        remove_handler(TEST_KEY).unwrap();
        write_handler(TEST_KEY, r"C:\Apps\BloxClient.exe").unwrap();

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let command: String = hkcu
            .open_subkey(format!(r"{TEST_KEY}\shell\open\command"))
            .unwrap()
            .get_value("")
            .unwrap();
        assert_eq!(command, r#""C:\Apps\BloxClient.exe" "%1""#);

        remove_handler(TEST_KEY).unwrap();
        remove_handler(TEST_KEY).unwrap();
        assert!(hkcu.open_subkey(TEST_KEY).is_err());
        hkcu.delete_subkey_all(TEST_PARENT).unwrap();
    }
}

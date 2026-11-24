//! Authenticode verification through the Windows trust provider. Checks both
//! that the signature chain is valid and who the signer is.

use std::mem::{size_of, zeroed};
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use windows_sys::Win32::Foundation::{CRYPT_E_NO_REVOCATION_CHECK, CRYPT_E_REVOCATION_OFFLINE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Security::Cryptography::{CertGetNameStringW, CERT_NAME_SIMPLE_DISPLAY_TYPE};
use windows_sys::Win32::Security::WinTrust::{
    WTHelperGetProvCertFromChain, WTHelperGetProvSignerFromChain, WTHelperProvDataFromStateData, WinVerifyTrust,
    WINTRUST_ACTION_GENERIC_VERIFY_V2, WINTRUST_DATA, WINTRUST_FILE_INFO, WTD_CHOICE_FILE, WTD_REVOKE_NONE,
    WTD_REVOKE_WHOLECHAIN, WTD_STATEACTION_CLOSE, WTD_STATEACTION_VERIFY, WTD_UI_NONE,
};

use super::error::DeploymentError;

pub const ROBLOX_PUBLISHER: &str = "Roblox Corporation";
pub const MICROSOFT_PUBLISHER: &str = "Microsoft Corporation";

pub fn verify_publisher(path: &Path, expected: &str) -> Result<(), DeploymentError> {
    let name = path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();

    let signer = match check(&wide, true) {
        // Being offline shouldn't block use. A certificate actually reported
        // as revoked is a different status and still fails.
        Err(status) if status == CRYPT_E_REVOCATION_OFFLINE || status == CRYPT_E_NO_REVOCATION_CHECK => {
            check(&wide, false)
        }
        result => result,
    }
    .map_err(|status| DeploymentError::Signature(format!("{name}: trust status {status:#010x}")))?;

    if signer == expected {
        Ok(())
    } else {
        Err(DeploymentError::Signature(format!("{name}: signed by {signer:?}")))
    }
}

fn check(path: &[u16], revocation: bool) -> Result<String, i32> {
    // SAFETY: all structs are zero-initialized then given their documented
    // sizes, `path` is NUL-terminated and outlives both calls, and the state
    // handle from the verify call is always released by the close call.
    unsafe {
        let mut file: WINTRUST_FILE_INFO = zeroed();
        file.cbStruct = size_of::<WINTRUST_FILE_INFO>() as u32;
        file.pcwszFilePath = path.as_ptr();

        let mut data: WINTRUST_DATA = zeroed();
        data.cbStruct = size_of::<WINTRUST_DATA>() as u32;
        data.dwUIChoice = WTD_UI_NONE;
        data.fdwRevocationChecks = if revocation { WTD_REVOKE_WHOLECHAIN } else { WTD_REVOKE_NONE };
        data.dwUnionChoice = WTD_CHOICE_FILE;
        data.Anonymous.pFile = &mut file;
        data.dwStateAction = WTD_STATEACTION_VERIFY;

        let mut action = WINTRUST_ACTION_GENERIC_VERIFY_V2;
        let window = INVALID_HANDLE_VALUE as _;
        // Every later access goes through this pointer because Windows writes
        // the state handle into the struct behind the compiler's back.
        let data_ptr = &raw mut data;

        let status = WinVerifyTrust(window, &mut action, data_ptr.cast());
        let result = if status == 0 {
            signer_name((*data_ptr).hWVTStateData).ok_or(status)
        } else {
            Err(status)
        };

        (*data_ptr).dwStateAction = WTD_STATEACTION_CLOSE;
        WinVerifyTrust(window, &mut action, data_ptr.cast());
        result
    }
}

/// # Safety
/// `state` must be a live handle from a successful `WinVerifyTrust` call.
unsafe fn signer_name(state: windows_sys::Win32::Foundation::HANDLE) -> Option<String> {
    let provider = WTHelperProvDataFromStateData(state);
    if provider.is_null() {
        return None;
    }
    let signer = WTHelperGetProvSignerFromChain(provider, 0, 0, 0);
    if signer.is_null() {
        return None;
    }
    let certificate = WTHelperGetProvCertFromChain(signer, 0);
    if certificate.is_null() || (*certificate).pCert.is_null() {
        return None;
    }

    let mut buffer = [0u16; 256];
    let written = CertGetNameStringW(
        (*certificate).pCert,
        CERT_NAME_SIMPLE_DISPLAY_TYPE,
        0,
        std::ptr::null(),
        buffer.as_mut_ptr(),
        buffer.len() as u32,
    ) as usize;
    (written > 1).then(|| String::from_utf16_lossy(&buffer[..written - 1]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsigned_file_is_rejected() {
        let path = std::env::temp_dir().join(format!("bloxclient-unsigned-{}.exe", std::process::id()));
        std::fs::write(&path, b"MZ not really a program").unwrap();
        let result = verify_publisher(&path, ROBLOX_PUBLISHER);
        std::fs::remove_file(&path).unwrap();
        assert!(matches!(result, Err(DeploymentError::Signature(_))));
    }

    #[test]
    fn missing_file_is_rejected() {
        let path = std::env::temp_dir().join("bloxclient-does-not-exist.exe");
        assert!(matches!(verify_publisher(&path, ROBLOX_PUBLISHER), Err(DeploymentError::Signature(_))));
    }

    /// Needs a local Roblox install: `cargo test -- --ignored signed_binaries`.
    #[test]
    #[ignore]
    fn signed_binaries_are_checked_by_publisher() {
        let versions = Path::new(&std::env::var("LOCALAPPDATA").unwrap()).join(r"Roblox\Versions");
        let install = std::fs::read_dir(versions)
            .unwrap()
            .flatten()
            .map(|entry| entry.path())
            .find(|path| path.join("RobloxPlayerBeta.exe").exists() && path.join("WebView2Loader.dll").exists())
            .expect("no Roblox install");

        verify_publisher(&install.join("RobloxPlayerBeta.exe"), ROBLOX_PUBLISHER).unwrap();
        verify_publisher(&install.join("WebView2Loader.dll"), MICROSOFT_PUBLISHER).unwrap();
        // A valid signature from the wrong company must not pass.
        assert!(verify_publisher(&install.join("WebView2Loader.dll"), ROBLOX_PUBLISHER).is_err());
    }
}

//! Roblox sign-in. BloxClient stores no cookie, password or token: it uses the
//! OAuth tokens once to read the profile, then revokes and discards them. The
//! only thing saved is the display profile, encrypted in the vault.

mod error;
mod model;
mod oauth;
mod pkce;
mod protocol;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use reqwest::Client;
use rusqlite::{params, OptionalExtension};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_opener::OpenerExt;

pub use error::AuthError;
pub use model::{Account, AuthState, AuthStatus};

use crate::db::Database;
use crate::error::AppResult;
use crate::state::AppState;
use crate::vault;
use oauth::CallbackResult;
use pkce::Secret;

const AUTH_EVENT: &str = "auth-changed";
const ACCOUNT_KEY: &str = "account";
const CLIENT_ID_KEY: &str = "auth.client_id";
const WELCOME_KEY: &str = "auth.welcome_seen";
const OAUTH_DASHBOARD: &str = "https://create.roblox.com/dashboard/credentials?activeTab=OAuthTab";
const SIGN_IN_TIMEOUT: Duration = Duration::from_secs(5 * 60);
const MAX_CLIENT_ID_CHARS: usize = 64;
/// Set at build time for published releases. The client id isn't a secret.
const BUILT_IN_CLIENT_ID: Option<&str> = option_env!("BLOXCLIENT_OAUTH_CLIENT_ID");

struct Pending {
    id: u64,
    state: Secret,
    verifier: Secret,
    client_id: String,
}

#[derive(Default)]
struct Flow {
    pending: Option<Pending>,
    error: Option<String>,
}

pub struct AuthManager {
    db: Arc<Database>,
    http: Client,
    flow: Mutex<Flow>,
    next_flow: AtomicU64,
}

fn validate_client_id(input: &str) -> Result<String, AuthError> {
    let value = input.trim();
    if value.to_ascii_uppercase().starts_with("RBX-") {
        return Err(AuthError::SecretEntered);
    }
    let valid = !value.is_empty()
        && value.len() <= MAX_CLIENT_ID_CHARS
        && value.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-');
    if valid {
        Ok(value.to_string())
    } else {
        Err(AuthError::InvalidClientId)
    }
}

impl AuthManager {
    pub fn new(db: Arc<Database>) -> Result<Self, AuthError> {
        Ok(Self {
            db,
            http: oauth::build_client()?,
            flow: Mutex::new(Flow::default()),
            next_flow: AtomicU64::new(1),
        })
    }

    fn flow(&self) -> std::sync::MutexGuard<'_, Flow> {
        self.flow.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn setting(&self, key: &str) -> AppResult<Option<String>> {
        let raw: Option<String> = self
            .db
            .connection()
            .query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| row.get(0))
            .optional()?;
        Ok(raw.and_then(|value| serde_json::from_str(&value).ok()))
    }

    fn save_setting(&self, key: &str, value: &str) -> AppResult<()> {
        self.db.connection().execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT (key) DO UPDATE SET value = excluded.value, updated_at = unixepoch()",
            params![key, serde_json::to_string(value)?],
        )?;
        Ok(())
    }

    fn client_id(&self) -> AppResult<Option<String>> {
        match BUILT_IN_CLIENT_ID {
            Some(id) if !id.is_empty() => Ok(Some(id.to_string())),
            _ => self.setting(CLIENT_ID_KEY),
        }
    }

    pub fn status(&self) -> AppResult<AuthStatus> {
        let account: Option<Account> = vault::read(&self.db.connection(), ACCOUNT_KEY)?;
        let (signing_in, error) = {
            let flow = self.flow();
            (flow.pending.is_some(), flow.error.clone())
        };
        let state = match (signing_in, &account) {
            (true, _) => AuthState::SigningIn,
            (false, Some(_)) => AuthState::Connected,
            (false, None) => AuthState::SignedOut,
        };
        Ok(AuthStatus {
            state,
            account,
            client_configured: self.client_id()?.is_some(),
            client_editable: BUILT_IN_CLIENT_ID.is_none_or(str::is_empty),
            redirect_uri: oauth::REDIRECT_URI.to_string(),
            welcome_seen: self.setting(WELCOME_KEY)?.is_some(),
            error,
        })
    }

    fn emit_status(&self, app: &AppHandle) {
        match self.status() {
            Ok(status) => {
                if let Err(error) = app.emit(AUTH_EVENT, status) {
                    log::warn!("could not send sign-in update: {error}");
                }
            }
            Err(error) => log::error!("could not read sign-in status: {error}"),
        }
    }

    pub fn set_client_id(&self, input: &str) -> AppResult<AuthStatus> {
        if BUILT_IN_CLIENT_ID.is_some_and(|id| !id.is_empty()) {
            return Err(AuthError::ClientIdLocked.into());
        }
        let client_id = validate_client_id(input)?;
        self.save_setting(CLIENT_ID_KEY, &client_id)?;
        self.flow().error = None;
        self.status()
    }

    pub fn dismiss_welcome(&self) -> AppResult<AuthStatus> {
        self.save_setting(WELCOME_KEY, "seen")?;
        self.status()
    }

    pub fn begin_sign_in(self: &Arc<Self>, app: &AppHandle) -> AppResult<AuthStatus> {
        let client_id = self.client_id()?.ok_or(AuthError::NotConfigured)?;
        let state = Secret::random()?;
        let verifier = Secret::random()?;
        let url = oauth::authorize_url(&client_id, &state, &pkce::challenge(&verifier))?;
        let id = self.next_flow.fetch_add(1, Ordering::SeqCst);

        {
            let mut flow = self.flow();
            if flow.pending.is_some() {
                return Err(AuthError::AlreadySigningIn.into());
            }
            protocol::register().map_err(|error| AuthError::Protocol(error.to_string()))?;
            flow.error = None;
            flow.pending = Some(Pending { id, state, verifier, client_id });
        }

        if let Err(error) = app.opener().open_url(url.as_str(), None::<&str>) {
            self.flow().pending = None;
            protocol::unregister();
            return Err(error.into());
        }

        let manager = Arc::clone(self);
        let handle = app.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(SIGN_IN_TIMEOUT).await;
            manager.expire(&handle, id);
        });

        self.emit_status(app);
        self.status()
    }

    fn expire(&self, app: &AppHandle, id: u64) {
        {
            let mut flow = self.flow();
            if flow.pending.as_ref().is_none_or(|pending| pending.id != id) {
                return;
            }
            flow.pending = None;
            flow.error = Some(AuthError::TimedOut.user_message());
        }
        protocol::unregister();
        self.emit_status(app);
    }

    pub fn cancel(&self, app: &AppHandle) -> AppResult<AuthStatus> {
        let was_pending = {
            let mut flow = self.flow();
            flow.error = None;
            flow.pending.take().is_some()
        };
        if was_pending {
            protocol::unregister();
        }
        self.emit_status(app);
        self.status()
    }

    /// Handles the browser's return. A link without the matching state value
    /// is ignored, so a website can't complete or disturb a sign-in.
    async fn complete(&self, app: &AppHandle, raw: &str) {
        let Ok(callback) = oauth::parse_callback(raw) else {
            log::warn!("ignored a malformed sign-in link");
            return;
        };
        let pending = {
            let mut flow = self.flow();
            let matches = flow
                .pending
                .as_ref()
                .is_some_and(|pending| pending.state.matches(&callback.state));
            if !matches {
                log::warn!("ignored a sign-in link with no matching request");
                return;
            }
            flow.pending.take()
        };
        protocol::unregister();
        let Some(pending) = pending else {
            return;
        };

        let result = self.finish(callback.result, &pending).await;
        if let Err(error) = &result {
            log::warn!("sign-in failed: {error}");
        } else {
            log::info!("Roblox account connected");
        }
        self.flow().error = result.err().map(|error| error.user_message());
        self.emit_status(app);
    }

    async fn finish(&self, result: CallbackResult, pending: &Pending) -> AppResult<()> {
        let code = match result {
            CallbackResult::Code(code) => code,
            CallbackResult::Denied => return Err(AuthError::Denied.into()),
            CallbackResult::Failed(reason) => return Err(AuthError::Authorization(reason).into()),
        };

        let tokens = oauth::exchange_code(&self.http, &pending.client_id, &code, &pending.verifier).await?;
        let account = oauth::fetch_account(&self.http, &tokens.access).await;

        // Tokens are finished with here, whether or not the profile loaded.
        if let Some(refresh) = &tokens.refresh {
            if !oauth::revoke(&self.http, &pending.client_id, refresh).await {
                log::warn!("Roblox did not confirm token revocation");
            }
        }
        drop(tokens);

        vault::write(&self.db.connection(), ACCOUNT_KEY, &account?)?;
        self.save_setting(WELCOME_KEY, "seen")?;
        Ok(())
    }

    /// Opens Roblox's OAuth app page for first-time setup.
    pub fn open_dashboard(&self, app: &AppHandle) -> AppResult<()> {
        app.opener().open_url(OAUTH_DASHBOARD, None::<&str>)?;
        Ok(())
    }

    pub fn sign_out(&self, app: &AppHandle) -> AppResult<AuthStatus> {
        let was_pending = {
            let mut flow = self.flow();
            flow.error = None;
            flow.pending.take().is_some()
        };
        if was_pending {
            protocol::unregister();
        }
        vault::remove(&self.db.connection(), ACCOUNT_KEY)?;
        log::info!("Roblox account disconnected");
        self.emit_status(app);
        self.status()
    }
}

/// Picks up the browser's return when Windows relaunches BloxClient.
pub fn handle_launch_args(app: &AppHandle, args: &[String]) {
    let Some(url) = args.iter().find(|arg| oauth::is_callback(arg)).cloned() else {
        return;
    };
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let manager = Arc::clone(&state.auth);
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        manager.complete(&handle, &url).await;
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manager() -> AuthManager {
        AuthManager::new(Arc::new(Database::in_memory().unwrap())).unwrap()
    }

    #[test]
    fn validates_client_ids() {
        assert_eq!(validate_client_id(" 3516920387 ").unwrap(), "3516920387");
        assert!(matches!(validate_client_id("RBX-abc123"), Err(AuthError::SecretEntered)));
        assert!(matches!(validate_client_id("rbx-abc123"), Err(AuthError::SecretEntered)));
        for bad in ["", "   ", "has space", "a&b=c", "x/y"] {
            assert!(matches!(validate_client_id(bad), Err(AuthError::InvalidClientId)), "accepted {bad:?}");
        }
        assert!(validate_client_id(&"1".repeat(MAX_CLIENT_ID_CHARS + 1)).is_err());
    }

    #[test]
    fn starts_signed_out() {
        let status = manager().status().unwrap();
        assert_eq!(status.state, AuthState::SignedOut);
        assert_eq!(status.account, None);
        assert!(!status.welcome_seen);
        assert_eq!(status.redirect_uri, "bloxclient://oauth/callback");
    }

    #[test]
    fn saves_client_id_and_welcome_state() {
        let manager = manager();
        if BUILT_IN_CLIENT_ID.is_none() {
            assert!(manager.set_client_id("3516920387").unwrap().client_configured);
        }
        assert!(manager.dismiss_welcome().unwrap().welcome_seen);
    }

    #[test]
    fn saved_account_is_encrypted_and_shows_connected() {
        let manager = manager();
        let account = Account {
            user_id: "42".into(),
            username: "PrivateName".into(),
            display_name: "Display".into(),
            avatar_url: None,
        };
        vault::write(&manager.db.connection(), ACCOUNT_KEY, &account).unwrap();

        let status = manager.status().unwrap();
        assert_eq!(status.state, AuthState::Connected);
        assert_eq!(status.account, Some(account));
        let raw: Vec<u8> = manager
            .db
            .connection()
            .query_row("SELECT value FROM vault WHERE key = ?1", [ACCOUNT_KEY], |row| row.get(0))
            .unwrap();
        assert!(!raw.windows(11).any(|window| window == b"PrivateName"));
    }
}

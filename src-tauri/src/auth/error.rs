/// Sign-in errors. Variants never carry tokens, codes or response bodies.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("no OAuth client id is configured")]
    NotConfigured,
    #[error("invalid OAuth client id")]
    InvalidClientId,
    #[error("the app secret was entered instead of the client id")]
    SecretEntered,
    #[error("this build has a built-in client id")]
    ClientIdLocked,
    #[error("a sign-in is already in progress")]
    AlreadySigningIn,
    #[error("sign-in callback was not valid")]
    InvalidCallback,
    #[error("the user declined access")]
    Denied,
    #[error("authorization error from Roblox: {0}")]
    Authorization(String),
    #[error("sign-in timed out")]
    TimedOut,
    #[error("sign-in code was rejected")]
    CodeRejected,
    #[error("Roblox rejected the app's OAuth settings: {0}")]
    ClientRejected(String),
    #[error("network error: {0}")]
    Network(String),
    #[error("unexpected response: {0}")]
    Response(String),
    #[error("secure random generator failed: {0}")]
    Random(String),
    #[error("could not set up the sign-in link: {0}")]
    Protocol(String),
}

impl AuthError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::NotConfigured => "auth_not_configured",
            Self::InvalidClientId => "auth_invalid_client_id",
            Self::SecretEntered => "auth_secret_entered",
            Self::ClientIdLocked => "auth_client_locked",
            Self::AlreadySigningIn => "auth_in_progress",
            Self::InvalidCallback => "auth_invalid_callback",
            Self::Denied => "auth_denied",
            Self::Authorization(_) => "auth_authorization",
            Self::TimedOut => "auth_timed_out",
            Self::CodeRejected => "auth_code_rejected",
            Self::ClientRejected(_) => "auth_client_rejected",
            Self::Network(_) => "auth_network",
            Self::Response(_) => "auth_response",
            Self::Random(_) => "auth_random",
            Self::Protocol(_) => "auth_protocol",
        }
    }

    pub fn user_message(&self) -> String {
        match self {
            Self::NotConfigured => "Sign-in isn't set up yet. Add your Roblox OAuth client ID first.".into(),
            Self::InvalidClientId => {
                "That doesn't look like a client ID. Copy it from your OAuth app in Creator Dashboard.".into()
            }
            Self::SecretEntered => "That's the app secret, which BloxClient must not store. Enter the client ID.".into(),
            Self::ClientIdLocked => "This version of BloxClient has its client ID built in.".into(),
            Self::AlreadySigningIn => "A sign-in is already open in your browser.".into(),
            Self::InvalidCallback => "That sign-in link wasn't valid. Start signing in again from BloxClient.".into(),
            Self::Denied => "Sign-in was cancelled on the Roblox page. No account was connected.".into(),
            Self::Authorization(_) => {
                "Roblox couldn't start sign-in. Check the app's redirect URL and scopes in Creator Dashboard.".into()
            }
            Self::TimedOut => "Sign-in timed out. Start again when you're ready.".into(),
            Self::CodeRejected => "That sign-in expired before it finished. Try signing in again.".into(),
            Self::ClientRejected(_) => {
                "Roblox rejected this app's sign-in settings. Check the client ID and redirect URL in Creator Dashboard.".into()
            }
            Self::Network(_) => "Roblox couldn't be reached. Check your connection and try again.".into(),
            Self::Response(_) => "Roblox sent a sign-in response BloxClient couldn't read.".into(),
            Self::Random(_) => "Windows couldn't generate a secure sign-in request. Try again.".into(),
            Self::Protocol(_) => "BloxClient couldn't set up the link that returns you from the browser.".into(),
        }
    }
}

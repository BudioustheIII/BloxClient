//! Roblox OAuth 2.0 authorization code flow with PKCE. BloxClient is a public
//! client: it has no app secret, and it requests identity scopes only.

use std::time::Duration;

use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use reqwest::redirect::Policy;
use reqwest::{Client, Response, StatusCode, Url};
use serde::de::DeserializeOwned;
use serde::Deserialize;

use super::error::AuthError;
use super::model::Account;
use super::pkce::Secret;

pub const REDIRECT_URI: &str = "bloxclient://oauth/callback";
const AUTHORIZE: &str = "https://apis.roblox.com/oauth/v1/authorize";
const TOKEN: &str = "https://apis.roblox.com/oauth/v1/token";
const REVOKE: &str = "https://apis.roblox.com/oauth/v1/token/revoke";
const USER_INFO: &str = "https://apis.roblox.com/oauth/v1/userinfo";
const SCOPES: &str = "openid profile";
const FORM_TYPE: &str = "application/x-www-form-urlencoded";
const MAX_RESPONSE_BYTES: usize = 128 * 1024;
const MAX_ERROR_CODE_CHARS: usize = 64;
const TRUSTED_AVATAR_PREFIX: &str = "https://tr.rbxcdn.com/";

pub enum CallbackResult {
    Code(Secret),
    Denied,
    Failed(String),
}

pub struct Callback {
    pub state: String,
    pub result: CallbackResult,
}

/// Tokens exist only for the few seconds it takes to read the profile.
pub struct Tokens {
    pub access: Secret,
    pub refresh: Option<Secret>,
}

#[derive(Deserialize)]
struct TokenBody {
    access_token: String,
    refresh_token: Option<String>,
}

#[derive(Deserialize)]
struct ErrorBody {
    error: Option<String>,
}

#[derive(Deserialize)]
struct UserInfo {
    sub: String,
    name: Option<String>,
    nickname: Option<String>,
    preferred_username: Option<String>,
    picture: Option<String>,
}

fn network(error: reqwest::Error) -> AuthError {
    AuthError::Network(error.without_url().to_string())
}

pub fn build_client() -> Result<Client, AuthError> {
    Client::builder()
        .user_agent(concat!("BloxClient/", env!("CARGO_PKG_VERSION")))
        .https_only(true)
        .redirect(Policy::none())
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(network)
}

pub fn authorize_url(client_id: &str, state: &Secret, challenge: &str) -> Result<Url, AuthError> {
    Url::parse_with_params(
        AUTHORIZE,
        &[
            ("client_id", client_id),
            ("redirect_uri", REDIRECT_URI),
            ("scope", SCOPES),
            ("response_type", "code"),
            ("code_challenge", challenge),
            ("code_challenge_method", "S256"),
            ("state", state.expose()),
        ],
    )
    .map_err(|_| AuthError::InvalidClientId)
}

pub fn is_callback(argument: &str) -> bool {
    argument.starts_with("bloxclient://oauth/callback?")
}

fn safe_error_code(value: &str) -> String {
    value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .take(MAX_ERROR_CODE_CHARS)
        .collect()
}

pub fn parse_callback(raw: &str) -> Result<Callback, AuthError> {
    let url = Url::parse(raw).map_err(|_| AuthError::InvalidCallback)?;
    if url.scheme() != "bloxclient" || url.host_str() != Some("oauth") || url.path() != "/callback" {
        return Err(AuthError::InvalidCallback);
    }
    let value = |name: &str| {
        url.query_pairs()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.into_owned())
    };

    let state = value("state").filter(|state| !state.is_empty()).ok_or(AuthError::InvalidCallback)?;
    let result = match (value("error"), value("code")) {
        (Some(error), _) if error == "access_denied" => CallbackResult::Denied,
        (Some(error), _) => CallbackResult::Failed(safe_error_code(&error)),
        (None, Some(code)) if !code.is_empty() => CallbackResult::Code(Secret::new(code)),
        _ => return Err(AuthError::InvalidCallback),
    };
    Ok(Callback { state, result })
}

fn form_body(pairs: &[(&str, &str)]) -> Result<String, AuthError> {
    let url = Url::parse_with_params("https://form.invalid/", pairs).map_err(|_| AuthError::InvalidClientId)?;
    Ok(url.query().unwrap_or_default().to_string())
}

async fn read_body(mut response: Response) -> Result<Vec<u8>, AuthError> {
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(network)? {
        if body.len() + chunk.len() > MAX_RESPONSE_BYTES {
            return Err(AuthError::Response("response too large".into()));
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

fn classify_failure(status: StatusCode, body: &[u8]) -> AuthError {
    let code = serde_json::from_slice::<ErrorBody>(body)
        .ok()
        .and_then(|body| body.error)
        .map(|code| safe_error_code(&code))
        .unwrap_or_default();
    match code.as_str() {
        "invalid_grant" => AuthError::CodeRejected,
        "invalid_client" | "unauthorized_client" | "invalid_request" | "invalid_scope" => {
            AuthError::ClientRejected(code)
        }
        _ if status.is_server_error() || status == StatusCode::TOO_MANY_REQUESTS => {
            AuthError::Network(format!("HTTP {status}"))
        }
        _ if status == StatusCode::UNAUTHORIZED => AuthError::ClientRejected("unauthorized".into()),
        _ => AuthError::Response(format!("HTTP {status}")),
    }
}

async fn parse_json<T: DeserializeOwned>(response: Response) -> Result<T, AuthError> {
    let status = response.status();
    let body = read_body(response).await?;
    if !status.is_success() {
        return Err(classify_failure(status, &body));
    }
    serde_json::from_slice(&body).map_err(|_| AuthError::Response("unreadable JSON".into()))
}

pub async fn exchange_code(
    http: &Client,
    client_id: &str,
    code: &Secret,
    verifier: &Secret,
) -> Result<Tokens, AuthError> {
    let body = form_body(&[
        ("grant_type", "authorization_code"),
        ("code", code.expose()),
        ("code_verifier", verifier.expose()),
        ("client_id", client_id),
    ])?;
    let response = http
        .post(TOKEN)
        .header(CONTENT_TYPE, FORM_TYPE)
        .body(body)
        .send()
        .await
        .map_err(network)?;
    let tokens: TokenBody = parse_json(response).await?;
    Ok(Tokens {
        access: Secret::new(tokens.access_token),
        refresh: tokens.refresh_token.map(Secret::new),
    })
}

fn to_account(info: UserInfo) -> Result<Account, AuthError> {
    if info.sub.is_empty() || !info.sub.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(AuthError::Response("invalid user id".into()));
    }
    let username = info
        .preferred_username
        .filter(|name| !name.is_empty())
        .ok_or_else(|| AuthError::Response("missing username".into()))?;
    let display_name = info
        .name
        .or(info.nickname)
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| username.clone());
    Ok(Account {
        user_id: info.sub,
        username,
        display_name,
        avatar_url: info.picture.filter(|url| url.starts_with(TRUSTED_AVATAR_PREFIX)),
    })
}

pub async fn fetch_account(http: &Client, access: &Secret) -> Result<Account, AuthError> {
    let response = http
        .get(USER_INFO)
        .header(AUTHORIZATION, format!("Bearer {}", access.expose()))
        .send()
        .await
        .map_err(network)?;
    to_account(parse_json(response).await?)
}

/// Asks Roblox to end this grant. The token is discarded locally either way.
pub async fn revoke(http: &Client, client_id: &str, refresh: &Secret) -> bool {
    let Ok(body) = form_body(&[("token", refresh.expose()), ("client_id", client_id)]) else {
        return false;
    };
    match http.post(REVOKE).header(CONTENT_TYPE, FORM_TYPE).body(body).send().await {
        Ok(response) => response.status().is_success(),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(sub: &str) -> UserInfo {
        UserInfo {
            sub: sub.to_string(),
            name: Some("Display".to_string()),
            nickname: None,
            preferred_username: Some("username".to_string()),
            picture: Some("https://tr.rbxcdn.com/avatar".to_string()),
        }
    }

    #[test]
    fn authorize_url_has_pkce_and_only_identity_scopes() {
        let url = authorize_url("123456", &Secret::new("state-value"), "challenge-value").unwrap();
        let pairs: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
        assert_eq!(pairs["client_id"], "123456");
        assert_eq!(pairs["redirect_uri"], REDIRECT_URI);
        assert_eq!(pairs["scope"], "openid profile");
        assert_eq!(pairs["response_type"], "code");
        assert_eq!(pairs["code_challenge"], "challenge-value");
        assert_eq!(pairs["code_challenge_method"], "S256");
        assert_eq!(pairs["state"], "state-value");
        assert!(!pairs.contains_key("client_secret"));
    }

    #[test]
    fn parses_a_successful_callback() {
        let callback = parse_callback("bloxclient://oauth/callback?code=abc&state=xyz").unwrap();
        assert_eq!(callback.state, "xyz");
        assert!(matches!(callback.result, CallbackResult::Code(code) if code.expose() == "abc"));
    }

    #[test]
    fn parses_denied_and_failed_callbacks() {
        let denied = parse_callback("bloxclient://oauth/callback?error=access_denied&state=x").unwrap();
        assert!(matches!(denied.result, CallbackResult::Denied));
        let failed = parse_callback("bloxclient://oauth/callback?error=bad%3Cscript%3E&state=x").unwrap();
        assert!(matches!(failed.result, CallbackResult::Failed(code) if code == "badscript"));
    }

    #[test]
    fn rejects_forged_or_malformed_callbacks() {
        for raw in [
            "not a url",
            "https://oauth/callback?code=a&state=b",
            "bloxclient://evil/callback?code=a&state=b",
            "bloxclient://oauth/other?code=a&state=b",
            "bloxclient://oauth/callback?code=a",
            "bloxclient://oauth/callback?state=b",
            "bloxclient://oauth/callback?code=&state=b",
            "bloxclient://oauth/callback?code=a&state=",
        ] {
            assert!(parse_callback(raw).is_err(), "accepted {raw}");
        }
        assert!(!is_callback("bloxclient://oauth/other?code=a"));
        assert!(!is_callback("--some-flag"));
    }

    #[test]
    fn form_values_are_encoded() {
        assert_eq!(form_body(&[("code", "a&b=c d")]).unwrap(), "code=a%26b%3Dc+d");
    }

    #[test]
    fn classifies_token_errors() {
        let classify = |status, body: &str| classify_failure(status, body.as_bytes());
        assert!(matches!(classify(StatusCode::BAD_REQUEST, r#"{"error":"invalid_grant"}"#), AuthError::CodeRejected));
        assert!(matches!(classify(StatusCode::BAD_REQUEST, r#"{"error":"invalid_client"}"#), AuthError::ClientRejected(_)));
        assert!(matches!(classify(StatusCode::UNAUTHORIZED, "nope"), AuthError::ClientRejected(_)));
        assert!(matches!(classify(StatusCode::BAD_GATEWAY, ""), AuthError::Network(_)));
        assert!(matches!(classify(StatusCode::IM_A_TEAPOT, "secret body"), AuthError::Response(detail) if !detail.contains("secret")));
    }

    #[test]
    fn builds_an_account_from_user_info() {
        let account = to_account(info("42")).unwrap();
        assert_eq!(account.user_id, "42");
        assert_eq!(account.username, "username");
        assert_eq!(account.display_name, "Display");
        assert!(account.avatar_url.is_some());
    }

    #[test]
    fn rejects_bad_identity_and_untrusted_avatars() {
        assert!(to_account(info("not-a-number")).is_err());
        let mut no_username = info("42");
        no_username.preferred_username = None;
        assert!(to_account(no_username).is_err());
        let mut bad_avatar = info("42");
        bad_avatar.picture = Some("https://evil.example/a.png".to_string());
        assert_eq!(to_account(bad_avatar).unwrap().avatar_url, None);
    }
}

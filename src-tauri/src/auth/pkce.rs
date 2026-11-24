//! PKCE values and a wrapper that keeps secrets out of logs.

use std::fmt;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use sha2::{Digest, Sha256};

use super::error::AuthError;

const RANDOM_BYTES: usize = 48;

/// A token, code or verifier. Debug output is always redacted.
pub struct Secret(String);

impl Secret {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// 64 URL-safe characters from the OS secure random generator.
    pub fn random() -> Result<Self, AuthError> {
        let mut bytes = [0u8; RANDOM_BYTES];
        getrandom::fill(&mut bytes).map_err(|error| AuthError::Random(error.to_string()))?;
        Ok(Self(URL_SAFE_NO_PAD.encode(bytes)))
    }

    pub fn expose(&self) -> &str {
        &self.0
    }

    /// Compares without stopping at the first differing byte.
    pub fn matches(&self, other: &str) -> bool {
        let (left, right) = (self.0.as_bytes(), other.as_bytes());
        left.len() == right.len() && left.iter().zip(right).fold(0u8, |diff, (a, b)| diff | (a ^ b)) == 0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("[redacted]")
    }
}

pub fn challenge(verifier: &Secret) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.expose().as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_the_rfc_7636_test_vector() {
        let verifier = Secret::new("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk");
        assert_eq!(challenge(&verifier), "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
    }

    #[test]
    fn random_values_meet_the_pkce_format_and_differ() {
        let first = Secret::random().unwrap();
        let second = Secret::random().unwrap();
        assert!((43..=128).contains(&first.expose().len()));
        assert!(first
            .expose()
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_')));
        assert!(!first.matches(second.expose()));
    }

    #[test]
    fn debug_output_never_contains_the_value() {
        let secret = Secret::new("super-secret-token");
        assert_eq!(format!("{secret:?}"), "[redacted]");
        assert!(!format!("{:?}", Some(&secret)).contains("super-secret"));
    }

    #[test]
    fn comparison_requires_an_exact_match() {
        let secret = Secret::new("abc");
        assert!(secret.matches("abc"));
        assert!(!secret.matches("abd"));
        assert!(!secret.matches("ab"));
        assert!(!secret.matches("abcd"));
    }
}

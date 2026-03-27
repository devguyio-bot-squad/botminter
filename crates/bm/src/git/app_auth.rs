use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// JWT claims for GitHub App authentication.
///
/// Per GitHub docs, the `iss` field should be the App's Client ID (recommended)
/// or App ID. The `iat` is backdated 60 seconds to account for clock drift.
/// The `exp` is set to 10 minutes (600 seconds) from now — the maximum allowed.
#[derive(Debug, Serialize)]
struct Claims {
    iss: String,
    iat: usize,
    exp: usize,
}

/// An installation access token returned by the GitHub API.
///
/// Installation tokens grant repository-level access and expire after 1 hour.
/// The daemon refreshes these at the 50-minute mark.
#[derive(Debug, Clone, Deserialize)]
pub struct InstallationToken {
    pub token: String,
    pub expires_at: DateTime<Utc>,
}

/// Generates a JWT for authenticating as a GitHub App.
///
/// The JWT is signed with RS256 using the App's private key PEM.
/// Claims: `iss` = Client ID, `iat` = now - 60s, `exp` = now + 600s.
pub fn generate_jwt(client_id: &str, private_key_pem: &str) -> Result<String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("System clock before UNIX epoch")?
        .as_secs() as usize;

    let claims = Claims {
        iss: client_id.to_string(),
        iat: now - 60,
        exp: now + 600,
    };

    let header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
    let key = jsonwebtoken::EncodingKey::from_rsa_pem(private_key_pem.as_bytes())
        .context("Invalid RSA private key PEM")?;

    jsonwebtoken::encode(&header, &claims, &key).context("Failed to encode JWT")
}

/// Exchanges a JWT for an installation access token.
///
/// Calls `POST /app/installations/{installation_id}/access_tokens` with the JWT
/// in the Authorization header. Returns the token and its expiration time.
///
/// Note: Installation tokens MUST NOT be validated via `/user` (returns 403).
/// Trust the exchange response.
pub fn exchange_for_installation_token(
    jwt: &str,
    installation_id: u64,
) -> Result<InstallationToken> {
    let url = format!(
        "https://api.github.com/app/installations/{installation_id}/access_tokens"
    );

    let client = reqwest::blocking::Client::new();
    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {jwt}"))
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "botminter")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .context("Failed to call GitHub installation token endpoint")?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().unwrap_or_default();
        anyhow::bail!(
            "GitHub API returned {status} when exchanging JWT for installation token: {body}"
        );
    }

    response
        .json::<InstallationToken>()
        .context("Failed to parse installation token response")
}

/// Uninstalls a GitHub App installation.
///
/// Calls `DELETE /app/installations/{installation_id}` with the JWT
/// in the Authorization header. Returns Ok(()) on success (204 No Content).
/// This removes the App's access to the organization but does NOT delete
/// the App itself — that must be done manually via GitHub UI.
pub fn uninstall_app(jwt: &str, installation_id: u64) -> Result<()> {
    let url = format!("https://api.github.com/app/installations/{installation_id}");

    let client = reqwest::blocking::Client::new();
    let response = client
        .delete(&url)
        .header("Authorization", format!("Bearer {jwt}"))
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "botminter")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .context("Failed to call GitHub App uninstallation endpoint")?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().unwrap_or_default();
        anyhow::bail!(
            "GitHub API returned {status} when uninstalling App installation: {body}"
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Load the test-only RSA private key from a fixture file.
    /// This key is NOT used in production — it exists solely to verify JWT signing logic.
    /// Kept in a separate file to avoid triggering GitHub secret scanning on the source.
    fn test_rsa_pem() -> String {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("test-fixtures")
            .join("test-only.pem");
        std::fs::read_to_string(&fixture)
            .unwrap_or_else(|e| panic!("Failed to read test PEM fixture {}: {e}", fixture.display()))
    }

    /// Decodes a base64url-encoded JWT segment into a JSON value.
    /// Test-only: uses a minimal base64 decoder to avoid adding a dependency.
    fn decode_jwt_part(segment: &str) -> serde_json::Value {
        // base64url → standard base64
        let mut b64 = segment.replace('-', "+").replace('_', "/");
        match b64.len() % 4 {
            2 => b64.push_str("=="),
            3 => b64.push('='),
            _ => {}
        }

        const TABLE: &[u8] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut output = Vec::new();
        let mut buf: u32 = 0;
        let mut bits: u32 = 0;
        for byte in b64.bytes() {
            if byte == b'=' {
                break;
            }
            let val = TABLE.iter().position(|&b| b == byte).expect("invalid base64") as u32;
            buf = (buf << 6) | val;
            bits += 6;
            if bits >= 8 {
                bits -= 8;
                output.push((buf >> bits) as u8);
                buf &= (1 << bits) - 1;
            }
        }
        serde_json::from_slice(&output).expect("JWT segment should be valid JSON")
    }

    #[test]
    fn generate_jwt_produces_valid_structure() {
        let pem = test_rsa_pem();
        let jwt = generate_jwt("Iv1.test_client_id", &pem).unwrap();

        // JWT has three dot-separated parts: header.payload.signature
        let parts: Vec<&str> = jwt.split('.').collect();
        assert_eq!(parts.len(), 3, "JWT must have 3 parts");

        // Decode the JWT header and payload manually (base64url → JSON).
        // We skip signature verification here — the RSA crypto is jsonwebtoken's
        // responsibility. We're testing that *our* claims are correct.
        let header_json = decode_jwt_part(parts[0]);
        let payload_json = decode_jwt_part(parts[1]);

        assert_eq!(header_json["alg"], "RS256");
        assert_eq!(header_json["typ"], "JWT");
        assert_eq!(payload_json["iss"], "Iv1.test_client_id");

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let iat = payload_json["iat"].as_i64().unwrap();
        let exp = payload_json["exp"].as_i64().unwrap();

        // iat should be approximately now - 60
        assert!(
            (iat - (now - 60)).abs() < 5,
            "iat should be ~now-60, got iat={iat}, now={now}"
        );
        // exp should be approximately now + 600
        assert!(
            (exp - (now + 600)).abs() < 5,
            "exp should be ~now+600, got exp={exp}, now={now}"
        );
    }

    #[test]
    fn generate_jwt_rejects_invalid_pem() {
        let result = generate_jwt("Iv1.test", "not-a-valid-pem");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Invalid RSA private key PEM"),
            "Error should mention invalid PEM, got: {err}"
        );
    }

    #[test]
    fn installation_token_deserializes() {
        let json = r#"{
            "token": "ghs_test_token_123",
            "expires_at": "2024-01-01T01:00:00Z",
            "permissions": {"issues": "write", "contents": "read"},
            "repository_selection": "all"
        }"#;

        let token: InstallationToken = serde_json::from_str(json).unwrap();
        assert_eq!(token.token, "ghs_test_token_123");
        assert_eq!(
            token.expires_at,
            "2024-01-01T01:00:00Z".parse::<DateTime<Utc>>().unwrap()
        );
    }

    #[test]
    fn installation_token_rejects_missing_fields() {
        // Missing expires_at
        let json = r#"{"token": "ghs_test"}"#;
        let result = serde_json::from_str::<InstallationToken>(json);
        assert!(result.is_err());

        // Missing token
        let json = r#"{"expires_at": "2024-01-01T01:00:00Z"}"#;
        let result = serde_json::from_str::<InstallationToken>(json);
        assert!(result.is_err());
    }
}

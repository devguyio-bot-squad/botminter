# Research: JWT Signing & App Lifecycle API

## JWT Authentication for GitHub Apps

### Required Claims

| Claim | Value | Notes |
|-------|-------|-------|
| `iss` | App's **Client ID** (recommended) or App ID | Client ID recommended per current docs |
| `iat` | Current Unix timestamp - 60s | 60s backdate for clock drift |
| `exp` | Current Unix timestamp + 600s max | Max 10 minutes TTL |

### Algorithm

RS256 (RSA with SHA-256) only. Header: `{"alg": "RS256", "typ": "JWT"}`.

### Key Endpoints (JWT-authenticated)

| Endpoint | Purpose |
|----------|---------|
| `GET /app` | Verify JWT / get App info |
| `GET /app/installations` | List all installations |
| `GET /repos/{owner}/{repo}/installation` | Get installation ID for a repo |
| `POST /app/installations/{id}/access_tokens` | Generate installation token (1hr TTL) |

### `jsonwebtoken` Crate (v10.3.0)

Requires feature selection:
```toml
jsonwebtoken = { version = "10", features = ["aws_lc_rs", "use_pem"] }
```

`EncodingKey::from_rsa_pem(&[u8])` exists (requires `use_pem` feature).

Example:
```rust
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};

#[derive(Serialize)]
struct Claims {
    iss: String,  // Client ID
    iat: usize,   // now - 60
    exp: usize,   // now + 600
}

fn generate_jwt(client_id: &str, pem: &[u8]) -> Result<String> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as usize;
    let claims = Claims {
        iss: client_id.to_string(),
        iat: now - 60,
        exp: now + 600,
    };
    encode(&Header::new(Algorithm::RS256), &claims, &EncodingKey::from_rsa_pem(pem)?)
}
```

## App Lifecycle API

### App Deletion

**There is NO REST API to delete a GitHub App registration.** Deletion is UI-only:
Settings -> Developer Settings -> GitHub Apps -> select -> Advanced -> Delete.

### Installation Management

| Endpoint | Purpose | Auth |
|----------|---------|------|
| `DELETE /app/installations/{id}` | Uninstall App from account | JWT (as App) |
| `PUT /app/installations/{id}/suspended` | Suspend installation | JWT |

### Implications for `bm fire`

- `bm fire` can **uninstall** the App (remove from repos) and clean up keyring
- `bm fire` CANNOT delete the App registration — it remains as a shell on GitHub
- The `--keep-app` flag becomes the default/only behavior for the App itself
- Print instructions for manual App deletion via GitHub UI

## Sources

- https://docs.github.com/en/apps/creating-github-apps/authenticating-with-a-github-app/generating-a-json-web-token-jwt-for-a-github-app
- https://docs.github.com/en/rest/apps/apps
- https://docs.github.com/en/apps/maintaining-github-apps/deleting-a-github-app
- https://docs.rs/jsonwebtoken/latest/jsonwebtoken/

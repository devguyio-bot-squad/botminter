# Research: GitHub App Manifest Flow

## Overview

The manifest flow creates a GitHub App from a JSON manifest with one-click user confirmation. Three steps: form POST -> user confirms -> code exchange.

## Endpoints

### Step 1: Form POST to GitHub

**Personal account:**
```
POST https://github.com/settings/apps/new?state={state}
```

**Organization:**
```
POST https://github.com/organizations/{org}/settings/apps/new?state={state}
```

The manifest JSON is a form field (`name="manifest"` in POST body). The `state` param is on the URL query string for CSRF protection.

### Step 2: User Confirms

User sees pre-filled App creation page, clicks "Create GitHub App". GitHub redirects to `redirect_url`:
```
{redirect_url}?code={code}&state={state}
```

### Step 3: Code Exchange

```
POST /app-manifests/{code}/conversions
```

**No authentication required.** The code itself is the credential.

**Response (201 Created):**
```json
{
  "id": 123,
  "slug": "my-app",
  "client_id": "Iv1.abc123",
  "client_secret": "...",
  "pem": "-----BEGIN RSA PRIVATE KEY-----\n...",
  "webhook_secret": "...",
  "permissions": { ... },
  "owner": { "login": "...", "type": "Organization" }
}
```

Code is single-use, expires after 1 hour. Rate limit: 5,000/hr on conversion endpoint.

## Manifest JSON Schema

### Required Fields
- `url` (string) -- homepage URL

### Key Optional Fields

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | App name (globally unique, shared namespace with orgs) |
| `redirect_url` | string | Callback URL for code delivery |
| `default_permissions` | object | Permission scopes |
| `default_events` | array | Webhook events |
| `hook_attributes` | object | Webhook URL config |
| `public` | boolean | Publicly installable? |

### Permissions for BotMinter

```json
"default_permissions": {
  "issues": "write",
  "contents": "write",
  "pull_requests": "write",
  "organization_projects": "admin"
}
```

**CRITICAL:** `projects` only covers classic project boards (deprecated). For Projects v2, use `organization_projects`. This is a correction from the initial spec which said `projects:admin`.

## CLI Implementation Pattern (Two-Server)

1. Serve `http://127.0.0.1:{port}/start` with auto-submitting HTML form containing manifest JSON
2. Set `redirect_url` in manifest to `http://127.0.0.1:{port}/callback`
3. Open browser to local start page (or print URL for headless)
4. Form auto-submits to GitHub -> user clicks "Create" -> GitHub redirects to callback
5. Capture code, exchange via `POST /app-manifests/{code}/conversions`
6. Show success page, shut down server

### Headless Fallback

For environments without a browser:
- Print the localhost URL and instruct operator to open manually
- The local server still handles the callback
- For truly headless (CI): use `--app-id`, `--private-key-file`, `--installation-id` flags

## Name Collisions

- App names are globally unique
- **Shared namespace with GitHub organizations** -- a name is rejected if an org with that name exists
- Collision detection: `GET https://github.com/apps/{slug}` returns 200 if taken
- Mitigation: use `{team}-{member}` naming, detect collisions, suggest alternatives

## Sources

- https://docs.github.com/en/apps/sharing-github-apps/registering-a-github-app-from-a-manifest
- https://docs.github.com/en/rest/apps/apps#create-a-github-app-from-a-manifest
- https://docs.github.com/en/apps/creating-github-apps/registering-a-github-app/choosing-permissions-for-a-github-app

# Research: Manifest Flow Installation Gap

## Finding

The GitHub App Manifest flow only **creates** the App. Installation is a completely separate step requiring additional browser interaction. There is NO REST API to create an installation programmatically.

## Corrected Flow (Probot Pattern)

The CLI flow requires two browser clicks (create + install), not one:

```
1. Local server serves /start         → auto-submitting form with manifest JSON
2. GitHub App creation page           → user clicks "Create GitHub App"
3. GitHub redirects to /callback      → local server exchanges code for credentials
4. Local server redirects browser     → to {html_url}/installations/new
5. GitHub App installation page       → user clicks "Install" on org, selects repos
6. GitHub redirects to setup_url      → local server receives installation event
7. Local server queries installations → GET /app/installations (JWT auth) → gets installation_id
8. Local server shuts down            → all credentials stored
```

## Key Details

### Manifest JSON must include `setup_url`

```json
{
  "name": "{team}-{member}",
  "url": "https://github.com/{org}/{team-repo}",
  "redirect_url": "http://127.0.0.1:{port}/callback",
  "setup_url": "http://127.0.0.1:{port}/installed",
  "default_permissions": { ... },
  "default_events": [],
  "public": false
}
```

- `redirect_url` — receives the `code` after App creation
- `setup_url` — receives the redirect after App installation (separate step)

### Code exchange does NOT return installation_id

`POST /app-manifests/{code}/conversions` returns:
- `id` (App ID)
- `client_id`
- `pem` (private key)
- `client_secret`
- `webhook_secret`
- `html_url` (e.g., `https://github.com/apps/my-team-superman`)

It does NOT return an installation ID because the App isn't installed yet.

### Installation requires browser interaction

- No REST API to create an installation
- User must visit `{html_url}/installations/new` and click "Install"
- After installation, GitHub redirects to `setup_url` if configured
- The local server then queries `GET /app/installations` (JWT-authenticated) to get the installation ID

### `GET /repos/{owner}/{repo}/installation` returns 404 before installation

This endpoint only works AFTER the App is installed on the repo. Cannot be used to "find" an installation that doesn't exist yet.

### `GET /app/installations` returns empty array before installation

Returns `[]` before anyone installs the App. After installation, returns the installation object with `id`, `account`, `repository_selection`, etc.

## Impact on Design

### Previous assumption (WRONG)
> One browser click: create App → get all credentials including installation_id

### Corrected flow
> Two browser clicks: create App (click 1) → install App on org (click 2) → query for installation_id

### Changes needed

1. **Manifest JSON** — add `setup_url` field
2. **Local server** — handle two callbacks: `/callback` (code exchange) and `/installed` (installation completion)
3. **Browser flow** — after code exchange, redirect browser to `{html_url}/installations/new`
4. **Installation ID retrieval** — after `/installed` callback, sign JWT and query `GET /app/installations`
5. **ADR-0011** — update to describe two-click flow
6. **Sprint 3 plan** — update Step 2 (manifest flow) with corrected sequence
7. **Headless fallback** — print TWO URLs if browser can't open (create URL + install URL after first callback)

### Unaffected paths

- `--reuse-app` with pre-generated credentials — bypasses entire browser flow
- Token lifecycle (JWT signing, installation token exchange) — unchanged
- Token delivery (`hosts.yml`) — unchanged

## Source

- Probot source: `src/apps/setup.ts` — redirects to `{html_url}/installations/new` after code exchange
- GitHub Docs: https://docs.github.com/en/apps/sharing-github-apps/registering-a-github-app-from-a-manifest
- GitHub Docs: https://docs.github.com/en/apps/using-github-apps/installing-your-own-github-app
- GitHub Docs: https://docs.github.com/en/rest/apps/installations

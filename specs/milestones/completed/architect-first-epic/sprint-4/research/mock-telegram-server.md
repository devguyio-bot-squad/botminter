# Research: Mock Telegram Server

## Context

Ralph's `telegram.api_url` / `RALPH_TELEGRAM_API_URL` redirects all teloxide API
calls to a custom URL. This feature is **untested** — Sprint 4 is the first real
validation. Ralph's docs reference `ghcr.io/nickolay/telegram-test-api` but that
image doesn't exist (docs were aspirational).

## Chosen: tg-mock

**Source:** https://github.com/watzon/tg-mock
**Docker:** `ghcr.io/watzon/tg-mock:latest`

A Go-based mock Telegram Bot API server designed for integration testing. Drop-in
replacement for `api.telegram.org` — point your bot at `http://localhost:8081` and
it validates requests, generates realistic responses, and exposes a control API.

### Installation

```bash
# Docker
docker pull ghcr.io/watzon/tg-mock:latest
docker run -p 8081:8081 ghcr.io/watzon/tg-mock

# Go binary
go install github.com/watzon/tg-mock/cmd/tg-mock@latest

# From source
git clone https://github.com/watzon/tg-mock.git && cd tg-mock && go build -o tg-mock ./cmd/tg-mock
```

### CLI Flags

- `--port` — custom HTTP port (default 8081)
- `--verbose` — detailed logging
- `--faker-seed` — fixed seed for reproducible responses (important for tests)
- `--config` — YAML config file

### Bot API Surface

Standard Telegram Bot API at `http://localhost:8081/bot<TOKEN>/<METHOD>`.
All standard methods work: `sendMessage`, `getUpdates`, `setWebhook`, etc.
Validates requests and returns properly formatted JSON.

### Control API (`/__control/`)

This is what the test harness uses:

**Inject updates (fake user messages):**
```bash
curl -X POST http://localhost:8081/__control/updates -d '{
  "message": {
    "message_id": 1,
    "text": "approved",
    "chat": {"id": 12345, "type": "private"},
    "from": {"id": 12345, "is_bot": false, "first_name": "TestUser"}
  }
}'
```

Per-token injection:
```bash
curl -X POST http://localhost:8081/__control/tokens/<TOKEN>/updates -d '{...}'
```

**Read bot's sent messages (request inspector):**
```bash
# All requests
curl http://localhost:8081/__control/requests

# Filtered
curl "http://localhost:8081/__control/requests?method=sendMessage&token=test-token-ha&limit=10"

# Clear
curl -X DELETE http://localhost:8081/__control/requests
```

Each record includes: timestamp, token, method, parameters, response, HTTP status.

**Scenarios (error simulation):**
```bash
curl -X POST http://localhost:8081/__control/scenarios -d '{
  "method": "sendMessage",
  "match": {"chat_id": 999},
  "times": 1,
  "response": {"error_code": 400, "description": "Bad Request: chat not found"}
}'
```

**Health:** `GET /` or any basic request.

### Why tg-mock Over Alternatives

| Feature | tg-mock | jehy/telegram-test-api | Custom Python mock |
|---------|---------|----------------------|-------------------|
| Language | Go | Node.js | Python |
| Docker image | Yes (`ghcr.io/watzon/tg-mock`) | No official | N/A |
| Control API | Full (`/__control/`) | Limited | Custom |
| Request inspector | Yes (filter by method/token) | No | Custom |
| Error simulation | Scenarios + headers | No | Custom |
| Dependencies | Single binary or Docker | npm/Node.js | Python stdlib |
| Deterministic | `--faker-seed` | No | N/A |

**Decision:** tg-mock. Docker image available, proper control API for both injecting
messages and reading bot output, no custom code needed for the mock itself. Only
dependency is Docker (or Go binary).

## Ralph API Surface (what tg-mock needs to handle)

Ralph uses teloxide. Endpoints agents hit:
- `POST /bot{token}/sendMessage` — questions, status, greetings
- `POST /bot{token}/getUpdates` — long-poll for replies
- `POST /bot{token}/sendDocument` — file attachments (not critical for Sprint 4)
- `POST /bot{token}/sendPhoto` — image attachments (not critical for Sprint 4)
- `POST /bot{token}/setMessageReaction` — emoji reactions

tg-mock handles all standard Bot API methods, so all of these are covered.

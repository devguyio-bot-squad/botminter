#!/usr/bin/env bash
# Spike: Boot Rocket.Chat + MongoDB via Podman Pod, create admin/bot users, prove bidirectional messaging.
# Usage: ./spike.sh [--keep]
#   --keep  Skip cleanup for manual inspection (leave pod running)

set -euo pipefail

POD_NAME="bm-rc-spike"
RC_PORT=3100
RC_URL="http://127.0.0.1:${RC_PORT}"
MONGO_IMAGE="docker.io/mongo:7.0"
RC_IMAGE="registry.rocket.chat/rocketchat/rocket.chat:latest"
ADMIN_USER="rcadmin"
ADMIN_PASS="rcadmin123"
ADMIN_EMAIL="admin@botminter.local"
BOT_USER="spike-bot"
CREATE_TOKEN_SECRET="bm-spike-secret-$(openssl rand -hex 8)"
KEEP=false

for arg in "$@"; do
  case "$arg" in
    --keep) KEEP=true ;;
    *) echo >&2 "Unknown argument: $arg"; exit 1 ;;
  esac
done

cleanup() {
  if [ "$KEEP" = true ]; then
    echo >&2 "[spike] --keep flag set, leaving pod running."
    echo >&2 "[spike] To clean up: podman pod stop ${POD_NAME} && podman pod rm ${POD_NAME}"
    return
  fi
  echo >&2 "[spike] Cleaning up pod ${POD_NAME}..."
  podman pod stop "${POD_NAME}" 2>/dev/null || true
  podman pod rm "${POD_NAME}" 2>/dev/null || true
}
trap cleanup EXIT

# ── 1. Create Podman Pod ──
echo >&2 "[spike] Creating Podman Pod: ${POD_NAME} (port ${RC_PORT}:3000)..."
# Remove any leftover pod from a previous run
podman pod stop "${POD_NAME}" 2>/dev/null || true
podman pod rm "${POD_NAME}" 2>/dev/null || true
podman pod create --name "${POD_NAME}" -p "${RC_PORT}:3000"

# ── 2. Start MongoDB ──
echo >&2 "[spike] Starting MongoDB 7.0 with replica set..."
podman run -d --pod "${POD_NAME}" --name "${POD_NAME}-mongo" \
  "${MONGO_IMAGE}" mongod --replSet rs0 --oplogSize 128

echo >&2 "[spike] Waiting for MongoDB to accept connections..."
for i in $(seq 1 30); do
  if podman exec "${POD_NAME}-mongo" mongosh --quiet --eval "db.runCommand({ping:1})" 2>/dev/null | grep -q 'ok.*1'; then
    echo >&2 "[spike] MongoDB is up (attempt ${i})."
    break
  fi
  if [ "$i" -eq 30 ]; then
    echo >&2 "[spike] FATAL: MongoDB did not start within 30 attempts."
    exit 1
  fi
  sleep 2
done

echo >&2 "[spike] Initializing replica set..."
podman exec "${POD_NAME}-mongo" mongosh --quiet --eval \
  'rs.initiate({_id:"rs0",members:[{_id:0,host:"localhost:27017"}]})'
sleep 2
echo >&2 "[spike] Replica set initialized."

# ── 3. Start Rocket.Chat ──
echo >&2 "[spike] Starting Rocket.Chat..."
podman run -d --pod "${POD_NAME}" --name "${POD_NAME}-rocketchat" \
  -e ROOT_URL="${RC_URL}" \
  -e MONGO_URL="mongodb://localhost:27017/rocketchat?replicaSet=rs0" \
  -e MONGO_OPLOG_URL="mongodb://localhost:27017/local?replicaSet=rs0" \
  -e OVERWRITE_SETTING_Show_Setup_Wizard="completed" \
  -e ADMIN_USERNAME="${ADMIN_USER}" \
  -e ADMIN_PASS="${ADMIN_PASS}" \
  -e ADMIN_EMAIL="${ADMIN_EMAIL}" \
  -e CREATE_TOKENS_FOR_USERS_SECRET="${CREATE_TOKEN_SECRET}" \
  -e OVERWRITE_SETTING_Accounts_TwoFactorAuthentication_Enabled=false \
  -e OVERWRITE_SETTING_Accounts_TwoFactorAuthentication_By_Email_Enabled=false \
  "${RC_IMAGE}"

# ── 4. Health check loop ──
echo >&2 "[spike] Waiting for Rocket.Chat to become healthy (up to 180s)..."
HEALTH_START=$(date +%s)
for i in $(seq 1 60); do
  HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" "${RC_URL}/api/info" 2>/dev/null || echo "000")
  if [ "$HTTP_CODE" = "200" ]; then
    HEALTH_ELAPSED=$(( $(date +%s) - HEALTH_START ))
    echo >&2 "[spike] Rocket.Chat is healthy (took ${HEALTH_ELAPSED}s)."
    break
  fi
  if [ "$i" -eq 60 ]; then
    echo >&2 "[spike] FATAL: Rocket.Chat did not become healthy within 180s."
    echo >&2 "[spike] Last HTTP code: ${HTTP_CODE}"
    podman logs "${POD_NAME}-rocketchat" 2>&1 | tail -20 >&2
    exit 1
  fi
  sleep 3
done

# ── 5. Admin login ──
echo >&2 "[spike] Logging in as admin..."
LOGIN_RESP=$(curl -s -X POST \
  -H "Content-Type: application/json" \
  -d "{\"user\":\"${ADMIN_USER}\",\"password\":\"${ADMIN_PASS}\"}" \
  "${RC_URL}/api/v1/login")

ADMIN_TOKEN=$(echo "$LOGIN_RESP" | jq -r '.data.authToken')
ADMIN_USER_ID=$(echo "$LOGIN_RESP" | jq -r '.data.userId')

if [ "$ADMIN_TOKEN" = "null" ] || [ -z "$ADMIN_TOKEN" ]; then
  echo >&2 "[spike] FATAL: Admin login failed."
  echo >&2 "$LOGIN_RESP"
  exit 1
fi
echo >&2 "[spike] Admin login successful. userId=${ADMIN_USER_ID}"

# ── 6. Create bot user ──
echo >&2 "[spike] Creating bot user: ${BOT_USER}..."
BOT_PASS=$(openssl rand -hex 16)
CREATE_BOT_RESP=$(curl -s -X POST \
  -H "Content-Type: application/json" \
  -H "X-Auth-Token: ${ADMIN_TOKEN}" \
  -H "X-User-Id: ${ADMIN_USER_ID}" \
  -d "{\"email\":\"bot@botminter.local\",\"name\":\"Spike Bot\",\"password\":\"${BOT_PASS}\",\"username\":\"${BOT_USER}\",\"roles\":[\"bot\"],\"verified\":true}" \
  "${RC_URL}/api/v1/users.create")

BOT_USER_ID=$(echo "$CREATE_BOT_RESP" | jq -r '.user._id')
if [ "$BOT_USER_ID" = "null" ] || [ -z "$BOT_USER_ID" ]; then
  echo >&2 "[spike] FATAL: Bot user creation failed."
  echo >&2 "$CREATE_BOT_RESP"
  exit 1
fi
echo >&2 "[spike] Bot user created. userId=${BOT_USER_ID}"

# ── 7. Generate bot auth token ──
echo >&2 "[spike] Generating bot auth token via users.createToken..."
TOKEN_RESP=$(curl -s -X POST \
  -H "Content-Type: application/json" \
  -H "X-Auth-Token: ${ADMIN_TOKEN}" \
  -H "X-User-Id: ${ADMIN_USER_ID}" \
  -d "{\"userId\":\"${BOT_USER_ID}\",\"secret\":\"${CREATE_TOKEN_SECRET}\"}" \
  "${RC_URL}/api/v1/users.createToken")

BOT_AUTH_TOKEN=$(echo "$TOKEN_RESP" | jq -r '.data.authToken')
if [ "$BOT_AUTH_TOKEN" = "null" ] || [ -z "$BOT_AUTH_TOKEN" ]; then
  echo >&2 "[spike] FATAL: Bot token generation failed."
  echo >&2 "$TOKEN_RESP"
  exit 1
fi
echo >&2 "[spike] Bot auth token generated."

# ── 8. Create a channel ──
echo >&2 "[spike] Creating channel: spike-test..."
CHANNEL_RESP=$(curl -s -X POST \
  -H "Content-Type: application/json" \
  -H "X-Auth-Token: ${ADMIN_TOKEN}" \
  -H "X-User-Id: ${ADMIN_USER_ID}" \
  -d "{\"name\":\"spike-test\",\"members\":[\"${BOT_USER}\"]}" \
  "${RC_URL}/api/v1/channels.create")

CHANNEL_ID=$(echo "$CHANNEL_RESP" | jq -r '.channel._id')
if [ "$CHANNEL_ID" = "null" ] || [ -z "$CHANNEL_ID" ]; then
  echo >&2 "[spike] FATAL: Channel creation failed."
  echo >&2 "$CHANNEL_RESP"
  exit 1
fi
echo >&2 "[spike] Channel created. channelId=${CHANNEL_ID}"

# ── 9. Verify REST API connectivity ──
echo >&2 "[spike] Verifying bot can list channels..."
LIST_RESP=$(curl -s \
  -H "X-Auth-Token: ${BOT_AUTH_TOKEN}" \
  -H "X-User-Id: ${BOT_USER_ID}" \
  "${RC_URL}/api/v1/channels.list")

CHANNEL_COUNT=$(echo "$LIST_RESP" | jq -r '.channels | length')
FOUND_CHANNEL=$(echo "$LIST_RESP" | jq -r '.channels[] | select(.name == "spike-test") | .name')
if [ "$FOUND_CHANNEL" != "spike-test" ]; then
  echo >&2 "[spike] FATAL: Bot cannot see spike-test channel."
  echo >&2 "$LIST_RESP"
  exit 1
fi
echo >&2 "[spike] Bot can see ${CHANNEL_COUNT} channel(s), including spike-test."

# ── 10. Bidirectional messaging proof ──
MESSAGES_SENT=0
BIDIR_TEST="fail"

echo >&2 "[spike] Sending message as bot..."
BOT_MSG_RESP=$(curl -s -X POST \
  -H "Content-Type: application/json" \
  -H "X-Auth-Token: ${BOT_AUTH_TOKEN}" \
  -H "X-User-Id: ${BOT_USER_ID}" \
  -d "{\"message\":{\"rid\":\"${CHANNEL_ID}\",\"msg\":\"Hello from spike-bot! This is a bidirectional test.\"}}" \
  "${RC_URL}/api/v1/chat.sendMessage")
BOT_MSG_OK=$(echo "$BOT_MSG_RESP" | jq -r '.success')
if [ "$BOT_MSG_OK" = "true" ]; then
  MESSAGES_SENT=$((MESSAGES_SENT + 1))
  echo >&2 "[spike] Bot message sent."
else
  echo >&2 "[spike] FATAL: Bot message send failed."
  echo >&2 "$BOT_MSG_RESP"
  exit 1
fi

echo >&2 "[spike] Sending reply as admin..."
ADMIN_MSG_RESP=$(curl -s -X POST \
  -H "Content-Type: application/json" \
  -H "X-Auth-Token: ${ADMIN_TOKEN}" \
  -H "X-User-Id: ${ADMIN_USER_ID}" \
  -d "{\"message\":{\"rid\":\"${CHANNEL_ID}\",\"msg\":\"Admin reply to bot. Bidirectional test confirmed.\"}}" \
  "${RC_URL}/api/v1/chat.sendMessage")
ADMIN_MSG_OK=$(echo "$ADMIN_MSG_RESP" | jq -r '.success')
if [ "$ADMIN_MSG_OK" = "true" ]; then
  MESSAGES_SENT=$((MESSAGES_SENT + 1))
  echo >&2 "[spike] Admin reply sent."
else
  echo >&2 "[spike] FATAL: Admin reply send failed."
  echo >&2 "$ADMIN_MSG_RESP"
  exit 1
fi

echo >&2 "[spike] Sending /status command message as bot..."
CMD_MSG_RESP=$(curl -s -X POST \
  -H "Content-Type: application/json" \
  -H "X-Auth-Token: ${BOT_AUTH_TOKEN}" \
  -H "X-User-Id: ${BOT_USER_ID}" \
  -d "{\"message\":{\"rid\":\"${CHANNEL_ID}\",\"msg\":\"/status\"}}" \
  "${RC_URL}/api/v1/chat.sendMessage")
CMD_MSG_OK=$(echo "$CMD_MSG_RESP" | jq -r '.success')
if [ "$CMD_MSG_OK" = "true" ]; then
  MESSAGES_SENT=$((MESSAGES_SENT + 1))
  echo >&2 "[spike] /status command message sent."
else
  echo >&2 "[spike] WARNING: /status command message failed (may be intercepted by RC)."
  echo >&2 "$CMD_MSG_RESP"
fi

echo >&2 "[spike] Retrieving channel history as bot..."
sleep 1
HISTORY_RESP=$(curl -s \
  -H "X-Auth-Token: ${BOT_AUTH_TOKEN}" \
  -H "X-User-Id: ${BOT_USER_ID}" \
  "${RC_URL}/api/v1/channels.history?roomId=${CHANNEL_ID}&count=10")

HISTORY_COUNT=$(echo "$HISTORY_RESP" | jq -r '.messages | length')
BOT_MSG_FOUND=$(echo "$HISTORY_RESP" | jq -r '[.messages[] | select(.u.username == "spike-bot")] | length')
ADMIN_MSG_FOUND=$(echo "$HISTORY_RESP" | jq -r '[.messages[] | select(.u.username == "rcadmin")] | length')

echo >&2 "[spike] History: ${HISTORY_COUNT} messages total, ${BOT_MSG_FOUND} from bot, ${ADMIN_MSG_FOUND} from admin."

if [ "$BOT_MSG_FOUND" -ge 1 ] && [ "$ADMIN_MSG_FOUND" -ge 1 ]; then
  BIDIR_TEST="pass"
  echo >&2 "[spike] Bidirectional messaging test PASSED."
else
  echo >&2 "[spike] Bidirectional messaging test FAILED."
fi

# ── 11. Output JSON summary ──
cat <<ENDJSON
{
  "server_url": "${RC_URL}",
  "admin_user_id": "${ADMIN_USER_ID}",
  "admin_auth_token": "${ADMIN_TOKEN}",
  "bot_user_id": "${BOT_USER_ID}",
  "bot_auth_token": "${BOT_AUTH_TOKEN}",
  "channel_id": "${CHANNEL_ID}",
  "channel_name": "spike-test",
  "messages_sent": ${MESSAGES_SENT},
  "bidirectional_test": "${BIDIR_TEST}"
}
ENDJSON

echo >&2 "[spike] Done."

#!/usr/bin/env bash
set -euo pipefail

# ── Globals ──────────────────────────────────────────────────────────────────
DEV_MODE=false
BM_REPO="https://github.com/botminter/botminter.git"
BM_BRANCH=""
YES_MODE=false
CLAUDE_VERIFIED=false
FAILED_STEP=""
FAILED_OUTPUT=""
PKG_MGR=""
CLAUDE_BIN="${BM_CLAUDE:-claude}"

# Known install locations for PATH detection
BM_INSTALL_DIR="${CARGO_HOME:-$HOME/.cargo}/bin"

# Temp files to clean up on exit
CLEANUP_FILES=()
cleanup() { for f in "${CLEANUP_FILES[@]}"; do rm -f "$f"; done; }
trap cleanup EXIT

# ── Helpers ──────────────────────────────────────────────────────────────────

cyan()  { printf '\033[1;36m%s\033[0m' "$1"; }
green() { printf '\033[1;32m%s\033[0m' "$1"; }
red()   { printf '\033[1;31m%s\033[0m' "$1"; }
bold()  { printf '\033[1m%s\033[0m' "$1"; }

info()  { echo "  $(green "✓") $1"; }
warn()  { echo "  $(bold "⚠") $1"; }
fail()  { echo "  $(red "✗") $1"; }

# Prompt to remove or skip an existing component.
# Returns 0 = skip, 1 = removed (proceed with install).
prompt_existing() {
  local name="$1" version="$2"
  warn "$name $version is already installed."
  if [[ "$YES_MODE" == true ]]; then
    info "Skipping $name (--yes)"
    return 0
  fi
  echo ""
  echo "  [r] Remove and reinstall"
  echo "  [s] Skip (default)"
  echo "  [q] Quit"
  read -r -p "  Choose [r/s/q]: " choice || true
  case "$choice" in
    r|R)
      local bin_path
      bin_path=$(command -v "$name" 2>/dev/null || echo "$BM_INSTALL_DIR/$name")
      if [[ "$name" == "claude-agent-acp" ]]; then
        if [[ "$PKG_MGR" == "pnpm" ]]; then
          pnpm remove -g @agentclientprotocol/claude-agent-acp 2>/dev/null || true
        else
          npm uninstall -g @agentclientprotocol/claude-agent-acp 2>/dev/null || true
        fi
      else
        rm -f "$bin_path"
      fi
      info "Removed $name"
      return 1
      ;;
    q|Q)
      echo ""
      echo "  Exiting."
      exit 0
      ;;
    *)
      info "Skipping $name"
      return 0
      ;;
  esac
}

resolve_release_url() {
  local repo="$1"
  local asset="$2"
  local tag
  tag=$(curl -sL "https://api.github.com/repos/$repo/releases" | jq -r '.[0].tag_name' 2>/dev/null)
  if [[ -z "$tag" || "$tag" == "null" ]]; then
    return 1
  fi
  echo "https://github.com/$repo/releases/download/$tag/$asset"
}

offer_claude_troubleshoot() {
  echo ""
  echo "$(bold "Installation failed at step:") $FAILED_STEP"
  echo ""
  echo "  Would you like to launch Claude Code to help troubleshoot?"
  read -r -p "  [y/N]: " launch_claude || true

  if [[ "$launch_claude" =~ ^[Yy] ]]; then
    local script_path
    script_path=$(realpath "${BASH_SOURCE[0]}" 2>/dev/null || echo "${BASH_SOURCE[0]}")

    local system_prompt="You are helping troubleshoot a BotMinter installation failure."
    system_prompt+=" Script: $script_path"
    system_prompt+=" Failed step: $FAILED_STEP"
    system_prompt+=" Error: $FAILED_OUTPUT"

    echo ""
    echo "  Claude Code can run with --dangerously-skip-permissions for faster"
    echo "  troubleshooting (it won't ask before running commands)."
    read -r -p "  Use --dangerously-skip-permissions? [y/N]: " skip_perms || true

    local claude_args=(
      --append-system-prompt "$system_prompt"
    )

    if [[ "$skip_perms" =~ ^[Yy] ]]; then
      claude_args+=(--dangerously-skip-permissions)
    fi

    echo "start" | "$CLAUDE_BIN" "${claude_args[@]}"
  fi
}

die() {
  fail "$1"
  FAILED_STEP="${2:-}"
  FAILED_OUTPUT="${3:-$1}"
  echo ""
  echo "  After fixing the issue, rerun the installer:"
  echo "    bash install.sh"
  echo ""
  if [[ "$CLAUDE_VERIFIED" == true && -n "$FAILED_STEP" ]]; then
    offer_claude_troubleshoot
  fi
  exit 1
}

usage() {
  cat <<'USAGE'
Usage: install.sh [OPTIONS]

Install the complete BotMinter stack.

OPTIONS
  --dev     Dev mode: clone the botminter repo and build from source
            (release mode components ralph + claude-agent-acp still
            install from published releases)
  --repo <url>      Override the repo URL for dev mode clone
  --branch <name>   Override the branch to checkout for dev mode
  --yes     Accept all defaults without prompting
  --help    Show this help and exit

COMPONENTS INSTALLED
  bm             BotMinter CLI
  bm-agent       Agent-side tooling binary
  ralph          Ralph Orchestrator CLI (from botminter fork)
  claude-agent-acp  Claude Agent ACP bridge (npm package)

PREREQUISITES
  curl       HTTP client (download installers)
  jq         JSON processor (Claude Code verification)
  podman     Container runtime (team environments)
  gh         GitHub CLI (team repo operations)
  npm/pnpm   Node package manager (claude-agent-acp)
  claude     Claude Code CLI (AI coding assistant)

DEV MODE ADDITIONAL PREREQUISITES
  rustc      Rust compiler
  cargo      Rust package manager
  just       Command runner
USAGE
  exit 0
}

# ── CLI Parsing ──────────────────────────────────────────────────────────────

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dev)  DEV_MODE=true; shift ;;
    --repo) BM_REPO="$2"; shift 2 ;;
    --branch) BM_BRANCH="$2"; shift 2 ;;
    --yes)  YES_MODE=true; shift ;;
    --help) usage ;;
    *)
      fail "Unknown option: $1"
      echo "  Run $(bold "install.sh --help") for usage."
      exit 1
      ;;
  esac
done

# ── Platform Check ──────────────────────────────────────────────────────────

if [[ "$(uname -s)" != "Linux" ]]; then
  fail "BotMinter currently supports Linux only."
  echo "  Detected platform: $(uname -s)"
  echo ""
  echo "  See: https://github.com/botminter/botminter/issues"
  exit 1
fi

# ── Banner ───────────────────────────────────────────────────────────────────

echo ""
printf '\033[1;36m'
cat <<'BANNER'
  ██████╗  ██████╗ ████████╗███╗   ███╗██╗███╗   ██╗████████╗███████╗██████╗
  ██╔══██╗██╔═══██╗╚══██╔══╝████╗ ████║██║████╗  ██║╚══██╔══╝██╔════╝██╔══██╗
  ██████╔╝██║   ██║   ██║   ██╔████╔██║██║██╔██╗ ██║   ██║   █████╗  ██████╔╝
  ██╔══██╗██║   ██║   ██║   ██║╚██╔╝██║██║██║╚██╗██║   ██║   ██╔══╝  ██╔══██╗
  ██████╔╝╚██████╔╝   ██║   ██║ ╚═╝ ██║██║██║ ╚████║   ██║   ███████╗██║  ██║
  ╚═════╝  ╚═════╝    ╚═╝   ╚═╝     ╚═╝╚═╝╚═╝  ╚═══╝   ╚═╝   ╚══════╝╚═╝  ╚═╝
BANNER
printf '\033[0m'
echo ""
echo "  $(cyan "Build your agentic team")"
echo ""

if [[ "$DEV_MODE" == true ]]; then
  echo "  Mode: $(bold "development") (build from source)"
else
  echo "  Mode: $(bold "release") (pre-built binaries)"
fi
echo ""
echo "  Components:"
echo "    • bm + bm-agent  — BotMinter CLI & agent tools"
echo "    • ralph          — Ralph Orchestrator CLI"
echo "    • claude-agent-acp — Claude Agent ACP bridge"
echo ""

# ── Prerequisite Checks ─────────────────────────────────────────────────────

echo "$(bold "Checking prerequisites...")"
echo ""

MISSING=()
MISSING_GUIDANCE=()

# Basic CLI tools
declare -A PREREQ_GUIDANCE=(
  [curl]="https://curl.se/ (or dnf/apt install curl)"
  [git]="https://git-scm.com/ (or dnf/apt install git)"
  [jq]="https://jqlang.org/ (or dnf/apt install jq)"
  [podman]="https://podman.io/ (or dnf/apt install podman)"
)

for cmd in curl git jq podman; do
  if command -v "$cmd" &>/dev/null; then
    info "$cmd found"
  else
    fail "$cmd not found"
    MISSING+=("$cmd")
    MISSING_GUIDANCE+=("${PREREQ_GUIDANCE[$cmd]}")
  fi
done

# Package manager detection
if command -v pnpm &>/dev/null; then
  PKG_MGR="pnpm"
  info "pnpm found"
elif command -v npm &>/dev/null; then
  PKG_MGR="npm"
  info "npm found"
  npm_prefix=$(npm config get prefix 2>/dev/null || true)
  case "$npm_prefix" in
    /usr|/usr/local)
      warn "npm global prefix is $npm_prefix (system path). Global installs may need sudo."
      ;;
  esac
else
  fail "npm/pnpm not found"
  MISSING+=("npm/pnpm")
  MISSING_GUIDANCE+=("https://pnpm.io/ or https://nodejs.org/")
fi

# Node.js runtime
if command -v node &>/dev/null; then
  info "node found ($(node --version 2>/dev/null || echo 'unknown'))"
else
  fail "node not found"
  MISSING+=("node")
  if [[ "$PKG_MGR" == "pnpm" ]]; then
    MISSING_GUIDANCE+=("pnpm runtime set node lts -g")
  else
    MISSING_GUIDANCE+=("https://nodejs.org/ (or pnpm runtime set node lts -g)")
  fi
fi

# Claude Code
if command -v "$CLAUDE_BIN" &>/dev/null; then
  info "Claude Code found ($CLAUDE_BIN)"
else
  fail "Claude Code not found"
  MISSING+=("claude")
  MISSING_GUIDANCE+=("https://docs.anthropic.com/en/docs/claude-code/overview")
fi

# GitHub CLI
if command -v gh &>/dev/null; then
  info "gh found"
else
  fail "gh not found"
  MISSING+=("gh")
  MISSING_GUIDANCE+=("https://cli.github.com/")
fi

# Dev mode prerequisites
if [[ "$DEV_MODE" == true ]]; then
  if command -v rustc &>/dev/null && command -v cargo &>/dev/null; then
    info "rustc found ($(rustc --version 2>/dev/null | awk '{print $2}' || echo 'unknown'))"
    info "cargo found ($(cargo --version 2>/dev/null | awk '{print $2}' || echo 'unknown'))"
  else
    fail "rustc/cargo not found (required for dev mode)"
    MISSING+=("rustc/cargo")
    MISSING_GUIDANCE+=("https://www.rust-lang.org/tools/install")
  fi

  if command -v just &>/dev/null; then
    info "just found ($(just --version 2>/dev/null | awk '{print $2}' || echo 'unknown'))"
  else
    fail "just not found (required for dev mode)"
    MISSING+=("just")
    MISSING_GUIDANCE+=("cargo install just  (or dnf/apt install just)")
  fi
fi

# Report missing prerequisites
if [[ ${#MISSING[@]} -gt 0 ]]; then
  echo ""
  echo "  ══════════════════════════════════════════════════"
  echo "  $(bold "Missing prerequisites")"
  echo "  ══════════════════════════════════════════════════"
  echo ""
  echo "  The following prerequisites are missing:"
  echo ""
  {
    printf "%s\t%s\n" "PREREQUISITE" "MORE INFO"
    printf "%s\t%s\n" "────────────" "─────────"
    for i in "${!MISSING[@]}"; do
      printf "%s\t%s\n" "${MISSING[$i]}" "${MISSING_GUIDANCE[$i]}"
    done
  } | column -t -s $'\t' | sed 's/^/  /'
  echo ""
  echo "  Install the missing prerequisites, then rerun:"
  echo "    bash install.sh"
  echo ""
  exit 1
fi

echo ""

# ── Functional Checks ──────────────────────────────────────────────────────

echo "$(bold "Verifying prerequisites...")"
echo ""

# Podman container test
echo "  Testing podman can run containers..."
if ! podman_output=$(podman run --rm fedora:latest true 2>&1); then
  fail "podman cannot run containers."
  echo ""
  echo "  Possible fixes:"
  echo "    • Ensure your user can run rootless containers:"
  echo "        podman system migrate"
  echo "    • Check subuid/subgid mappings:"
  echo "        grep \$(whoami) /etc/subuid /etc/subgid"
  echo "    • Reset podman storage:"
  echo "        podman system reset"
  echo ""
  die "podman run --rm fedora:latest true failed: $podman_output" \
      "prereq-podman-run" "$podman_output"
fi
info "podman can run containers"

# GitHub CLI auth check
echo "  Checking gh authentication..."
if ! gh_status=$(gh auth status 2>&1); then
  fail "gh is not authenticated."
  echo ""
  echo "  BotMinter requires an authenticated gh session."
  echo "  Run:"
  echo "    gh auth login --git-protocol https"
  echo "    gh auth setup-git"
  echo ""
  echo "  If using a classic PAT, ensure it has repo, project, and"
  echo "  read:org scopes."
  die "gh not authenticated." "prereq-gh-auth"
fi
info "gh authenticated"

# Check gh token scopes
gh_scopes=$(gh auth status 2>&1 | grep -i "token scopes" || echo "")
if [[ -n "$gh_scopes" ]]; then
  missing_scopes=()
  for scope in repo read:org project; do
    if ! echo "$gh_scopes" | grep -qw "$scope"; then
      missing_scopes+=("$scope")
    fi
  done
  if [[ ${#missing_scopes[@]} -gt 0 ]]; then
    warn "gh token may be missing scopes: ${missing_scopes[*]}"
    echo "  BotMinter needs: repo, read:org, project"
    echo "  Re-authenticate or use a PAT with these scopes."
  fi
fi

# Claude Code LLM test
echo "  Testing Claude Code with LLM verification..."
claude_test_cmd="$CLAUDE_BIN --append-system-prompt '...' -p 'print the following message <message>hello BotMinter<message>' --output-format json"
claude_test_output=$("$CLAUDE_BIN" \
  --append-system-prompt "you're running in a test env, print exactly what you'll be told, char for char" \
  -p 'print the following message <message>hello BotMinter<message>' \
  --output-format json 2>/dev/null | jq -r '.result' 2>/dev/null) || true

if [[ "$claude_test_output" != *"hello BotMinter"* ]]; then
  fail "Claude Code LLM verification failed."
  echo ""
  echo "  We tried to verify that Claude Code is properly configured and"
  echo "  working by running a simple test prompt:"
  echo ""
  echo "    $claude_test_cmd"
  echo ""
  echo "  Output:"
  echo "    ${claude_test_output:-<empty>}"
  echo ""
  echo "  Make sure Claude Code is properly configured with valid API"
  echo "  credentials and can respond to prompts."
  die "Claude Code LLM test failed." "prereq-claude-llm"
fi
CLAUDE_VERIFIED=true
info "Claude Code verified"

echo ""

# Keyring check
echo "  Checking keyring (secret storage)..."
if ! command -v gnome-keyring-daemon &>/dev/null; then
  fail "gnome-keyring-daemon not found."
  echo ""
  echo "  BotMinter uses the system keyring to store credentials securely."
  echo "  Install gnome-keyring:"
  echo "    dnf install gnome-keyring  (or your distro's package manager)"
  die "gnome-keyring not installed." "prereq-keyring"
fi

KEYRING_DBUS="unix:path=/run/user/$(id -u)/bus"
export DBUS_SESSION_BUS_ADDRESS="$KEYRING_DBUS"

keyring_locked=$(busctl --user get-property org.freedesktop.secrets \
  /org/freedesktop/secrets/collection/login \
  org.freedesktop.Secret.Collection Locked 2>/dev/null || echo "unavailable")

if [[ "$keyring_locked" == "b false" ]]; then
  info "Keyring unlocked"
elif [[ "$keyring_locked" == "b true" ]]; then
  fail "Keyring is locked."
  echo ""
  echo "  The keyring daemon is running but the keyring is locked."
  echo "  BotMinter needs an unlocked keyring to store credentials."
  echo ""
  echo "  If you have the keyring helper script installed:"
  echo "    keyring unlock"
  echo ""
  echo "  Otherwise, unlock manually:"
  echo "    echo -n \"<password>\" | gnome-keyring-daemon --replace --unlock --components=secrets --daemonize"
  die "Keyring locked." "prereq-keyring-locked"
else
  # Daemon not running or no D-Bus — likely a non-login session (SSH/su)
  fail "Keyring is not available."
  echo ""
  echo "  BotMinter uses the system keyring to store credentials securely."
  echo "  On non-login sessions (SSH, su), the keyring daemon needs to be"
  echo "  started manually."
  echo ""
  echo "  We provide a small helper script that makes this easy:"
  echo "    keyring unlock   — start the daemon and unlock the keyring"
  echo "    keyring lock     — stop the daemon"
  echo "    keyring status   — check if the keyring is unlocked"
  echo ""

  install_helper=false
  if [[ "$YES_MODE" == true ]]; then
    install_helper=true
  else
    read -r -p "  Install the keyring helper script to ~/.local/bin/keyring? [Y/n]: " install_choice || true
    if [[ ! "$install_choice" =~ ^[Nn] ]]; then
      install_helper=true
    fi
  fi

  if [[ "$install_helper" == true ]]; then
    mkdir -p "$HOME/.local/bin"
    cat > "$HOME/.local/bin/keyring" << 'KEYRING_SCRIPT'
#!/bin/bash
set -euo pipefail

export DBUS_SESSION_BUS_ADDRESS="unix:path=/run/user/$(id -u)/bus"

case "${1:-}" in
  lock|l)
    pkill -f -u "$(id -u)" gnome-keyring-daemon && echo "Keyring daemon killed." || echo "No keyring daemon running."
    ;;
  unlock|u)
    read -s -p "Keyring password: " password
    echo
    echo -n "$password" | gnome-keyring-daemon --replace --unlock --components=secrets --daemonize > /dev/null
    sleep 1
    locked=$(busctl --user get-property org.freedesktop.secrets /org/freedesktop/secrets/collection/login org.freedesktop.Secret.Collection Locked 2>/dev/null || echo "unknown")
    if [[ "$locked" == "b false" ]]; then
      echo "Keyring unlocked."
    else
      echo "Failed to unlock keyring."
      exit 1
    fi
    ;;
  status|s)
    if ! pgrep -f -u "$(id -u)" gnome-keyring-daemon > /dev/null; then
      echo "Keyring daemon not running."
    else
      locked=$(busctl --user get-property org.freedesktop.secrets /org/freedesktop/secrets/collection/login org.freedesktop.Secret.Collection Locked 2>/dev/null || echo "unknown")
      if [[ "$locked" == "b false" ]]; then
        echo "Keyring unlocked."
      elif [[ "$locked" == "b true" ]]; then
        echo "Keyring locked."
      else
        echo "Keyring status unknown."
      fi
    fi
    ;;
  *)
    echo "Usage: keyring {lock|unlock|status}"
    exit 1
    ;;
esac
KEYRING_SCRIPT
    chmod +x "$HOME/.local/bin/keyring"
    info "Installed keyring helper to ~/.local/bin/keyring"

    if ! echo "$PATH" | grep -q "$HOME/.local/bin"; then
      warn "~/.local/bin is not on your PATH. Add it to your shell profile."
    fi

    echo ""
    echo "  To unlock the keyring, run:"
    echo "    keyring unlock"
    echo ""
    echo "  You will be prompted to set a keyring password (first time)."
    echo "  The keyring must be unlocked before starting your team."
  else
    echo ""
    echo "  The keyring must be unlocked before running bm hire or bm start."
    echo "  You can set it up manually with gnome-keyring-daemon."
  fi
  echo ""
  echo "  Unlock the keyring, then rerun the installer:"
  echo "    bash install.sh"
  echo ""
  exit 1
fi

echo ""

# ── Installation ─────────────────────────────────────────────────────────────

echo "$(bold "Installing components...")"
echo ""

# ── bm + bm-agent ────────────────────────────────────────────────────────────

if [[ "$DEV_MODE" == true ]]; then
  # Dev mode: clone and build from source
  bm_skip=false
  if command -v bm &>/dev/null; then
    prompt_existing "bm" "$(bm --version 2>/dev/null || echo '')" && bm_skip=true
  elif [[ -x "$BM_INSTALL_DIR/bm" ]]; then
    prompt_existing "bm" "($BM_INSTALL_DIR/bm, not on PATH)" && bm_skip=true
  fi

  if [[ "$bm_skip" == true ]]; then
    info "Skipping dev mode build"
  else
  default_clone_dir="$HOME/.botminter/src/botminter"
  clone_dir="$default_clone_dir"

  if [[ "$YES_MODE" != true ]]; then
    echo "  Clone directory for botminter source?"
    read -r -p "  [$default_clone_dir]: " user_dir || true
    if [[ -n "$user_dir" ]]; then
      clone_dir="$user_dir"
    fi
  fi

  if [[ -d "$clone_dir/.git" ]]; then
    info "Existing clone found at $clone_dir — pulling latest"
    if [[ -n "$BM_BRANCH" ]]; then
      git -C "$clone_dir" checkout "$BM_BRANCH" || \
        die "git checkout $BM_BRANCH failed in $clone_dir" "dev-git-checkout"
    fi
    git -C "$clone_dir" pull --ff-only || \
      die "git pull failed in $clone_dir" "dev-git-pull"
  else
    info "Cloning botminter to $clone_dir"
    mkdir -p "$(dirname "$clone_dir")"
    clone_args=("$BM_REPO" "$clone_dir")
    if [[ -n "$BM_BRANCH" ]]; then
      clone_args=(--branch "$BM_BRANCH" "$BM_REPO" "$clone_dir")
    fi
    git clone "${clone_args[@]}" || \
      die "git clone failed" "dev-git-clone"
  fi

  info "Building bm + bm-agent via just install"
  (cd "$clone_dir" && just install) || \
    die "just install failed in $clone_dir" "dev-just-install"
  fi

else
  # Release mode: cargo-dist installer
  bm_skip=false
  if command -v bm &>/dev/null; then
    prompt_existing "bm" "$(bm --version 2>/dev/null || echo '')" && bm_skip=true
  elif [[ -x "$BM_INSTALL_DIR/bm" ]]; then
    prompt_existing "bm" "($BM_INSTALL_DIR/bm, not on PATH)" && bm_skip=true
  fi
  if [[ "$bm_skip" == false ]]; then
    info "Installing bm + bm-agent via cargo-dist..."
    bm_url=$(resolve_release_url "botminter/botminter" "bm-installer.sh") || \
      die "Failed to resolve latest bm release from GitHub API." "install-bm-resolve"

    bm_installer=$(mktemp /tmp/bm-installer-XXXXXX.sh)
    CLEANUP_FILES+=("$bm_installer")

    if ! curl --proto '=https' --tlsv1.2 -LsSf \
         "$bm_url" \
         -o "$bm_installer"; then
      die "Failed to download bm installer from $bm_url" "install-bm-download"
    fi

    if ! bash "$bm_installer"; then
      die "bm installer script failed." "install-bm-run"
    fi
    info "bm + bm-agent installed"
  fi
fi

# ── ralph ────────────────────────────────────────────────────────────────────

ralph_skip=false
if command -v ralph &>/dev/null; then
  prompt_existing "ralph" "$(ralph --version 2>/dev/null || echo '')" && ralph_skip=true
elif [[ -x "$BM_INSTALL_DIR/ralph" ]]; then
  prompt_existing "ralph" "($BM_INSTALL_DIR/ralph, not on PATH)" && ralph_skip=true
fi
if [[ "$ralph_skip" == false ]]; then
  info "Installing ralph via cargo-dist..."
  ralph_url=$(resolve_release_url "botminter/ralph-orchestrator" "ralph-cli-installer.sh") || \
    die "Failed to resolve latest ralph release from GitHub API." "install-ralph-resolve"

  ralph_installer=$(mktemp /tmp/ralph-installer-XXXXXX.sh)
  CLEANUP_FILES+=("$ralph_installer")

  if ! curl --proto '=https' --tlsv1.2 -LsSf \
       "$ralph_url" \
       -o "$ralph_installer"; then
    die "Failed to download ralph installer from $ralph_url" "install-ralph-download"
  fi

  if ! bash "$ralph_installer"; then
    die "ralph installer script failed." "install-ralph-run"
  fi
  info "ralph installed"
fi

# ── claude-agent-acp ─────────────────────────────────────────────────────────

acp_skip=false
if command -v claude-agent-acp &>/dev/null; then
  prompt_existing "claude-agent-acp" "" && acp_skip=true
fi
if [[ "$acp_skip" == false ]]; then
  info "Installing claude-agent-acp via $PKG_MGR..."
  if [[ "$PKG_MGR" == "pnpm" ]]; then
    if ! pnpm add -g @agentclientprotocol/claude-agent-acp; then
      die "pnpm add -g @agentclientprotocol/claude-agent-acp failed." \
          "install-acp" ""
    fi
  else
    if ! npm install -g @agentclientprotocol/claude-agent-acp; then
      die "npm install -g @agentclientprotocol/claude-agent-acp failed." \
          "install-acp" ""
    fi
  fi
  info "claude-agent-acp installed"
fi

echo ""

# ── Post-Install Verification ────────────────────────────────────────────────

echo "$(bold "Verifying installation...")"
echo ""

path_fixups=()

for bin_name in bm bm-agent ralph; do
  if command -v "$bin_name" &>/dev/null; then
    version=$("$bin_name" --version 2>/dev/null || echo "unknown")
    info "$bin_name $version"
  elif [[ -x "$BM_INSTALL_DIR/$bin_name" ]]; then
    version=$("$BM_INSTALL_DIR/$bin_name" --version 2>/dev/null || echo "unknown")
    warn "$bin_name $version (installed at $BM_INSTALL_DIR/$bin_name but not on PATH)"
    path_fixups+=("$bin_name")
  else
    warn "$bin_name not found"
  fi
done

# claude-agent-acp is an ACP server — running it directly blocks on stdin.
# Get version from the package manager instead.
if command -v claude-agent-acp &>/dev/null; then
  acp_bin=$(command -v claude-agent-acp)
  acp_version=$(grep -oP 'claude-agent-acp@\K[0-9]+\.[0-9]+\.[0-9]+' "$acp_bin" 2>/dev/null | head -1)
  if [[ -z "$acp_version" ]]; then
    acp_real=$(readlink -f "$acp_bin" 2>/dev/null || echo "")
    acp_pkg_dir=$(dirname "$acp_real" 2>/dev/null || echo "")
    if [[ -f "$acp_pkg_dir/package.json" ]]; then
      acp_version=$(jq -r '.version' "$acp_pkg_dir/package.json" 2>/dev/null || echo "")
    fi
  fi
  info "claude-agent-acp ${acp_version:-unknown}"
else
  warn "claude-agent-acp not found"
fi

if [[ ${#path_fixups[@]} -gt 0 ]]; then
  echo ""
  warn "Some binaries are installed but not on your PATH."
  echo "  Add this to your shell profile (~/.bashrc, ~/.zshrc, etc.):"
  echo ""
  echo "    export PATH=\"$BM_INSTALL_DIR:\$PATH\""
  echo ""
  echo "  Then reload your shell:"
  echo "    source ~/.bashrc  # or ~/.zshrc"
fi

echo ""

# ── Next Steps ───────────────────────────────────────────────────────────────

echo "$(bold "Next steps:")"
echo ""
echo "  1. Create your first team:"
echo "       bm init"
echo ""
echo "  2. Getting started guide:"
echo "       https://www.botminter.ai/getting-started/"
echo ""
echo "  3. Found a bug? File an issue:"
echo "       https://github.com/botminter/botminter/issues"
echo ""
echo "  $(green "Installation complete!")"
echo ""

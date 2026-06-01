#!/usr/bin/env bash
set -euo pipefail

# ── Globals ──────────────────────────────────────────────────────────────────
DEV_MODE=false
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
      -p "start"
    )

    if [[ "$skip_perms" =~ ^[Yy] ]]; then
      claude_args+=(--dangerously-skip-permissions)
    fi

    exec "$CLAUDE_BIN" "${claude_args[@]}"
  fi
}

die() {
  fail "$1"
  FAILED_STEP="${2:-}"
  FAILED_OUTPUT="${3:-$1}"
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
    --yes)  YES_MODE=true; shift ;;
    --help) usage ;;
    *)
      fail "Unknown option: $1"
      echo "  Run $(bold "install.sh --help") for usage."
      exit 1
      ;;
  esac
done

# ── Banner ───────────────────────────────────────────────────────────────────

cat <<'BANNER'

  ____        _   __  __ _       _
 | __ )  ___ | |_|  \/  (_)_ __ | |_ ___ _ __
 |  _ \ / _ \| __| |\/| | | '_ \| __/ _ \ '__|
 | |_) | (_) | |_| |  | | | | | | ||  __/ |
 |____/ \___/ \__|_|  |_|_|_| |_|\__\___|_|

BANNER
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

# Basic CLI tools
for cmd in curl jq; do
  if ! command -v "$cmd" &>/dev/null; then
    die "$cmd is not installed. Install it with your system package manager (e.g., dnf install $cmd)." \
        "prereq-$cmd"
  fi
  info "$cmd found"
done

# Podman
if ! command -v podman &>/dev/null; then
  die "podman is not installed. Install it with your system package manager (e.g., dnf install podman)." \
      "prereq-podman"
fi
info "podman found"

echo ""
echo "  Verifying podman can run containers..."
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

# Package manager detection
if command -v pnpm &>/dev/null; then
  PKG_MGR="pnpm"
elif command -v npm &>/dev/null; then
  PKG_MGR="npm"
  npm_prefix=$(npm config get prefix 2>/dev/null || true)
  case "$npm_prefix" in
    /usr|/usr/local)
      warn "npm global prefix is $npm_prefix (system path). Global installs may need sudo."
      ;;
  esac
fi

if [[ -z "$PKG_MGR" ]]; then
  fail "Neither npm nor pnpm is installed."
  echo ""
  echo "  Install pnpm (recommended):"
  echo "    curl -fsSL https://get.pnpm.io/install.sh | sh -"
  echo ""
  echo "  Or install npm via Node.js:"
  echo "    https://nodejs.org/"
  echo ""
  die "No package manager (npm/pnpm) found." "prereq-pkg-mgr"
fi
info "$PKG_MGR found"

# Claude Code verification
if ! command -v "$CLAUDE_BIN" &>/dev/null; then
  fail "Claude Code is not installed."
  echo ""
  echo "  Install Claude Code:"
  echo "    https://docs.anthropic.com/en/docs/claude-code/overview"
  echo ""
  echo "  BotMinter requires Claude Code to operate team members."
  exit 1
fi
info "Claude Code found ($CLAUDE_BIN)"

echo ""
echo "  Verifying Claude Code with LLM test..."
claude_test_output=$("$CLAUDE_BIN" \
  --append-system-prompt "you're running in a test env, print exactly what you'll be told, char for char" \
  -p 'print the following message <message>hello BotMinter<message>' \
  --output-format json 2>/dev/null | jq -r '.result' 2>/dev/null) || true

if [[ "$claude_test_output" != *"hello BotMinter"* ]]; then
  fail "Claude Code LLM verification failed."
  echo "  Expected output containing: hello BotMinter"
  echo "  Got: ${claude_test_output:-<empty>}"
  echo ""
  echo "  Ensure Claude Code is configured with valid API credentials."
  exit 1
fi
CLAUDE_VERIFIED=true
info "Claude Code verified"

# Dev mode prerequisites
if [[ "$DEV_MODE" == true ]]; then
  for cmd in rustc cargo just; do
    if ! command -v "$cmd" &>/dev/null; then
      die "$cmd is not installed but is required for dev mode." \
          "prereq-dev-$cmd"
    fi
    info "$cmd found (dev mode)"
  done
fi

# GitHub CLI
if ! command -v gh &>/dev/null; then
  fail "gh (GitHub CLI) is not installed."
  echo ""
  echo "  Install gh:"
  echo "    https://cli.github.com/"
  echo ""
  echo "  BotMinter uses gh for team repo operations."
  die "gh CLI not found." "prereq-gh"
fi
info "gh found"

echo ""

# ── Installation ─────────────────────────────────────────────────────────────

echo "$(bold "Installing components...")"
echo ""

# ── bm + bm-agent ────────────────────────────────────────────────────────────

if [[ "$DEV_MODE" == true ]]; then
  # Dev mode: clone and build from source
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
    git -C "$clone_dir" pull --ff-only || \
      die "git pull failed in $clone_dir" "dev-git-pull"
  else
    info "Cloning botminter to $clone_dir"
    mkdir -p "$(dirname "$clone_dir")"
    git clone https://github.com/botminter/botminter.git "$clone_dir" || \
      die "git clone failed" "dev-git-clone"
  fi

  info "Building bm + bm-agent via just install"
  (cd "$clone_dir" && just install) || \
    die "just install failed in $clone_dir" "dev-just-install"

else
  # Release mode: cargo-dist installer
  if command -v bm &>/dev/null; then
    info "bm already installed — skipping"
  elif [[ -x "$BM_INSTALL_DIR/bm" ]]; then
    info "bm found at $BM_INSTALL_DIR/bm but not on PATH — skipping install"
  else
    info "Installing bm + bm-agent via cargo-dist..."
    bm_installer=$(mktemp /tmp/bm-installer-XXXXXX.sh)
    CLEANUP_FILES+=("$bm_installer")

    if ! curl --proto '=https' --tlsv1.2 -LsSf \
         "https://github.com/botminter/botminter/releases/latest/download/bm-installer.sh" \
         -o "$bm_installer"; then
      die "Failed to download bm installer." "install-bm-download"
    fi

    if ! bash "$bm_installer"; then
      die "bm installer script failed." "install-bm-run"
    fi
    info "bm + bm-agent installed"
  fi
fi

# ── ralph ────────────────────────────────────────────────────────────────────

if command -v ralph &>/dev/null; then
  info "ralph already installed — skipping"
elif [[ -x "$BM_INSTALL_DIR/ralph" ]]; then
  info "ralph found at $BM_INSTALL_DIR/ralph but not on PATH — skipping install"
else
  info "Installing ralph via cargo-dist..."
  ralph_installer=$(mktemp /tmp/ralph-installer-XXXXXX.sh)
  CLEANUP_FILES+=("$ralph_installer")

  if ! curl --proto '=https' --tlsv1.2 -LsSf \
       "https://github.com/botminter/ralph-orchestrator/releases/latest/download/ralph-cli-installer.sh" \
       -o "$ralph_installer"; then
    die "Failed to download ralph installer." "install-ralph-download"
  fi

  if ! bash "$ralph_installer"; then
    die "ralph installer script failed." "install-ralph-run"
  fi
  info "ralph installed"
fi

# ── claude-agent-acp ─────────────────────────────────────────────────────────

if command -v claude-agent-acp &>/dev/null; then
  info "claude-agent-acp already installed — skipping"
else
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

for bin_name in bm bm-agent ralph claude-agent-acp; do
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

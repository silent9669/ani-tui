#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

SANDBOX_NAME="${1:-ani-tui-linux}"
COMMAND="${2:-test}"

DAYTONA_API_KEY="${DAYTONA_API_KEY:-}"
if [ -z "$DAYTONA_API_KEY" ]; then
    if [ -f "$PROJECT_ROOT/.env" ]; then
        export $(grep -v '^#' "$PROJECT_ROOT/.env" | xargs)
    fi
fi

if [ -z "$DAYTONA_API_KEY" ]; then
    echo "Error: DAYTONA_API_KEY not set"
    echo "Please set it in your environment or .env file"
    exit 1
fi

echo "Daytona Sandbox Test Runner"
echo "==========================="
echo "Sandbox: $SANDBOX_NAME"
echo "Command: $COMMAND"
echo ""

cd "$PROJECT_ROOT"

if ! command -v daytona &> /dev/null; then
    echo "Installing Daytona CLI..."
    if [[ "$OSTYPE" == "darwin"* ]]; then
        brew install daytonaio/tap/daytona
    else
        curl -L https://github.com/daytonaio/daytona/releases/latest/download/daytona-linux-amd64 -o /tmp/daytona
        chmod +x /tmp/daytona
        sudo mv /tmp/daytona /usr/local/bin/
    fi
fi

echo "Creating sandbox: $SANDBOX_NAME"
daytona sandbox create --name "$SANDBOX_NAME" \
    --image ubuntu:22.04 \
    --env "DAYTONA_API_KEY=$DAYTONA_API_KEY" \
    || echo "Sandbox may already exist, continuing..."

echo "Waiting for sandbox to be ready..."
daytona sandbox start "$SANDBOX_NAME" || true

sleep 5

echo "Uploading project files..."
daytona sandbox ssh "$SANDBOX_NAME" --command "mkdir -p /workspace/ani-tui"
daytona sandbox scp "$PROJECT_ROOT" "$SANDBOX_NAME:/workspace/ani-tui" --recursive

echo "Running setup..."
daytona sandbox ssh "$SANDBOX_NAME" --command "
    cd /workspace/ani-tui
    apt-get update -qq
    apt-get install -y -qq curl build-essential libssl-dev pkg-config
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
    source \"\$HOME/.cargo/env\"
    rustup target add \"\$TARGET\" 2>/dev/null || true
"

echo "Executing: $COMMAND"
case "$COMMAND" in
    build)
        daytona sandbox ssh "$SANDBOX_NAME" --command "
            cd /workspace/ani-tui
            source \"\$HOME/.cargo/env\"
            cargo build --release --target \"\$TARGET\"
        "
        ;;
    test)
        daytona sandbox ssh "$SANDBOX_NAME" --command "
            cd /workspace/ani-tui
            source \"\$HOME/.cargo/env\"
            cargo test --release --target \"\$TARGET\"
        "
        ;;
    check)
        daytona sandbox ssh "$SANDBOX_NAME" --command "
            cd /workspace/ani-tui
            source \"\$HOME/.cargo/env\"
            cargo check --release --target \"\$TARGET\"
        "
        ;;
    clippy)
        daytona sandbox ssh "$SANDBOX_NAME" --command "
            cd /workspace/ani-tui
            source \"\$HOME/.cargo/env\"
            cargo clippy --all-targets --all-features -- -D warnings
        "
        ;;
    fmt)
        daytona sandbox ssh "$SANDBOX_NAME" --command "
            cd /workspace/ani-tui
            source \"\$HOME/.cargo/env\"
            cargo fmt --all -- --check
        "
        ;;
    shell)
        echo "Opening interactive shell in sandbox..."
        daytona sandbox ssh "$SANDBOX_NAME"
        ;;
    *)
        daytona sandbox ssh "$SANDBOX_NAME" --command "
            cd /workspace/ani-tui
            source \"\$HOME/.cargo/env\"
            $COMMAND
        "
        ;;
esac

TEST_RESULT=$?

if [ "$TEST_RESULT" -eq 0 ]; then
    echo ""
    echo "✓ Tests passed!"
else
    echo ""
    echo "✗ Tests failed with exit code: $TEST_RESULT"
fi

echo ""
read -p "Stop and remove sandbox? (y/N): " -n 1 -r
echo
if [[ \$REPLY =~ ^[Yy]$ ]]; then
    echo "Stopping sandbox..."
    daytona sandbox stop "$SANDBOX_NAME" || true
    echo "Removing sandbox..."
    daytona sandbox remove "$SANDBOX_NAME" || true
    echo "✓ Cleanup complete"
else
    echo "Sandbox '$SANDBOX_NAME' is still running"
    echo "To reconnect: daytona sandbox ssh $SANDBOX_NAME"
    echo "To stop: daytona sandbox stop $SANDBOX_NAME"
fi

exit $TEST_RESULT

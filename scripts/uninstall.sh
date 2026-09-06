#!/usr/bin/env bash
set -e

echo "=== Code-Warden Complete Uninstallation ==="
read -r -p "This will delete all 14 cloned engines, cached environments, and the 'cwd' binary. Proceed? (y/N): " confirm
if [[ ! "$confirm" =~ ^([yY][eE][sS]|[yY])$ ]]; then
    echo "Aborted."
    exit 0
fi

# 1. Remove binary
if [ -f "/usr/local/bin/cwd" ]; then
    echo "Removing /usr/local/bin/cwd..."
    sudo rm -f "/usr/local/bin/cwd"
fi

# 2. Purge ~/.code-warden
CODE_WARDEN_HOME="${HOME}/.code-warden"
if [ -d "${CODE_WARDEN_HOME}" ]; then
    echo "Purging ${CODE_WARDEN_HOME} (repos, venv, memory, config)..."
    rm -rf "${CODE_WARDEN_HOME}"
fi

echo ""
echo "[✔] Code-Warden and all associated engines successfully removed."
echo "Note: System compilers (Go, Python 3.12, Java) were left intact to avoid disrupting other applications."

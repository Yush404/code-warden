#!/usr/bin/env bash
set -e

echo "=== Installing Code-Warden (cwd) & Persona Engines ==="

CODE_WARDEN_HOME="${HOME}/.code-warden"
ENGINES_DIR="${CODE_WARDEN_HOME}/engines"
BIN_DIR="${CODE_WARDEN_HOME}/bin"
VENV_DIR="${ENGINES_DIR}/venv"

mkdir -p "${ENGINES_DIR}" "${BIN_DIR}" "${CODE_WARDEN_HOME}/memory"

echo "[1/4] Checking and installing system runtimes..."
if command -v dnf >/dev/null 2>&1; then
    sudo dnf install -y git curl golang python3.12 java-latest-openjdk npm
elif command -v apt-get >/dev/null 2>&1; then
    sudo apt-get update -qq
    sudo apt-get install -y -qq git curl golang python3 python3-venv python3-pip default-jre npm
elif command -v pacman >/dev/null 2>&1; then
    sudo pacman -Sy --noconfirm git curl go python python-virtualenv jre-openjdk npm
fi

echo "[2/4] Cloning Persona Repositories into ~/.code-warden/engines/..."

clone_or_pull() {
    local repo_url="$1"
    local dest_name="$2"
    local target_path="${ENGINES_DIR}/${dest_name}"

    if [ -d "${target_path}/.git" ]; then
        echo "  -> Updating ${dest_name}..."
        git -C "${target_path}" pull --ff-only || true
    else
        echo "  -> Cloning ${dest_name}..."
        git clone --depth 1 "${repo_url}" "${target_path}"
    fi
}

clone_or_pull "https://github.com/facebook/infer.git" "infer"
clone_or_pull "https://github.com/pmd/pmd.git" "pmd"
clone_or_pull "https://github.com/RetireJS/retire.js.git" "retire.js"
clone_or_pull "https://github.com/semgrep/semgrep-rules.git" "semgrep-rules"
clone_or_pull "https://github.com/protectai/modelscan.git" "modelscan"
clone_or_pull "https://github.com/leondz/garak.git" "garak"
clone_or_pull "https://github.com/trufflesecurity/trufflehog.git" "trufflehog"
clone_or_pull "https://github.com/liamg/traitor.git" "traitor"
clone_or_pull "https://github.com/rhysd/actionlint.git" "actionlint"
clone_or_pull "https://github.com/bridgecrewio/checkov.git" "checkov"
clone_or_pull "https://github.com/google/go-licenses.git" "go-licenses"
clone_or_pull "https://github.com/nexB/scancode-toolkit.git" "scancode-toolkit"
clone_or_pull "https://github.com/sqlfluff/sqlfluff.git" "sqlfluff"
clone_or_pull "https://github.com/schemacrawler/SchemaCrawler.git" "schemacrawler"

echo "[3/4] Building and provisioning engine binaries..."

PYTHON_EXEC="python3"
if command -v python3.12 >/dev/null 2>&1; then
    PYTHON_EXEC="python3.12"
fi

if [ ! -d "${VENV_DIR}" ]; then
    "${PYTHON_EXEC}" -m venv "${VENV_DIR}"
fi
"${VENV_DIR}/bin/pip" install --upgrade pip setuptools wheel --quiet

echo "  -> Installing Python tools (modelscan, checkov, sqlfluff)..."
"${VENV_DIR}/bin/pip" install "${ENGINES_DIR}/modelscan" "${ENGINES_DIR}/checkov" "${ENGINES_DIR}/sqlfluff" --quiet || true

if command -v go >/dev/null 2>&1; then
    echo "  -> Building Go tools (actionlint, trufflehog, go-licenses)..."
    (cd "${ENGINES_DIR}/actionlint" && go build -o "${BIN_DIR}/actionlint" ./cmd/actionlint) || true
    (cd "${ENGINES_DIR}/trufflehog" && go build -o "${BIN_DIR}/trufflehog" .) || true
    (cd "${ENGINES_DIR}/go-licenses" && go build -o "${BIN_DIR}/go-licenses" .) || true
fi

echo "[4/4] Compiling and registering cwd binary..."
cargo build --release --bin cwd
sudo cp target/release/cwd /usr/local/bin/cwd
sudo chmod 755 /usr/local/bin/cwd
sudo chown -R "$(whoami):$(whoami)" "${CODE_WARDEN_HOME}"

echo ""
echo "=== Setup Complete! ==="

# Optional API Key Configuration
read -r -p "Would you like to configure your Gemini API Studio key now? (y/N): " configure_key
if [[ "$configure_key" =~ ^([yY][eE][sS]|[yY])$ ]]; then
    read -r -p "Enter your Gemini API key: " user_key
    if [ -n "$user_key" ]; then
        echo "GEMINI_API_KEY=${user_key}" > "${CODE_WARDEN_HOME}/config.env"
        chmod 600 "${CODE_WARDEN_HOME}/config.env"
        echo "[✔] API key saved to ${CODE_WARDEN_HOME}/config.env"
    fi
else
    echo "You can set your API key anytime later by running: cwd config set-key"
fi

echo ""
echo "Run 'cwd audit' to scan a project or 'cwd update' to pull upstream engines."

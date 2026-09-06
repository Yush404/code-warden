#!/usr/bin/env bash
set -e

echo "=== Installing Code-Warden (cwd) & Persona Engines ==="

CODE_WARDEN_HOME="${HOME}/.code-warden"
ENGINES_DIR="${CODE_WARDEN_HOME}/engines"
BIN_DIR="${CODE_WARDEN_HOME}/bin"
VENV_DIR="${ENGINES_DIR}/venv"

mkdir -p "${ENGINES_DIR}" "${BIN_DIR}" "${CODE_WARDEN_HOME}/memory"

echo "[1/5] Checking and installing system runtimes..."
if command -v dnf >/dev/null 2>&1; then
    sudo dnf install -y git curl golang python3.12 java-latest-openjdk npm
elif command -v apt-get >/dev/null 2>&1; then
    sudo apt-get update -qq
    sudo apt-get install -y -qq git curl golang python3 python3-venv python3-pip default-jre npm
elif command -v pacman >/dev/null 2>&1; then
    sudo pacman -Sy --noconfirm git curl go python python-virtualenv jre-openjdk npm
fi

echo "[2/5] Setting up Ollama & Qwen Coder 2.5 (1.5B)..."
if ! command -v ollama >/dev/null 2>&1; then
    echo "  -> Installing Ollama via official installer..."
    curl -fsSL https://ollama.com/install.sh | sh
fi

# Ensure Ollama daemon is running
if ! curl -s http://127.0.0.1:11434/api/tags >/dev/null 2>&1; then
    echo "  -> Starting Ollama daemon in the background..."
    if command -v systemctl >/dev/null 2>&1 && systemctl is-active --quiet ollama; then
        sudo systemctl restart ollama
    else
        nohup ollama serve >/dev/null 2>&1 &
        sleep 3
    fi
fi

echo "  -> Pulling qwen2.5-coder:1.5b model..."
ollama pull qwen2.5-coder:1.5b

echo "[3/5] Cloning Persona Repositories into ~/.code-warden/engines/..."

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

echo "[4/5] Building and provisioning engine binaries..."

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

echo "[5/5] Compiling and registering cwd binary..."
cargo build --release --bin cwd
sudo cp target/release/cwd /usr/local/bin/cwd
sudo chmod 755 /usr/local/bin/cwd
sudo chown -R "$(whoami):$(whoami)" "${CODE_WARDEN_HOME}"

echo ""
echo "=== Setup Complete! ==="
echo "Select your AI Provider Configuration:"
echo "  1) Hybrid (Recommended) - Use Gemini API with automatic fallback to local Ollama"
echo "  2) Local Only - Exclusively use Ollama (qwen2.5-coder:1.5b)"
echo "  3) Gemini Only - Exclusively use Google AI Studio"
read -r -p "Enter choice [1-3] (default 1): " prov_choice

prov_choice=${prov_choice:-1}

case "$prov_choice" in
    2)
        echo "DEFAULT_PROVIDER=ollama" > "${CODE_WARDEN_HOME}/config.env"
        echo "[✔] Set default provider to local Ollama."
        ;;
    3)
        echo "DEFAULT_PROVIDER=gemini" > "${CODE_WARDEN_HOME}/config.env"
        read -r -p "Enter your Gemini API Studio key: " g_key
        if [ -n "$g_key" ]; then
            echo "GEMINI_API_KEY=${g_key}" >> "${CODE_WARDEN_HOME}/config.env"
        fi
        echo "[✔] Set default provider to Gemini."
        ;;
    *)
        echo "DEFAULT_PROVIDER=hybrid" > "${CODE_WARDEN_HOME}/config.env"
        read -r -p "Enter your Gemini API Studio key (press Enter to skip): " g_key
        if [ -n "$g_key" ]; then
            echo "GEMINI_API_KEY=${g_key}" >> "${CODE_WARDEN_HOME}/config.env"
            echo "[✔] Gemini API key configured with Ollama fallback active."
        else
            echo "[*] No API key entered. Code-Warden will use local Ollama until a key is added."
        fi
        ;;
esac

chmod 600 "${CODE_WARDEN_HOME}/config.env" 2>/dev/null || true
echo ""
echo "Run 'cwd audit' to scan a project or 'cwd config show' to view settings."

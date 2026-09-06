# Code-Warden (`cwd`) 🛡️

<p align="center"> <img src="https://github.com/Yush404/code-warden/blob/b960f9d9c343f4657a3c1db16e81933d1c44bb44/logo.png" alt="Project Logo" width="200"> </p>

Code-Warden (`cwd`) is an open-source, cross-platform static code security auditor and automated codebase remediation CLI built in Rust. It pairs multi-tier deterministic security scanners with persona-driven AI review engines, backed by strict anti-hallucination guardrails, local Ollama integration, and persistent episodic memory.

---

## Architecture Overview

Code-Warden operates across three specialized auditing tiers:

1. **Tier 1: Baseline Static Discovery**
	- Ingests structured JSON diagnostics directly from foundational scanners: **Gitleaks** (secrets), **Semgrep** (AST-based SAST), and **Trivy** (dependency CVEs/lockfiles).
	- Automatically excludes `.env`, `target/`, `.git/`, and `.code-warden/` directories to prevent false positives from build artifacts and local secrets.
2. **Tier 2: Targeted Persona-Triggered Scanners**
	- Automatically activates ecosystem-specific tools from local compiled binaries (`~/.code-warden/bin/`) and managed environments (`~/.code-warden/engines/venv/`):
	  - **DevOps:** `rhysd/actionlint` (GitHub Actions workflows) & `bridgecrewio/checkov` (IaC, Docker, K8s).
	  - **AI / ML:** `protectai/modelscan` (`.pkl`, `.pt`, `.onnx`, `.bin` deserialization exploits).
	  - **Database:** `sqlfluff/sqlfluff` (`.sql` syntax and injection anti-pattern linting).
	  - **Red Team:** `trufflesecurity/trufflehog` (high-entropy verified credential scanner).
3. **Tier 3: Persona RAG & Remediation Knowledge Base**
	- Grounded rule profiles derived from industry-reference standards (Meta's `facebook/infer`, `pmd/pmd`, `RetireJS/retire.js`, `OWASP Top 10`, `leondz/garak`, and `schemacrawler/SchemaCrawler`).

---

## Workspace Structure

The project is structured as a modular Rust Cargo workspace:

- `crates/core`: Shared data models (`Persona`, `UnifiedFinding`, `Severity`), scanner mappings, and target file extensions.
- `crates/scanners`: Execution engines and JSON parsers for Gitleaks, Semgrep, Trivy, Trufflehog, Modelscan, Checkov, and Sqlfluff with cross-platform binary resolution.
- `crates/llm`: Gateway for Google Gemini AI Studio and local Ollama inference, dynamic model discovery, hybrid auto-fallback, and persistent project memory.
- `crates/remediator`: Deterministic code auto-fix runner supporting `cargo fix`, `ruff`, and `@biomejs/biome`.
- `crates/cli`: Main CLI binary (`cwd`) entry point providing `audit`, `update`, `config`, and `uninstall` subcommands.
- `scripts/`: Platform installation and uninstallation scripts for Linux/macOS (`install.sh`, `uninstall.sh`) and native Windows PowerShell (`install.ps1`, `uninstall.ps1`).

---

## Persona Engines

Audit your codebase through seven specialized operational lenses using `--persona <NAME>`:

| Persona | Flag Alias | Backing Frameworks & Rule Standards | Focus Areas |
| :--- | :--- | :--- | :--- |
| **Software Engineer** | `swe` | `facebook/infer`, `pmd/pmd`, `clippy` | Memory safety, resource lifecycle leaks, tight architectural coupling. |
| **Web Developer** | `web` | `RetireJS/retire.js`, `semgrep/p/owasp-top-ten`, `biome` | OWASP Top 10, SSRF, XSS vectors, broken object-level authorization (BOLA). |
| **AI/ML Engineer** | `ai` | `protectai/modelscan`, `leondz/garak` | Pickle/serialization RCE, prompt injection boundaries, inference limits. |
| **Red Team Hacker** | `red-team` | `trufflesecurity/trufflehog`, `liamg/traitor`, `gitleaks` | Active verified secrets, privilege escalation paths, dynamic evaluation holes. |
| **DevOps & Platform** | `devops` | `rhysd/actionlint`, `bridgecrewio/checkov`, `trivy` | Unpinned action tags (40-char SHA enforcement), least-privilege CI tokens, Docker root users. |
| **Legal & OSS** | `oss` | `google/go-licenses`, `nexB/scancode-toolkit` | Copyleft licensing contamination (GPL/AGPL), third-party attribution tracking. |
| **Database Engineer** | `db` | `sqlfluff/sqlfluff`, `schemacrawler/SchemaCrawler` | Dynamic SQL concatenation, unindexed joins, missing connection pool bounds. |

---

## Managed Persona Engines Ecosystem (`~/.code-warden/`)

The installer provisions and maintains 14 open-source security engines and rulesets inside `~/.code-warden/engines/`:

1. `facebook/infer` - Static program analysis and memory leak detection
2. `pmd/pmd` - Multi-language source code anti-pattern analyzer
3. `RetireJS/retire.js` - Vulnerable JavaScript dependency scanner
4. `semgrep/semgrep-rules` - Curated AST-based SAST rule collections
5. `protectai/modelscan` - ML model serialization and tensor payload security
6. `leondz/garak` - Generative AI vulnerability and jailbreak assessment
7. `trufflesecurity/trufflehog` - Real-time secret and cryptographic key scanner
8. `liamg/traitor` - Host and container privilege escalation analyzer
9. `rhysd/actionlint` - GitHub Actions static workflow validator
10. `bridgecrewio/checkov` - Infrastructure as Code (IaC) and container configuration auditor
11. `google/go-licenses` - Go dependency compliance and license extractor
12. `nexB/scancode-toolkit` - Licensing boundary and copyright attribution engine
13. `sqlfluff/sqlfluff` - Dialect-aware SQL linter and injection anti-pattern detector
14. `schemacrawler/SchemaCrawler` - Relational database schema linter and design analyzer

---

## Dual LLM Architecture & Automatic Hybrid Fallback

Code-Warden supports cloud-based and local offline inference:

- **Hybrid Mode (Default):** Attempts synthesis via Google Gemini (auto-selecting the lowest token-cost model). If the API key is not configured, rate-limited (`429`), busy (`503`), or offline, it automatically falls back to local Ollama running `qwen2.5-coder:1.5b`.
- **Local-Only Mode:** Routes all inference directly to local Ollama (`http://127.0.0.1:11434`), requiring zero external network calls or API keys.
- **Gemini-Only Mode:** Uses Google AI Studio exclusively with dynamic model discovery.

---

## Anti-Hallucination & Persistent Memory

- **Strict Factual Grounding:** Prompts run at low temperature (`0.1`) with explicit instructions forbidding speculative syntax or imaginary imports. If file context is insufficient to create a deterministic patch, the engine outputs `INSUFFICIENT_CONTEXT`.
- **Persistent Memory (`.code-warden/memory/`):**
  - `session_history.md`: Chronological log of audit runs, detected findings, and remediations.
  - `memory_state.json`: Machine-readable state tracking `total_audits`, `resolved_rule_ids`, and `ignored_rule_ids` to eliminate redundant warnings on subsequent runs.
- **Safe Remediation:** `--action fix` runs deterministic auto-fixers (`cargo fix`, `ruff check --fix`, `biome check --write`) and tracks resolutions only after fixes are executed.

---

## Operating System Compatibility & Windows Parity

| Platform | Native Compatibility | Notes & Setup |
| :--- | :--- | :--- |
| **Fedora / RHEL** | 100% Native | Supported out-of-the-box via `scripts/install.sh`. |
| **Debian / Ubuntu** | 100% Native | Supported out-of-the-box via `scripts/install.sh`. |
| **Arch Linux** | 100% Native | Supported out-of-the-box via `scripts/install.sh`. |
| **Windows 11 / 10** | 100% Native & WSL2 | Supported via cross-platform path abstraction & `scripts/install.ps1`. |

- **Cross-Platform Path Abstraction:** Dynamic switching between `USERPROFILE` and `HOME`.
- **Virtualenv Path Handling:** Switches between Windows `venv\Scripts\` and Unix `venv/bin/` for Python tooling (`modelscan`, `checkov`, `sqlfluff`).
- **Binary Suffix Handling:** Appends `.exe` suffixes to tool calls (`actionlint.exe`, `trufflehog.exe`, `cwd.exe`) on Windows targets.
- **Cross-Compilation Verified:** Codebase verifies cleanly against `x86_64-pc-windows-gnu` via MinGW.

---

## Installation

### Linux & macOS (Bash)
Run the master installer script:

```bash
./scripts/install.sh
```

The script will:

1. Install system runtimes (Git, Go, Python 3.12, Java, Node.js).
2. Install Ollama and pull `qwen2.5-coder:1.5b`.
3. Clone and provision all 14 engine repositories in `~/.code-warden/engines/`.
4. Compile and install `cwd` to `/usr/local/bin/cwd`.
5. Offer an interactive menu to set your preferred provider (Hybrid, Local, or Gemini).

### Windows (PowerShell as Administrator)
Install dependencies via Winget, configure Ollama, pull `qwen2.5-coder:1.5b`, clone engines, and register `cwd.exe` to your `PATH`:

PowerShell

```
powershell -ExecutionPolicy Bypass -File .\scripts\install.ps1
```

## Configuration
Manage settings without editing configuration files manually:

Bash

```
# Set active AI provider (options: hybrid, ollama, gemini)
cwd config set-provider --provider hybrid

# Set Gemini API key interactively
cwd config set-key

# Set Gemini API key directly
cwd config set-key --key "AIzaSy..."

# View current configuration and provider status
cwd config show

# Remove stored global API key
cwd config unset-key
```

## Usage
Bash

```
# Run audit using default Red Team persona (with hybrid auto-fallback)
cwd audit --target .

# Run audit using a specific persona in report-only mode
cwd audit --target . --persona swe --action report

# Force local Ollama execution
cwd audit --target . --provider ollama

# Pull latest updates for all 14 cloned persona repositories
cwd update

# Inspect persistent session memory
cat .code-warden/memory/session_history.md
```

## Complete Uninstallation
Code-Warden can be removed completely via the CLI or standalone scripts:

1. **Via CLI subcommand:**

Bash

```
cwd uninstall
# Or bypass interactive prompt:
cwd uninstall --yes
```
2. **Via standalone scripts:**

- Linux/macOS: `./scripts/uninstall.sh`
- Windows: `powershell -ExecutionPolicy Bypass -File .\scripts\uninstall.ps1`

## CLI Options
**Command / Subcommand** | **Flag** | **Description** | **Default**
--- | --- | --- | ---
`cwd audit` | — | Run security scans and persona synthesis | `cwd audit`
`cwd audit` | `-t, --target <DIR>` | Target codebase directory to scan | `.`
`cwd audit` | `--persona <NAME>` | Persona lens: `swe`, `web`, `ai`, `red-team`, `devops`, `oss`, `db` | `red-team`
`cwd audit` | `--action <ACTION>` | Execution strategy: `report`, `fix`, or `both` | `both`
`cwd audit` | `--provider <PROV>` | AI provider: `hybrid`, `gemini`, or `ollama` | `hybrid`
`cwd audit` | `--model <MODEL>` | Gemini model tag (or `auto` for lowest-token discovery) | `auto`
`cwd audit` | `--ollama-model <M>` | Local Ollama model tag | `qwen2.5-coder:1.5b`
`cwd audit` | `--api-key <KEY>` | Explicit Gemini API Studio key override | Resolved via config
`cwd update` | — | Pull latest commits for all 14 engines in `~/.code-warden/engines/` | —
`cwd config set-provider` | `--provider <P>` | Set default provider: `hybrid`, `gemini`, or `ollama` | —
`cwd config set-key` | `[--key <KEY>]` | Save API key to global `~/.code-warden/config.env` | —
`cwd config show` | — | Display current API key and provider preference | —
`cwd config unset-key` | — | Remove stored global API key | —
`cwd uninstall` | `[-y, --yes]` | Remove `cwd` binary and purge `~/.code-warden` | —

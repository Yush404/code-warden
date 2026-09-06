# Code-Warden (`cwd`) 🛡️

Code-Warden (`cwd`) is an open-source, cross-platform static security auditor and automated codebase remediation CLI written in Rust. It pairs multi-tier deterministic security scanners with persona-driven AI review engines, backed by strict anti-hallucination guardrails and persistent episodic memory.

---

## Architecture Overview

Code-Warden operates across three specialized auditing tiers to maximize precision without hallucination or context window exhaustion:

1. **Tier 1: Baseline Static Discovery**
   - Ingests structured JSON diagnostics directly from foundational scanners: **Gitleaks** (secrets), **Semgrep** (AST-based SAST), and **Trivy** (dependency CVEs/lockfiles).
   - Automatically excludes local `.env` configuration files to prevent false-positive secret leakage alerts on local development setups.
2. **Tier 2: Targeted Persona-Triggered Scanners**
   - Automatically activates ecosystem-specific tools from local compiled binaries (`~/.code-warden/bin/`) and managed environments (`~/.code-warden/engines/venv/`):
     - **DevOps:** `rhysd/actionlint` (GitHub Actions `.github/workflows/`) & `bridgecrewio/checkov` (IaC, Docker, K8s).
     - **AI / ML:** `protectai/modelscan` (`.pkl`, `.pt`, `.onnx`, `.bin` deserialization exploits).
     - **Database:** `sqlfluff/sqlfluff` (`.sql` syntax and anti-pattern linting).
     - **Red Team:** `trufflesecurity/trufflehog` (high-entropy verified credential scanner).
3. **Tier 3: Persona RAG & Remediation Knowledge Base**
   - Grounded rule profiles derived from industry-reference standards (Meta's `facebook/infer`, `pmd/pmd`, `RetireJS/retire.js`, `OWASP Top 10`, `leondz/garak`, and `schemacrawler/SchemaCrawler`).

---

## Workspace Structure

The project is structured as a modular Rust Cargo workspace:

- `crates/core`: Shared data models (`Persona`, `UnifiedFinding`, `Severity`), scanner mappings, and target file extensions.
- `crates/scanners`: Execution engines and JSON parsers for Gitleaks, Semgrep, Trivy, Trufflehog, Modelscan, Checkov, and Sqlfluff with cross-platform binary resolution.
- `crates/llm`: Gateway for Google Gemini AI Studio and local Ollama inference, dynamic model discovery, persona prompt templates, and persistent project memory.
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

## AI Studio Integration & Dynamic Model Discovery

Code-Warden integrates natively with Google Gemini via Google AI Studio and supports local offline inference via Ollama:

- **Dynamic Lowest-Token Model Discovery:** `cwd` queries available models under the provided API key at runtime, filtering for `generateContent` support and auto-selecting the most lightweight, token-efficient tier available (e.g., `gemini-flash-lite-latest` or `gemini-flash` variants).
- **Resilient Request Dispatch:** Built-in exponential backoff automatically retries requests on transient upstream server capacity spikes (`503 UNAVAILABLE` / `429 RESOURCE_EXHAUSTED`).
- **Flexible Credential Hierarchy:** Credentials resolve hierarchically:
  1. CLI parameter: `--api-key <KEY>`
  2. Local project configuration: `.env` (`GEMINI_API_KEY=...`)
  3. Global persistent configuration: `~/.code-warden/config.env`

---

## Anti-Hallucination & Persistent Memory

- **Strict Factual Grounding:** Prompts run at low temperature (`0.1`) with explicit instructions forbidding speculative syntax or imaginary imports. If file context is insufficient to create a deterministic patch, the engine outputs `INSUFFICIENT_CONTEXT`.
- **Persistent Memory (`.code-warden/memory/`):**
  - `session_history.md`: Chronological log of audit runs, detected findings, and remediations.
  - `memory_state.json`: Machine-readable state tracking `total_audits`, `resolved_rule_ids`, and `ignored_rule_ids` to eliminate redundant warnings on subsequent runs.
- **Safe Remediation:** `--action fix` runs deterministic auto-fixers (`cargo fix`, `ruff check --fix`, `biome check --write`) and tracks resolutions only after fixes are executed.

---

## Operating System Compatibility & Windows Support

| Platform | Native Compatibility | Notes & Setup |
| :--- | :--- | :--- |
| **Fedora / RHEL** | 100% Native | Fully supported out-of-the-box via `scripts/install.sh`. |
| **Debian / Ubuntu** | 100% Native | Fully supported out-of-the-box via `scripts/install.sh`. |
| **Arch Linux** | 100% Native | Fully supported out-of-the-box via `scripts/install.sh`. |
| **Windows 11 / 10** | 100% Native & WSL2 | Fully supported via cross-platform path abstraction & `scripts/install.ps1`. |

### Native Windows Architectural Support
- **Cross-Platform Path Abstraction:** Dynamic switching between `USERPROFILE` and `HOME`, ensuring paths resolve cleanly across Windows and POSIX systems.
- **Virtualenv Path Handling:** Automatically switches between Windows `venv\Scripts\` and Unix `venv/bin/` when executing Python tooling (`modelscan`, `checkov`, `sqlfluff`).
- **Binary Suffix Handling:** Automatically appends `.exe` suffixes to tool calls (`actionlint.exe`, `trufflehog.exe`, `cwd.exe`) on Windows targets.
- **Cross-Compilation Verified:** Codebase verifies cleanly against `x86_64-pc-windows-gnu` via MinGW.

---

## Installation

### Linux & macOS (Bash)
Run the master installer script to provision runtime dependencies (Go, Python 3.12, OpenJDK, npm), clone the 14 engine repositories into `~/.code-warden/engines/`, compile binaries into `~/.code-warden/bin/`, and install the `cwd` executable to `/usr/local/bin/cwd`:

`./scripts/install.sh`

During installation, the script offers an interactive prompt to set up your Gemini API Studio key immediately, saving it securely to `~/.code-warden/config.env` with `600` permissions. You can also skip this and configure it later.

Remote installation:
`curl -fsSL https://raw.githubusercontent.com/yush404/code-warden/main/scripts/install.sh | bash`

### Windows (PowerShell as Administrator)
Install system runtimes (Git, Go, Python 3.12, Node.js, OpenJDK 17, Ollama, VS Build Tools) via Winget, clone the engines, compile tools, register `cwd.exe` to `%LOCALAPPDATA%\Programs\CodeWarden`, and add it to your user `PATH`:

`powershell -ExecutionPolicy Bypass -File .\scripts\install.ps1`

Remote installation:
`irm https://raw.githubusercontent.com/yush404/code-warden/main/scripts/install.ps1 | iex`

---

## Configuration

Manage your global API keys and settings without editing files manually:

- **Set API key interactively:**
  `cwd config set-key`
- **Set API key directly via flag:**
  `cwd config set-key --key "AIzaSy..."`
- **Display active key status (masked):**
  `cwd config show`
- **Remove stored global key:**
  `cwd config unset-key`

---

## Usage

Audit target repository using default Red Team persona and auto-selected Gemini model:
`cwd audit --target .`

Run as DevOps Engineer with report-only mode:
`cwd audit --target . --persona devops --action report`

Run as Software Architect with automatic fix application:
`cwd audit --target . --persona swe --action both`

Pull upstream updates across all 14 cloned persona repositories:
`cwd update`

Inspect persistent session memory:
`cat .code-warden/memory/session_history.md`

---

## Complete Uninstallation

Code-Warden provides clean uninstallation mechanisms that remove the `cwd` binary, purge `~/.code-warden` (all cloned repositories, Python virtual environments, compiled Go binaries, and memory files), while leaving host system runtimes intact:

1. **Via CLI subcommand:**
  `cwd uninstall`
  *(Or bypass interactive confirmation with: `cwd uninstall --yes`)*

2. **Via standalone uninstallation script:**
  - Linux/macOS: `./scripts/uninstall.sh`
  - Windows: `powershell -ExecutionPolicy Bypass -File .\scripts\uninstall.ps1`

---

## CLI Options

| Command / Subcommand | Flag | Description | Default |
| :--- | :--- | :--- | :--- |
| `cwd audit` | — | Run security scans and persona synthesis | — |
| `cwd audit` | `-t, --target <DIR>` | Target codebase directory to scan | `.` |
| `cwd audit` | `--persona <NAME>` | Persona lens: `swe`, `web`, `ai`, `red-team`, `devops`, `oss`, `db` | `red-team` |
| `cwd audit` | `--action <ACTION>` | Execution strategy: `report`, `fix`, or `both` | `both` |
| `cwd audit` | `--provider <PROV>` | AI inference provider: `gemini` or `ollama` | `gemini` |
| `cwd audit` | `--model <MODEL>` | Model tag (or `auto` for lowest-token discovery) | `auto` |
| `cwd audit` | `--api-key <KEY>` | Gemini API Studio key | Resolved via config/env |
| `cwd update` | — | Pull latest commits for all 14 engines in `~/.code-warden/engines/` | — |
| `cwd config set-key` | `[--key <KEY>]` | Save API key to global `~/.code-warden/config.env` | — |
| `cwd config show` | — | Display current API key masked verification status | — |
| `cwd config unset-key`| — | Remove stored global API key | — |
| `cwd uninstall` | `[-y, --yes]` | Remove `cwd` binary and purge `~/.code-warden` | — |

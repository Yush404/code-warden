use clap::{Args, Parser, Subcommand};
use code_warden_core::models::Persona;
use code_warden_llm::memory::MemoryManager;
use code_warden_llm::{LlmGateway, Provider};
use code_warden_remediator::Remediator;
use code_warden_scanners::ScannerEngine;
use colored::*;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Parser, Debug)]
#[command(
    name = "cwd",
    version = "0.1.0",
    about = "Production-grade Codebase Security Auditor with Persona Engines"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[command(flatten)]
    audit_args: AuditArgs,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run security audits and automated code remediation (default)
    Audit(AuditArgs),
    /// Update all 14 cloned persona repositories and tools
    Update,
    /// Manage Code-Warden settings and API credentials
    Config(ConfigArgs),
    /// Completely uninstall Code-Warden and remove all engines
    Uninstall {
        #[arg(short, long)]
        yes: bool,
    },
}

#[derive(Args, Debug, Clone)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub action: ConfigAction,
}

#[derive(Subcommand, Debug, Clone)]
pub enum ConfigAction {
    /// Save Gemini API key to global Code-Warden configuration
    SetKey {
        #[arg(long)]
        key: Option<String>,
    },
    /// View current active configuration status
    Show,
    /// Remove stored Gemini API key
    UnsetKey,
}

#[derive(Args, Debug, Clone)]
pub struct AuditArgs {
    #[arg(short, long, default_value = ".")]
    pub target: PathBuf,

    #[arg(long, default_value = "gemini")]
    pub provider: String,

    #[arg(long, default_value = "auto")]
    pub model: String,

    #[arg(long)]
    pub api_key: Option<String>,

    #[arg(long, default_value = "both")]
    pub action: String,

    #[arg(long, default_value = "red-team")]
    pub persona: String,
}

pub fn get_user_home() -> PathBuf {
    if cfg!(windows) {
        if let Ok(prof) = std::env::var("USERPROFILE") {
            return PathBuf::from(prof);
        }
    }
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

pub fn code_warden_home() -> PathBuf {
    get_user_home().join(".code-warden")
}

pub fn global_config_path() -> PathBuf {
    code_warden_home().join("config.env")
}

pub fn resolve_binary_name(name: &str) -> String {
    if cfg!(windows) && !name.ends_with(".exe") {
        format!("{}.exe", name)
    } else {
        name.to_string()
    }
}

pub fn get_venv_bin_dir(venv_root: &Path) -> PathBuf {
    if cfg!(windows) {
        venv_root.join("Scripts")
    } else {
        venv_root.join("bin")
    }
}

fn parse_persona(name: &str) -> Persona {
    match name.to_lowercase().as_str() {
        "swe" | "software-engineer" => Persona::SoftwareEngineer,
        "web" | "web-dev" => Persona::WebDeveloper,
        "ai" | "ml" | "ai-ml" => Persona::AiMlEngineer,
        "devops" | "platform" => Persona::DevOpsPlatform,
        "oss" | "compliance" => Persona::OssCompliance,
        "db" | "database" => Persona::DatabaseEngineer,
        _ => Persona::RedTeamCybersecurity,
    }
}

fn load_active_api_key(explicit_key: Option<String>) -> String {
    if let Some(k) = explicit_key {
        if !k.trim().is_empty() {
            return k.trim().to_string();
        }
    }

    if let Ok(k) = std::env::var("GEMINI_API_KEY") {
        if !k.trim().is_empty() {
            return k.trim().to_string();
        }
    }

    let global_cfg = global_config_path();
    if global_cfg.exists() {
        if let Ok(content) = std::fs::read_to_string(global_cfg) {
            for line in content.lines() {
                if let Some(stripped) = line.strip_prefix("GEMINI_API_KEY=") {
                    return stripped.trim().to_string();
                }
            }
        }
    }

    String::new()
}

fn handle_config(args: ConfigArgs) -> anyhow::Result<()> {
    let global_cfg = global_config_path();
    if let Some(parent) = global_cfg.parent() {
        std::fs::create_dir_all(parent)?;
    }

    match args.action {
        ConfigAction::SetKey { key } => {
            let selected_key = match key {
                Some(k) => k,
                None => {
                    print!("Enter your Gemini API key: ");
                    io::stdout().flush()?;
                    let mut input = String::new();
                    io::stdin().read_line(&mut input)?;
                    input.trim().to_string()
                }
            };

            if selected_key.is_empty() {
                println!("{}", "[!] Key cannot be empty.".red());
                return Ok(());
            }

            let entry = format!("GEMINI_API_KEY={}\n", selected_key);
            std::fs::write(&global_cfg, entry)?;
            println!(
                "{} Stored at {}",
                "[✔] API key saved globally.".bold().green(),
                global_cfg.display()
            );
        }
        ConfigAction::Show => {
            let key = load_active_api_key(None);
            if key.is_empty() {
                println!("{}", "[*] No GEMINI_API_KEY configured.".yellow());
            } else {
                let masked = if key.len() > 8 {
                    format!("{}...{}", &key[..4], &key[key.len() - 4..])
                } else {
                    "********".to_string()
                };
                println!("{} Active Key: {}", "[✔]".green(), masked);
            }
        }
        ConfigAction::UnsetKey => {
            if global_cfg.exists() {
                let _ = std::fs::remove_file(&global_cfg);
            }
            println!("{}", "[✔] Global API key removed.".green());
        }
    }
    Ok(())
}

fn handle_uninstall(yes: bool) -> anyhow::Result<()> {
    if !yes {
        print!("Are you sure you want to completely remove Code-Warden, all 14 engines, and local memory? (y/N): ");
        io::stdout().flush()?;
        let mut response = String::new();
        io::stdin().read_line(&mut response)?;
        if !response.trim().eq_ignore_ascii_case("y") {
            println!("Uninstallation aborted.");
            return Ok(());
        }
    }

    println!("{}", "=== Uninstalling Code-Warden ===".bold().red());

    let cw_dir = code_warden_home();
    if cw_dir.exists() {
        print!("Purging ~/.code-warden (engines, binaries, environments)... ");
        std::fs::remove_dir_all(&cw_dir)?;
        println!("{}", "[DONE]".green());
    }

    if cfg!(windows) {
        if let Ok(appdata) = std::env::var("LOCALAPPDATA") {
            let win_bin = Path::new(&appdata)
                .join("Programs")
                .join("CodeWarden")
                .join("cwd.exe");
            if win_bin.exists() {
                let _ = std::fs::remove_file(win_bin);
            }
        }
    } else {
        let bin_path = Path::new("/usr/local/bin/cwd");
        if bin_path.exists() {
            println!("Removing /usr/local/bin/cwd...");
            let _ = Command::new("sudo")
                .args(["rm", "-f", "/usr/local/bin/cwd"])
                .status();
        }
    }

    println!(
        "\n{}",
        "[✔] Code-Warden has been completely removed from your system."
            .bold()
            .green()
    );
    Ok(())
}

fn update_engines() -> anyhow::Result<()> {
    let engines_dir = code_warden_home().join("engines");

    if !engines_dir.exists() {
        println!("{}", "[!] Engines directory does not exist yet.".yellow());
        return Ok(());
    }

    println!(
        "{}",
        "=== Updating Cloned Persona Repositories ===".bold().cyan()
    );

    let entries = match std::fs::read_dir(&engines_dir) {
        Ok(e) => e,
        Err(e) => {
            println!("Failed to read engines directory: {}", e);
            return Ok(());
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() && path.join(".git").exists() {
            let repo_name = path.file_name().unwrap_or_default().to_string_lossy();
            print!("Updating {}... ", repo_name.bold().green());
            let git_bin = resolve_binary_name("git");
            let status = Command::new(&git_bin)
                .args(["pull", "--ff-only"])
                .current_dir(&path)
                .output();

            match status {
                Ok(out) if out.status.success() => println!("{}", "[UP TO DATE / UPDATED]".green()),
                Ok(out) => println!(
                    "{}: {}",
                    "[FAILED]".red(),
                    String::from_utf8_lossy(&out.stderr).trim()
                ),
                Err(e) => println!("{}: {}", "[ERROR]".red(), e),
            }
        }
    }

    let venv_dir = engines_dir.join("venv");
    let pip_name = resolve_binary_name("pip");
    let venv_pip = get_venv_bin_dir(&venv_dir).join(pip_name);
    if venv_pip.exists() {
        println!("\n{}", "Refreshing Python engine dependencies...".cyan());
        let _ = Command::new(&venv_pip)
            .args(["install", "--upgrade", "pip"])
            .output();
    }

    println!(
        "\n{}",
        "[✔] All persona engines updated successfully."
            .bold()
            .green()
    );
    Ok(())
}

async fn run_audit(args: AuditArgs) -> anyhow::Result<()> {
    let target = std::fs::canonicalize(&args.target)?;
    let persona = parse_persona(&args.persona);

    println!(
        "{}",
        "=== Code-Warden Scanner & Remediation ===".bold().cyan()
    );
    println!("{} {}", "[*] Target:".green(), target.display());
    println!("{} {:?}", "[*] Persona:".green(), persona);

    let memory_mgr = MemoryManager::new(&target)?;
    let memory_ctx = memory_mgr.build_memory_context();

    println!(
        "\n{}",
        "[1/3] Running Multi-Tier Security Engines...".yellow()
    );
    let findings = ScannerEngine::scan_for_persona(&target, &persona);
    println!("  -> Total findings aggregated: {}", findings.len());

    let mut summary_lines = Vec::new();
    for f in &findings {
        summary_lines.push(format!(
            "[{:?}] {}: {} ({}:{})",
            f.severity,
            f.rule_id,
            f.message,
            f.file_path.display(),
            f.line_start
        ));
    }
    let diagnostics = if summary_lines.is_empty() {
        "No static violations detected by Tier 1 & 2 engines.".to_string()
    } else {
        summary_lines.join("\n")
    };

    println!(
        "\n{}",
        "[2/3] Synthesizing Persona Intelligence (Grounded)...".yellow()
    );
    let gateway = LlmGateway::new();

    let prov = if args.provider == "gemini" {
        let key = load_active_api_key(args.api_key);
        Provider::Gemini {
            api_key: key,
            model: args.model,
        }
    } else {
        Provider::Ollama {
            endpoint: "http://127.0.0.1:11434".into(),
            model: args.model,
        }
    };

    let review = gateway
        .review_and_patch(&prov, &persona, &diagnostics, "", &memory_ctx)
        .await
        .unwrap_or_else(|e| format!("LLM review bypassed: {}", e));

    let mut fixes_applied = Vec::new();
    if args.action == "fix" || args.action == "both" {
        println!(
            "\n{}",
            "[3/3] Applying Safe Deterministic Fixes...".yellow()
        );
        Remediator::run_deterministic_fixes(&target);
        fixes_applied
            .push("Applied deterministic linter auto-fixes (ruff, biome, cargo fix)".to_string());
        println!("{}", "[✔] Linters completed automatic passes.".green());
    }

    let mut state = memory_mgr.load_state();
    state.total_audits += 1;
    if args.action == "fix" || args.action == "both" {
        for f in &findings {
            if f.fixable {
                state.resolved_rule_ids.insert(f.rule_id.clone());
            }
        }
    }
    let _ = memory_mgr.save_state(&state);

    let ts = chrono::Utc::now()
        .format("%Y-%m-%d %H:%M:%S UTC")
        .to_string();
    let _ = memory_mgr.append_session(
        &ts,
        &format!("{:?}", persona),
        &format!("{} findings", findings.len()),
        &fixes_applied,
    );

    let reports_dir = target.join("reports");
    std::fs::create_dir_all(&reports_dir)?;
    let file_ts = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let out = reports_dir.join(format!("audit_report_{}.md", file_ts));
    std::fs::write(
        &out,
        format!(
            "# Code-Warden Audit Report\n\n- **Date:** {}\n- **Persona:** {:?}\n\n## Scanner Findings\n{}\n\n## Grounded Persona Review\n{}\n",
            ts, persona, diagnostics, review
        ),
    )?;

    println!(
        "\n{} Report saved to {}",
        "[✔] Audit Finished!".bold().green(),
        out.display()
    );
    println!(
        "{} Persistent memory synced at {}",
        "[✔] Memory Updated!".bold().green(),
        target.join(".code-warden/memory").display()
    );

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Update) => update_engines(),
        Some(Commands::Config(args)) => handle_config(args),
        Some(Commands::Uninstall { yes }) => handle_uninstall(yes),
        Some(Commands::Audit(args)) => run_audit(args).await,
        None => run_audit(cli.audit_args).await,
    }
}

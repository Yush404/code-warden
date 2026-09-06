pub mod linters;

use anyhow::{Context, Result};
use code_warden_core::models::{Persona, ScannerSource, Severity, UnifiedFinding};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::NamedTempFile;

pub struct ScannerEngine;

impl ScannerEngine {
    /// Resolve binary location: prioritize ~/.code-warden/bin and ~/.code-warden/engines/venv/bin
    fn resolve_tool_bin(tool_name: &str) -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let managed_bin = Path::new(&home)
            .join(".code-warden")
            .join("bin")
            .join(tool_name);
        if managed_bin.exists() {
            return managed_bin;
        }

        let venv_bin = Path::new(&home)
            .join(".code-warden")
            .join("engines")
            .join("venv")
            .join("bin")
            .join(tool_name);
        if venv_bin.exists() {
            return venv_bin;
        }

        // Fallback to system path
        PathBuf::from(tool_name)
    }

    // ==========================================
    // Tier 1: Foundation Scanners
    // ==========================================

    pub fn scan_gitleaks(target: &Path) -> Result<Vec<UnifiedFinding>> {
        let tool = Self::resolve_tool_bin("gitleaks");
        let temp_report = NamedTempFile::new()?;
        let report_path = temp_report.path().to_string_lossy().to_string();

        let _ = Command::new(tool)
            .args([
                "detect",
                "--source",
                &target.to_string_lossy(),
                "--no-git",
                "--report-format",
                "json",
                "--report-path",
                &report_path,
                "--redact",
            ])
            .output();

        if !Path::new(&report_path).exists() {
            return Ok(Vec::new());
        }

        let content = std::fs::read_to_string(&report_path)?;
        let raw_json: Value = serde_json::from_str(&content).unwrap_or(Value::Array(vec![]));
        let mut findings = Vec::new();

        if let Some(leaks) = raw_json.as_array() {
            for item in leaks {
                let file_str = item["File"].as_str().unwrap_or("unknown");
                if file_str.ends_with(".env") || file_str.contains("/.env") {
                    continue;
                }
                findings.push(UnifiedFinding {
                    id: format!("GITLEAKS-{}", uuid::Uuid::new_v4()),
                    source: ScannerSource::Gitleaks,
                    rule_id: item["RuleID"].as_str().unwrap_or("secret").to_string(),
                    severity: Severity::Critical,
                    message: item["Description"]
                        .as_str()
                        .unwrap_or("Secret leak detected")
                        .to_string(),
                    file_path: PathBuf::from(item["File"].as_str().unwrap_or("unknown")),
                    line_start: item["StartLine"].as_u64().unwrap_or(1) as usize,
                    line_end: item["EndLine"].as_u64().map(|l| l as usize),
                    matched_content: item["Match"].as_str().map(|s| s.to_string()),
                    fixable: false,
                    suggested_patch: None,
                });
            }
        }
        Ok(findings)
    }

    pub fn scan_semgrep(target: &Path) -> Result<Vec<UnifiedFinding>> {
        let tool = Self::resolve_tool_bin("semgrep");
        let output = Command::new(tool)
            .args([
                "scan",
                "--config=p/default",
                "--json",
                &target.to_string_lossy(),
            ])
            .output()
            .context("Failed to execute semgrep.")?;

        let parsed: Value = serde_json::from_slice(&output.stdout).unwrap_or(Value::Null);
        let mut findings = Vec::new();

        if let Some(results) = parsed.get("results").and_then(|r| r.as_array()) {
            for item in results {
                findings.push(UnifiedFinding {
                    id: format!("SEMGREP-{}", uuid::Uuid::new_v4()),
                    source: ScannerSource::Semgrep,
                    rule_id: item["check_id"].as_str().unwrap_or("sast").to_string(),
                    severity: Severity::High,
                    message: item["extra"]["message"].as_str().unwrap_or("").to_string(),
                    file_path: PathBuf::from(item["path"].as_str().unwrap_or("")),
                    line_start: item["start"]["line"].as_u64().unwrap_or(1) as usize,
                    line_end: item["end"]["line"].as_u64().map(|l| l as usize),
                    matched_content: item["extra"]["lines"].as_str().map(|s| s.to_string()),
                    fixable: item["extra"]["fix"].is_string(),
                    suggested_patch: item["extra"]["fix"].as_str().map(|s| s.to_string()),
                });
            }
        }
        Ok(findings)
    }

    pub fn scan_trivy(target: &Path) -> Result<Vec<UnifiedFinding>> {
        let tool = Self::resolve_tool_bin("trivy");
        let output = Command::new(tool)
            .args([
                "fs",
                "--format",
                "json",
                "--security-checks",
                "vuln",
                &target.to_string_lossy(),
            ])
            .output()
            .context("Failed to execute trivy.")?;

        let parsed: Value = serde_json::from_slice(&output.stdout).unwrap_or(Value::Null);
        let mut findings = Vec::new();

        if let Some(results) = parsed.get("Results").and_then(|r| r.as_array()) {
            for grp in results {
                let target_path = grp["Target"].as_str().unwrap_or("dependencies");
                if let Some(vulns) = grp.get("Vulnerabilities").and_then(|v| v.as_array()) {
                    for vuln in vulns {
                        findings.push(UnifiedFinding {
                            id: format!("TRIVY-{}", uuid::Uuid::new_v4()),
                            source: ScannerSource::Trivy,
                            rule_id: vuln["VulnerabilityID"]
                                .as_str()
                                .unwrap_or("CVE")
                                .to_string(),
                            severity: Severity::High,
                            message: vuln["Title"]
                                .as_str()
                                .unwrap_or("Dependency vulnerability")
                                .to_string(),
                            file_path: PathBuf::from(target_path),
                            line_start: 1,
                            line_end: None,
                            matched_content: None,
                            fixable: vuln["FixedVersion"].is_string(),
                            suggested_patch: vuln["FixedVersion"]
                                .as_str()
                                .map(|v| format!("Upgrade to {}", v)),
                        });
                    }
                }
            }
        }
        Ok(findings)
    }

    // ==========================================
    // Tier 2: Targeted Persona Scanners
    // ==========================================

    /// DevOps: actionlint
    pub fn scan_actionlint(target: &Path) -> Result<Vec<UnifiedFinding>> {
        let workflows_dir = target.join(".github").join("workflows");
        if !workflows_dir.exists() {
            return Ok(Vec::new());
        }

        let tool = Self::resolve_tool_bin("actionlint");
        let output = Command::new(tool)
            .args(["-format", "{{json .}}"])
            .current_dir(target)
            .output();

        let mut findings = Vec::new();
        if let Ok(out) = output {
            let parsed: Value = serde_json::from_slice(&out.stdout).unwrap_or(Value::Array(vec![]));
            if let Some(arr) = parsed.as_array() {
                for item in arr {
                    findings.push(UnifiedFinding {
                        id: format!("ACTIONLINT-{}", uuid::Uuid::new_v4()),
                        source: ScannerSource::Actionlint,
                        rule_id: item["kind"].as_str().unwrap_or("ci-error").to_string(),
                        severity: Severity::High,
                        message: item["message"].as_str().unwrap_or("").to_string(),
                        file_path: PathBuf::from(item["filepath"].as_str().unwrap_or("")),
                        line_start: item["line"].as_u64().unwrap_or(1) as usize,
                        line_end: None,
                        matched_content: item["snippet"].as_str().map(|s| s.to_string()),
                        fixable: false,
                        suggested_patch: None,
                    });
                }
            }
        }
        Ok(findings)
    }

    /// DevOps: Checkov IaC
    pub fn scan_checkov(target: &Path) -> Result<Vec<UnifiedFinding>> {
        let tool = Self::resolve_tool_bin("checkov");
        let output = Command::new(tool)
            .args([
                "-d",
                &target.to_string_lossy(),
                "-o",
                "json",
                "--compact",
                "--quiet",
            ])
            .output();

        let mut findings = Vec::new();
        if let Ok(out) = output {
            let parsed: Value = serde_json::from_slice(&out.stdout).unwrap_or(Value::Null);
            let checks = if parsed.is_array() {
                parsed.as_array().cloned().unwrap_or_default()
            } else if let Some(arr) = parsed
                .get("results")
                .and_then(|r| r.get("failed_checks"))
                .and_then(|f| f.as_array())
            {
                arr.clone()
            } else {
                vec![]
            };

            for check in checks {
                findings.push(UnifiedFinding {
                    id: format!("CHECKOV-{}", uuid::Uuid::new_v4()),
                    source: ScannerSource::Checkov,
                    rule_id: check["check_id"].as_str().unwrap_or("CKV_IAC").to_string(),
                    severity: Severity::High,
                    message: check["check_name"]
                        .as_str()
                        .unwrap_or("IaC misconfiguration")
                        .to_string(),
                    file_path: PathBuf::from(check["file_path"].as_str().unwrap_or("")),
                    line_start: check["file_line_range"]
                        .get(0)
                        .and_then(|v| v.as_u64())
                        .unwrap_or(1) as usize,
                    line_end: check["file_line_range"]
                        .get(1)
                        .and_then(|v| v.as_u64())
                        .map(|v| v as usize),
                    matched_content: None,
                    fixable: false,
                    suggested_patch: None,
                });
            }
        }
        Ok(findings)
    }

    /// Red Team: TruffleHog Verified Secrets
    pub fn scan_trufflehog(target: &Path) -> Result<Vec<UnifiedFinding>> {
        let tool = Self::resolve_tool_bin("trufflehog");
        let output = Command::new(tool)
            .args([
                "filesystem",
                &target.to_string_lossy(),
                "--json",
                "--only-verified",
            ])
            .output();

        let mut findings = Vec::new();
        if let Ok(out) = output {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for line in stdout.lines() {
                if let Ok(item) = serde_json::from_str::<Value>(line) {
                    findings.push(UnifiedFinding {
                        id: format!("TRUFFLEHOG-{}", uuid::Uuid::new_v4()),
                        source: ScannerSource::Trufflehog,
                        rule_id: item["DetectorName"]
                            .as_str()
                            .unwrap_or("verified-secret")
                            .to_string(),
                        severity: Severity::Critical,
                        message: format!(
                            "Verified exposed credential: {}",
                            item["DetectorName"].as_str().unwrap_or("secret")
                        ),
                        file_path: PathBuf::from(
                            item["SourceMetadata"]["Data"]["Filesystem"]["file"]
                                .as_str()
                                .unwrap_or("unknown"),
                        ),
                        line_start: item["SourceMetadata"]["Data"]["Filesystem"]["line"]
                            .as_u64()
                            .unwrap_or(1) as usize,
                        line_end: None,
                        matched_content: None,
                        fixable: false,
                        suggested_patch: None,
                    });
                }
            }
        }
        Ok(findings)
    }

    /// AI/ML: modelscan
    pub fn scan_modelscan(target: &Path) -> Result<Vec<UnifiedFinding>> {
        let tool = Self::resolve_tool_bin("modelscan");
        let temp_report = NamedTempFile::new()?;
        let report_path = temp_report.path().to_string_lossy().to_string();

        let output = Command::new(tool)
            .args([
                "-p",
                &target.to_string_lossy(),
                "-r",
                "json",
                "-o",
                &report_path,
            ])
            .output();

        let mut findings = Vec::new();
        if output.is_ok() {
            if let Ok(content) = std::fs::read_to_string(&report_path) {
                let parsed: Value = serde_json::from_str(&content).unwrap_or(Value::Null);
                if let Some(issues) = parsed["issues"].as_array() {
                    for issue in issues {
                        findings.push(UnifiedFinding {
                            id: format!("MODELSCAN-{}", uuid::Uuid::new_v4()),
                            source: ScannerSource::Modelscan,
                            rule_id: issue["scanner"]
                                .as_str()
                                .unwrap_or("unsafe-deserialization")
                                .to_string(),
                            severity: Severity::Critical,
                            message: issue["description"].as_str().unwrap_or("").to_string(),
                            file_path: PathBuf::from(issue["source"].as_str().unwrap_or("model")),
                            line_start: 1,
                            line_end: None,
                            matched_content: None,
                            fixable: false,
                            suggested_patch: None,
                        });
                    }
                }
            }
        }
        Ok(findings)
    }

    /// Database: sqlfluff
    pub fn scan_sqlfluff(target: &Path) -> Result<Vec<UnifiedFinding>> {
        let tool = Self::resolve_tool_bin("sqlfluff");
        let output = Command::new(tool)
            .args(["lint", "--format", "json", &target.to_string_lossy()])
            .output();

        let mut findings = Vec::new();
        if let Ok(out) = output {
            let parsed: Value = serde_json::from_slice(&out.stdout).unwrap_or(Value::Array(vec![]));
            if let Some(files) = parsed.as_array() {
                for file_entry in files {
                    let path = file_entry["filepath"].as_str().unwrap_or("");
                    if let Some(violations) = file_entry["violations"].as_array() {
                        for v in violations {
                            findings.push(UnifiedFinding {
                                id: format!("SQLFLUFF-{}", uuid::Uuid::new_v4()),
                                source: ScannerSource::Sqlfluff,
                                rule_id: v["code"]
                                    .as_str()
                                    .unwrap_or("sql-anti-pattern")
                                    .to_string(),
                                severity: Severity::Medium,
                                message: v["description"].as_str().unwrap_or("").to_string(),
                                file_path: PathBuf::from(path),
                                line_start: v["line_no"].as_u64().unwrap_or(1) as usize,
                                line_end: None,
                                matched_content: None,
                                fixable: false,
                                suggested_patch: None,
                            });
                        }
                    }
                }
            }
        }
        Ok(findings)
    }

    /// Targeted Runner Coordinator
    pub fn scan_for_persona(target: &Path, persona: &Persona) -> Vec<UnifiedFinding> {
        let mut results = Vec::new();

        // Tier 1 Baseline
        results.extend(Self::scan_gitleaks(target).unwrap_or_default());
        results.extend(Self::scan_semgrep(target).unwrap_or_default());
        results.extend(Self::scan_trivy(target).unwrap_or_default());

        // Selective Tier 2 based on persona
        match persona {
            Persona::DevOpsPlatform => {
                results.extend(Self::scan_actionlint(target).unwrap_or_default());
                results.extend(Self::scan_checkov(target).unwrap_or_default());
            }
            Persona::RedTeamCybersecurity => {
                results.extend(Self::scan_trufflehog(target).unwrap_or_default());
            }
            Persona::AiMlEngineer => {
                results.extend(Self::scan_modelscan(target).unwrap_or_default());
            }
            Persona::DatabaseEngineer => {
                results.extend(Self::scan_sqlfluff(target).unwrap_or_default());
            }
            _ => {}
        }

        results
    }
}

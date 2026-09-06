use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Copy)]
pub enum Persona {
    SoftwareEngineer,
    WebDeveloper,
    AiMlEngineer,
    RedTeamCybersecurity,
    DevOpsPlatform,
    OssCompliance,
    DatabaseEngineer,
}

impl Persona {
    /// Return the list of all supported personas
    pub fn all() -> &'static [Persona] {
        &[
            Persona::SoftwareEngineer,
            Persona::WebDeveloper,
            Persona::AiMlEngineer,
            Persona::RedTeamCybersecurity,
            Persona::DevOpsPlatform,
            Persona::OssCompliance,
            Persona::DatabaseEngineer,
        ]
    }

    /// Primary tools and GitHub-backed engines associated with this persona
    pub fn tool_backends(&self) -> &'static [&'static str] {
        match self {
            Persona::SoftwareEngineer => &["facebook/infer", "pmd/pmd", "cargo-clippy"],
            Persona::WebDeveloper => &["RetireJS/retire.js", "semgrep/p/owasp-top-ten", "biome"],
            Persona::AiMlEngineer => &["protectai/modelscan", "leondz/garak"],
            Persona::RedTeamCybersecurity => {
                &["trufflesecurity/trufflehog", "liamg/traitor", "gitleaks"]
            }
            Persona::DevOpsPlatform => &["rhysd/actionlint", "bridgecrewio/checkov", "trivy"],
            Persona::OssCompliance => &["google/go-licenses", "nexB/scancode-toolkit"],
            Persona::DatabaseEngineer => &["sqlfluff/sqlfluff", "schemacrawler/SchemaCrawler"],
        }
    }

    /// File triggers: when should this persona activate its Tier 2 scanners?
    pub fn file_triggers(&self) -> &'static [&'static str] {
        match self {
            Persona::SoftwareEngineer => &[".rs", ".java", ".c", ".cpp", ".go"],
            Persona::WebDeveloper => &[".js", ".jsx", ".ts", ".tsx", ".html", ".vue", ".svelte"],
            Persona::AiMlEngineer => &[
                ".pkl",
                ".bin",
                ".pt",
                ".pth",
                ".onnx",
                ".safetensors",
                ".h5",
            ],
            Persona::RedTeamCybersecurity => &["*"], // Red team audits full attack surface
            Persona::DevOpsPlatform => &[
                ".github/workflows",
                "Dockerfile",
                ".tf",
                ".yaml",
                ".yml",
                "k8s",
            ],
            Persona::OssCompliance => &[
                "Cargo.toml",
                "package.json",
                "go.mod",
                "requirements.txt",
                "pom.xml",
            ],
            Persona::DatabaseEngineer => &[".sql", "migrations", "schema.prisma", ".prisma"],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ScannerSource {
    Gitleaks,
    Semgrep,
    Trivy,
    Clippy,
    Ruff,
    Biome,
    Actionlint,
    Checkov,
    Modelscan,
    Trufflehog,
    Sqlfluff,
    AiGenerated(Persona),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedFinding {
    pub id: String,
    pub source: ScannerSource,
    pub rule_id: String,
    pub severity: Severity,
    pub message: String,
    pub file_path: PathBuf,
    pub line_start: usize,
    pub line_end: Option<usize>,
    pub matched_content: Option<String>,
    pub fixable: bool,
    pub suggested_patch: Option<String>,
}

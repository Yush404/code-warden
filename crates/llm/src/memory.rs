use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemoryState {
    pub resolved_rule_ids: HashSet<String>,
    pub ignored_rule_ids: HashSet<String>,
    pub total_audits: usize,
}

pub struct MemoryManager {
    _memory_dir: PathBuf,
    history_file: PathBuf,
    state_file: PathBuf,
}

impl MemoryManager {
    pub fn new(target: &Path) -> Result<Self> {
        let memory_dir = target.join(".code-warden").join("memory");
        fs::create_dir_all(&memory_dir)?;

        let history_file = memory_dir.join("session_history.md");
        let state_file = memory_dir.join("memory_state.json");

        Ok(Self {
            _memory_dir: memory_dir,
            history_file,
            state_file,
        })
    }

    /// Load structured memory state
    pub fn load_state(&self) -> MemoryState {
        if let Ok(data) = fs::read_to_string(&self.state_file) {
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            MemoryState::default()
        }
    }

    /// Save updated memory state
    pub fn save_state(&self, state: &MemoryState) -> Result<()> {
        let json = serde_json::to_string_pretty(state)?;
        fs::write(&self.state_file, json)?;
        Ok(())
    }

    /// Record a completed session entry into history
    pub fn append_session(
        &self,
        timestamp: &str,
        persona: &str,
        issues_summary: &str,
        fixes_applied: &[String],
    ) -> Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.history_file)?;

        writeln!(file, "### Session: {}", timestamp)?;
        writeln!(file, "- **Persona:** {}", persona)?;
        writeln!(file, "- **Detected:** {}", issues_summary)?;
        if fixes_applied.is_empty() {
            writeln!(file, "- **Remediations:** None (Report Only)")?;
        } else {
            writeln!(file, "- **Remediations:**")?;
            for fix in fixes_applied {
                writeln!(file, "  - {}", fix)?;
            }
        }
        writeln!(file, "\n---\n")?;
        Ok(())
    }

    /// Build context string of previous memory to ground the LLM
    pub fn build_memory_context(&self) -> String {
        let state = self.load_state();
        if state.total_audits == 0 && state.resolved_rule_ids.is_empty() {
            return "No previous audit memory available. This is a baseline scan.".to_string();
        }

        let mut lines = Vec::new();
        lines.push(format!(
            "Historical Audits Completed: {}",
            state.total_audits
        ));
        if !state.resolved_rule_ids.is_empty() {
            lines.push(format!(
                "Previously Resolved Issues: {}",
                state
                    .resolved_rule_ids
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if !state.ignored_rule_ids.is_empty() {
            lines.push(format!(
                "Known Ignored False Positives: {}",
                state
                    .ignored_rule_ids
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }

        lines.join("\n")
    }
}

use anyhow::{Context, Result};
use std::path::Path;

pub fn create_fix_branch(target: &Path, branch_name: &str) -> Result<()> {
    let repo = git2::Repository::open(target).context("Failed to open git repository")?;
    let head = repo.head()?.peel_to_commit()?;
    repo.branch(branch_name, &head, false)?;
    let obj = repo.revparse_single(&format!("refs/heads/{}", branch_name))?;
    repo.checkout_tree(&obj, None)?;
    repo.set_head(&format!("refs/heads/{}", branch_name))?;
    Ok(())
}

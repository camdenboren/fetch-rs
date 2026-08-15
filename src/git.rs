use crate::{
    config::Config,
    util::{ExitCode, notify},
};
use chrono::prelude::*;
use octocrab::{
    Octocrab,
    models::{repos::RepoCommit, workflows::Run},
};
use std::{
    path::PathBuf,
    process::{Command, exit},
};

/// Create the git config and add the flake as a safe directory
pub fn git_config(git_file_path: PathBuf, flake_dir: String) {
    if std::fs::metadata(git_file_path.clone()).is_err() {
        match std::fs::write(git_file_path.clone(), "") {
            Ok(_) => tracing::info!(
                "Wrote initial git config to: {} - feel free to adjust specific settings",
                git_file_path.display()
            ),
            Err(_) => tracing::info!("Failed to create git config file"),
        }
    }

    // ensure we can run git ops on the flake dir
    // this should probably be refactored to a distinct function
    // that checks the value before blindly setting
    match Command::new("git")
        .args([
            "config",
            "--global",
            "--add",
            "safe.directory",
            flake_dir.as_ref(),
        ])
        .output()
    {
        Ok(_) => tracing::info!("Added flake_dir as a safe directory"),
        Err(_) => tracing::info!("Failed to add flake_dir as a safe directory"),
    }
}

/// Retrieve the remote origin of the repo
pub fn origin(flake_dir: String) -> Vec<String> {
    let origin_output = Command::new("git")
        .args([
            "-C",
            flake_dir.as_ref(),
            "config",
            "--get",
            "remote.origin.url",
        ])
        .output();
    if origin_output.is_err() {
        tracing::error!("Unable to get remote origin's URL, doing nothing");
        exit(ExitCode::NoOp.into())
    }
    let origin_stdout = match origin_output.ok().map(|o| o.stdout).unwrap_or_default() {
        s if s.is_empty() => {
            tracing::error!("Unable to get remote origin's URL: command produced no output");
            exit(ExitCode::NoOp.into());
        }
        s => String::from_utf8(s).unwrap_or_default(),
    };
    let origin_output = origin_stdout.trim();
    let mut path = if let Some(idx) = origin_output.find("://") {
        &origin_output[idx + 3..]
    } else {
        origin_output
    };

    if path.starts_with("git@github.com:") {
        path = &path["git@github.com:".len()..];
    } else if path.starts_with("github.com/") {
        path = &path["github.com/".len()..];
    } else if let Some(idx) = path.find("/github.com/") {
        path = &path[idx + "/github.com/".len()..];
    } else {
        // Not a GitHub URL we understand
        tracing::info!(
            "Unable to identify the owner of the remote origin: {}",
            path,
        );
        exit(ExitCode::NoOp.into());
    }

    let path = if let Some(stripped) = path.strip_suffix(".git") {
        stripped
    } else {
        path
    };

    let v_ref: Vec<&str> = path.split('/').collect();
    v_ref.iter().map(|s| (*s).to_owned()).collect()
}

/// Fetch the latest remote commit
pub async fn latest_remote_commit(octocrab: &Octocrab, cfg: Config) -> RepoCommit {
    let latest_remote = match octocrab
        .repos(cfg.owner.clone(), cfg.repo.clone())
        .list_commits()
        .branch(cfg.branch.clone())
        .per_page(1)
        .send()
        .await
    {
        Ok(l) => l,
        Err(e) => {
            tracing::info!("Failed to fetch latest remote commit list: {}", e);
            exit(ExitCode::NoOp.into());
        }
    };

    match latest_remote.items.first() {
        Some(c) => c.clone(),
        None => {
            tracing::info!("No commits found in repository");
            exit(ExitCode::NoOp.into());
        }
    }
}

/// Determine the date of the latest local commit, trying GitHub before falling
/// back to local metadata
pub async fn latest_local_date(octocrab: &Octocrab, cfg: Config) -> DateTime<Utc> {
    let flake_dir = cfg.flake_dir.as_ref();
    let latest_local_cmd = match Command::new("git")
        .args(["-C", flake_dir, "rev-parse", "HEAD"])
        .output()
    {
        Ok(output) => output,
        Err(e) => {
            tracing::error!("Unable to get rev of latest_local commit: {}", e);
            exit(ExitCode::NoOp.into());
        }
    };
    let latest_local_stdout = if latest_local_cmd.stdout.is_empty() {
        tracing::error!("Unable to get rev of latest_local commit: command produced no output");
        exit(ExitCode::NoOp.into());
    } else {
        String::from_utf8(latest_local_cmd.stdout).unwrap_or_default()
    };
    let latest_local = latest_local_stdout.trim();

    // grabs the local computer's stored time of the git commit in case
    // we can't grab it remotely
    let latest_local_date_backup_cmd = match Command::new("git")
        .args(["-C", flake_dir, "show", "-s", "--format=%ci", latest_local])
        .output()
    {
        Ok(output) => output,
        Err(e) => {
            tracing::error!("Unable to get date of latest_local commit: {}", e);
            exit(ExitCode::NoOp.into());
        }
    };
    let latest_local_date_backup_stdout = if latest_local_date_backup_cmd.stdout.is_empty() {
        tracing::error!("Unable to get date of latest_local commit: command produced no output");
        exit(ExitCode::NoOp.into());
    } else {
        String::from_utf8(latest_local_date_backup_cmd.stdout).unwrap_or_default()
    };
    let latest_local_date_backup = latest_local_date_backup_stdout.trim();

    // Attempt to fetch the local commit's metadata from GitHub, falling
    // back to `latest_local_date_backup` if any step fails
    match octocrab
        .commits(cfg.owner, cfg.repo)
        .get(latest_local)
        .await
    {
        Ok(commit) => {
            let author = match commit.commit.author {
                Some(a) => a,
                None => {
                    tracing::error!("No author found for latest local commit");
                    exit(ExitCode::NoOp.into());
                }
            };
            match author.date {
                Some(d) => d,
                None => {
                    tracing::error!("No date found in latest local commit author metadata");
                    exit(ExitCode::NoOp.into());
                }
            }
        }
        Err(e) => {
            tracing::info!("Failed to fetch local commit from GitHub: {}", e);
            chrono::DateTime::parse_from_str(latest_local_date_backup, "%Y-%m-%d %H:%M:%S %z")
                .map_or_else(
                    |e| {
                        tracing::error!("Failed to parse fallback date: {}", e);
                        exit(ExitCode::NoOp.into());
                    },
                    |dt| dt.with_timezone(&Utc),
                )
        }
    }
}

/// Retrieve the name of the local checked-out branch
pub fn local_branch(cfg: Config) -> String {
    let local_branch_cmd = match Command::new("git")
        .args([
            "-C",
            cfg.flake_dir.as_ref(),
            "rev-parse",
            "--abbrev-ref",
            "HEAD",
        ])
        .output()
    {
        Ok(output) => output,
        Err(e) => {
            tracing::error!("Unable to get name of local branch: {}", e);
            exit(ExitCode::NoOp.into());
        }
    };
    let local_branch = if local_branch_cmd.stdout.is_empty() {
        tracing::error!("Unable to get name of local branch: command produced no output");
        exit(ExitCode::NoOp.into());
    } else {
        String::from_utf8(local_branch_cmd.stdout).unwrap_or_default()
    };
    local_branch.trim().to_owned()
}

/// Retrieve the workflow run of the given latest commit
pub async fn latest_run(octocrab: &Octocrab, commit: RepoCommit, cfg: Config) -> Run {
    let latest_run = match octocrab
        .workflows(cfg.owner.clone(), cfg.repo.clone())
        .list_runs(cfg.workflow.clone())
        .branch(cfg.branch.clone())
        .head_sha(commit.sha)
        .per_page(1)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::info!("Failed to fetch latest workflow run: {}", e);
            exit(ExitCode::NoOp.into());
        }
    };

    match latest_run.items.first() {
        Some(r) => r.clone(),
        None => {
            tracing::info!("No workflow runs found for the given commit");
            exit(ExitCode::NoOp.into());
        }
    }
}

/// Switch to the main branch and fetch the most recent commit
pub fn switch(cfg: Config) {
    tracing::info!("Switching to {}", cfg.branch);
    let switch_output = match Command::new("git")
        .args(["-C", cfg.flake_dir.as_ref(), "switch", cfg.branch.as_ref()])
        .output()
    {
        Ok(output) => output,
        Err(e) => {
            tracing::error!("Failed to switch branches: {}", e);
            exit(ExitCode::NoOp.into());
        }
    };
    if switch_output.status.success() {
        let switch_output_stdout = String::from_utf8(switch_output.stdout).unwrap_or_default();
        if !switch_output_stdout.trim().is_empty() {
            tracing::info!("Switch stdout: {}", switch_output_stdout.trim());
        }
    } else {
        let switch_output_stderr = String::from_utf8(switch_output.stderr).unwrap_or_default();
        if cfg.notify {
            notify();
        }
        if !switch_output_stderr.trim().is_empty() {
            tracing::error!(
                "Failed to switch to {} branch (stderr): {}",
                cfg.branch,
                switch_output_stderr.trim()
            );
        } else {
            tracing::error!(
                "Failed to switch to {} branch (no stderr output)",
                cfg.branch
            );
        }
        exit(ExitCode::Failure.into());
    }
}

/// Fetch the most recent commit
pub fn fetch(cfg: Config) {
    tracing::info!("Fetching latest commit on {}", cfg.branch);
    let fetch_output = match Command::new("git")
        .args(["-C", cfg.flake_dir.as_ref(), "fetch"])
        .output()
    {
        Ok(output) => output,
        Err(e) => {
            tracing::error!("Failed to fetch latest commit: {}", e);
            exit(ExitCode::NoOp.into());
        }
    };
    if fetch_output.status.success() {
        let fetch_output_stdout = String::from_utf8(fetch_output.stdout).unwrap_or_default();
        if !fetch_output_stdout.trim().is_empty() {
            tracing::info!("Fetch stdout: {}", fetch_output_stdout.trim());
        }
    } else {
        let fetch_output_stderr = String::from_utf8(fetch_output.stderr).unwrap_or_default();
        if cfg.notify {
            notify();
        }
        if !fetch_output_stderr.trim().is_empty() {
            tracing::error!(
                "Error encountered while fetching latest (stderr): {}",
                fetch_output_stderr.trim()
            );
        } else {
            tracing::error!("Error encountered while fetching latest (no stderr output)");
        }
        tracing::error!("Failed to fetch latest commit");
        exit(ExitCode::Failure.into());
    }
}

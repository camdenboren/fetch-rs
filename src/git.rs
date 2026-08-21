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
    let origin_output = String::from_utf8(origin_output.unwrap().stdout)
        .expect("Unable to stringify remote origin");
    let origin_output = origin_output.trim();
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
    let latest_local = Command::new("git")
        .args(["-C", flake_dir, "rev-parse", "HEAD"])
        .output()
        .expect("Unable to get rev of latest_local commit");
    let latest_local = String::from_utf8(latest_local.stdout)
        .expect("Unable to stringify latest_local commit's rev");
    let latest_local = latest_local.trim();

    // grabs the local computer's stored time of the git commit in case
    // we can't grab it remotely
    let latest_local_date_backup = Command::new("git")
        .args(["-C", flake_dir, "show", "-s", "--format=%ci", latest_local])
        .output()
        .expect("Unable to get rev of latest_local commit");
    let latest_local_date_backup = String::from_utf8(latest_local_date_backup.stdout)
        .expect("Unable to stringify latest_local commit's rev");
    let latest_local_date_backup = latest_local_date_backup.trim();

    // Attempt to fetch the local commit's metadata from GitHub, falling
    // back to `latest_local_date_backup` if any step fails
    match octocrab
        .commits(cfg.owner, cfg.repo)
        .get(latest_local)
        .await
    {
        Ok(commit) => commit.commit.author.unwrap().date.unwrap(),
        Err(_) => {
            chrono::DateTime::parse_from_str(latest_local_date_backup, "%Y-%m-%d %H:%M:%S %z")
                .expect("Failed to parse fallback local commit date")
                .with_timezone(&Utc)
        }
    }
}

/// Retrieve the name of the local checked-out branch
pub fn local_branch(cfg: Config) -> String {
    let local_branch = Command::new("git")
        .args([
            "-C",
            cfg.flake_dir.as_ref(),
            "rev-parse",
            "--abbrev-ref",
            "HEAD",
        ])
        .output()
        .expect("Unable to get name of local branch");
    let local_branch =
        String::from_utf8(local_branch.stdout).expect("Unable to stringify name of local branch");
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
    let switch_output = Command::new("git")
        .args(["-C", cfg.flake_dir.as_ref(), "switch", cfg.branch.as_ref()])
        .output()
        .expect("Failed to switch branches");
    if switch_output.status.success() {
        let switch_output_stdout =
            String::from_utf8(switch_output.stdout).expect("Unable to stringify switch_output");
        tracing::info!("{}", switch_output_stdout);
    } else {
        let switch_output_stderr =
            String::from_utf8(switch_output.stderr).expect("Unable to stringify switch_output");
        if cfg.notify {
            notify(cfg.clone());
        }
        tracing::error!(
            "Failed to switch to {} branch, doing nothing: {}",
            cfg.branch,
            switch_output_stderr
        );
        exit(ExitCode::Failure.into());
    }
}

/// Fetch the most recent commit
pub fn fetch(cfg: Config) {
    tracing::info!("Fetching latest commit on {}", cfg.branch);
    let fetch_output = Command::new("git")
        .args(["-C", cfg.flake_dir.as_ref(), "fetch"])
        .output()
        .expect("Failed to fetch latest commit");
    if fetch_output.status.success() {
        let fetch_output_stdout =
            String::from_utf8(fetch_output.stdout).expect("Unable to stringify fetch_output");
        tracing::info!("{}", fetch_output_stdout);
    } else {
        let fetch_output_stderr =
            String::from_utf8(fetch_output.stderr).expect("Unable to stringify fetch_output");
        tracing::info!(
            "Error encountered while fetching latest, doing nothing: {}",
            fetch_output_stderr
        );
        if cfg.notify {
            notify(cfg);
        }
        tracing::error!("Failed to fetch latest commit");
        exit(ExitCode::Failure.into());
    }
}

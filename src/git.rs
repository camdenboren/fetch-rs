use crate::{
    config::Config,
    util::{ExitCode, notify},
};
use chrono::prelude::*;
use octocrab::{
    Octocrab,
    models::{repos::RepoCommit, workflows::Run},
};
use std::process::{Command, exit};

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
            eprintln!("Failed to fetch latest remote commit list: {}", e);
            exit(ExitCode::NoOp.into());
        }
    };

    match latest_remote.items.first() {
        Some(c) => c.clone(),
        None => {
            eprintln!("No commits found in repository");
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
            eprintln!("Failed to fetch latest workflow run: {}", e);
            exit(ExitCode::NoOp.into());
        }
    };

    match latest_run.items.first() {
        Some(r) => r.clone(),
        None => {
            eprintln!("No workflow runs found for the given commit");
            exit(ExitCode::NoOp.into());
        }
    }
}

/// Switch to the main branch and fetch the most recent commit
pub fn switch(cfg: Config) {
    println!("\nSwitching to {}", cfg.branch);
    let switch_output = Command::new("git")
        .args(["-C", cfg.flake_dir.as_ref(), "switch", cfg.branch.as_ref()])
        .output()
        .expect("Failed to switch branches");
    if switch_output.status.success() {
        let switch_output_stdout =
            String::from_utf8(switch_output.stdout).expect("Unable to stringify switch_output");
        println!("  {}", switch_output_stdout);
    } else {
        let switch_output_stderr =
            String::from_utf8(switch_output.stderr).expect("Unable to stringify switch_output");
        eprintln!(
            "  Error encountered while switching branches, doing nothing: {}",
            switch_output_stderr
        );
        if cfg.notify {
            notify(cfg);
        }
        exit(ExitCode::Failure.into());
    }
}

/// Fetch the most recent commit
pub fn fetch(cfg: Config) {
    println!("Fetching latest commit on {}", cfg.branch);
    let fetch_output = Command::new("git")
        .args(["-C", cfg.flake_dir.as_ref(), "fetch"])
        .output()
        .expect("Failed to fetch latest commit");
    if fetch_output.status.success() {
        let fetch_output_stdout =
            String::from_utf8(fetch_output.stdout).expect("Unable to stringify fetch_output");
        println!("  {}", fetch_output_stdout);
    } else {
        let fetch_output_stderr =
            String::from_utf8(fetch_output.stderr).expect("Unable to stringify fetch_output");
        eprintln!(
            "  Error encountered while fetching latest, doing nothing: {}",
            fetch_output_stderr
        );
        if cfg.notify {
            notify(cfg);
        }
        exit(ExitCode::Failure.into());
    }
}

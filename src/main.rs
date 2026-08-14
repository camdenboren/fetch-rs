use chrono::prelude::*;
use fetch_rs::{
    config::{CFG_FILE, Config},
    git::{fetch, latest_local_date, latest_remote_commit, latest_run, local_branch, switch},
    util::ExitCode,
};
use octocrab::Octocrab;
use std::{path::PathBuf, process::exit};

/// Determine whether the machine should be rebuilt by comparing the date
/// of the latest remote commit vs. the latest local commit
async fn should_rebuild(
    cfg: Config,
    octocrab: Octocrab,
    latest_remote_date: DateTime<Utc>,
    status: &str,
) -> bool {
    // We grab the name of the local branch so we can determine whether
    // main has been fast-forwarded from it. I.e., we want to rebuild
    // if this is the case, but we DON'T want to rebuild if we're already
    // on an up-to-date main)
    let local_branch = local_branch(cfg.clone());
    let latest_local_date = latest_local_date(&octocrab, cfg.clone()).await;

    // Rebuild when local is any branch other than main AND there's a commit
    // that's at least as new OR when local branch is main and there's a commit
    // that's strictly newer
    let mut rebuild = false;
    if (local_branch != cfg.branch && latest_remote_date >= latest_local_date)
        || (local_branch == cfg.branch && latest_remote_date > latest_local_date)
    {
        rebuild = true;
    }

    println!("Latest remote commit time: {}", latest_remote_date);
    println!("Latest local commit time: {}\n", latest_local_date);
    println!("All actions succeeded on latest remote commit: {}", status);
    println!("Rebuilding on latest remote commit: {}", rebuild);

    rebuild
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let path = PathBuf::from(CFG_FILE);
    let config_content = Config::read(path.clone()).unwrap_or("".into());
    let cfg = Config::deserialize(config_content);
    let octocrab = Octocrab::builder().build()?;

    let latest_remote = latest_remote_commit(&octocrab, cfg.clone()).await;
    let latest_remote_date = latest_remote.commit.author.as_ref().unwrap().date.unwrap();

    let latest_run = latest_run(&octocrab, latest_remote.clone(), cfg.clone()).await;
    let status = latest_run.conclusion.unwrap_or("".into());
    let status = status.as_ref();

    match status {
        "success" => {
            if should_rebuild(cfg.clone(), octocrab, latest_remote_date, status).await {
                // If these go smoothly, then the exit code will be 0, triggering a
                // rebuild via bin/rebuild.rs
                switch(cfg.clone());
                fetch(cfg);
            } else {
                exit(ExitCode::NoOp.into())
            }
        }
        "failure" => {
            eprintln!("Workflow failed: doing nothing.");
            exit(ExitCode::NoOp.into())
        }
        _ => {
            eprintln!("Unknown status for workflow: doing nothing.");
            exit(ExitCode::NoOp.into())
        }
    }

    Ok(())
}

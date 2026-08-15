use chrono::prelude::*;
use fetch_rs::{
    config::{CFG_FILE, Config},
    git::{fetch, latest_local_date, latest_remote_commit, latest_run, local_branch, switch},
    util::ExitCode,
};
use octocrab::Octocrab;
use std::{env, path::PathBuf, process::exit};

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

    tracing::info!("Latest remote commit time: {}", latest_remote_date);
    tracing::info!("Latest local commit time: {}", latest_local_date);
    tracing::info!("All actions succeeded on latest remote commit: {}", status);
    tracing::info!("Rebuilding on latest remote commit: {}", rebuild);

    rebuild
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    tracing::subscriber::set_global_default(subscriber)?;
    let cfg_path = env::var("F_RS_CONFIG");
    if cfg_path.is_err() {
        tracing::error!("Unable to read $F_RS_CONFIG-is it set? Failed to create initial config");
        exit(ExitCode::NoOp.into());
    }
    let cfg_path = PathBuf::from(cfg_path.unwrap_or_default()).join(CFG_FILE);
    let config_content = Config::read(cfg_path.clone()).unwrap_or("".into());
    let cfg = Config::deserialize(config_content);
    let octocrab = Octocrab::builder().build()?;

    let latest_remote = latest_remote_commit(&octocrab, cfg.clone()).await;
    let author = match latest_remote.commit.author.as_ref() {
        Some(a) => a,
        None => {
            tracing::error!("No author found for latest remote commit");
            exit(ExitCode::NoOp.into());
        }
    };
    let latest_remote_date = match author.date {
        Some(d) => d,
        None => {
            tracing::error!("No date found in latest remote commit author metadata");
            exit(ExitCode::NoOp.into());
        }
    };

    let latest_run = latest_run(&octocrab, latest_remote.clone(), cfg.clone()).await;
    let status = match latest_run.conclusion {
        Some(c) => c.trim().to_owned(),
        None => {
            tracing::error!("No conclusion found in latest run's metadata");
            exit(ExitCode::NoOp.into());
        }
    };
    let status = status.as_ref();

    match status {
        "success" => {
            if should_rebuild(cfg.clone(), octocrab, latest_remote_date, status).await {
                // If these go smoothly, then the exit code will be 0, triggering a
                // rebuild via bin/rebuild.rs
                switch(cfg.clone());
                fetch(cfg);
            } else {
                tracing::info!("On newer commit: doing nothing");
                exit(ExitCode::NoOp.into())
            }
        }
        "failure" => {
            tracing::info!("Workflow failed: doing nothing.");
            exit(ExitCode::NoOp.into())
        }
        _ => {
            tracing::info!("Unknown status for workflow: doing nothing.");
            exit(ExitCode::NoOp.into())
        }
    }

    Ok(())
}

use fetch_rs::{
    config::{CFG_FILE, Config},
    git::{fetch, latest_local_sha, latest_remote_commit, latest_run, local_branch, switch},
    util::ExitCode,
};
use octocrab::Octocrab;
use std::{env, path::PathBuf, process::exit};

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
            tracing::info!("All actions succeeded on latest remote commit");

            // if the SHAs don't match, then we need to sync the
            // local repo and rebuild the system
            if latest_local_sha(cfg.clone()).await != latest_remote.sha {
                tracing::info!("Rebuilding on latest remote commit");
                // If these go smoothly, then the exit code will be 0, triggering a
                // rebuild via bin/rebuild.rs
                if local_branch(cfg.clone()) != cfg.branch {
                    switch(cfg.clone());
                }
                fetch(cfg);
            } else {
                tracing::info!("On latest passing commit: doing nothing");
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

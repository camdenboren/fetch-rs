use chrono::prelude::*;
use fetch_rs::{
    config::{Config, flake_dir},
    util::*,
};
use octocrab::Octocrab;
use std::process::Command;

/// Determine whether the machine should be rebuilt by comparing the date
/// of the latest remote commit vs. the latest local commit
async fn should_rebuild(
    cfg: Config,
    octocrab: Octocrab,
    latest_remote_date: DateTime<Utc>,
    status: &str,
) -> Result<bool, octocrab::Error> {
    let flake_dir = flake_dir(cfg.clone());
    let latest_local = Command::new("git")
        .args(["-C", flake_dir.as_ref(), "rev-parse", "HEAD"])
        .output()
        .expect("Unable to get rev of latest_local commit");
    let latest_local = String::from_utf8(latest_local.stdout)
        .expect("Unable to stringify latest_local commit's rev");
    let latest_local = latest_local.trim();

    // grabs the local computer's stored time of the git commit in case
    // we can't grab it remotely
    let latest_local_date_backup = Command::new("git")
        .args([
            "-C",
            flake_dir.as_ref(),
            "show",
            "-s",
            "--format=%ci",
            latest_local,
        ])
        .output()
        .expect("Unable to get rev of latest_local commit");
    let latest_local_date_backup = String::from_utf8(latest_local_date_backup.stdout)
        .expect("Unable to stringify latest_local commit's rev");
    let latest_local_date_backup = latest_local_date_backup.trim();

    // Attempt to fetch the local commit's metadata from github, falling
    // back to `latest_local_date_backup` if any step fails
    let latest_local_date: DateTime<Utc> = match octocrab
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
    };

    // Grab the name of the local branch so we can determine whether
    // main has been fast-forwarded from it (i.e., we want to rebuild
    // if this is the case, but we DON'T want to rebuild if we're already
    // on an up-to-date main)
    let local_branch = Command::new("git")
        .args([
            "-C",
            flake_dir.as_ref(),
            "rev-parse",
            "--abbrev-ref",
            "HEAD",
        ])
        .output()
        .expect("Unable to get name of local branch");
    let local_branch =
        String::from_utf8(local_branch.stdout).expect("Unable to stringify name of local branch");
    let local_branch = local_branch.trim();

    let mut rebuild = false;
    // rebuild when local is any branch other than main AND there's a commit that's at least as new
    // OR when local branch is main and there's a commit that's strictly newer
    if (local_branch != cfg.branch && latest_remote_date >= latest_local_date)
        || (local_branch == cfg.branch && latest_remote_date > latest_local_date)
    {
        rebuild = true;
    }

    println!("Latest remote commit time: {}", latest_remote_date);
    println!("Latest local commit time: {}\n", latest_local_date);
    println!("All actions succeeded on latest remote commit: {}", status);
    println!("Rebuilding on latest remote commit: {}", rebuild);

    Ok(rebuild)
}

/// Switch to the main branch and fetch the most recent commit
fn switch_and_fetch(cfg: Config) {
    let flake_dir = flake_dir(cfg.clone());
    println!("\nSwitching to {}", cfg.branch);
    let switch_output = Command::new("git")
        .args(["-C", flake_dir.as_ref(), "switch", cfg.branch.as_ref()])
        .output()
        .expect("Failed to switch branches");
    if switch_output.status.success() {
        let switch_output_stdout =
            String::from_utf8(switch_output.stdout).expect("Unable to stringify switch_output");
        println!("  {}", switch_output_stdout);
    } else {
        let switch_output_stderr =
            String::from_utf8(switch_output.stderr).expect("Unable to stringify switch_output");
        println!(
            "  Error encountered while switching branches, doing nothing: {}",
            switch_output_stderr
        );
        if cfg.notify {
            notify(cfg);
        }
        return;
    }

    println!("Fetching latest commit on {}", cfg.branch);
    let fetch_output = Command::new("git")
        .args(["-C", flake_dir.as_ref(), "fetch"])
        .output()
        .expect("Failed to fetch latest commit");
    if fetch_output.status.success() {
        let fetch_output_stdout =
            String::from_utf8(fetch_output.stdout).expect("Unable to stringify fetch_output");
        println!("  {}", fetch_output_stdout);
    } else {
        let fetch_output_stderr =
            String::from_utf8(fetch_output.stderr).expect("Unable to stringify fetch_output");
        println!(
            "  Error encountered while fetching latest, doing nothing: {}",
            fetch_output_stderr
        );
        if cfg.notify {
            notify(cfg);
        }
    }
}

#[tokio::main]
async fn main() -> octocrab::Result<()> {
    let path = dirs::config_dir().unwrap_or_default().join("fetch-rs");
    let config_content = Config::read(path.clone()).unwrap_or("".into());
    let cfg = Config::deserialize(config_content);

    let octocrab = Octocrab::builder().build()?;
    let latest_remote = octocrab
        .repos(cfg.owner.clone(), cfg.repo.clone())
        .list_commits()
        .branch(cfg.branch.clone())
        .per_page(1)
        .send()
        .await?;

    if let Some(commit) = latest_remote.items.first() {
        let latest_remote = commit.clone().sha;
        let latest_remote_date = commit.clone().commit.author.unwrap().date.unwrap();

        let latest_run = octocrab
            .workflows(cfg.owner.clone(), cfg.repo.clone())
            .list_runs(cfg.workflow.clone())
            .branch(cfg.branch.clone())
            .head_sha(latest_remote)
            .per_page(1)
            .send()
            .await?;

        if let Some(run) = latest_run.items.first() {
            let status = run.clone().conclusion.unwrap_or("".into());
            let status = status.as_ref();

            match status {
                "success" => {
                    if should_rebuild(cfg.clone(), octocrab, latest_remote_date, status).await? {
                        switch_and_fetch(cfg);
                    }
                }
                "failure" => println!("Workflow failed: doing nothing."),
                _ => println!("Unknown status for workflow: doing nothing."),
            }
        } else {
            println!("No run found: doing nothing.");
        }
    } else {
        println!("No commits found: doing nothing.");
    }

    Ok(())
}

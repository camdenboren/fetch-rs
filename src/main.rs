use chrono::prelude::*;
use octocrab::Octocrab;
use std::env;
use std::process::Command;

const OWNER: &str = "camdenboren";
const REPO: &str = "nixos";
const BRANCH: &str = "main";
const WORKFLOW: &str = "build.yaml";
const NH_CMD: &str = "switch";
#[cfg(not(target_os = "macos"))]
const NH_SYSTEM: &str = "os";
#[cfg(target_os = "macos")]
const NH_SYSTEM: &str = "darwin";

/// Retrieve the NixOS / nix-darwin config directory
fn cfg_dir() -> String {
    let default_home_dir = format!("/home/{}", OWNER);
    let home_dir = env::home_dir().unwrap_or(default_home_dir.clone().into());
    let home_dir = home_dir.to_str().unwrap_or(default_home_dir.as_str());
    return format!("{}/etc/nixos", home_dir);
}

/// Determine whether the machine should be rebuilt by comparing the date
/// of the latest remote commit vs. the latest local commit
async fn should_rebuild(
    octocrab: Octocrab,
    latest_remote_date: DateTime<Utc>,
    status: &str,
) -> Result<bool, octocrab::Error> {
    let cfg_dir = cfg_dir();
    let latest_local = Command::new("git")
        .args(["-C", cfg_dir.as_ref(), "rev-parse", "HEAD"])
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
            cfg_dir.as_ref(),
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
    let latest_local_date: DateTime<Utc> =
        match octocrab.commits(OWNER, REPO).get(latest_local).await {
            Ok(commit) => commit.commit.author.unwrap().date.unwrap(),
            Err(_) => {
                chrono::DateTime::parse_from_str(latest_local_date_backup, "%Y-%m-%d %H:%M:%S %z")
                    .expect("Failed to parse fallback local commit date")
                    .with_timezone(&Utc)
            }
        };

    let mut rebuild = false;
    if latest_remote_date > latest_local_date {
        rebuild = true;
    }

    println!("Latest remote commit time: {}", latest_remote_date);
    println!("Latest local commit time: {}\n", latest_local_date);
    println!("All actions succeeded on latest remote commit: {}", status);
    println!("Rebuilding on latest remote commit: {}", rebuild);

    Ok(rebuild)
}

/// Switch to the main branch, fetch the most recent commit, and rebuild via nh
fn fetch_and_rebuild() {
    let cfg_dir = cfg_dir();
    println!("\nSwitching to {}", BRANCH);
    let switch_output = Command::new("git")
        .args(["-C", cfg_dir.as_ref(), "switch", BRANCH])
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
        return;
    }

    println!("Fetching latest commit on {}", BRANCH);
    let fetch_output = Command::new("git")
        .args(["-C", cfg_dir.as_ref(), "fetch"])
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
        return;
    }

    // `spawn()` would probably enable visualizing this
    println!("Rebuilding");
    let nh_output = Command::new("nh")
        .args([NH_SYSTEM, NH_CMD])
        .output()
        .expect("Failed to rebuild");
    if nh_output.status.success() {
        let nh_output_stdout =
            String::from_utf8(nh_output.stdout).expect("Unable to stringify nh_output");
        println!("  {}", nh_output_stdout);
    } else {
        let nh_output_stderr =
            String::from_utf8(nh_output.stderr).expect("Unable to stringify nh_output");
        println!(
            "  Error encountered while rebuilding, doing nothing: {}",
            nh_output_stderr
        );
    }
}

#[tokio::main]
async fn main() -> octocrab::Result<()> {
    let octocrab = Octocrab::builder().build()?;
    let latest_remote = octocrab
        .repos(OWNER, REPO)
        .list_commits()
        .branch(BRANCH)
        .per_page(1)
        .send()
        .await?;

    if let Some(commit) = latest_remote.items.get(0) {
        let latest_remote = commit.clone().sha;
        let latest_remote_date = commit.clone().commit.author.unwrap().date.unwrap();

        let latest_run = octocrab
            .workflows(OWNER, REPO)
            .list_runs(WORKFLOW)
            .branch(BRANCH)
            .head_sha(latest_remote)
            .per_page(1)
            .send()
            .await?;

        if let Some(run) = latest_run.items.get(0) {
            let status = run.clone().conclusion.unwrap_or("".into());
            let status = status.as_ref();

            match status {
                "success" => {
                    if should_rebuild(octocrab, latest_remote_date, status).await? {
                        fetch_and_rebuild();
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

use chrono::prelude::*;
use octocrab::Octocrab;
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::{
    env,
    fs::{File, write},
    io::{Read, stdin},
    path::PathBuf,
};

#[derive(Serialize, Deserialize, Clone)]
struct Config {
    owner: String,
    repo: String,
    branch: String,
    workflow: String,
    nh_cmd: String,
    nh_system: String,
    notify: bool,
    ntfy_url: String,
    ntfy_topic: String,
}

impl Config {
    fn new(owner: &str, repo: &str) -> Self {
        Config {
            owner: owner.into(),
            repo: repo.into(),
            branch: "main".into(),
            workflow: "build.yaml".into(),
            nh_cmd: "switch".into(),
            nh_system: "os".into(),
            notify: false,
            ntfy_url: "ntfy.sh".into(),
            ntfy_topic: "".into(),
        }
    }

    /// Deserialize the raw configuration content
    fn deserialize(config_content: String) -> Self {
        match toml::from_str(&config_content) {
            Ok(config) => config,
            Err(_) => {
                println!();
                Config::new("", "")
            }
        }
    }

    /// Serialize the raw configuration content
    fn serialize(owner: &str, repo: &str) -> String {
        let config = Config::new(owner, repo);
        toml::to_string(&config).unwrap_or_default()
    }

    /// Read the raw configuration content at the given path, creating the file if needed.
    fn read(path: PathBuf) -> Result<String, anyhow::Error> {
        let file_path = path.join("config.toml");
        if std::fs::metadata(&file_path).is_err() {
            Config::write();
        }

        let mut config_file = File::open(file_path)?;
        let mut config_content = String::new();
        match config_file.read_to_string(&mut config_content) {
            Ok(_) => (),
            Err(_) => {
                config_content = String::from("");
                println!();
            }
        }

        Ok(config_content)
    }

    /// Write the default configuration content to the
    /// config file
    fn write() {
        println!(
            "Running first time setup-let's start with some basic info on your GitHub-based nix config\n"
        );
        let config_content =
            Config::serialize(&user_input("Repo Owner: "), &user_input("\nRepo Name: "));
        let path = dirs::config_dir().unwrap_or_default().join("fetch-rs");
        if std::fs::metadata(&path).is_err() {
            match std::fs::create_dir(&path) {
                Ok(_) => (),
                Err(_) => println!(),
            }
        }
        if std::fs::metadata(path.join("config.toml")).is_err() {
            match write(path.join("config.toml"), &config_content) {
                Ok(_) => (),
                Err(_) => println!("Failed to create config file"),
            }
            println!(
                "\nWrote initial config to: {}\nFeel free to adjust specific settings like (e.g., change branch name, enable notifications, etc.)\n",
                path.join("config.toml").display()
            )
        }
    }
}

/// Retrieve the NixOS / nix-darwin config directory
fn nix_cfg_dir(cfg: Config) -> String {
    let default_home_dir = format!("/home/{}", cfg.owner);
    let home_dir = env::home_dir().unwrap_or(default_home_dir.clone().into());
    let home_dir = home_dir.to_str().unwrap_or(default_home_dir.as_str());
    format!("{}/etc/nixos", home_dir)
}

/// Prompt the user for input and return it
fn user_input(message: &str) -> String {
    println!("{}", message);
    let mut buffer = String::new();
    let stdin = stdin(); // We get `Stdin` here.
    stdin.read_line(&mut buffer).unwrap();
    buffer.trim().into()
}

/// Send a notification via ntfy-sh
fn notify(cfg: Config) {
    let url = format!("{}/{}", cfg.ntfy_url, cfg.ntfy_topic);
    let mut url_sequence: Vec<&str> = Vec::new();
    if url.contains("https") {
        url_sequence.push("-L");
    }
    url_sequence.push(url.as_ref());
    let mut args = vec!["-d", "Rebuild failed"];
    args.append(&mut url_sequence);

    println!("Notifying via ntfy-sh server at: {}", url);
    let notify_output = Command::new("curl")
        .args(args)
        .output()
        .expect("Failed to notify");
    if notify_output.status.success() {
        let notify_output_stdout =
            String::from_utf8(notify_output.stdout).expect("Unable to stringify notify_output");
        println!("  {}", notify_output_stdout);
    } else {
        let notify_output_stderr =
            String::from_utf8(notify_output.stderr).expect("Unable to stringify notify_output");
        println!(
            "  Error encountered while rebuilding, doing nothing: {}",
            notify_output_stderr
        );
    }
}

/// Determine whether the machine should be rebuilt by comparing the date
/// of the latest remote commit vs. the latest local commit
async fn should_rebuild(
    cfg: Config,
    octocrab: Octocrab,
    latest_remote_date: DateTime<Utc>,
    status: &str,
) -> Result<bool, octocrab::Error> {
    let nix_cfg_dir = nix_cfg_dir(cfg.clone());
    let latest_local = Command::new("git")
        .args(["-C", nix_cfg_dir.as_ref(), "rev-parse", "HEAD"])
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
            nix_cfg_dir.as_ref(),
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
            nix_cfg_dir.as_ref(),
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

/// Switch to the main branch, fetch the most recent commit, and rebuild via nh
fn fetch_and_rebuild(cfg: Config) {
    let nix_cfg_dir = nix_cfg_dir(cfg.clone());
    println!("\nSwitching to {}", cfg.branch);
    let switch_output = Command::new("git")
        .args(["-C", nix_cfg_dir.as_ref(), "switch", cfg.branch.as_ref()])
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
        .args(["-C", nix_cfg_dir.as_ref(), "fetch"])
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
        return;
    }

    // `spawn()` would probably enable visualizing this
    println!("Rebuilding");
    let nh_output = Command::new("nh")
        .args([cfg.nh_system.clone(), cfg.nh_cmd.clone()])
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
                        fetch_and_rebuild(cfg);
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

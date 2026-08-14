use crate::util::{ExitCode, user_input};
use serde::{Deserialize, Serialize};
use std::{env, fs::File, io::Read, path::PathBuf, process::exit};

pub const CFG_FILE: &str = "/etc/fetch-rs/config.toml";

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub flake_dir: String,
    pub owner: String,
    pub repo: String,
    pub branch: String,
    pub workflow: String,
    pub rebuild_system: String,
    pub rebuild_cmd: String,
    pub notify: bool,
    pub ntfy_url: String,
    pub ntfy_topic: String,
}

impl Config {
    pub fn new(flake_dir: &str, owner: &str, repo: &str) -> Self {
        Config {
            flake_dir: flake_dir.into(),
            owner: owner.into(),
            repo: repo.into(),
            branch: "main".into(),
            workflow: "build.yaml".into(),
            #[cfg(not(target_os = "macos"))]
            rebuild_system: "nixos".into(),
            #[cfg(target_os = "macos")]
            rebuild_system: "darwin".into(),
            rebuild_cmd: "switch".into(),
            notify: false,
            ntfy_url: "ntfy.sh".into(),
            ntfy_topic: "".into(),
        }
    }

    /// Deserialize the raw configuration content
    pub fn deserialize(config_content: String) -> Self {
        match toml::from_str(&config_content) {
            Ok(config) => config,
            Err(_) => {
                eprintln!();
                Config::new("", "", "")
            }
        }
    }

    /// Serialize the raw configuration content
    fn serialize(flake_dir: &str, owner: &str, repo: &str) -> String {
        let config = Config::new(flake_dir, owner, repo);
        toml::to_string(&config).unwrap_or_default()
    }

    /// Read the raw configuration content at the given path, creating the file if needed.
    pub fn read(path: PathBuf) -> Result<String, anyhow::Error> {
        if std::fs::metadata(&path).is_err() {
            Config::create();
            exit(ExitCode::NoOp.into())
        }

        let mut config_file = File::open(path)?;
        let mut config_content = String::new();
        match config_file.read_to_string(&mut config_content) {
            Ok(_) => (),
            Err(_) => {
                config_content = String::from("");
                eprintln!();
            }
        }

        Ok(config_content)
    }

    /// Create the default configuration content
    fn create() {
        println!(
            "Running first time setup-let's start with some basic info on your GitHub-based nix config\n"
        );
        let mut flake =
            user_input("Flake Directory (`~` will be replaced with your current $HOME): ");
        let owner = user_input("\nRepo Owner: ");
        let repo = user_input("\nRepo Name: ");
        flake = Self::flake_dir(flake, owner.clone());
        let config_content = Config::serialize(&flake, &owner, &repo);
        let path = PathBuf::from(CFG_FILE);

        println!(
            "\nHere's your initial config:\n{}\nTo proceed, write it to: {}\nFeel free to adjust specific settings like (e.g., change branch name, enable notifications, etc.)",
            config_content,
            path.display()
        )
    }

    /// Retrieve the NixOS / nix-darwin config directory
    fn flake_dir(mut dir: String, owner: String) -> String {
        if dir.starts_with("~") {
            let fallback_home_dir = format!("/home/{}", owner);
            let home_dir = env::home_dir().unwrap_or(fallback_home_dir.clone().into());
            let home_dir = home_dir.to_str().unwrap_or(fallback_home_dir.as_str());
            dir.remove(0);
            format!("{}{}", home_dir, dir)
        } else {
            dir
        }
    }
}

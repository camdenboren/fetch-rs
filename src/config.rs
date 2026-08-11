use crate::util::user_input;
use serde::{Deserialize, Serialize};
use std::{
    env,
    fs::{File, write},
    io::Read,
    path::PathBuf,
};

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    flake_dir: String,
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
                println!();
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
        let config_content = Config::serialize(
            &user_input("Flake Directory: "),
            &user_input("\nRepo Owner: "),
            &user_input("\nRepo Name: "),
        );
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
pub fn flake_dir(cfg: Config) -> String {
    let mut flake_dir = cfg.flake_dir;
    if flake_dir.starts_with("~") {
        let default_home_dir = format!("/home/{}", cfg.owner);
        let home_dir = env::home_dir().unwrap_or(default_home_dir.clone().into());
        let home_dir = home_dir.to_str().unwrap_or(default_home_dir.as_str());
        flake_dir.remove(0);
        format!("{}{}", home_dir, flake_dir)
    } else {
        flake_dir
    }
}

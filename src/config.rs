use crate::{
    git::{git_config, origin},
    util::fallback_user,
};
use serde::{Deserialize, Serialize};
use std::{env, fs::File, io::Read, path::PathBuf};

pub const CFG_DIR: &str = "/etc/fetch-rs";
pub const CFG_FILE: &str = "config.toml";
pub const GIT_CFG_FILE: &str = ".gitconfig";
#[cfg(not(target_os = "macos"))]
const FALLBACK_HOME: &str = "/home";
#[cfg(target_os = "macos")]
const FALLBACK_HOME: &str = "/Users";

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
                tracing::info!("Failed to deserialize configuration-passing the default");
                Config::new("", "", "")
            }
        }
    }

    /// Serialize the raw configuration content
    fn serialize(flake_dir: &str, owner: &str, repo: &str) -> String {
        // $F_RS_FLAKE takes precedence
        let mut config = Config::new(flake_dir, owner, repo);
        if let Ok(flake_var) = env::var("F_RS_FLAKE") {
            config.flake_dir = Self::flake_dir(flake_var, fallback_user());
        }
        toml::to_string(&config).unwrap_or_default()
    }

    /// Read the raw configuration content at the given path, creating the file if needed.
    pub fn read(path: PathBuf) -> Result<String, anyhow::Error> {
        if std::fs::metadata(&path).is_err() {
            Config::write();
        }

        let mut config_file = File::open(path)?;
        let mut config_content = String::new();
        match config_file.read_to_string(&mut config_content) {
            Ok(_) => tracing::info!("Read config content"),
            Err(_) => {
                config_content = String::from("");
                tracing::info!(
                    "Failed to parse the content of the configuration file-defaulting to nothing"
                );
            }
        }

        Ok(config_content)
    }

    /// Create the default configuration content
    fn write() {
        let dir_path = PathBuf::from(CFG_DIR);
        let file_path = dir_path.join(CFG_FILE);
        let git_file_path = dir_path.join(GIT_CFG_FILE);
        let mut flake = env::var("F_RS_FLAKE")
            .expect("Unable to create initial config due missing $F_RS_FLAKE - doing nothing");
        flake = Self::flake_dir(flake, fallback_user());
        git_config(git_file_path, flake.clone());

        let origin = origin(flake.clone());
        let owner = origin.first().expect("");
        let repo = origin.last().expect("");
        let config_content = Config::serialize(&flake, owner, repo);

        if std::fs::metadata(&dir_path).is_err() {
            tracing::info!("Launched without access to the config directory");
        }
        if std::fs::metadata(file_path.clone()).is_err() {
            match std::fs::write(file_path.clone(), &config_content) {
                Ok(_) => tracing::info!(
                    "Wrote initial config to: {} - feel free to adjust specific settings like (e.g., change branch name, enable notifications, etc.)",
                    file_path.display()
                ),
                Err(_) => tracing::info!("Failed to create config file"),
            }
        }
    }

    /// Replace `~` w/ the first user listed
    fn flake_dir(mut dir: String, fallback_user: String) -> String {
        if dir.starts_with("~") {
            tracing::info!(
                "Defaulting to {} user since `~` is ambiguous",
                fallback_user
            );
            let fallback_home_dir = format!("{}/{}", FALLBACK_HOME, fallback_user);
            dir.remove(0);
            format!("{}{}", fallback_home_dir, dir)
        } else {
            dir
        }
    }
}

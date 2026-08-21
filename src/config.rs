use crate::{git::origin, util::ExitCode};
use serde::{Deserialize, Serialize};
use std::{env, fs::File, io::Read, path::PathBuf, process::exit};

pub const CFG_FILE: &str = "config.toml";

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
            config.flake_dir = Self::validate_flake_dir(flake_var);
        }
        toml::to_string(&config).unwrap_or_default()
    }

    /// Read the raw configuration content at the given path, creating the file if needed.
    pub fn read(path: PathBuf) -> Result<String, anyhow::Error> {
        if std::fs::metadata(&path).is_err() {
            Config::write();
        }

        let mut config_file = File::open(path.clone())?;
        let mut config_content = String::new();
        match config_file.read_to_string(&mut config_content) {
            Ok(_) => tracing::info!("Read config content"),
            Err(_) => {
                config_content = String::from("");
                tracing::error!(
                    "Failed to parse the content of the configuration file at {} - defaulting to nothing",
                    path.display()
                );
            }
        }

        Ok(config_content)
    }

    /// Create the default configuration content
    fn write() {
        let flake = env::var("F_RS_FLAKE");
        if flake.is_err() {
            tracing::error!(
                "Unable to read $F_RS_FLAKE-is it set? Failed to create initial config"
            );
            exit(ExitCode::NoOp.into());
        }
        let mut flake = flake.unwrap();
        let cfg_dir = env::var("F_RS_CONFIG");
        if cfg_dir.is_err() {
            tracing::error!(
                "Unable to read $F_RS_CONFIG-is it set? Failed to create initial config"
            );
            exit(ExitCode::NoOp.into());
        }
        let cfg_dir = PathBuf::from(cfg_dir.unwrap());
        let cfg_path = cfg_dir.join(CFG_FILE);
        flake = Self::validate_flake_dir(flake);

        let origin = origin(flake.clone());
        let owner = origin.first().expect("");
        let repo = origin.last().expect("");
        let config_content = Config::serialize(&flake, owner, repo);

        if std::fs::metadata(&cfg_dir).is_err() {
            tracing::info!("Launched without access to the config directory");
        }
        if std::fs::metadata(cfg_path.clone()).is_err() {
            match std::fs::write(cfg_path.clone(), &config_content) {
                Ok(_) => tracing::info!(
                    "Wrote initial config to: {} - feel free to adjust specific settings like (e.g., change branch name, enable notifications, etc.)",
                    cfg_path.display()
                ),
                Err(_) => tracing::info!("Failed to create config file"),
            }
        }
    }

    /// Make sure $F_RS_FLAKE doesn't include ambiguous `~`
    fn validate_flake_dir(dir: String) -> String {
        if dir.starts_with("~") {
            tracing::error!(
                "Ambiguous $HOME included in $F_RS_FLAKE. As fetch-rs leverages multiple users, use an absolute path to avoid ambiguity"
            );
            exit(ExitCode::NoOp.into());
        } else {
            dir
        }
    }
}

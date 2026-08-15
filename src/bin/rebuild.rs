use fetch_rs::{
    config::{CFG_FILE, Config},
    git::git_config,
    util::*,
};
use std::{
    env,
    os::unix::process::ExitStatusExt,
    path::PathBuf,
    process::{Command, exit},
};

fn main() -> Result<(), anyhow::Error> {
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    tracing::subscriber::set_global_default(subscriber)?;
    let cfg_path = env::var("F_RS_CONFIG");
    if cfg_path.is_err() {
        tracing::error!("Unable to read $F_RS_CONFIG-is it set? Failed to create initial config");
        exit(ExitCode::NoOp.into());
    }
    let cfg_path = PathBuf::from(cfg_path.unwrap_or_default()).join(CFG_FILE);
    let git_path = env::var("GIT_CONFIG_GLOBAL");
    if git_path.is_err() {
        tracing::error!(
            "Unable to read $GIT_CONFIG_GLOBAL-is it set? Failed to create initial config"
        );
        exit(ExitCode::NoOp.into());
    }
    let git_path = PathBuf::from(git_path.unwrap_or_default());
    let config_content = Config::read(cfg_path.clone()).unwrap_or("".into());
    let cfg = Config::deserialize(config_content);

    if std::fs::metadata(&cfg_path).is_err() {
        tracing::info!("Unable to access cfg path: {}", cfg_path.display());
    }
    if std::fs::metadata(&git_path).is_err() {
        tracing::info!("Unable to access git cfg path: {}", git_path.display());
        git_config(git_path, cfg.flake_dir.clone());
    }
    if std::fs::metadata(cfg.flake_dir.clone()).is_err() {
        tracing::info!("Unable to access flake path: {}", cfg.flake_dir.clone());
    }

    // `spawn()` would probably enable visualizing this
    tracing::info!("Rebuilding via flake in {}", cfg.flake_dir.clone());
    let rebuild_output = Command::new(format!("{}-rebuild", cfg.rebuild_system))
        .args([
            cfg.rebuild_cmd.clone(),
            "--flake".into(),
            cfg.flake_dir.clone(),
        ])
        .output();
    match rebuild_output {
        Ok(output) => {
            let output_status = output.status;
            match output_status.into_raw() {
                0 => {
                    let output_stdout = String::from_utf8(output.stdout).unwrap_or_default();
                    if !output_stdout.trim().is_empty() {
                        tracing::info!("Rebuild succeeded (stdout): {}", output_stdout.trim());
                    } else {
                        tracing::info!("Rebuild succeeded (no stdout output)");
                    }
                }
                _ => {
                    if cfg.notify {
                        notify();
                    }
                    let output_stderr = String::from_utf8(output.stderr).unwrap_or_default();
                    if !output_stderr.trim().is_empty() {
                        tracing::error!("Rebuild failed (stderr): {}", output_stderr.trim());
                    } else {
                        tracing::error!("Rebuild failed (no stderr output)");
                    }
                    exit(ExitCode::Failure.into())
                }
            }
        }
        Err(e) => {
            if cfg.notify {
                notify();
            }
            tracing::error!("Rebuild failed. Error {}", e)
        }
    }

    Ok(())
}

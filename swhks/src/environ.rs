//! environ.rs
//! Defines modules and structs for handling environment variables and paths.

use std::{env::VarError, path::PathBuf};

use nix::unistd;

// The main struct for handling environment variables.
// Contains the values of the environment variables in the form of PathBuffers.
pub struct Env {
    pub data_home: PathBuf,
    pub runtime_dir: PathBuf,
}

impl Env {
    /// Constructs a new Env struct.
    /// This function is called only once and the result is stored in a static variable.
    pub fn construct() -> Result<Self, VarError> {
        // Should exist in any system, so not handling errors here.
        let home = Self::get_env("HOME")?;

        let data_home = Self::get_env("XDG_DATA_HOME").unwrap_or_else(|_| {
            log::warn!("XDG_DATA_HOME not set, falling back to ~/.local/share");
            home.join(".local/share")
        });

        let runtime_dir = Self::get_env("XDG_RUNTIME_DIR").unwrap_or_else(|_| {
            log::warn!("XDG_RUNTIME_DIR not set, falling back to /run/user/<uid>");
            PathBuf::from(format!("/run/user/{}", unistd::Uid::current()))
        });

        Ok(Self { data_home, runtime_dir })
    }

    /// Function to ensure paths are available.
    pub fn ensure_paths_exist(&self) -> std::io::Result<()> {
        // Create data_home directory in case of clean system installation
        if !self.data_home.exists() {
            log::info!("Creating data directory: {}", self.data_home.display());
            std::fs::create_dir_all(&self.data_home)?;
        }

        // For runtime_dir, only check existence as it must be created by the system (systemd/init) at /run/user/<uid>
        if !self.runtime_dir.exists() {
            log::error!(
                "Runtime directory {} does not exist. This should be created by the system.",
                self.runtime_dir.display()
            );
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Runtime directory does not exist",
            ));
        }

        Ok(())
    }

    /// Gets an environment variable and converts it to PathBuf.
    /// Does not check if the path exists.
    fn get_env(name: &str) -> Result<PathBuf, VarError> {
        std::env::var(name).map(PathBuf::from)
    }
}

//! Environ.rs
//! Defines modules and structs for handling environment variables and paths.

use std::{env::VarError, path::PathBuf};

use nix::unistd;

// The main struct for handling environment variables.
// Contains the values of the environment variables in the form of PathBuffers.
pub struct Env {
    pub data_home: PathBuf,
    pub runtime_dir: PathBuf,
}

/// Error type for the Env struct.
/// Contains all the possible errors that can occur when trying to get an environment variable.
#[derive(Debug)]
pub enum EnvError {
    DataHomeNotSet,
    HomeNotSet,
    PathNotFound,
    GenericError(String),
}

impl Env {
    /// Constructs a new Env struct.
    /// This function is called only once and the result is stored in a static variable.
    pub fn construct() -> Self {
        let home = match Self::get_env("HOME") {
            Ok(val) => val,
            Err(e) => match e {
                EnvError::HomeNotSet => {
                    log::error!(
                        "HOME Variable is not set/found, cannot fall back on hardcoded path for XDG_DATA_HOME."
                    );
                    std::process::exit(1);
                }
                EnvError::GenericError(err) => {
                    log::error!("Generic error: {err:#?}");
                    std::process::exit(1);
                }
                _ => {
                    log::error!("Unexpected error: {e:#?}");
                    std::process::exit(1);
                }
            },
        };

        let data_home = match Self::get_env("XDG_DATA_HOME") {
            Ok(val) => val,
            Err(e) => match e {
                EnvError::DataHomeNotSet | EnvError::PathNotFound => {
                    log::warn!(
                        "XDG_DATA_HOME Variable is not set, falling back on hardcoded path."
                    );
                    home.join(".local/share")
                }
                EnvError::GenericError(err) => {
                    log::error!("Generic error: {err:#?}");
                    std::process::exit(1);
                }
                _ => {
                    log::error!("Unexpected error: {e:#?}");
                    std::process::exit(1);
                }
            },
        };

        let runtime_dir = PathBuf::from(format!("/run/user/{}", unistd::Uid::current()));

        Self { data_home, runtime_dir }
    }

    /// Actual interface to get the environment variable.
    fn get_env(name: &str) -> Result<PathBuf, EnvError> {
        match std::env::var(name) {
            Ok(val) => match PathBuf::from(&val).exists() {
                true => Ok(PathBuf::from(val)),
                false => Err(EnvError::PathNotFound),
            },
            Err(e) => match e {
                VarError::NotPresent => match name {
                    "XDG_DATA_HOME" => Err(EnvError::DataHomeNotSet),
                    "HOME" => Err(EnvError::HomeNotSet),
                    _ => Err(EnvError::GenericError(format!("{name} not set"))),
                },
                VarError::NotUnicode(_) => {
                    Err(EnvError::GenericError(format!("{name} not unicode")))
                }
            },
        }
    }
}

use std::{env::VarError, path::PathBuf};

pub struct Env {
    pub pkexec_id: u32,
    pub config_folder_location: PathBuf,
    pub runtime_socket: PathBuf,
    pub runtime_dir: PathBuf,
}

#[derive(Debug)]
pub enum EnvError {
    PkexecNotFound,
    GenericError(String),
}

impl Env {
    pub fn construct() -> Self {
        let pkexec_id = match Self::get_env("PKEXEC_UID") {
            Ok(val) => match val.parse::<u32>() {
                Ok(val) => val,
                Err(_) => {
                    log::error!("Failed to launch swhkd!!!");
                    log::error!("Make sure to launch the binary with pkexec.");
                    std::process::exit(1);
                }
            },
            Err(_) => {
                log::error!("Failed to launch swhkd!!!");
                log::error!("Make sure to launch the binary with pkexec.");
                std::process::exit(1);
            }
        };
        let config_folder_location = PathBuf::from("/etc");
        let runtime_socket = PathBuf::from(format!("/run/user/{}", pkexec_id));
        let runtime_dir = PathBuf::from(format!("/run/user/{}", pkexec_id));

        Self { pkexec_id, config_folder_location, runtime_dir, runtime_socket }
    }

    fn get_env(name: &str) -> Result<String, EnvError> {
        match std::env::var(name) {
            Ok(val) => Ok(val),
            Err(e) => match e {
                VarError::NotPresent => match name {
                    "PKEXEC_UID" => Err(EnvError::PkexecNotFound),
                    _ => Err(EnvError::GenericError(e.to_string())),
                },
                VarError::NotUnicode(_) => {
                    Err(EnvError::GenericError("Not a valid unicode".to_string()))
                }
            },
        }
    }

    pub fn fetch_config_path(&self) -> PathBuf {
        PathBuf::from(&self.config_folder_location).join("swhkd/swhkdrc")
    }

    pub fn fetch_runtime_socket_path(&self) -> PathBuf {
        PathBuf::from(&self.runtime_dir).join("swhkd.sock")
    }
}

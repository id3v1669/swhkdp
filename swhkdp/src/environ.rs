use std::{env::VarError, path::PathBuf};

pub struct Env {
    pub pkexec_id: u32,
    pub config_folder_location: PathBuf,
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
                Err(_) => Self::pkexec_err(),
            },
            Err(e) => match e {
                EnvError::PkexecNotFound => {
                    log::error!("PKEXEC_UID not found in environment variables.");
                    Self::pkexec_err();
                }
                EnvError::GenericError(e) => {
                    log::error!("Error: {e}");
                    Self::pkexec_err();
                }
            },
        };
        let config_folder_location = PathBuf::from("/etc");
        let runtime_dir = PathBuf::from(format!("/run/user/{pkexec_id}"));

        Self { pkexec_id, config_folder_location, runtime_dir }
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
        PathBuf::from(&self.config_folder_location).join("swhkdp/swhkdp.yml")
    }

    pub fn fetch_runtime_socket_path(&self) -> PathBuf {
        PathBuf::from(&self.runtime_dir).join("swhkdp.sock")
    }

    pub fn pkexec_err() -> ! {
        log::error!("Failed to launch swhkdp!!!");
        log::error!("Make sure to launch the binary with pkexec.");
        std::process::exit(1);
    }
}

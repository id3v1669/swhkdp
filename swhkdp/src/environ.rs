use std::path::PathBuf;

pub struct Env {
    pub runtime_dir: PathBuf,
}

impl Env {
    pub fn construct() -> Self {
        let pkexec_id = match std::env::var("PKEXEC_UID") {
            Ok(val) => match val.parse::<u32>() {
                Ok(val) => val,
                Err(_) => Self::pkexec_err(),
            },
            Err(_) => {
                log::error!("PKEXEC_UID not found in environment variables.");
                Self::pkexec_err()
            }
        };
        let runtime_dir = PathBuf::from(format!("/run/user/{pkexec_id}"));

        Self { runtime_dir }
    }

    pub fn fetch_runtime_socket_path(&self) -> PathBuf {
        self.runtime_dir.join("swhkdp.sock")
    }

    pub fn pkexec_err() -> ! {
        log::error!("Failed to launch swhkdp!!!");
        log::error!("Make sure to launch the binary with pkexec.");
        std::process::exit(1);
    }
}

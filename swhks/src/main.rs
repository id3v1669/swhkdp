use clap::Parser;
use environ::Env;
use nix::libc;
use nix::sys::stat::{Mode, umask};
use std::io::Read;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{
    env,
    env::VarError,
    fs,
    fs::OpenOptions,
    os::unix::net::UnixListener,
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
    process::{Command, Stdio, exit, id},
};
use sysinfo::{ProcessRefreshKind, RefreshKind, System, UpdateKind};

mod environ;

/// IPC Server for swhkdp
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// Set a custom log file. (Defaults to ${XDG_DATA_HOME:-$HOME/.local/share}/swhks-current_unix_time.log)
    #[arg(short, long, value_name = "FILE")]
    log: Option<PathBuf>,

    /// Enable Debug Mode
    #[arg(short, long)]
    debug: bool,
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();
    if args.debug {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("swhks=trace"))
            .init();
    } else {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("swhks=warn"))
            .init();
    }

    log::trace!("Setting process umask.");
    umask(Mode::S_IWGRP | Mode::S_IWOTH);

    // This is used to initialize the environment variables only once
    let environ = Env::construct().unwrap_or_else(|e| {
        match e {
            VarError::NotPresent => {
                eprintln!("HOME environment variable is not set");
            }
            VarError::NotUnicode(_) => {
                eprintln!("HOME environment variable contains invalid Unicode");
            }
        }
        std::process::exit(1);
    });

    environ.ensure_paths_exist().unwrap_or_else(|e| {
        eprintln!("Failed to create/verify necessary directories: {e}");
        std::process::exit(1);
    });

    let (pid_file_path, sock_file_path) = get_file_paths(&environ);

    let log_file_name = if let Some(val) = args.log {
        val
    } else {
        let time = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(n) => n.as_secs().to_string(),
            Err(_) => {
                log::error!("SystemTime before UnixEpoch!");
                exit(1);
            }
        };

        format!("{}/swhks/swhks-{}.log", environ.data_home.to_string_lossy(), time).into()
    };

    let log_path = Path::new(&log_file_name);
    if let Some(p) = log_path.parent()
        && !p.exists()
        && let Err(e) = fs::create_dir_all(p)
    {
        log::error!("Failed to create log dir: {e}");
    }

    if Path::new(&pid_file_path).exists() {
        log::trace!("Reading {pid_file_path} file and checking for running instances.");
        let swhks_pid = match fs::read_to_string(&pid_file_path) {
            Ok(swhks_pid) => swhks_pid,
            Err(e) => {
                log::error!("Unable to read {e} to check all running instances");
                exit(1);
            }
        };
        log::debug!("Previous PID: {swhks_pid}");

        let mut sys = System::new_with_specifics(
            RefreshKind::nothing().with_processes(
                ProcessRefreshKind::nothing().with_exe(UpdateKind::Always)
            ),
        );
        sys.refresh_all();
        for (pid, process) in sys.processes() {
            if pid.to_string() == swhks_pid && process.exe() == env::current_exe().ok().as_deref()
            {
                log::error!("Server is already running!");
                exit(1);
            }
        }
    }

    if Path::new(&sock_file_path).exists() {
        log::trace!("Sockfile exists, attempting to remove it.");
        match fs::remove_file(&sock_file_path) {
            Ok(_) => {
                log::debug!("Removed old socket file");
            }
            Err(e) => {
                log::error!("Error removing the socket file!: {e}");
                log::error!("You can manually remove the socket file: {sock_file_path}");
                exit(1);
            }
        };
    }

    match fs::write(&pid_file_path, id().to_string()) {
        Ok(_) => {}
        Err(e) => {
            log::error!("Unable to write to {pid_file_path}: {e}");
            exit(1);
        }
    }

    let listener = UnixListener::bind(sock_file_path)?;
    loop {
        match listener.accept() {
            Ok((mut socket, address)) => {
                let mut response = String::new();
                socket.read_to_string(&mut response)?;
                run_system_command(&response, log_path);
                log::debug!("Socket: {socket:?} Address: {address:?} Response: {response}");
            }
            Err(e) => log::error!("accept function failed: {e:?}"),
        }
    }
}

fn get_file_paths(env: &Env) -> (String, String) {
    let pid_file_path = format!("{}/swhks.pid", env.runtime_dir.to_string_lossy());
    let sock_file_path = format!("{}/swhkdp.sock", env.runtime_dir.to_string_lossy());

    (pid_file_path, sock_file_path)
}

fn run_system_command(command: &str, log_path: &Path) {
    // Double-fork with setsid to fully detach child processes.
    // This ensures commands survive swhks restarts/stops when running under a systemd service.
    match unsafe { nix::unistd::fork() } {
        Ok(nix::unistd::ForkResult::Parent { child }) => {
            let _ = nix::sys::wait::waitpid(child, None);
        }
        Ok(nix::unistd::ForkResult::Child) => {
            let _ = nix::unistd::setsid();

            match unsafe { nix::unistd::fork() } {
                Ok(nix::unistd::ForkResult::Parent { .. }) => {
                    unsafe { libc::_exit(0) };
                }
                Ok(nix::unistd::ForkResult::Child) => {
                    let err = Command::new("sh")
                        .arg("-c")
                        .arg(command)
                        .stdin(Stdio::null())
                        .stdout(match OpenOptions::new().append(true).create(true).open(log_path) {
                            Ok(f) => f,
                            Err(_) => unsafe { libc::_exit(1) },
                        })
                        .stderr(match OpenOptions::new().append(true).create(true).open(log_path) {
                            Ok(f) => f,
                            Err(_) => unsafe { libc::_exit(1) },
                        })
                        .exec();

                    // exec() only returns on error
                    log::error!("Failed to exec command: {err}");
                    unsafe { libc::_exit(1) };
                }
                Err(_) => {
                    log::error!("Second fork failed for command: {command}");
                    unsafe { libc::_exit(1) };
                }
            }
        }
        Err(e) => {
            log::error!("Fork failed for command: {command}: {e}");
        }
    }
}

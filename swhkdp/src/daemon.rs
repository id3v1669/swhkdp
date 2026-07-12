use crate::config::Value;
use clap::Parser;
use config::Hotkey;
use evdev::{AttributeSet, Device, EventSummary, KeyCode};
use nix::{
    sys::stat::{Mode, umask},
    unistd::Uid,
};
use signal_hook::consts::signal::*;
use signal_hook_tokio::Signals;
#[cfg(feature = "macro")]
use std::sync::Arc;
#[cfg(feature = "macro")]
use std::sync::atomic::{AtomicBool, Ordering};
use std::{
    collections::HashMap,
    error::Error,
    fs,
    fs::Permissions,
    io::prelude::*,
    os::unix::fs::PermissionsExt,
    os::unix::io::AsRawFd,
    path::{Path, PathBuf},
    process::{exit, id},
};
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System, UpdateKind};
use tokio::io::AsyncWriteExt;
use tokio::net::UnixStream;
use tokio::select;
use tokio::time::Duration;
use tokio::time::{Instant, sleep};
use tokio_stream::{StreamExt, StreamMap};
use tokio_udev::{AsyncMonitorSocket, EventType, MonitorBuilder};

// shouldn't be reached
// TODO: #shrink
const IPC_QUEUE_CAP: usize = 256;

/// The fixed system config path. In release builds this is the only config
const RELEASE_CONFIG_PATH: &str = "/etc/swhkdp/config.kdl";

// TODO: #shrink2
#[cfg(feature = "macro")]
const MACRO_QUEUE_CAP: usize = 256;

mod config;
mod environ;
#[cfg(feature = "macro")]
mod macro_runner;
#[cfg(not(debug_assertions))]
mod perms;
mod rel_mask;
mod uinput;

#[cfg(feature = "macro")]
struct MacroState {
    stop: Arc<AtomicBool>,
    handle: tokio::task::JoinHandle<()>,
    macro_type: config::MacroType,
    trigger_keybind: config::KeyBinding,
}

#[cfg(not(feature = "macro"))]
struct MacroState;

struct DeviceState {
    state_modifiers: AttributeSet<KeyCode>,
    state_modifiers_count: usize,
    state_keysyms: AttributeSet<KeyCode>,
    allowed_rel: u16,
}

impl DeviceState {
    fn new(device: &Device) -> DeviceState {
        let allowed_rel = rel_mask::allowed_rel_axes(device.supported_relative_axes());
        if let Err(e) = rel_mask::apply_kernel_rel_mask(device.as_raw_fd(), allowed_rel) {
            log::debug!(
                "EVIOCSMASK failed for '{}' ({e}), filtering REL events in userspace",
                device.name().unwrap_or("[unknown]")
            );
        }

        DeviceState {
            state_modifiers: AttributeSet::new(),
            state_modifiers_count: 0,
            state_keysyms: AttributeSet::new(),
            allowed_rel,
        }
    }
}

/// Simple Wayland Hotkey Daemon
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// Set a custom config file path. Debug builds only
    #[cfg(debug_assertions)]
    #[arg(short = 'c', long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Set a custom repeat cooldown duration. Default is 250ms.
    #[arg(short = 'C', long)]
    cooldown: Option<u64>,

    /// Enable Debug Mode
    #[arg(short, long)]
    debug: bool,

    /// Take a list of devices from the user
    #[arg(short = 'D', long, num_args = 0.., value_delimiter = '|')]
    devices: Vec<String>,

    /// Take a list of devices to ignore from the user
    #[arg(short = 'I', long = "ignore-devices", num_args = 0.., value_delimiter = '|')]
    ignore_devices: Vec<String>,

    /// Read and print keys
    #[arg(short = 'w', long)]
    watch: bool,

    /// Verify config
    #[arg(long = "verify-config")]
    verify_config: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let default_cooldown: u64 = 250;

    if args.debug {
        env_logger::Builder::from_env(env_logger::Env::default().filter_or("RUST_LOG", "debug"))
            .init();
    } else {
        env_logger::Builder::from_env(
            env_logger::Env::default().filter_or("RUST_LOG", "warn,info"),
        )
        .init();
    }
    log::debug!("Logger initialized.");

    if args.verify_config {
        return run_verify_mode(&resolve_config_path(&args));
    }

    check_pkexec();

    let env = environ::Env::construct();
    log::debug!("Environment Aquired");

    setup_swhkdp(&env.runtime_dir);

    if args.watch {
        let runtime_dir = env.runtime_dir;
        let pidfile = runtime_dir.join("swhkdp.pid");
        if pidfile.exists()
            && let Ok(swhkdp_pid_str) = fs::read_to_string(&pidfile)
        {
            let swhkdp_pid = swhkdp_pid_str.trim();
            if !swhkdp_pid.is_empty()
                && let Ok(pid) = swhkdp_pid.parse::<u32>()
            {
                let mut sys = System::new_with_specifics(
                    RefreshKind::nothing()
                        .with_processes(ProcessRefreshKind::nothing().with_exe(UpdateKind::Always)),
                );
                sys.refresh_processes_specifics(
                    ProcessesToUpdate::All,
                    true,
                    ProcessRefreshKind::nothing().with_exe(UpdateKind::Always),
                );
                let current_pid = id();
                let current_exe = std::env::current_exe().unwrap();
                for (p, process) in sys.processes() {
                    if p.as_u32() != current_pid
                        && p.as_u32() == pid
                        && process.exe() == Some(&current_exe)
                    {
                        log::error!("Another instance of swhkdp is already running!");
                        log::error!("pid of existing swhkdp process: {p}");
                        exit(1);
                    }
                }
            }
        }
        return run_watch_mode(&args.devices, &args.ignore_devices).await;
    }

    let config_file_path = resolve_config_path(&args);

    let load_config = || {
        log::debug!("Using config file path: {config_file_path:#?}");

        if !config_file_path.exists() {
            log::warn!("No config found at path: {config_file_path:#?}");
            create_default_config(&config_file_path);
        }

        let content = read_config_content(&config_file_path);

        match config::load_from_str(&content) {
            Err(e) => {
                log::error!("Config Error: {e}");
                exit(1)
            }
            Ok(out) => out,
        }
    };

    let mut config = load_config();
    let mut modes = config.modes;
    let mut current_mode: usize = config.default_mode;
    let mut default_mode: usize = config.default_mode;
    let arg_add_devices = args.devices;
    let arg_ignore_devices = args.ignore_devices;

    let to_add =
        |dev: &Device| arg_add_devices.contains(&dev.name().unwrap_or("[unknown]").to_string());
    let to_ignore =
        |dev: &Device| arg_ignore_devices.contains(&dev.name().unwrap_or("[unknown]").to_string());

    let supported_devices: Vec<(PathBuf, Device)> = {
        if arg_add_devices.is_empty() {
            log::debug!("Attempting to find all supported devices file descriptors.");
            evdev::enumerate()
                .filter(|(_, dev)| !to_ignore(dev) && check_device_is_supported(dev))
                .collect()
        } else {
            evdev::enumerate().filter(|(_, dev)| !to_ignore(dev) && to_add(dev)).collect()
        }
    };

    //printing all supported devices
    for (path, device) in supported_devices.iter() {
        log::info!("Supported device: {}", device.name().unwrap_or("[unknown]"));
        log::info!("Path: {}", path.to_str().unwrap());
    }

    if supported_devices.is_empty() {
        log::error!("No valid device was detected!");
        exit(1);
    }

    log::debug!("Supported device(s) detected: {}", supported_devices.len());

    // TODO: #libissue
    // Apparently, having a single uinput device with keys, relative axes and switches
    // prevents some libraries to listen to these events. The easy fix is to have separate
    // virtual devices, one for keys and relative axes (`uinput_device`) and another one
    // just for switches (`uinput_switches_device`).
    let mut uinput_device = match uinput::create_uinput_device() {
        Ok(dev) => dev,
        Err(e) => {
            log::error!("Failed to create uinput device: {e:#?}");
            exit(1);
        }
    };

    let mut uinput_switches_device = match uinput::create_uinput_switches_device() {
        Ok(dev) => dev,
        Err(e) => {
            log::error!("Failed to create uinput switches device: {e:#?}");
            exit(1);
        }
    };

    let mut udev =
        AsyncMonitorSocket::new(MonitorBuilder::new()?.match_subsystem("input")?.listen()?)?;

    let repeat_cooldown_duration: u64 = args.cooldown.unwrap_or(default_cooldown);

    let mut signals = Signals::new([
        SIGUSR1, SIGUSR2, SIGHUP, SIGABRT, SIGBUS, SIGCHLD, SIGCONT, SIGINT, SIGPIPE, SIGQUIT,
        SIGSYS, SIGTERM, SIGTRAP, SIGTSTP, SIGVTALRM, SIGXCPU, SIGXFSZ,
    ])?;

    let mut execution_is_paused = false;
    let mut last_hotkey: Option<config::Hotkey> = None;
    let mut pending_release: bool = false;
    let mut active_macro: Option<MacroState> = None;
    let mut device_states = HashMap::new();
    let mut device_stream_map = StreamMap::new();

    for (path, mut device) in supported_devices.into_iter() {
        let _ = device.grab();
        let path = match path.to_str() {
            Some(p) => p,
            None => {
                continue;
            }
        };
        device_states.insert(path.to_string(), DeviceState::new(&device));
        device_stream_map.insert(path.to_string(), device.into_event_stream()?);
    }

    // The initial sleep duration is never read because last_hotkey is initialized to None
    let hotkey_repeat_timer = sleep(Duration::from_millis(0));
    tokio::pin!(hotkey_repeat_timer);

    // macro->loop channel (loop owns the device, no lock), pressure instead of dropping.
    // never drop macro events
    #[cfg(feature = "macro")]
    let (macro_emit_tx, mut macro_emit_rx) =
        tokio::sync::mpsc::channel::<Vec<evdev::InputEvent>>(MACRO_QUEUE_CAP);

    let socket_file_path = env.fetch_runtime_socket_path();
    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::channel::<String>(IPC_QUEUE_CAP);
    tokio::spawn(ipc_sender(cmd_rx, socket_file_path));
    loop {
        select! {
            _ = &mut hotkey_repeat_timer, if repeat_timer_active(last_hotkey.as_ref()) => {
                let hotkey = last_hotkey.clone().unwrap();
                dispatch_hotkey(hotkey.clone(), &cmd_tx, &modes, &mut current_mode, default_mode, &mut uinput_device, #[cfg(feature = "macro")] &macro_emit_tx, &mut active_macro);
                hotkey_repeat_timer.as_mut().reset(Instant::now() + Duration::from_millis(repeat_cooldown_duration));
            }

            //not fully in macro due to `select!` limitations
            Some(events) = macro_emit_next(
                #[cfg(feature = "macro")]
                &mut macro_emit_rx,
            ) => {
                emit_or_warn(&mut uinput_device, &events);
            }

            Some(signal) = signals.next() => {
                match signal {
                    SIGUSR1 => {
                        execution_is_paused = true;
                        for stream in device_stream_map.values_mut() {
                            let _ = stream.device_mut().ungrab();
                        }
                    }

                    SIGUSR2 => {
                        execution_is_paused = false;
                        for stream in device_stream_map.values_mut() {
                            let _ = stream.device_mut().grab();
                        }
                    }

                    SIGHUP => {
                        config = load_config();
                        modes = config.modes;
                        default_mode = config.default_mode;
                        current_mode = config.default_mode;
                    }

                    SIGINT => {
                        for stream in device_stream_map.values_mut() {
                            let _ = stream.device_mut().ungrab();
                        }
                        log::warn!("Received SIGINT signal, exiting...");
                        exit(1);
                    }

                    _ => {
                        for stream in device_stream_map.values_mut() {
                            let _ = stream.device_mut().ungrab();
                        }
                        log::warn!("Received signal: {signal:#?}");
                        log::warn!("Exiting...");
                        exit(1);
                    }
                }
            }

            Some(Ok(event)) = udev.next() => {
                if !event.is_initialized() {
                    log::warn!("Received udev event with uninitialized device.");
                }

                let node = match event.devnode() {
                    None => { continue; },
                    Some(node) => {
                        match node.to_str() {
                            None => { continue; },
                            Some(node) => node,
                        }
                    },
                };

                match event.event_type() {
                    EventType::Add => {
                        let mut device = match Device::open(node) {
                            Err(e) => {
                                log::error!("Could not open evdev device at {node}: {e}");
                                continue;
                            },
                            Ok(device) => device
                        };
                        if !to_ignore(&device) && (to_add(&device) || check_device_is_supported(&device)) {
                            let name = device.name().unwrap_or("[unknown]");
                            log::info!("Device '{name}' at '{node}' added.");
                            let _ = device.grab();
                            device_states.insert(node.to_string(), DeviceState::new(&device));
                            device_stream_map.insert(node.to_string(), device.into_event_stream()?);
                        }
                    }
                    EventType::Remove => {
                        if device_stream_map.contains_key(node) {
                            device_states.remove(node);
                            let stream = device_stream_map.remove(node).expect("device not in stream_map");
                            let name = stream.device().name().unwrap_or("[unknown]");
                            log::info!("Device '{name}' at '{node}' removed");
                        }
                    }
                    _ => {
                        log::debug!("Ignored udev event of type: {:?}", event.event_type());
                    }
                }
            }

            Some((node, Ok(mut event))) = device_stream_map.next() => {
                #[cfg(feature = "macro")]
                if let Some(ref state) = active_macro && state.handle.is_finished() {
                    active_macro = None;
                }

                let device_state = &mut device_states.get_mut(&node).expect("device not in states map");
                let key = match event.destructure() {
                    EventSummary::Key(_, keycode, _) => {
                        match modes[current_mode].remaps.get(&keycode) {
                            Some(remapped_keycode) => {
                                event = evdev::InputEvent::new(event.event_type().0, remapped_keycode.0, event.value());
                                *remapped_keycode
                            },
                            _ => keycode
                        }
                    },
                    EventSummary::Switch(..) => {
                        emit_or_warn(&mut uinput_switches_device, &[event]);
                        continue
                    }
                    EventSummary::RelativeAxis(_, rlcode, _) => {
                        if rel_mask::is_allowed(device_state.allowed_rel, rlcode) {
                            emit_or_warn(&mut uinput_device, &[event]);
                        }
                        continue
                    }
                    _ => {
                        emit_or_warn(&mut uinput_device, &[event]);
                        continue
                    }
                };
                log::debug!("Key: {key:#?}");

                match event.value() {
                    // Key press
                    1 => {
                        #[cfg(feature = "macro")]
                        if let Some(ref state) = active_macro
                            && matches!(state.macro_type, config::MacroType::Endless)
                            && key == KeyCode::KEY_ESC
                        {
                            state.stop.store(true, Ordering::Relaxed);
                            continue;
                        }

                        if config::ALLOWED_MODIFIERS.contains(&key) {
                            device_state.state_modifiers.insert(key);
                            device_state.state_modifiers_count += 1;
                        } else {
                            device_state.state_keysyms.insert(key);
                        }
                    }

                    // Key release
                    0 => {
                        #[cfg(feature = "macro")]
                        if let Some(ref state) = active_macro
                            && matches!(state.macro_type, config::MacroType::Hold)
                        {
                            let is_trigger_key = key == state.trigger_keybind.keysym
                                || state.trigger_keybind.modifiers.contains(&key);
                            if is_trigger_key {
                                state.stop.store(true, Ordering::Relaxed);
                            }
                        }

                        if last_hotkey.is_some() && pending_release {
                            pending_release = false;
                            dispatch_hotkey(last_hotkey.clone().unwrap(), &cmd_tx, &modes, &mut current_mode, default_mode, &mut uinput_device, #[cfg(feature = "macro")] &macro_emit_tx, &mut active_macro);
                            last_hotkey = None;
                        }
                        if config::ALLOWED_MODIFIERS.contains(&key) {
                            if let Some(hotkey) = &last_hotkey && hotkey.modifiers().contains(&key) {
                                    let evict = hotkey.keysym();
                                    last_hotkey = None;
                                    device_state.state_keysyms.remove(evict);

                            }
                            if device_state.state_modifiers.contains(key) {
                                device_state.state_modifiers.remove(key);
                                device_state.state_modifiers_count -= 1;
                            }
                        } else if device_state.state_keysyms.contains(key) {
                            if let Some(hotkey) = &last_hotkey && key == hotkey.keysym() {
                                    last_hotkey = None;

                            }
                            device_state.state_keysyms.remove(key);
                        }
                    }
                    _ => {}
                }

                // Single pass over the mode's hotkeys, no allocation
                let (event_in_hotkeys, any_possible) = modes[current_mode].hotkeys.iter().fold(
                    (false, false),
                    |(eih, ap), hotkey| {
                        (
                            eih || event_consumed(
                                hotkey,
                                &device_state.state_modifiers,
                                device_state.state_modifiers_count,
                                event.code(),
                            ),
                            ap || hotkey.keybind.modifiers.len() == device_state.state_modifiers_count,
                        )
                    },
                );

                // Only emit event to virtual device when swallow option is off
                if !modes[current_mode].options.swallow
                    // Don't emit event to virtual device if it's from a valid hotkey
                    && !event_in_hotkeys
                    // Don't forward keys to virtual device when macro is running.
                    // Needed because otherwise macro keys get interupted by our keys even when they are part of shortcut
                    && active_macro.is_none()
                {
                    emit_or_warn(&mut uinput_device, &[event]);
                }

                if execution_is_paused || !any_possible || last_hotkey.is_some() || active_macro.is_some() {
                    continue;
                }

                log::debug!("state_modifiers: {:#?}", device_state.state_modifiers);
                log::debug!("state_keysyms: {:#?}", device_state.state_keysyms);

                for hotkey in modes[current_mode]
                    .hotkeys
                    .iter()
                    .filter(|hotkey| hotkey.keybind.modifiers.len() == device_state.state_modifiers_count)
                {
                    if hotkey_armed(
                        hotkey,
                        &device_state.state_modifiers,
                        &device_state.state_keysyms,
                        device_state.state_modifiers_count,
                    ) {
                        last_hotkey = Some(hotkey.clone());
                        if pending_release { break; }
                        if hotkey.is_on_release() {
                            pending_release = true;
                            break;
                        }
                        dispatch_hotkey(hotkey.clone(), &cmd_tx, &modes, &mut current_mode, default_mode, &mut uinput_device, #[cfg(feature = "macro")] &macro_emit_tx, &mut active_macro);
                        hotkey_repeat_timer.as_mut().reset(Instant::now() + Duration::from_millis(repeat_cooldown_duration));
                        continue;
                    }
                }
            }
        }
    }
}

async fn socket_write(command: &str, socket_path: PathBuf) -> Result<(), Box<dyn Error>> {
    let mut stream = UnixStream::connect(socket_path).await?;
    stream.write_all(command.as_bytes()).await?;
    Ok(())
}

// Sends que to swhks one at a time,
async fn ipc_sender(mut rx: tokio::sync::mpsc::Receiver<String>, socket_path: PathBuf) {
    while let Some(cmd) = rx.recv().await {
        if let Err(e) = socket_write(&cmd, socket_path.clone()).await {
            log::error!("Failed to send command to swhks through IPC.");
            log::error!("Please make sure that swhks is running.");
            log::error!("Err: {e:#?}");
        }
    }
    log::debug!("IPC sender stopped (all senders dropped).");
}

// `select!` workaround
#[cfg(feature = "macro")]
async fn macro_emit_next(
    rx: &mut tokio::sync::mpsc::Receiver<Vec<evdev::InputEvent>>,
) -> Option<Vec<evdev::InputEvent>> {
    rx.recv().await
}

#[cfg(not(feature = "macro"))]
async fn macro_emit_next() -> Option<Vec<evdev::InputEvent>> {
    std::future::pending().await
}

// Had to deal with a few glitchy devices, fail to write shouldn't be fatal. Just logged
// and dropped to keep daemon alive instead of abort.
fn emit_or_warn(dev: &mut evdev::uinput::VirtualDevice, events: &[evdev::InputEvent]) {
    if let Err(e) = dev.emit(events) {
        log::warn!("uinput emit failed (dropped {} event(s)): {e}", events.len());
    }
}

pub fn check_pkexec() {
    if !Uid::current().is_root() {
        log::error!("swhkdp must be started via pkexec.");
        log::error!("Consider using `pkexec swhkdp ...`");
        exit(1);
    }
}

pub fn check_device_is_supported(device: &Device) -> bool {
    if device.supported_events().contains(evdev::EventType::KEY)
        && !device.supported_events().contains(evdev::EventType::FORCEFEEDBACK)
        && !device.supported_keys().is_some_and(|keys| keys.contains(KeyCode::BTN_TOUCH))
    {
        if device.name() == Some("swhkdp virtual output") {
            return false;
        }
        log::debug!("Device: {}", device.name().unwrap_or("Unknown device name"),);
        log::debug!("Supported Events: {:?}", device.supported_events());
        true
    } else {
        log::debug!("Other: {}", device.name().unwrap_or("Unknown device name"),);
        false
    }
}

pub fn setup_swhkdp(runtime_path: &Path) {
    log::debug!("Setting process umask.");
    umask(Mode::S_IWGRP | Mode::S_IWOTH);

    if !runtime_path.exists() {
        match fs::create_dir_all(runtime_path) {
            Ok(_) => {
                log::debug!("Created runtime directory.");
                match fs::set_permissions(runtime_path, Permissions::from_mode(0o600)) {
                    Ok(_) => log::debug!("Set runtime directory to readonly."),
                    Err(e) => log::error!("Failed to set runtime directory to readonly: {e}"),
                }
            }
            Err(e) => log::error!("Failed to create runtime directory: {e}"),
        }
    }

    let pidfile = runtime_path.join("swhkdp.pid");
    if pidfile.exists() {
        log::debug!("Reading {} file and checking for running instances.", pidfile.display());
        let swhkdp_pid = match fs::read_to_string(&pidfile) {
            Ok(swhkdp_pid) => swhkdp_pid,
            Err(e) => {
                log::error!("Unable to read {e} to check all running instances");
                exit(1);
            }
        };
        log::debug!("Previous PID: {swhkdp_pid}");

        let mut sys = System::new_with_specifics(
            RefreshKind::nothing()
                .with_processes(ProcessRefreshKind::nothing().with_exe(UpdateKind::Always)),
        );
        sys.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing().with_exe(UpdateKind::Always),
        );
        for (pid, process) in sys.processes() {
            if pid.to_string() == swhkdp_pid
                && process.exe() == Some(&std::env::current_exe().unwrap())
            {
                log::error!("Another instance of swhkdp is already running!");
                exit(1);
            }
        }
    }
    match fs::write(&pidfile, id().to_string()) {
        Ok(_) => {}
        Err(e) => {
            log::error!("Unable to write to {}: {e}", pidfile.display());
            exit(1);
        }
    }
}

pub fn create_default_config(config_file_path: &Path) {
    match fs::File::create(config_file_path) {
        Ok(mut file) => {
            log::debug!("Created default swhkdp config at: {config_file_path:#?}");
            _ = file.write_all(b"// Default swhkdp config\nmaster {\n  // Uncomment to use:\n  // KEY_LEFTMETA+KEY_RETURN \"alacritty\"\n}\ngeneral {\n  default master\n  oneoff #false\n  swallow #false\n}\n");
        }
        Err(err) => {
            log::error!("Error creating config file: {err}");
            exit(1);
        }
    };
}

fn resolve_config_path(args: &Args) -> PathBuf {
    #[cfg(debug_assertions)]
    if let Some(path) = args.config.as_ref() {
        return path.clone();
    }
    #[cfg(not(debug_assertions))]
    let _ = args;
    PathBuf::from(RELEASE_CONFIG_PATH)
}

fn read_config_content(config_file_path: &Path) -> String {
    #[cfg(not(debug_assertions))]
    if !perms::chain_is_root_write_only(config_file_path) {
        log::error!(
            "Refusing config {}: its directory chain must be owned by root and writable only by root",
            config_file_path.display()
        );
        exit(1);
    }

    let mut file = match fs::File::open(config_file_path) {
        Ok(file) => file,
        Err(e) => {
            log::error!("Failed to open config {}: {e}", config_file_path.display());
            exit(1);
        }
    };

    #[cfg(not(debug_assertions))]
    {
        let st = match nix::sys::stat::fstat(&file) {
            Ok(st) => st,
            Err(e) => {
                log::error!("Failed to stat config {}: {e}", config_file_path.display());
                exit(1);
            }
        };
        if !perms::root_write_only(st.st_uid, st.st_mode as u32) {
            log::error!(
                "Refusing config {}: must be owned by root and writable only by root",
                config_file_path.display()
            );
            exit(1);
        }
    }

    let mut content = String::new();
    if let Err(e) = file.read_to_string(&mut content) {
        log::error!("Failed to read config {}: {e}", config_file_path.display());
        exit(1);
    }
    content
}

fn hotkey_armed(
    hotkey: &config::Hotkey,
    state_modifiers: &AttributeSet<KeyCode>,
    state_keysyms: &AttributeSet<KeyCode>,
    state_modifiers_count: usize,
) -> bool {
    hotkey.keybind.modifiers.len() == state_modifiers_count
        && state_modifiers.iter().all(|m| hotkey.keybind.modifiers.contains(&m))
        && state_keysyms.contains(hotkey.keybind.keysym)
}

fn event_consumed(
    hotkey: &config::Hotkey,
    state_modifiers: &AttributeSet<KeyCode>,
    state_modifiers_count: usize,
    event_code: u16,
) -> bool {
    hotkey.keybind.keysym.code() == event_code
        && state_modifiers.iter().all(|m| hotkey.keybind.modifiers.contains(&m))
        && state_modifiers_count == hotkey.keybind.modifiers.len()
        && !hotkey.is_send()
}

fn repeat_timer_active(last_hotkey: Option<&config::Hotkey>) -> bool {
    last_hotkey.is_some_and(|hotkey| !hotkey.keybind.on_release)
}

#[cfg_attr(feature = "macro", allow(clippy::too_many_arguments))]
fn dispatch_hotkey(
    hotkey: Hotkey,
    cmd_tx: &tokio::sync::mpsc::Sender<String>,
    modes: &[config::Mode],
    current_mode: &mut usize,
    default_mode: usize,
    uinput: &mut evdev::uinput::VirtualDevice,
    #[cfg(feature = "macro")] macro_emit_tx: &tokio::sync::mpsc::Sender<Vec<evdev::InputEvent>>,
    active_macro: &mut Option<MacroState>,
) {
    log::info!("Hotkey pressed: {hotkey:#?}");
    #[cfg(not(feature = "macro"))]
    let _ = (active_macro, uinput);
    if modes[*current_mode].options.oneoff {
        *current_mode = default_mode;
    }

    match hotkey.action {
        config::HotkeyAction::Shell(command) => {
            let mut commands_to_send = String::new();
            if command.contains('@') {
                let commands = command.split("&&").map(|s| s.trim()).collect::<Vec<_>>();
                for cmd in commands {
                    let mut words = cmd.split_whitespace();
                    match words.next().unwrap() {
                        config::MODE_ENTER_STATEMENT => {
                            let enter_mode = cmd.split(' ').nth(1).unwrap();
                            if enter_mode == "default" {
                                *current_mode = default_mode;
                                log::info!(
                                    "Switching to default mode: {}",
                                    modes[*current_mode].name
                                );
                            } else {
                                let mut found = false;
                                for (i, mode) in modes.iter().enumerate() {
                                    if mode.name == enter_mode {
                                        *current_mode = i;
                                        found = true;
                                        break;
                                    }
                                }
                                if found {
                                    log::info!("Switching to mode: {}", modes[*current_mode].name);
                                } else {
                                    log::warn!("Mode not found: {enter_mode}");
                                }
                            }
                        }
                        _ => commands_to_send.push_str(format!("{cmd} &&").as_str()),
                    }
                }
            } else {
                commands_to_send = command.to_string();
            }
            if commands_to_send.ends_with(" &&") {
                commands_to_send = commands_to_send.strip_suffix(" &&").unwrap().to_string();
            }
            if !commands_to_send.is_empty() {
                match cmd_tx.try_send(commands_to_send) {
                    Ok(()) => {}
                    Err(tokio::sync::mpsc::error::TrySendError::Full(cmd)) => {
                        log::warn!("swhks command queue full ({IPC_QUEUE_CAP}); dropping: {cmd:?}");
                    }
                    Err(tokio::sync::mpsc::error::TrySendError::Closed(cmd)) => {
                        log::error!("swhks command queue closed; dropping: {cmd:?}");
                    }
                }
            }
        }

        #[cfg(feature = "macro")]
        config::HotkeyAction::Macro(macro_def) => {
            if let Some(state) = active_macro.as_ref() {
                state.stop.store(true, Ordering::Relaxed);
            }

            for &modifier in &hotkey.keybind.modifiers {
                emit_or_warn(
                    uinput,
                    &[evdev::InputEvent::new(evdev::EventType::KEY.0, modifier.0, 0)],
                );
            }
            if hotkey.keybind.send {
                emit_or_warn(
                    uinput,
                    &[evdev::InputEvent::new(evdev::EventType::KEY.0, hotkey.keybind.keysym.0, 0)],
                );
            }

            let macro_type = macro_def.macro_type;
            let trigger_keybind = hotkey.keybind.clone();
            let stop = Arc::new(AtomicBool::new(false));
            let stop_clone = stop.clone();
            let emit_tx = macro_emit_tx.clone();

            let handle = tokio::spawn(async move {
                macro_runner::run_macro(macro_def, emit_tx, stop_clone).await;
            });

            *active_macro = Some(MacroState { stop, handle, macro_type, trigger_keybind });
        }
    }
}

fn run_verify_mode(config_file_path: &Path) -> Result<(), Box<dyn Error>> {
    if !config_file_path.exists() {
        log::info!("Error: Config file not found at: {}", config_file_path.display());
        exit(1);
    }
    match config::load(config_file_path) {
        Ok(cfg) => {
            log::info!("Config file is valid: {}", config_file_path.display());
            log::info!("Modes: {}", cfg.modes.len());
            for (i, mode) in cfg.modes.iter().enumerate() {
                let default_marker = if i == cfg.default_mode { " (default)" } else { "" };
                log::info!(
                    "  - {}{}: {} hotkeys, {} remaps",
                    mode.name,
                    default_marker,
                    mode.hotkeys.len(),
                    mode.remaps.len(),
                );
            }
            Ok(())
        }
        Err(e) => {
            log::error!("Error: Invalid config file: {}", config_file_path.display());
            log::error!("{e}");
            exit(1);
        }
    }
}

async fn run_watch_mode(
    arg_add_devices: &[String],
    arg_ignore_devices: &[String],
) -> Result<(), Box<dyn Error>> {
    let to_add =
        |dev: &Device| arg_add_devices.contains(&dev.name().unwrap_or("[unknown]").to_string());
    let to_ignore =
        |dev: &Device| arg_ignore_devices.contains(&dev.name().unwrap_or("[unknown]").to_string());

    let supported_devices: Vec<(PathBuf, Device)> = {
        if arg_add_devices.is_empty() {
            evdev::enumerate()
                .filter(|(_, dev)| !to_ignore(dev) && check_device_is_supported(dev))
                .collect()
        } else {
            evdev::enumerate().filter(|(_, dev)| !to_ignore(dev) && to_add(dev)).collect()
        }
    };

    if supported_devices.is_empty() {
        log::error!("No valid device was detected!");
        exit(1);
    }

    log::debug!("Watch mode: {} device(s) detected", supported_devices.len());

    let mut signals = Signals::new([SIGINT, SIGTERM, SIGQUIT])?;

    let mut udev =
        AsyncMonitorSocket::new(MonitorBuilder::new()?.match_subsystem("input")?.listen()?)?;

    let mut device_stream_map = StreamMap::new();

    for (path, device) in supported_devices.into_iter() {
        let path = match path.to_str() {
            Some(p) => p,
            None => continue,
        };
        device_stream_map.insert(path.to_string(), device.into_event_stream()?);
    }

    println!("Watching for key events. Press Ctrl+C to exit.");

    loop {
        select! {
            Some(signal) = signals.next() => {
                match signal {
                    SIGINT | SIGTERM | SIGQUIT => {
                        log::debug!("Watch mode: received signal {signal:#?}, exiting...");
                        exit(0);
                    }
                    _ => {}
                }
            }

            Some(Ok(event)) = udev.next() => {
                if !event.is_initialized() {
                    continue;
                }

                let node = match event.devnode() {
                    None => continue,
                    Some(node) => match node.to_str() {
                        None => continue,
                        Some(node) => node,
                    },
                };

                match event.event_type() {
                    EventType::Add => {
                        let device = match Device::open(node) {
                            Err(e) => {
                                log::error!("Could not open evdev device at {node}: {e}");
                                continue;
                            },
                            Ok(device) => device
                        };
                        if !to_ignore(&device) && (to_add(&device) || check_device_is_supported(&device)) {
                            let name = device.name().unwrap_or("[unknown]");
                            log::info!("Watch mode: device '{name}' at '{node}' added.");
                            device_stream_map.insert(node.to_string(), device.into_event_stream()?);
                        }
                    }
                    EventType::Remove
                        if device_stream_map.contains_key(node) => {
                            let stream = device_stream_map.remove(node).expect("device not in stream_map");
                            let name = stream.device().name().unwrap_or("[unknown]");
                            log::info!("Watch mode: device '{name}' at '{node}' removed");
                        }

                    _ => {}
                }
            }

            Some((_node, Ok(event))) = device_stream_map.next() => {
                if let EventSummary::Key(_, keycode, 1) = event.destructure() {
                    println!("{:?}", keycode);
                }
            }
        }
    }
}

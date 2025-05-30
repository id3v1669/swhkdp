use crate::config::Value;
use clap::Parser;
use config::Hotkey;
use evdev::{AttributeSet, Device, EventSummary, KeyCode};
use nix::{
    sys::stat::{umask, Mode},
    unistd::{Group, Uid},
};
use signal_hook::consts::signal::*;
use signal_hook_tokio::Signals;
use std::{
    collections::HashMap,
    env,
    error::Error,
    fs,
    fs::Permissions,
    io::prelude::*,
    os::unix::{fs::PermissionsExt, net::UnixStream},
    path::{Path, PathBuf},
    process::{exit, id},
};
use sysinfo::System;
use tokio::select;
use tokio::time::Duration;
use tokio::time::{sleep, Instant};
use tokio_stream::{StreamExt, StreamMap};
use tokio_udev::{AsyncMonitorSocket, EventType, MonitorBuilder};

mod config;
mod environ;
mod perms;
mod uinput;

struct DeviceState {
    state_modifiers: AttributeSet<KeyCode>,
    state_keysyms: AttributeSet<KeyCode>,
}

impl DeviceState {
    fn new() -> DeviceState {
        DeviceState { state_modifiers: AttributeSet::new(), state_keysyms: AttributeSet::new() }
    }
}

/// Simple Wayland Hotkey Daemon
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// Set a custom config file path.
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
    #[arg(short = 'I', long, num_args = 0.., value_delimiter = '|')]
    ignoredevices: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let default_cooldown: u64 = 250;
    env::set_var("RUST_LOG", "swhkdp=warn");

    if args.debug {
        env::set_var("RUST_LOG", "swhkdp=trace");
    }

    env_logger::init();
    log::trace!("Logger initialized.");

    let env = environ::Env::construct();
    log::trace!("Environment Aquired");

    let invoking_uid = env.pkexec_id;

    setup_swhkdp(invoking_uid, env.runtime_dir.clone().to_string_lossy().to_string());

    let load_config = || {
        // Drop privileges to the invoking user.
        perms::drop_privileges(invoking_uid);

        let config_file_path: PathBuf =
            args.config.as_ref().map_or_else(|| env.fetch_config_path(), |file| file.clone());

        log::debug!("Using config file path: {:#?}", config_file_path);

        // If no config is present
        // Creates a default config at location (read man 5 swhkdp)

        if !Path::new(&config_file_path).exists() {
            log::warn!("No config found at path: {:#?}", config_file_path);
            create_default_config(invoking_uid, &config_file_path);
        }

        match config::load(&config_file_path) {
            Err(e) => {
                log::error!("Config Error: {}", e);
                exit(1)
            }
            Ok(out) => {
                // Escalate back to the root user after reading the config file.
                perms::raise_privileges();
                out
            }
        }
    };

    let mut config = load_config();
    let mut modes = config.modes;
    let mut remaps = config.remaps;
    let mut mode_stack: Vec<usize> = vec![0];
    let arg_add_devices = args.devices;
    let arg_ignore_devices = args.ignoredevices;

    let to_add =
        |dev: &Device| arg_add_devices.contains(&dev.name().unwrap_or("[unknown]").to_string());
    let to_ignore =
        |dev: &Device| arg_ignore_devices.contains(&dev.name().unwrap_or("[unknown]").to_string());

    let supported_devices: Vec<(PathBuf, Device)> = {
        if arg_add_devices.is_empty() {
            log::trace!("Attempting to find all supported devices file descriptors.");
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

    // Apparently, having a single uinput device with keys, relative axes and switches
    // prevents some libraries to listen to these events. The easy fix is to have separate
    // virtual devices, one for keys and relative axes (`uinput_device`) and another one
    // just for switches (`uinput_switches_device`).
    let mut uinput_device = match uinput::create_uinput_device() {
        Ok(dev) => dev,
        Err(e) => {
            log::error!("Failed to create uinput device: {:#?}", e);
            exit(1);
        }
    };

    let mut uinput_switches_device = match uinput::create_uinput_switches_device() {
        Ok(dev) => dev,
        Err(e) => {
            log::error!("Failed to create uinput switches device: {:#?}", e);
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
        device_states.insert(path.to_string(), DeviceState::new());
        device_stream_map.insert(path.to_string(), device.into_event_stream()?);
    }

    // The initial sleep duration is never read because last_hotkey is initialized to None
    let hotkey_repeat_timer = sleep(Duration::from_millis(0));
    tokio::pin!(hotkey_repeat_timer);

    // The socket we're sending the commands to.
    let socket_file_path = env.fetch_runtime_socket_path();
    loop {
        select! {
            _ = &mut hotkey_repeat_timer, if &last_hotkey.is_some() => {
                let hotkey = last_hotkey.clone().unwrap();
                if hotkey.keybind.on_release {
                    continue;
                }
                send_command(hotkey.clone(), &socket_file_path, &modes, &mut mode_stack);
                hotkey_repeat_timer.as_mut().reset(Instant::now() + Duration::from_millis(repeat_cooldown_duration));
            }

            Some(signal) = signals.next() => {
                match signal {
                    SIGUSR1 => {
                        execution_is_paused = true;
                        for mut device in evdev::enumerate().map(|(_, device)| device).filter(check_device_is_supported) {
                            let _ = device.ungrab();
                        }
                    }

                    SIGUSR2 => {
                        execution_is_paused = false;
                        for mut device in evdev::enumerate().map(|(_, device)| device).filter(check_device_is_supported) {
                            let _ = device.grab();
                        }
                    }

                    SIGHUP => {
                        config = load_config();
                        modes = config.modes;
                        remaps = config.remaps;
                        mode_stack = vec![0];
                    }

                    SIGINT => {
                        for mut device in evdev::enumerate().map(|(_, device)| device).filter(check_device_is_supported) {
                            let _ = device.ungrab();
                        }
                        log::warn!("Received SIGINT signal, exiting...");
                        exit(1);
                    }

                    _ => {
                        for mut device in evdev::enumerate().map(|(_, device)| device).filter(check_device_is_supported) {
                            let _ = device.ungrab();
                        }

                        log::warn!("Received signal: {:#?}", signal);
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
                                log::error!("Could not open evdev device at {}: {}", node, e);
                                continue;
                            },
                            Ok(device) => device
                        };
                        if !to_ignore(&device) && (to_add(&device) || check_device_is_supported(&device)) {
                            let name = device.name().unwrap_or("[unknown]");
                            log::info!("Device '{}' at '{}' added.", name, node);
                            let _ = device.grab();
                            device_states.insert(node.to_string(), DeviceState::new());
                            device_stream_map.insert(node.to_string(), device.into_event_stream()?);
                        }
                    }
                    EventType::Remove => {
                        if device_stream_map.contains_key(node) {
                            device_states.remove(node);
                            let stream = device_stream_map.remove(node).expect("device not in stream_map");
                            let name = stream.device().name().unwrap_or("[unknown]");
                            log::info!("Device '{}' at '{}' removed", name, node);
                        }
                    }
                    _ => {
                        log::trace!("Ignored udev event of type: {:?}", event.event_type());
                    }
                }
            }

            Some((node, Ok(mut event))) = device_stream_map.next() => {
                let device_state = &mut device_states.get_mut(&node).expect("device not in states map");
                let key = match event.destructure() {
                    EventSummary::Key(_, keycode, _) => {
                        match remaps.get(&keycode) {
                            Some(remapped_keycode) => {
                                event = evdev::InputEvent::new(event.event_type().0, remapped_keycode.0, event.value());
                                *remapped_keycode
                            },
                            _ => keycode
                        }
                    },
                    EventSummary::Switch(..) => {
                        uinput_switches_device.emit(&[event]).unwrap();
                        continue
                    }
                    EventSummary::RelativeAxis(_, rlcode, _) => {
                        match rlcode {
                            // temp solution for double input on mouse wheel
                            // TODO: use REL_WHEEL_HI_RES by default and fallback to REL_WHEEL if not supported by device
                            evdev::RelativeAxisCode::REL_WHEEL_HI_RES => {}
                            _ => uinput_device.emit(&[event]).unwrap(),
                        }
                        continue
                    }
                    _ => {
                        uinput_device.emit(&[event]).unwrap();
                        continue
                    }
                };
                log::debug!("Key: {:#?}", key);

                match event.value() {
                    // Key press
                    1 => {
                        if config::ALLOWED_MODIFIERS.contains(&key) {
                            device_state.state_modifiers.insert(key);
                        } else {
                            device_state.state_keysyms.insert(key);
                        }
                    }

                    // Key release
                    0 => {
                        if last_hotkey.is_some() && pending_release {
                            pending_release = false;
                            send_command(last_hotkey.clone().unwrap(), &socket_file_path, &modes, &mut mode_stack);
                            last_hotkey = None;
                        }
                        if config::ALLOWED_MODIFIERS.contains(&key) {
                            if let Some(hotkey) = &last_hotkey {
                                if hotkey.modifiers().contains(&key) {
                                    last_hotkey = None;
                                }
                            }
                            device_state.state_modifiers.remove(key);
                        } else if device_state.state_keysyms.contains(key) {
                            if let Some(hotkey) = &last_hotkey {
                                if key == hotkey.keysym() {
                                    last_hotkey = None;
                                }
                            }
                            device_state.state_keysyms.remove(key);
                        }
                    }
                    _ => {}
                }

                let possible_hotkeys: Vec<&config::Hotkey> = modes[mode_stack[mode_stack.len() - 1]].hotkeys.iter()
                    .filter(|hotkey| hotkey.modifiers().len() == device_state.state_modifiers.iter().count())
                    .collect();

                let event_in_hotkeys = modes[mode_stack[mode_stack.len() - 1]].hotkeys.iter().any(|hotkey| {
                    hotkey.keysym().code() == event.code() &&
                        (device_state.state_modifiers
                        .iter()
                        .all(|x| hotkey.modifiers().contains(&x)) &&
                    device_state.state_modifiers.iter().len() == hotkey.modifiers().len())
                    && !hotkey.is_send()
                        });

                // Only emit event to virtual device when swallow option is off
                if !modes[mode_stack[mode_stack.len()-1]].options.swallow
                // Don't emit event to virtual device if it's from a valid hotkey
                && !event_in_hotkeys {
                    uinput_device.emit(&[event]).unwrap();
                }

                if execution_is_paused || possible_hotkeys.is_empty() || last_hotkey.is_some() {
                    continue;
                }

                log::debug!("state_modifiers: {:#?}", device_state.state_modifiers);
                log::debug!("state_keysyms: {:#?}", device_state.state_keysyms);
                //log::debug!("hotkey: {:#?}", possible_hotkeys);

                for hotkey in possible_hotkeys {
                    // this should check if state_modifiers and hotkey.modifiers have the same elements
                    if (device_state.state_modifiers.iter().all(|x| hotkey.modifiers().contains(&x))
                        && device_state.state_modifiers.iter().len() == hotkey.modifiers().len())
                        && device_state.state_keysyms.contains(hotkey.keysym())
                    {
                        last_hotkey = Some(hotkey.clone());
                        if pending_release { break; }
                        if hotkey.is_on_release() {
                            pending_release = true;
                            break;
                        }
                        send_command(hotkey.clone(), &socket_file_path, &modes, &mut mode_stack);
                        hotkey_repeat_timer.as_mut().reset(Instant::now() + Duration::from_millis(repeat_cooldown_duration));
                        continue;
                    }
                }
            }
        }
    }
}

fn socket_write(command: &str, socket_path: PathBuf) -> Result<(), Box<dyn Error>> {
    let mut stream = UnixStream::connect(socket_path)?;
    stream.write_all(command.as_bytes())?;
    Ok(())
}

pub fn check_input_group() -> Result<(), Box<dyn Error>> {
    if !Uid::current().is_root() {
        let groups = nix::unistd::getgroups();
        for groups in groups.iter() {
            for group in groups {
                let group = Group::from_gid(*group);
                if group.unwrap().unwrap().name == "input" {
                    log::error!("Note: INVOKING USER IS IN INPUT GROUP!!!!");
                    log::error!("THIS IS A HUGE SECURITY RISK!!!!");
                }
            }
        }
        log::error!("Consider using `pkexec swhkdp ...`");
        exit(1);
    } else {
        log::warn!("Running swhkdp as root!");
        Ok(())
    }
}

pub fn check_device_is_supported(device: &Device) -> bool {
    if device.supported_events().contains(evdev::EventType::KEY)
        && !device.supported_keys().is_some_and(|keys| keys.contains(KeyCode::BTN_TOUCH))
    {
        if device.name() == Some("swhkdp virtual output") {
            return false;
        }
        log::debug!("Device: {}", device.name().unwrap(),);
        true
    } else {
        log::trace!("Other: {}", device.name().unwrap(),);
        false
    }
}

pub fn setup_swhkdp(invoking_uid: u32, runtime_path: String) {
    // Set a sane process umask.
    log::trace!("Setting process umask.");
    umask(Mode::S_IWGRP | Mode::S_IWOTH);

    // Get the runtime path and create it if needed.
    if !Path::new(&runtime_path).exists() {
        match fs::create_dir_all(Path::new(&runtime_path)) {
            Ok(_) => {
                log::debug!("Created runtime directory.");
                match fs::set_permissions(Path::new(&runtime_path), Permissions::from_mode(0o600)) {
                    Ok(_) => log::debug!("Set runtime directory to readonly."),
                    Err(e) => log::error!("Failed to set runtime directory to readonly: {}", e),
                }
            }
            Err(e) => log::error!("Failed to create runtime directory: {}", e),
        }
    }

    // Get the PID file path for instance tracking.
    let pidfile: String = format!("{}swhkdp_{}.pid", runtime_path, invoking_uid);
    if Path::new(&pidfile).exists() {
        log::trace!("Reading {} file and checking for running instances.", pidfile);
        let swhkdp_pid = match fs::read_to_string(&pidfile) {
            Ok(swhkdp_pid) => swhkdp_pid,
            Err(e) => {
                log::error!("Unable to read {} to check all running instances", e);
                exit(1);
            }
        };
        log::debug!("Previous PID: {}", swhkdp_pid);

        // Check if swhkdp is already running!
        let mut sys = System::new_all();
        sys.refresh_all();
        for (pid, process) in sys.processes() {
            if pid.to_string() == swhkdp_pid
                && process.exe() == env::current_exe().unwrap().parent()
            {
                log::error!("swhkdp is already running!");
                log::error!("pid of existing swhkdp process: {}", pid.to_string());
                log::error!("To close the existing swhkdp process, run `sudo killall swhkdp`");
                exit(1);
            }
        }
    }

    // Write to the pid file.
    match fs::write(&pidfile, id().to_string()) {
        Ok(_) => {}
        Err(e) => {
            log::error!("Unable to write to {}: {}", pidfile, e);
            exit(1);
        }
    }

    // Check if the user is in input group.
    if check_input_group().is_err() {
        exit(1);
    }
}

pub fn create_default_config(invoking_uid: u32, config_file_path: &PathBuf) {
    // Initializes a default swhkdp config at specific config path

    perms::raise_privileges();
    match fs::File::create(config_file_path) {
        Ok(mut file) => {
            log::debug!("Created default swhkdp config at: {:#?}", config_file_path);
            _ = file.write_all(b"# Comments start with #, uncomment to use \n#start a terminal\n#super + return\n#\talacritty # replace with terminal of your choice");
        }
        Err(err) => {
            log::error!("Error creating config file: {}", err);
            exit(1);
        }
    };
    perms::drop_privileges(invoking_uid);
}

pub fn send_command(
    hotkey: Hotkey,
    socket_path: &Path,
    modes: &[config::Mode],
    mode_stack: &mut Vec<usize>,
) {
    log::info!("Hotkey pressed: {:#?}", hotkey);
    let command = hotkey.action;
    let mut commands_to_send = String::new();
    if modes[mode_stack[mode_stack.len() - 1]].options.oneoff {
        mode_stack.pop();
    }
    if command.contains('@') {
        let commands = command.split("&&").map(|s| s.trim()).collect::<Vec<_>>();
        for cmd in commands {
            let mut words = cmd.split_whitespace();
            match words.next().unwrap() {
                config::MODE_ENTER_STATEMENT => {
                    let enter_mode = cmd.split(' ').nth(1).unwrap();
                    for (i, mode) in modes.iter().enumerate() {
                        if mode.name == enter_mode {
                            mode_stack.push(i);
                            break;
                        }
                    }
                    log::info!("Entering mode: {}", modes[mode_stack[mode_stack.len() - 1]].name);
                }
                config::MODE_ESCAPE_STATEMENT => {
                    mode_stack.pop();
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
    if let Err(e) = socket_write(&commands_to_send, socket_path.to_path_buf()) {
        log::error!("Failed to send command to swhks through IPC.");
        log::error!("Please make sure that swhks is running.");
        log::error!("Err: {:#?}", e)
    };
}

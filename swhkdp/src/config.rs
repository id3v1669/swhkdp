use evdev::KeyCode;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::str::FromStr;
use std::{fmt, path::Path};

#[derive(Debug, Deserialize)]
pub struct ConfigRead {
    pub modes: HashMap<String, ConfigReadMode>,
    pub remaps: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize)]
pub struct ConfigReadMode {
    pub hotkeys: HashMap<String, CommandConfig>,
    pub swallow: Option<bool>,
    pub oneoff: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct CommandConfig {
    pub action_type: Option<String>,
    pub action: String,
    pub send: Option<bool>,
    pub on_release: Option<bool>,
}

pub struct Config {
    pub modes: Vec<Mode>,
    pub remaps: HashMap<KeyCode, KeyCode>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Mode {
    pub name: String,
    pub hotkeys: Vec<Hotkey>,
    pub unbinds: Vec<KeyBinding>,
    pub options: ModeOptions,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Hotkey {
    pub action_type: ActionType,
    pub keybind: KeyBinding,
    pub action: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub enum ActionType {
    Command,
    SingleCommand,
}

impl ActionType {
    pub fn from_str(s: &str) -> Result<Self, ()> {
        match s {
            "Command" | "command" => Ok(ActionType::Command),
            "SingleCommand" | "Singlecommand" | "singlecommand" => Ok(ActionType::SingleCommand),
            _ => Err(()),
        }
    }
}

impl Value for &Hotkey {
    fn keysym(&self) -> evdev::KeyCode {
        self.keybind.keysym
    }
    fn modifiers(&self) -> &HashSet<evdev::KeyCode> {
        &self.keybind.modifiers
    }
    fn is_send(&self) -> bool {
        self.keybind.send
    }
    fn is_on_release(&self) -> bool {
        self.keybind.on_release
    }
}

impl Value for KeyBinding {
    fn keysym(&self) -> evdev::KeyCode {
        self.keysym
    }
    fn modifiers(&self) -> &HashSet<evdev::KeyCode> {
        &self.modifiers
    }
    fn is_send(&self) -> bool {
        self.send
    }
    fn is_on_release(&self) -> bool {
        self.on_release
    }
}

pub trait Value {
    fn keysym(&self) -> evdev::KeyCode;
    fn modifiers(&self) -> &HashSet<evdev::KeyCode>;
    fn is_send(&self) -> bool;
    fn is_on_release(&self) -> bool;
}

#[derive(Debug, Clone, PartialEq)]
pub struct KeyBinding {
    pub keysym: evdev::KeyCode,
    pub modifiers: HashSet<evdev::KeyCode>,
    pub send: bool,
    pub on_release: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ModeOptions {
    pub swallow: bool,
    pub oneoff: bool,
}

pub fn load(path: &Path) -> Result<Config, Error> {
    let config_content_raw = fs::read_to_string(path).unwrap();
    let config_new: ConfigRead = serde_yml::from_str(&config_content_raw).unwrap();
    let mut modes_to_return: Vec<Mode> = Vec::new();
    let mut remaps_to_return: HashMap<KeyCode, KeyCode> = HashMap::new();
    for mode in config_new.modes.iter() {
        let mut mode_from_config = Mode {
            name: mode.0.clone(),
            hotkeys: vec![],
            unbinds: vec![],
            options: ModeOptions {
                swallow: mode.1.swallow.unwrap_or(false),
                oneoff: mode.1.oneoff.unwrap_or(false),
            },
        };
        for (keycodes, command) in mode.1.hotkeys.iter() {
            let mut modifiers: HashSet<KeyCode> = HashSet::new();
            let mut keys: Vec<KeyCode> = Vec::new();
            let mut commands: Vec<String> = Vec::new();
            let keycodes: String = keycodes.chars().filter(|&c| c != ' ' && c != '\t').collect();
            let objects = keycodes.split('+').collect::<Vec<_>>();
            let action_type = match ActionType::from_str(
                command.action_type.clone().unwrap_or("command".to_string()).as_str(),
            ) {
                Ok(action_type) => action_type,
                Err(_) => {
                    log::warn!("Failed to parce action_type for keycodes line: {keycodes:?}");
                    continue;
                }
            };
            if (objects.len() < 2 && action_type == ActionType::Command)
                || (objects.len() != 1 && action_type == ActionType::SingleCommand)
            {
                log::warn!(
                    "Invalid keycodes line, action_type \"command\" must contain >2 keycodes or choose action_type \"singlecommand\": {keycodes:?}"
                );
                continue;
            }
            match objects
                .clone()
                .into_iter()
                .take(objects.len() - 1)
                .map(KeyCode::from_str)
                .collect::<Result<HashSet<_>, _>>()
            {
                Ok(tokens) => {
                    if tokens.iter().any(|token| !ALLOWED_MODIFIERS.contains(token)) {
                        log::warn!("Invalid modifier for keycodes line: {keycodes:?}");
                        continue;
                    }
                    modifiers = tokens;
                }
                Err(_) => log::warn!("Failed parsing modifiers for keycodes line: {keycodes:?}"),
            }
            let keys_string = objects.last().unwrap();
            if keys_string.starts_with('{') && keys_string.ends_with('}') {
                let keys_vec_string =
                    keys_string[1..keys_string.len() - 1].split(',').collect::<Vec<_>>();
                for key_string in keys_vec_string {
                    if !key_string.contains('-') {
                        match KeyCode::from_str(key_string) {
                            Ok(key) => keys.push(key),
                            Err(_) => log::warn!("Failed to parse key: {key_string:?}"),
                        }
                        continue;
                    }
                    let range: Vec<&str> = key_string.split('-').collect();
                    if range.len() != 2 {
                        log::warn!("Invalid range for keys: {key_string:?}");
                        continue;
                    }
                    let rfrom = match KeyCode::from_str(range[0]) {
                        Ok(key) => key,
                        Err(_) => {
                            log::warn!("Failed to parse key: {:?}", range[0]);
                            continue;
                        }
                    };
                    let rto = match KeyCode::from_str(range[1]) {
                        Ok(key) => key,
                        Err(_) => {
                            log::warn!("Failed to parse key: {:?}", range[1]);
                            continue;
                        }
                    };
                    for i in rfrom.code()..=rto.code() {
                        keys.push(KeyCode::new(i));
                    }
                }
                let pattern = format!(r"\{{([^{{}}]*?,){{{}}}[^{{}}]*?\}}", keys.len() - 1);
                let re = regex::Regex::new(&pattern).unwrap();
                let pattern_from_action_orig: String =
                    re.find_iter(&command.action).map(|m| m.as_str().to_string()).collect();
                if pattern_from_action_orig.is_empty() {
                    log::debug!("Failed to find pattern in action: {:?}", command.action);
                    continue;
                }
                let pattern_from_action_stripped =
                    &pattern_from_action_orig[1..pattern_from_action_orig.len() - 1];
                for element in pattern_from_action_stripped.split(',') {
                    commands
                        .push(command.action.clone().replace(&pattern_from_action_orig, element));
                }
            } else {
                match KeyCode::from_str(keys_string) {
                    Ok(key) => keys.push(key),
                    Err(_) => log::warn!("Failed to parse key: {keys_string:?}"),
                }
                commands.push(command.action.clone());
            }
            for i in 0..keys.len() {
                mode_from_config.hotkeys.push(Hotkey {
                    action_type: action_type.clone(),
                    keybind: KeyBinding {
                        keysym: keys[i],
                        modifiers: modifiers.clone(),
                        send: command.send.unwrap_or(false),
                        on_release: command.on_release.unwrap_or(false),
                    },
                    action: commands[i]
                        .clone()
                        .strip_suffix('\n')
                        .unwrap_or(&commands[i].clone())
                        .to_string(),
                });
            }
        }
        log::debug!("before hotkeys");
        for hotkey in mode_from_config.hotkeys.iter() {
            log::debug!("Hotkey: {hotkey:?}");
        }
        log::debug!("after hotkeys");
        modes_to_return.push(mode_from_config);
    }
    for (keycode_from, keycode_to) in config_new.remaps.unwrap_or_default() {
        match (KeyCode::from_str(&keycode_from), KeyCode::from_str(&keycode_to)) {
            (Ok(keycode_from), Ok(keycode_to)) => {
                remaps_to_return.insert(keycode_from, keycode_to);
            }
            (Err(_), _) => log::warn!("Failed to parse keycode_from: {keycode_from:?}"),
            (_, Err(_)) => log::warn!("Failed to parse keycode_to: {keycode_to:?}"),
        }
    }

    Ok(Config { modes: modes_to_return, remaps: remaps_to_return })
}

#[derive(Debug)]
pub enum Error {
    ConfigNotFound,
    Io(std::io::Error),
}

impl From<std::io::Error> for Error {
    fn from(val: std::io::Error) -> Self {
        if val.kind() == std::io::ErrorKind::NotFound {
            Error::ConfigNotFound
        } else {
            Error::Io(val)
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::ConfigNotFound => "Config file not found.".fmt(f),

            Error::Io(io_err) => format!("I/O Error while parsing config file: {io_err}").fmt(f),
        }
    }
}

//pub const IMPORT_STATEMENT: &str = "include";
//pub const UNBIND_STATEMENT: &str = "ignore";
pub const MODE_ENTER_STATEMENT: &str = "@enter";
pub const MODE_ESCAPE_STATEMENT: &str = "@escape";

pub const ALLOWED_MODIFIERS: [KeyCode; 8] = [
    evdev::KeyCode::KEY_LEFTMETA,
    evdev::KeyCode::KEY_RIGHTMETA,
    evdev::KeyCode::KEY_LEFTALT,
    evdev::KeyCode::KEY_RIGHTALT,
    evdev::KeyCode::KEY_LEFTCTRL,
    evdev::KeyCode::KEY_RIGHTCTRL,
    evdev::KeyCode::KEY_LEFTSHIFT,
    evdev::KeyCode::KEY_RIGHTSHIFT,
];

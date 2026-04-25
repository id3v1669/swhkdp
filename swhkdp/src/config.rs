use evdev::KeyCode;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::str::FromStr;
use std::{fmt, path::Path};

pub struct Config {
    pub modes: Vec<Mode>,
    pub default_mode: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Mode {
    pub name: String,
    pub hotkeys: Vec<Hotkey>,
    pub remaps: HashMap<KeyCode, KeyCode>,
    pub unbinds: Vec<KeyBinding>,
    pub options: ModeOptions,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Hotkey {
    pub keybind: KeyBinding,
    pub action: HotkeyAction,
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

#[derive(Debug, Clone, PartialEq)]
pub enum HotkeyAction {
    Shell(String),
    Macro(MacroDef),
}

#[derive(Debug, Clone, PartialEq)]
pub struct MacroDef {
    pub macro_type: MacroType,
    pub steps: Vec<MacroStep>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MacroType {
    Simple,
    Endless,
    Hold,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MacroStep {
    KeyAction { key: KeyCode, action: KeyAction },
    Move {
        x: i32,
        y: i32,
        duration: u32,
        move_type: MoveType,
        path: MovePath,
    },
    Repeat { count: u32, steps: Vec<MacroStep> },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KeyAction { Down, Up, Click }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MoveType { Constant, Accelerate, Decelerate }

#[derive(Debug, Clone, PartialEq)]
pub enum MovePath {
    Direct,
    Arc { clockwise: bool },
}

struct GeneralSettings {
    default_mode: String,
    oneoff: bool,
    swallow: bool,
}

fn parse_general(doc: &kdl::KdlDocument) -> GeneralSettings {
    let mut settings =
        GeneralSettings { default_mode: "master".to_string(), oneoff: false, swallow: false };

    let general_node = match doc.get("general") {
        Some(node) => node,
        None => return settings,
    };

    let children = match general_node.children() {
        Some(children) => children,
        None => return settings,
    };

    for node in children.nodes() {
        let name = node.name().value();
        match name {
            "default" => {
                if let Some(val) = node.get(0) {
                    if let Some(s) = val.as_string() {
                        settings.default_mode = s.to_string();
                    } else {
                        log::warn!("general.default value must be a string");
                    }
                }
            }
            "oneoff" => {
                if let Some(val) = node.get(0) {
                    if let Some(b) = val.as_bool() {
                        settings.oneoff = b;
                    } else {
                        log::warn!("general.oneoff value must be a boolean");
                    }
                }
            }
            "swallow" => {
                if let Some(val) = node.get(0) {
                    if let Some(b) = val.as_bool() {
                        settings.swallow = b;
                    } else {
                        log::warn!("general.swallow value must be a boolean");
                    }
                }
            }
            _ => {
                log::warn!("Unknown general setting: {name}");
            }
        }
    }

    settings
}

fn action_has_empty_segment(action: &str) -> bool {
    action.contains('@') && action.split("&&").map(str::trim).any(str::is_empty)
}

fn parse_macro_steps(doc: &kdl::KdlDocument) -> Vec<MacroStep> {
    let mut steps = vec![];
    for node in doc.nodes() {
        let name = node.name().value();
        match name {
            "move" => {
                let x = match node.get("x") {
                    None => 0i32,
                    Some(v) => match v.as_integer() {
                        Some(n) => n as i32,
                        None => { log::warn!("move x must be an integer; defaulting to 0"); 0 }
                    },
                };
                let y = match node.get("y") {
                    None => 0i32,
                    Some(v) => match v.as_integer() {
                        Some(n) => n as i32,
                        None => { log::warn!("move y must be an integer; defaulting to 0"); 0 }
                    },
                };
                let duration = match node.get("duration") {
                    None => 0u32,
                    Some(v) => match v.as_integer() {
                        Some(n) if n >= 0 => n as u32,
                        Some(n) => { log::warn!("move duration must be >= 0, got {n}; defaulting to 0"); 0 }
                        None => { log::warn!("move duration must be an integer; defaulting to 0"); 0 }
                    },
                };
                let move_type = match node.get("type").and_then(|v| v.as_string()) {
                    None | Some("constant") => MoveType::Constant,
                    Some("accelerate") => MoveType::Accelerate,
                    Some("decelerate") => MoveType::Decelerate,
                    Some(other) => {
                        log::warn!("Unknown move type {other:?}; defaulting to \"constant\"");
                        MoveType::Constant
                    }
                };
                let path = match node.get("path").and_then(|v| v.as_string()) {
                    None | Some("direct") => MovePath::Direct,
                    Some("arc") => {
                        let clockwise = match node.get("direction").and_then(|v| v.as_string()) {
                            None | Some("cw") => true,
                            Some("ccw") => false,
                            Some(other) => {
                                log::warn!("Unknown arc direction {other:?}; defaulting to \"cw\"");
                                true
                            }
                        };
                        MovePath::Arc { clockwise }
                    }
                    Some(other) => {
                        log::warn!("Unknown path {other:?}; defaulting to \"direct\"");
                        MovePath::Direct
                    }
                };
                steps.push(MacroStep::Move { x, y, duration, move_type, path });
            }
            "repeat" => {
                let count = match node.get(0).and_then(|v| v.as_integer()) {
                    Some(n) if n >= 2 => match u32::try_from(n) {
                        Ok(v) => v,
                        Err(_) => {
                            log::warn!("repeat count {n} exceeds u32::MAX; clamping");
                            u32::MAX
                        }
                    },
                    Some(n) => {
                        log::warn!("repeat count must be >= 2, got {n}; skipping");
                        continue;
                    }
                    None => {
                        log::warn!("repeat count must be >= 2; skipping");
                        continue;
                    }
                };
                let inner = match node.children() {
                    Some(c) => parse_macro_steps(c),
                    None => vec![],
                };
                steps.push(MacroStep::Repeat { count, steps: inner });
            }
            _ => {
                match KeyCode::from_str(name) {
                    Ok(key) => {
                        let action_str =
                            node.get(0).and_then(|v| v.as_string()).unwrap_or("click");
                        let action = match action_str {
                            "down" => KeyAction::Down,
                            "up" => KeyAction::Up,
                            "click" => KeyAction::Click,
                            other => {
                                log::warn!("Unknown button action in macro: {other:?}");
                                continue;
                            }
                        };
                        steps.push(MacroStep::KeyAction { key, action });
                    }
                    Err(_) => {
                        log::warn!("Unknown macro step node: {name:?}");
                    }
                }
            }
        }
    }
    steps
}

fn build_macro_hotkey(
    node: &kdl::KdlNode,
    keysym: KeyCode,
    modifiers: HashSet<KeyCode>,
    send: bool,
    on_release: bool,
    keycodes_raw: &str,
) -> Hotkey {
    let macro_type = match node.get(1).and_then(|v| v.as_string()) {
        None | Some("simple") => MacroType::Simple,
        Some("endless") => MacroType::Endless,
        Some("hold") => MacroType::Hold,
        Some(unknown) => {
            log::warn!("unknown macro type {unknown:?} for {keycodes_raw:?}; defaulting to simple");
            MacroType::Simple
        }
    };
    let steps = match node.children() {
        Some(c) => parse_macro_steps(c),
        None => {
            log::warn!("@macro hotkey has no body: {keycodes_raw:?}");
            vec![]
        }
    };
    Hotkey {
        keybind: KeyBinding { keysym, modifiers, send, on_release },
        action: HotkeyAction::Macro(MacroDef { macro_type, steps }),
    }
}

fn parse_mode(mode_name: &str, mode_node: &kdl::KdlNode, general: &GeneralSettings) -> Mode {
    let mut mode = Mode {
        name: mode_name.to_string(),
        hotkeys: vec![],
        remaps: HashMap::new(),
        unbinds: vec![],
        options: ModeOptions { swallow: general.swallow, oneoff: general.oneoff },
    };

    let children = match mode_node.children() {
        Some(children) => children,
        None => return mode,
    };

    for hotkey_node in children.nodes() {
        let keycodes_raw = hotkey_node.name().value().to_string();

        let action_value = match hotkey_node.get(0) {
            Some(val) => match val.as_string() {
                Some(s) => s.to_string(),
                None => {
                    log::warn!("Action value for keycodes line must be a string: {keycodes_raw:?}");
                    continue;
                }
            },
            None => {
                log::warn!("Missing action for keycodes line: {keycodes_raw:?}");
                continue;
            }
        };

        let on_release = hotkey_node.get("on_release").and_then(|v| v.as_bool()).unwrap_or(false);
        let send = hotkey_node.get("send").and_then(|v| v.as_bool()).unwrap_or(false);

        let keycodes: String = keycodes_raw.chars().filter(|&c| c != ' ' && c != '\t').collect();
        let objects = keycodes.split('+').collect::<Vec<_>>();

        if objects.len() == 1 && !objects[0].starts_with('<') {
            let key_str = objects[0];
            match KeyCode::from_str(key_str) {
                Ok(from_key) => {
                    if action_value == "@macro" {
                        mode.hotkeys.push(build_macro_hotkey(
                            hotkey_node,
                            from_key,
                            HashSet::new(),
                            send,
                            on_release,
                            &keycodes_raw,
                        ));
                        continue;
                    }
                    if let Ok(to_key) = KeyCode::from_str(&action_value) {
                        mode.remaps.insert(from_key, to_key);
                        continue;
                    }
                    let action =
                        action_value.strip_suffix('\n').unwrap_or(&action_value).to_string();
                    if action_has_empty_segment(&action) {
                        log::warn!(
                            "Skipping hotkey '{keycodes_raw}': action has empty '&&' segment: {action:?}"
                        );
                        continue;
                    }
                    mode.hotkeys.push(Hotkey {
                        keybind: KeyBinding {
                            keysym: from_key,
                            modifiers: HashSet::new(),
                            send,
                            on_release,
                        },
                        action: HotkeyAction::Shell(action),
                    });
                    continue;
                }
                Err(_) => {
                    log::warn!("Failed to parse key: {key_str:?}");
                    continue;
                }
            }
        }

        if objects.len() < 2 {
            log::warn!(
                "Invalid keycodes line, multi-key bindings must contain at least 2 keys: {keycodes:?}"
            );
            continue;
        }

        let modifiers = match objects[..objects.len() - 1]
            .iter()
            .map(|s| KeyCode::from_str(s))
            .collect::<Result<HashSet<_>, _>>()
        {
            Ok(tokens) => {
                if tokens.iter().any(|token| !ALLOWED_MODIFIERS.contains(token)) {
                    log::warn!("Invalid modifier for keycodes line: {keycodes:?}");
                    continue;
                }
                tokens
            }
            Err(_) => {
                log::warn!("Failed parsing modifiers for keycodes line: {keycodes:?}");
                continue;
            }
        };

        let keys_string = objects.last().unwrap();
        let mut keys: Vec<KeyCode> = Vec::new();
        let mut commands: Vec<String> = Vec::new();

        if keys_string.starts_with('<') && keys_string.ends_with('>') {
            let keys_vec_string =
                keys_string[1..keys_string.len() - 1].split(',').collect::<Vec<_>>();
            for key_string in &keys_vec_string {
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

            if keys.is_empty() {
                log::warn!("No valid keys parsed for multi-key binding: {keycodes_raw:?}");
                continue;
            }
            let pattern = format!(r"\{{([^{{}}]*?,){{{}}}[^{{}}]*?\}}", keys.len() - 1);
            let re = match regex::Regex::new(&pattern) {
                Ok(re) => re,
                Err(e) => {
                    log::warn!("Failed to build key expansion regex for '{keycodes_raw}': {e}");
                    continue;
                }
            };
            let pattern_from_action_orig: String =
                re.find_iter(&action_value).map(|m| m.as_str().to_string()).collect();
            if pattern_from_action_orig.is_empty() {
                log::debug!("Failed to find pattern in action: {:?}", action_value);
                continue;
            }
            let pattern_from_action_stripped =
                &pattern_from_action_orig[1..pattern_from_action_orig.len() - 1];
            for element in pattern_from_action_stripped.split(',') {
                commands.push(action_value.replace(&pattern_from_action_orig, element));
            }
        } else {
            match KeyCode::from_str(keys_string) {
                Ok(key) => keys.push(key),
                Err(_) => log::warn!("Failed to parse key: {keys_string:?}"),
            }
            commands.push(action_value.clone());
        }

        if action_value == "@macro" {
            if keys.len() > 1 {
                log::warn!("@macro does not support key group expansion: {keycodes_raw:?}; skipping");
                continue;
            }
            mode.hotkeys.push(build_macro_hotkey(
                hotkey_node,
                keys[0],
                modifiers.clone(),
                send,
                on_release,
                &keycodes_raw,
            ));
            continue;
        }
        for i in 0..keys.len() {
            if i >= commands.len() {
                break;
            }
            let action = commands[i].strip_suffix('\n').unwrap_or(&commands[i]).to_string();
            if action_has_empty_segment(&action) {
                log::warn!(
                    "Skipping hotkey '{keycodes_raw}': action has empty '&&' segment: {action:?}"
                );
                continue;
            }
            mode.hotkeys.push(Hotkey {
                keybind: KeyBinding {
                    keysym: keys[i],
                    modifiers: modifiers.clone(),
                    send,
                    on_release,
                },
                action: HotkeyAction::Shell(action),
            });
        }
    }

    log::debug!("before hotkeys");
    for hotkey in mode.hotkeys.iter() {
        log::debug!("Hotkey: {hotkey:?}");
    }
    log::debug!("after hotkeys");

    mode
}

pub fn load(path: &Path) -> Result<Config, Error> {
    let content = fs::read_to_string(path)?;
    load_from_str(&content)
}

pub fn load_from_str(content: &str) -> Result<Config, Error> {
    let doc: kdl::KdlDocument =
        content.parse().map_err(|e: kdl::KdlError| Error::Parse(e.to_string()))?;
    let general = parse_general(&doc);
    let mut modes: Vec<Mode> = Vec::new();
    for node in doc.nodes() {
        let name = node.name().value();
        if name == "general" {
            continue;
        }
        modes.push(parse_mode(name, node, &general));
    }
    let default_mode = modes
        .iter()
        .position(|m| m.name == general.default_mode)
        .ok_or_else(|| Error::Parse(format!("Default mode '{}' not found", general.default_mode)))?;
    Ok(Config { modes, default_mode })
}

#[derive(Debug)]
pub enum Error {
    ConfigNotFound,
    Io(std::io::Error),
    Parse(String),
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
            Error::Parse(msg) => format!("Config parse error: {msg}").fmt(f),
        }
    }
}

//pub const IMPORT_STATEMENT: &str = "include";
//pub const UNBIND_STATEMENT: &str = "ignore";
pub const MODE_ENTER_STATEMENT: &str = "@enter";

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

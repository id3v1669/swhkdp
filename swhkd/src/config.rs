use itertools::Itertools;
use std::collections::HashMap;
use std::fs;
use std::{
    fmt,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub enum Error {
    ConfigNotFound,
    Io(std::io::Error),
    InvalidConfig(ParseError),
}

#[derive(Debug, PartialEq, Eq)]
pub enum ParseError {
    // u32 is the line number where an error occured
    UnknownSymbol(PathBuf, u32),
    InvalidModifier(PathBuf, u32),
    InvalidKeysym(PathBuf, u32),
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

            Error::Io(io_err) => format!("I/O Error while parsing config file: {}", io_err).fmt(f),
            Error::InvalidConfig(parse_err) => match parse_err {
                ParseError::UnknownSymbol(path, line_nr) => format!(
                    "Error parsing config file {:?}. Unknown symbol at line {}.",
                    path, line_nr
                )
                .fmt(f),
                ParseError::InvalidKeysym(path, line_nr) => format!(
                    "Error parsing config file {:?}. Invalid keysym at line {}.",
                    path, line_nr
                )
                .fmt(f),
                ParseError::InvalidModifier(path, line_nr) => format!(
                    "Error parsing config file {:?}. Invalid modifier at line {}.",
                    path, line_nr
                )
                .fmt(f),
            },
        }
    }
}

pub const IMPORT_STATEMENT: &str = "include";
pub const UNBIND_STATEMENT: &str = "ignore";
pub const MODE_STATEMENT: &str = "mode";
pub const MODE_END_STATEMENT: &str = "endmode";
pub const MODE_ENTER_STATEMENT: &str = "@enter";
pub const MODE_ESCAPE_STATEMENT: &str = "@escape";
pub const MODE_SWALLOW_STATEMENT: &str = "swallow";
pub const MODE_ONEOFF_STATEMENT: &str = "oneoff";

#[derive(Debug, PartialEq, Clone, Eq)]
pub struct Config {
    pub path: PathBuf,
    pub contents: String,
    pub imports: Vec<PathBuf>,
}

impl Config {
    pub fn get_imports(contents: &str) -> Result<Vec<PathBuf>, Error> {
        let mut imports = Vec::new();
        for line in contents.lines() {
            if line.split(' ').next().unwrap() == IMPORT_STATEMENT {
                if let Some(import_path) = line.split(' ').nth(1) {
                    imports.push(Path::new(import_path).to_path_buf());
                }
            }
        }
        Ok(imports)
    }

    pub fn new(path: &Path) -> Result<Self, Error> {
        let contents = fs::read_to_string(path)?;
        let imports = Self::get_imports(&contents)?;
        Ok(Config { path: path.to_path_buf(), contents, imports })
    }

    pub fn load_to_configs(&self) -> Result<Vec<Self>, Error> {
        let mut configs = Vec::new();
        for import in &self.imports {
            configs.push(Self::new(import)?)
        }
        Ok(configs)
    }

    pub fn load_and_merge(config: Self) -> Result<Vec<Self>, Error> {
        let mut configs = vec![config];
        let mut prev_count = 0;
        let mut current_count = configs.len();
        while prev_count != current_count {
            prev_count = configs.len();
            for config in configs.clone() {
                for import in Self::load_to_configs(&config)? {
                    if !configs.contains(&import) {
                        configs.push(import);
                    }
                }
            }
            current_count = configs.len();
        }
        Ok(configs)
    }
}

pub fn load(path: &Path) -> Result<Vec<Mode>, Error> {
    let config_self = Config::new(path)?;
    let mut configs: Vec<Config> = Config::load_and_merge(config_self.clone())?;
    configs.remove(0);
    configs.push(config_self);
    let mut modes: Vec<Mode> = vec![Mode::default()];
    for config in configs {
        let mut output = parse_contents(path.to_path_buf(), config.contents)?;
        for hotkey in output[0].hotkeys.drain(..) {
            modes[0].hotkeys.retain(|hk| hk.keybinding != hotkey.keybinding);
            modes[0].hotkeys.push(hotkey);
        }
        for unbind in output[0].unbinds.drain(..) {
            modes[0].hotkeys.retain(|hk| hk.keybinding != unbind);
        }
        output.remove(0);
        for mut mode in output {
            mode.hotkeys.retain(|x| !mode.unbinds.contains(&x.keybinding));
            modes.push(mode);
        }
    }
    Ok(modes)
}

#[derive(Debug, Clone)]
pub struct KeyBinding {
    pub keysym: evdev::KeyCode,
    pub modifiers: Vec<Modifier>,
    pub send: bool,
    pub on_release: bool,
}

impl PartialEq for KeyBinding {
    fn eq(&self, other: &Self) -> bool {
        self.keysym == other.keysym
            && self.modifiers.iter().all(|modifier| other.modifiers.contains(modifier))
            && self.modifiers.len() == other.modifiers.len()
            && self.send == other.send
            && self.on_release == other.on_release
    }
}

pub trait Prefix {
    fn send(self) -> Self;
    fn on_release(self) -> Self;
}

pub trait Value {
    fn keysym(&self) -> evdev::KeyCode;
    fn modifiers(&self) -> Vec<Modifier>;
    fn is_send(&self) -> bool;
    fn is_on_release(&self) -> bool;
}

impl KeyBinding {
    pub fn new(keysym: evdev::KeyCode, modifiers: Vec<Modifier>) -> Self {
        KeyBinding { keysym, modifiers, send: false, on_release: false }
    }
    pub fn on_release(mut self) -> Self {
        self.on_release = true;
        self
    }
}

impl Prefix for KeyBinding {
    fn send(mut self) -> Self {
        self.send = true;
        self
    }
    fn on_release(mut self) -> Self {
        self.on_release = true;
        self
    }
}

impl Value for KeyBinding {
    fn keysym(&self) -> evdev::KeyCode {
        self.keysym
    }
    fn modifiers(&self) -> Vec<Modifier> {
        self.clone().modifiers
    }
    fn is_send(&self) -> bool {
        self.send
    }
    fn is_on_release(&self) -> bool {
        self.on_release
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Hotkey {
    pub keybinding: KeyBinding,
    pub command: String,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone, Hash)]
pub enum Modifier {
    Super,
    Alt,
    Altgr,
    Control,
    Shift,
    Any,
}

impl Hotkey {
    pub fn from_keybinding(keybinding: KeyBinding, command: String) -> Self {
        Hotkey { keybinding, command }
    }
    #[cfg(test)]
    pub fn new(keysym: evdev::KeyCode, modifiers: Vec<Modifier>, command: String) -> Self {
        Hotkey { keybinding: KeyBinding::new(keysym, modifiers), command }
    }
}

impl Prefix for Hotkey {
    fn send(mut self) -> Self {
        self.keybinding.send = true;
        self
    }
    fn on_release(mut self) -> Self {
        self.keybinding.on_release = true;
        self
    }
}

impl Value for &Hotkey {
    fn keysym(&self) -> evdev::KeyCode {
        self.keybinding.keysym
    }
    fn modifiers(&self) -> Vec<Modifier> {
        self.keybinding.clone().modifiers
    }
    fn is_send(&self) -> bool {
        self.keybinding.send
    }
    fn is_on_release(&self) -> bool {
        self.keybinding.on_release
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Mode {
    pub name: String,
    pub hotkeys: Vec<Hotkey>,
    pub unbinds: Vec<KeyBinding>,
    pub options: ModeOptions,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ModeOptions {
    pub swallow: bool,
    pub oneoff: bool,
}

impl Mode {
    pub fn new(name: String) -> Self {
        Self { name, hotkeys: Vec::new(), unbinds: Vec::new(), options: ModeOptions::default() }
    }

    pub fn default() -> Self {
        Self::new("normal".to_string())
    }
}

pub fn parse_contents(path: PathBuf, contents: String) -> Result<Vec<Mode>, Error> {
    // Don't forget to update valid key list on the man page if you do change this list.
    let key_to_evdev_key: HashMap<&str, evdev::KeyCode> = HashMap::from([
        ("q", evdev::KeyCode::KEY_Q),
        ("w", evdev::KeyCode::KEY_W),
        ("e", evdev::KeyCode::KEY_E),
        ("r", evdev::KeyCode::KEY_R),
        ("t", evdev::KeyCode::KEY_T),
        ("y", evdev::KeyCode::KEY_Y),
        ("u", evdev::KeyCode::KEY_U),
        ("i", evdev::KeyCode::KEY_I),
        ("o", evdev::KeyCode::KEY_O),
        ("p", evdev::KeyCode::KEY_P),
        ("a", evdev::KeyCode::KEY_A),
        ("s", evdev::KeyCode::KEY_S),
        ("d", evdev::KeyCode::KEY_D),
        ("f", evdev::KeyCode::KEY_F),
        ("g", evdev::KeyCode::KEY_G),
        ("h", evdev::KeyCode::KEY_H),
        ("j", evdev::KeyCode::KEY_J),
        ("k", evdev::KeyCode::KEY_K),
        ("l", evdev::KeyCode::KEY_L),
        ("z", evdev::KeyCode::KEY_Z),
        ("x", evdev::KeyCode::KEY_X),
        ("c", evdev::KeyCode::KEY_C),
        ("v", evdev::KeyCode::KEY_V),
        ("b", evdev::KeyCode::KEY_B),
        ("n", evdev::KeyCode::KEY_N),
        ("m", evdev::KeyCode::KEY_M),
        ("1", evdev::KeyCode::KEY_1),
        ("2", evdev::KeyCode::KEY_2),
        ("3", evdev::KeyCode::KEY_3),
        ("4", evdev::KeyCode::KEY_4),
        ("5", evdev::KeyCode::KEY_5),
        ("6", evdev::KeyCode::KEY_6),
        ("7", evdev::KeyCode::KEY_7),
        ("8", evdev::KeyCode::KEY_8),
        ("9", evdev::KeyCode::KEY_9),
        ("0", evdev::KeyCode::KEY_0),
        ("escape", evdev::KeyCode::KEY_ESC),
        ("backspace", evdev::KeyCode::KEY_BACKSPACE),
        ("capslock", evdev::KeyCode::KEY_CAPSLOCK),
        ("return", evdev::KeyCode::KEY_ENTER),
        ("enter", evdev::KeyCode::KEY_ENTER),
        ("tab", evdev::KeyCode::KEY_TAB),
        ("space", evdev::KeyCode::KEY_SPACE),
        ("plus", evdev::KeyCode::KEY_KPPLUS), // Shouldn't this be kpplus?
        ("kp0", evdev::KeyCode::KEY_KP0),
        ("kp1", evdev::KeyCode::KEY_KP1),
        ("kp2", evdev::KeyCode::KEY_KP2),
        ("kp3", evdev::KeyCode::KEY_KP3),
        ("kp4", evdev::KeyCode::KEY_KP4),
        ("kp5", evdev::KeyCode::KEY_KP5),
        ("kp6", evdev::KeyCode::KEY_KP6),
        ("kp7", evdev::KeyCode::KEY_KP7),
        ("kp8", evdev::KeyCode::KEY_KP8),
        ("kp9", evdev::KeyCode::KEY_KP9),
        ("kpasterisk", evdev::KeyCode::KEY_KPASTERISK),
        ("kpcomma", evdev::KeyCode::KEY_KPCOMMA),
        ("kpdot", evdev::KeyCode::KEY_KPDOT),
        ("kpenter", evdev::KeyCode::KEY_KPENTER),
        ("kpequal", evdev::KeyCode::KEY_KPEQUAL),
        ("kpjpcomma", evdev::KeyCode::KEY_KPJPCOMMA),
        ("kpleftparen", evdev::KeyCode::KEY_KPLEFTPAREN),
        ("kpminus", evdev::KeyCode::KEY_KPMINUS),
        ("kpplusminus", evdev::KeyCode::KEY_KPPLUSMINUS),
        ("kprightparen", evdev::KeyCode::KEY_KPRIGHTPAREN),
        ("minus", evdev::KeyCode::KEY_MINUS),
        ("-", evdev::KeyCode::KEY_MINUS),
        ("equal", evdev::KeyCode::KEY_EQUAL),
        ("=", evdev::KeyCode::KEY_EQUAL),
        ("grave", evdev::KeyCode::KEY_GRAVE),
        ("`", evdev::KeyCode::KEY_GRAVE),
        ("print", evdev::KeyCode::KEY_SYSRQ),
        ("volumeup", evdev::KeyCode::KEY_VOLUMEUP),
        ("xf86audioraisevolume", evdev::KeyCode::KEY_VOLUMEUP),
        ("volumedown", evdev::KeyCode::KEY_VOLUMEDOWN),
        ("xf86audiolowervolume", evdev::KeyCode::KEY_VOLUMEDOWN),
        ("mute", evdev::KeyCode::KEY_MUTE),
        ("xf86audiomute", evdev::KeyCode::KEY_MUTE),
        ("brightnessup", evdev::KeyCode::KEY_BRIGHTNESSUP),
        ("xf86monbrightnessup", evdev::KeyCode::KEY_BRIGHTNESSUP),
        ("brightnessdown", evdev::KeyCode::KEY_BRIGHTNESSDOWN),
        ("xf86audiomedia", evdev::KeyCode::KEY_MEDIA),
        ("xf86audiomicmute", evdev::KeyCode::KEY_MICMUTE),
        ("micmute", evdev::KeyCode::KEY_MICMUTE),
        ("xf86audionext", evdev::KeyCode::KEY_NEXTSONG),
        ("xf86audioplay", evdev::KeyCode::KEY_PLAYPAUSE),
        ("xf86audioprev", evdev::KeyCode::KEY_PREVIOUSSONG),
        ("xf86audiostop", evdev::KeyCode::KEY_STOP),
        ("xf86monbrightnessdown", evdev::KeyCode::KEY_BRIGHTNESSDOWN),
        (",", evdev::KeyCode::KEY_COMMA),
        ("comma", evdev::KeyCode::KEY_COMMA),
        (".", evdev::KeyCode::KEY_DOT),
        ("dot", evdev::KeyCode::KEY_DOT),
        ("period", evdev::KeyCode::KEY_DOT),
        ("/", evdev::KeyCode::KEY_SLASH),
        ("question", evdev::KeyCode::KEY_QUESTION),
        ("slash", evdev::KeyCode::KEY_SLASH),
        ("backslash", evdev::KeyCode::KEY_BACKSLASH),
        ("leftbrace", evdev::KeyCode::KEY_LEFTBRACE),
        ("[", evdev::KeyCode::KEY_LEFTBRACE),
        ("bracketleft", evdev::KeyCode::KEY_LEFTBRACE),
        ("rightbrace", evdev::KeyCode::KEY_RIGHTBRACE),
        ("]", evdev::KeyCode::KEY_RIGHTBRACE),
        ("bracketright", evdev::KeyCode::KEY_RIGHTBRACE),
        (";", evdev::KeyCode::KEY_SEMICOLON),
        ("scroll_lock", evdev::KeyCode::KEY_SCROLLLOCK),
        ("semicolon", evdev::KeyCode::KEY_SEMICOLON),
        ("'", evdev::KeyCode::KEY_APOSTROPHE),
        ("apostrophe", evdev::KeyCode::KEY_APOSTROPHE),
        ("left", evdev::KeyCode::KEY_LEFT),
        ("right", evdev::KeyCode::KEY_RIGHT),
        ("up", evdev::KeyCode::KEY_UP),
        ("down", evdev::KeyCode::KEY_DOWN),
        ("pause", evdev::KeyCode::KEY_PAUSE),
        ("home", evdev::KeyCode::KEY_HOME),
        ("delete", evdev::KeyCode::KEY_DELETE),
        ("insert", evdev::KeyCode::KEY_INSERT),
        ("end", evdev::KeyCode::KEY_END),
        ("pause", evdev::KeyCode::KEY_PAUSE),
        ("prior", evdev::KeyCode::KEY_PAGEDOWN),
        ("next", evdev::KeyCode::KEY_PAGEUP),
        ("pagedown", evdev::KeyCode::KEY_PAGEDOWN),
        ("pageup", evdev::KeyCode::KEY_PAGEUP),
        ("f1", evdev::KeyCode::KEY_F1),
        ("f2", evdev::KeyCode::KEY_F2),
        ("f3", evdev::KeyCode::KEY_F3),
        ("f4", evdev::KeyCode::KEY_F4),
        ("f5", evdev::KeyCode::KEY_F5),
        ("f6", evdev::KeyCode::KEY_F6),
        ("f7", evdev::KeyCode::KEY_F7),
        ("f8", evdev::KeyCode::KEY_F8),
        ("f9", evdev::KeyCode::KEY_F9),
        ("f10", evdev::KeyCode::KEY_F10),
        ("f11", evdev::KeyCode::KEY_F11),
        ("f12", evdev::KeyCode::KEY_F12),
        ("f13", evdev::KeyCode::KEY_F13),
        ("f14", evdev::KeyCode::KEY_F14),
        ("f15", evdev::KeyCode::KEY_F15),
        ("f16", evdev::KeyCode::KEY_F16),
        ("f17", evdev::KeyCode::KEY_F17),
        ("f18", evdev::KeyCode::KEY_F18),
        ("f19", evdev::KeyCode::KEY_F19),
        ("f20", evdev::KeyCode::KEY_F20),
        ("f21", evdev::KeyCode::KEY_F21),
        ("f22", evdev::KeyCode::KEY_F22),
        ("f23", evdev::KeyCode::KEY_F23),
        ("f24", evdev::KeyCode::KEY_F24),
    ]);

    // Don't forget to update modifier list on the man page if you do change this list.
    let mod_to_mod_enum: HashMap<&str, Modifier> = HashMap::from([
        ("ctrl", Modifier::Control),
        ("control", Modifier::Control),
        ("super", Modifier::Super),
        ("mod4", Modifier::Super),
        ("alt", Modifier::Alt),
        ("mod1", Modifier::Alt),
        ("altgr", Modifier::Altgr),
        ("mod5", Modifier::Altgr),
        ("shift", Modifier::Shift),
        ("any", Modifier::Any),
    ]);

    let lines: Vec<&str> = contents.split('\n').collect();
    let mut modes: Vec<Mode> = vec![Mode::default()];
    let mut current_mode: usize = 0;

    // Go through each line, ignore comments and empty lines, mark lines starting with whitespace
    // as commands, and mark the other lines as keysyms. Mark means storing a line's type and the
    // line number in a vector.
    let mut lines_with_types: Vec<(&str, u32)> = Vec::new();
    for (line_number, line) in lines.iter().enumerate() {
        if line.trim().starts_with('#')
            || line.split(' ').next().unwrap() == IMPORT_STATEMENT
            || line.trim().is_empty()
        {
            continue;
        }
        if line.starts_with(' ') || line.starts_with('\t') {
            lines_with_types.push(("command", line_number as u32));
        } else if line.starts_with(UNBIND_STATEMENT) {
            lines_with_types.push(("unbind", line_number as u32));
        } else if line.starts_with(MODE_STATEMENT) {
            lines_with_types.push(("modestart", line_number as u32));
        } else if line.starts_with(MODE_END_STATEMENT) {
            lines_with_types.push(("modeend", line_number as u32));
        } else {
            lines_with_types.push(("keysym", line_number as u32));
        }
    }

    // Edge case: return a blank vector if no lines detected
    if lines_with_types.is_empty() {
        return Ok(modes);
    }

    let mut actual_lines: Vec<(&str, u32, String)> = Vec::new();

    if contents.contains('\\') {
        // Go through lines_with_types, and add the next line over and over until the current line no
        // longer ends with backslash. (Only if the lines have the same type)
        let mut current_line_type = lines_with_types[0].0;
        let mut current_line_number = lines_with_types[0].1;
        let mut current_line_string = String::new();
        let mut continue_backslash;

        for (line_type, line_number) in lines_with_types {
            if line_type != current_line_type {
                current_line_type = line_type;
                current_line_number = line_number;
                current_line_string = String::new();
            }

            let line_to_add = lines[line_number as usize].trim();
            continue_backslash = line_to_add.ends_with('\\');

            let line_to_add = line_to_add.strip_suffix('\\').unwrap_or(line_to_add);

            current_line_string.push_str(line_to_add);

            if !continue_backslash {
                actual_lines.push((current_line_type, current_line_number, current_line_string));
                current_line_type = line_type;
                current_line_number = line_number;
                current_line_string = String::new();
            }
        }
    } else {
        for (line_type, line_number) in lines_with_types {
            actual_lines.push((
                line_type,
                line_number,
                lines[line_number as usize].trim().to_string(),
            ));
        }
    }

    drop(lines);

    for (i, item) in actual_lines.iter().enumerate() {
        let line_type = item.0;
        let line_number = item.1;
        let line = &item.2;

        if line_type == "unbind" {
            let to_unbind = line.trim_start_matches(UNBIND_STATEMENT).trim();
            modes[current_mode].unbinds.push(parse_keybind(
                path.clone(),
                to_unbind,
                line_number + 1,
                &key_to_evdev_key,
                &mod_to_mod_enum,
            )?);
        }

        if line_type == "modestart" {
            let tokens = line.split(' ').collect_vec();
            let modename = tokens[1];
            let mut mode = Mode::new(modename.to_string());
            mode.options.swallow = tokens.contains(&MODE_SWALLOW_STATEMENT);
            mode.options.oneoff = tokens.contains(&MODE_ONEOFF_STATEMENT);
            modes.push(mode);
            current_mode = modes.len() - 1;
        }

        if line_type == "modeend" {
            current_mode = 0;
        }

        if line_type != "keysym" {
            continue;
        }

        let next_line = actual_lines.get(i + 1);
        if next_line.is_none() {
            break;
        }
        let next_line = next_line.unwrap();

        if next_line.0 != "command" {
            continue; // this should ignore keysyms that are not followed by a command
        }

        let extracted_keys = extract_curly_brace(line);
        let extracted_commands = extract_curly_brace(&next_line.2);

        for (key, command) in extracted_keys.iter().zip(extracted_commands.iter()) {
            let keybinding = parse_keybind(
                path.clone(),
                key,
                line_number + 1,
                &key_to_evdev_key,
                &mod_to_mod_enum,
            )?;
            let hotkey = Hotkey::from_keybinding(keybinding, command.to_string());

            // Override latter
            modes[current_mode].hotkeys.retain(|h| h.keybinding != hotkey.keybinding);
            modes[current_mode].hotkeys.push(hotkey);
        }
    }

    Ok(modes)
}

// We need to get the reference to key_to_evdev_key
// and mod_to_mod enum instead of recreating them
// after each function call because it's too expensive
fn parse_keybind(
    path: PathBuf,
    line: &str,
    line_nr: u32,
    key_to_evdev_key: &HashMap<&str, evdev::KeyCode>,
    mod_to_mod_enum: &HashMap<&str, Modifier>,
) -> Result<KeyBinding, Error> {
    let line = line.split('#').next().unwrap();
    let tokens: Vec<String> =
        line.split('+').map(|s| s.trim().to_lowercase()).filter(|s| s != "_").collect();

    let mut tokens_new = Vec::new();
    for mut token in tokens {
        while token.trim().starts_with('_') {
            token = token.trim().strip_prefix('_').unwrap().to_string();
        }
        tokens_new.push(token.trim().to_string());
    }

    let last_token = tokens_new.last().unwrap().trim();

    // Check if last_token is prefixed with @ or ~ or even both.
    // If prefixed @, on_release = true; if prefixed ~, send = true
    let send = last_token.starts_with('~') || last_token.starts_with("@~");
    let on_release = last_token.starts_with('@') || last_token.starts_with("~@");

    // Delete the @ and ~ in the last token
    fn strip_at(token: &str) -> &str {
        token.trim_start_matches(['@', '~'])
    }

    let last_token = strip_at(last_token);
    let tokens_no_at: Vec<_> = tokens_new.iter().map(|token| strip_at(token)).collect();

    // Check if each token is valid
    for token in &tokens_no_at {
        if key_to_evdev_key.contains_key(token) {
            // Can't have a keysym that's like a modifier
            if *token != last_token {
                return Err(Error::InvalidConfig(ParseError::InvalidModifier(path, line_nr)));
            }
        } else if mod_to_mod_enum.contains_key(token) {
            // Can't have a modifier that's like a keysym
            if *token == last_token {
                return Err(Error::InvalidConfig(ParseError::InvalidKeysym(path, line_nr)));
            }
        } else {
            return Err(Error::InvalidConfig(ParseError::UnknownSymbol(path, line_nr)));
        }
    }

    // Translate keypress into evdev key
    let keysym = key_to_evdev_key.get(last_token).unwrap();

    let modifiers: Vec<Modifier> = tokens_no_at[0..(tokens_no_at.len() - 1)]
        .iter()
        .map(|token| *mod_to_mod_enum.get(token).unwrap())
        .collect();

    let mut keybinding = KeyBinding::new(*keysym, modifiers);
    if send {
        keybinding = keybinding.send();
    }
    if on_release {
        keybinding = keybinding.on_release();
    }
    Ok(keybinding)
}

pub fn extract_curly_brace(line: &str) -> Vec<String> {
    if !line.contains('{') || !line.contains('}') || !line.is_ascii() {
        return vec![line.to_string()];
    }

    // go through each character in the line and mark the position of each { and }
    // if a { is not followed by a  }, return the line as is
    let mut brace_positions: Vec<usize> = Vec::new();
    let mut flag = false;
    for (i, c) in line.chars().enumerate() {
        if c == '{' {
            if flag {
                return vec![line.to_string()];
            }
            brace_positions.push(i);
            flag = true;
        } else if c == '}' {
            if !flag {
                return vec![line.to_string()];
            }
            brace_positions.push(i);
            flag = false;
        }
    }

    // now we have a list of positions of { and }
    // we should extract the items between each pair of braces and store them in a vector
    let mut items: Vec<String> = Vec::new();
    let mut remaining_line: Vec<String> = Vec::new();
    let mut start_index = 0;
    for i in brace_positions.chunks(2) {
        items.push(line[i[0] + 1..i[1]].to_string());
        remaining_line.push(line[start_index..i[0]].to_string());
        start_index = i[1] + 1;
    }

    // now we have a list of items between each pair of braces
    // we should extract the items between each comma and store them in a vector
    let mut tokens_vec: Vec<Vec<String>> = Vec::new();
    for item in items {
        // Edge case: escape periods
        // example:
        // ```
        // super + {\,, .}
        //    riverctl focus-output {previous, next}
        // ```
        let item = item.replace("\\,", "comma");

        let items: Vec<String> = item.split(',').map(|s| s.trim().to_string()).collect();
        tokens_vec.push(handle_ranges(items));
    }

    fn handle_ranges(items: Vec<String>) -> Vec<String> {
        let mut output: Vec<String> = Vec::new();
        for item in items {
            if !item.contains('-') {
                output.push(item);
                continue;
            }
            let mut range = item.split('-').map(|s| s.trim());

            let begin_char: &str = if let Some(b) = range.next() {
                b
            } else {
                output.push(item);
                continue;
            };

            let end_char: &str = if let Some(e) = range.next() {
                e
            } else {
                output.push(item);
                continue;
            };

            // Do not accept range values that are longer than one char
            // Example invalid: {ef-p} {3-56}
            // Beginning of the range cannot be greater than end
            // Example invalid: {9-4} {3-2}
            if begin_char.len() != 1 || end_char.len() != 1 || begin_char > end_char {
                output.push(item);
                continue;
            }

            // In swhkd we will parse the full range using ASCII values.

            let begin_ascii_val = begin_char.parse::<char>().unwrap() as u8;
            let end_ascii_val = end_char.parse::<char>().unwrap() as u8;

            for ascii_number in begin_ascii_val..=end_ascii_val {
                output.push((ascii_number as char).to_string());
            }
        }
        output
    }

    // now write the tokens back to the line and output a vector
    let mut output: Vec<String> = Vec::new();
    // generate a cartesian product iterator for all the vectors in tokens_vec
    let cartesian_product_iter = tokens_vec.iter().multi_cartesian_product();
    for tokens in cartesian_product_iter.collect_vec() {
        let mut line_to_push = String::new();
        for i in 0..remaining_line.len() {
            line_to_push.push_str(&remaining_line[i]);
            line_to_push.push_str(tokens[i]);
        }
        if brace_positions[brace_positions.len() - 1] < line.len() - 1 {
            line_to_push.push_str(&line[brace_positions[brace_positions.len() - 1] + 1..]);
        }
        output.push(line_to_push);
    }
    output
}
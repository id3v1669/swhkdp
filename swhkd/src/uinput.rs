use evdev::{
    uinput::{VirtualDevice, VirtualDeviceBuilder},
    AttributeSet, Key, RelativeAxisType, SwitchType,
};

#[cfg(feature = "rfkill")]
use nix::ioctl_none;
#[cfg(feature = "rfkill")]
use std::fs::File;
#[cfg(feature = "rfkill")]
use std::os::unix::io::AsRawFd;

#[cfg(feature = "rfkill")]
ioctl_none!(rfkill_noinput, b'R', 1);

pub fn create_uinput_device() -> Result<VirtualDevice, Box<dyn std::error::Error>> {
    let mut keys = AttributeSet::<Key>::new();
    for key in get_all_keys() {
        keys.insert(key);
    }

    let mut relative_axes = AttributeSet::<RelativeAxisType>::new();
    for axis in get_all_relative_axes() {
        relative_axes.insert(axis);
    }

    let device = VirtualDeviceBuilder::new()?
        .name("swhkd virtual output")
        .with_keys(&keys)?
        .with_relative_axes(&relative_axes)?
        .build()
        .unwrap();
    Ok(device)
}

pub fn create_uinput_switches_device() -> Result<VirtualDevice, Box<dyn std::error::Error>> {
    let mut switches = AttributeSet::<SwitchType>::new();
    for switch in get_all_switches() {
        switches.insert(switch);
    }

    // We have to disable rfkill-input to avoid blocking all radio devices. When
    // a new device (virtual or physical) with the SW_RFKILL_ALL capability bit
    // set appears, rfkill reacts immediately depending on the value bit. This
    // value bit defaults to unset, which causes rfkill to use its default mode
    // (which is eop - emergency power off). The uinput API does not give any
    // way to set the corresponding value bit before creating the device, and we
    // have no way to avoid rfkill acting upon the device creation or to change
    // its default mode. Thus, we disable rfkill-input temporarily, hopefully
    // fast enough that it won't impact anyone. rfkill-input will be enabled
    // again when the file gets closed.
    // Implemented as feature because in some versions of nixos rfkill is broken
    // By default feature is enabled
    #[cfg(feature = "rfkill")]
    {
        let rfkill_file = File::open("/dev/rfkill")?;
        unsafe {
            rfkill_noinput(rfkill_file.as_raw_fd())?;
        }
    }

    let device = VirtualDeviceBuilder::new()?
        .name("swhkd switches virtual output")
        .with_switches(&switches)?
        .build()
        .unwrap();
    Ok(device)
}
pub fn get_all_keys() -> Vec<Key> {
    vec![
        evdev::Key::KEY_RESERVED,
        evdev::Key::KEY_ESC,
        evdev::Key::KEY_1,
        evdev::Key::KEY_2,
        evdev::Key::KEY_3,
        evdev::Key::KEY_4,
        evdev::Key::KEY_5,
        evdev::Key::KEY_6,
        evdev::Key::KEY_7,
        evdev::Key::KEY_8,
        evdev::Key::KEY_9,
        evdev::Key::KEY_0,
        evdev::Key::KEY_MINUS,
        evdev::Key::KEY_EQUAL,
        evdev::Key::KEY_BACKSPACE,
        evdev::Key::KEY_TAB,
        evdev::Key::KEY_Q,
        evdev::Key::KEY_W,
        evdev::Key::KEY_E,
        evdev::Key::KEY_R,
        evdev::Key::KEY_T,
        evdev::Key::KEY_Y,
        evdev::Key::KEY_U,
        evdev::Key::KEY_I,
        evdev::Key::KEY_O,
        evdev::Key::KEY_P,
        evdev::Key::KEY_LEFTBRACE,
        evdev::Key::KEY_RIGHTBRACE,
        evdev::Key::KEY_ENTER,
        evdev::Key::KEY_LEFTCTRL,
        evdev::Key::KEY_A,
        evdev::Key::KEY_S,
        evdev::Key::KEY_D,
        evdev::Key::KEY_F,
        evdev::Key::KEY_G,
        evdev::Key::KEY_H,
        evdev::Key::KEY_J,
        evdev::Key::KEY_K,
        evdev::Key::KEY_L,
        evdev::Key::KEY_SEMICOLON,
        evdev::Key::KEY_APOSTROPHE,
        evdev::Key::KEY_GRAVE,
        evdev::Key::KEY_LEFTSHIFT,
        evdev::Key::KEY_BACKSLASH,
        evdev::Key::KEY_Z,
        evdev::Key::KEY_X,
        evdev::Key::KEY_C,
        evdev::Key::KEY_V,
        evdev::Key::KEY_B,
        evdev::Key::KEY_N,
        evdev::Key::KEY_M,
        evdev::Key::KEY_COMMA,
        evdev::Key::KEY_DOT,
        evdev::Key::KEY_SLASH,
        evdev::Key::KEY_RIGHTSHIFT,
        evdev::Key::KEY_KPASTERISK,
        evdev::Key::KEY_LEFTALT,
        evdev::Key::KEY_SPACE,
        evdev::Key::KEY_CAPSLOCK,
        evdev::Key::KEY_F1,
        evdev::Key::KEY_F2,
        evdev::Key::KEY_F3,
        evdev::Key::KEY_F4,
        evdev::Key::KEY_F5,
        evdev::Key::KEY_F6,
        evdev::Key::KEY_F7,
        evdev::Key::KEY_F8,
        evdev::Key::KEY_F9,
        evdev::Key::KEY_F10,
        evdev::Key::KEY_NUMLOCK,
        evdev::Key::KEY_SCROLLLOCK,
        evdev::Key::KEY_KP7,
        evdev::Key::KEY_KP8,
        evdev::Key::KEY_KP9,
        evdev::Key::KEY_KPMINUS,
        evdev::Key::KEY_KP4,
        evdev::Key::KEY_KP5,
        evdev::Key::KEY_KP6,
        evdev::Key::KEY_KPPLUS,
        evdev::Key::KEY_KP1,
        evdev::Key::KEY_KP2,
        evdev::Key::KEY_KP3,
        evdev::Key::KEY_KP0,
        evdev::Key::KEY_KPDOT,
        evdev::Key::KEY_ZENKAKUHANKAKU,
        evdev::Key::KEY_102ND,
        evdev::Key::KEY_F11,
        evdev::Key::KEY_F12,
        evdev::Key::KEY_RO,
        evdev::Key::KEY_KATAKANA,
        evdev::Key::KEY_HIRAGANA,
        evdev::Key::KEY_HENKAN,
        evdev::Key::KEY_KATAKANAHIRAGANA,
        evdev::Key::KEY_MUHENKAN,
        evdev::Key::KEY_KPJPCOMMA,
        evdev::Key::KEY_KPENTER,
        evdev::Key::KEY_RIGHTCTRL,
        evdev::Key::KEY_KPSLASH,
        evdev::Key::KEY_SYSRQ,
        evdev::Key::KEY_RIGHTALT,
        evdev::Key::KEY_LINEFEED,
        evdev::Key::KEY_HOME,
        evdev::Key::KEY_UP,
        evdev::Key::KEY_PAGEUP,
        evdev::Key::KEY_LEFT,
        evdev::Key::KEY_RIGHT,
        evdev::Key::KEY_END,
        evdev::Key::KEY_DOWN,
        evdev::Key::KEY_PAGEDOWN,
        evdev::Key::KEY_INSERT,
        evdev::Key::KEY_DELETE,
        evdev::Key::KEY_MACRO,
        evdev::Key::KEY_MUTE,
        evdev::Key::KEY_VOLUMEDOWN,
        evdev::Key::KEY_VOLUMEUP,
        evdev::Key::KEY_POWER,
        evdev::Key::KEY_KPEQUAL,
        evdev::Key::KEY_KPPLUSMINUS,
        evdev::Key::KEY_PAUSE,
        evdev::Key::KEY_SCALE,
        evdev::Key::KEY_KPCOMMA,
        evdev::Key::KEY_HANGEUL,
        evdev::Key::KEY_HANJA,
        evdev::Key::KEY_YEN,
        evdev::Key::KEY_LEFTMETA,
        evdev::Key::KEY_RIGHTMETA,
        evdev::Key::KEY_COMPOSE,
        evdev::Key::KEY_STOP,
        evdev::Key::KEY_AGAIN,
        evdev::Key::KEY_PROPS,
        evdev::Key::KEY_UNDO,
        evdev::Key::KEY_FRONT,
        evdev::Key::KEY_COPY,
        evdev::Key::KEY_OPEN,
        evdev::Key::KEY_PASTE,
        evdev::Key::KEY_FIND,
        evdev::Key::KEY_CUT,
        evdev::Key::KEY_HELP,
        evdev::Key::KEY_MENU,
        evdev::Key::KEY_CALC,
        evdev::Key::KEY_SETUP,
        evdev::Key::KEY_SLEEP,
        evdev::Key::KEY_WAKEUP,
        evdev::Key::KEY_FILE,
        evdev::Key::KEY_SENDFILE,
        evdev::Key::KEY_DELETEFILE,
        evdev::Key::KEY_XFER,
        evdev::Key::KEY_PROG1,
        evdev::Key::KEY_PROG2,
        evdev::Key::KEY_WWW,
        evdev::Key::KEY_MSDOS,
        evdev::Key::KEY_COFFEE,
        evdev::Key::KEY_DIRECTION,
        evdev::Key::KEY_ROTATE_DISPLAY,
        evdev::Key::KEY_CYCLEWINDOWS,
        evdev::Key::KEY_MAIL,
        evdev::Key::KEY_BOOKMARKS,
        evdev::Key::KEY_COMPUTER,
        evdev::Key::KEY_BACK,
        evdev::Key::KEY_FORWARD,
        evdev::Key::KEY_CLOSECD,
        evdev::Key::KEY_EJECTCD,
        evdev::Key::KEY_EJECTCLOSECD,
        evdev::Key::KEY_NEXTSONG,
        evdev::Key::KEY_PLAYPAUSE,
        evdev::Key::KEY_PREVIOUSSONG,
        evdev::Key::KEY_STOPCD,
        evdev::Key::KEY_RECORD,
        evdev::Key::KEY_REWIND,
        evdev::Key::KEY_PHONE,
        evdev::Key::KEY_ISO,
        evdev::Key::KEY_CONFIG,
        evdev::Key::KEY_HOMEPAGE,
        evdev::Key::KEY_REFRESH,
        evdev::Key::KEY_EXIT,
        evdev::Key::KEY_MOVE,
        evdev::Key::KEY_EDIT,
        evdev::Key::KEY_SCROLLUP,
        evdev::Key::KEY_SCROLLDOWN,
        evdev::Key::KEY_KPLEFTPAREN,
        evdev::Key::KEY_KPRIGHTPAREN,
        evdev::Key::KEY_NEW,
        evdev::Key::KEY_REDO,
        evdev::Key::KEY_F13,
        evdev::Key::KEY_F14,
        evdev::Key::KEY_F15,
        evdev::Key::KEY_F16,
        evdev::Key::KEY_F17,
        evdev::Key::KEY_F18,
        evdev::Key::KEY_F19,
        evdev::Key::KEY_F20,
        evdev::Key::KEY_F21,
        evdev::Key::KEY_F22,
        evdev::Key::KEY_F23,
        evdev::Key::KEY_F24,
        evdev::Key::KEY_PLAYCD,
        evdev::Key::KEY_PAUSECD,
        evdev::Key::KEY_PROG3,
        evdev::Key::KEY_PROG4,
        evdev::Key::KEY_DASHBOARD,
        evdev::Key::KEY_SUSPEND,
        evdev::Key::KEY_CLOSE,
        evdev::Key::KEY_PLAY,
        evdev::Key::KEY_FASTFORWARD,
        evdev::Key::KEY_BASSBOOST,
        evdev::Key::KEY_PRINT,
        evdev::Key::KEY_HP,
        evdev::Key::KEY_CAMERA,
        evdev::Key::KEY_SOUND,
        evdev::Key::KEY_QUESTION,
        evdev::Key::KEY_EMAIL,
        evdev::Key::KEY_CHAT,
        evdev::Key::KEY_SEARCH,
        evdev::Key::KEY_CONNECT,
        evdev::Key::KEY_FINANCE,
        evdev::Key::KEY_SPORT,
        evdev::Key::KEY_SHOP,
        evdev::Key::KEY_ALTERASE,
        evdev::Key::KEY_CANCEL,
        evdev::Key::KEY_BRIGHTNESSDOWN,
        evdev::Key::KEY_BRIGHTNESSUP,
        evdev::Key::KEY_MEDIA,
        evdev::Key::KEY_SWITCHVIDEOMODE,
        evdev::Key::KEY_KBDILLUMTOGGLE,
        evdev::Key::KEY_KBDILLUMDOWN,
        evdev::Key::KEY_KBDILLUMUP,
        evdev::Key::KEY_SEND,
        evdev::Key::KEY_REPLY,
        evdev::Key::KEY_FORWARDMAIL,
        evdev::Key::KEY_SAVE,
        evdev::Key::KEY_DOCUMENTS,
        evdev::Key::KEY_BATTERY,
        evdev::Key::KEY_BLUETOOTH,
        evdev::Key::KEY_WLAN,
        evdev::Key::KEY_UWB,
        evdev::Key::KEY_UNKNOWN,
        evdev::Key::KEY_VIDEO_NEXT,
        evdev::Key::KEY_VIDEO_PREV,
        evdev::Key::KEY_BRIGHTNESS_CYCLE,
        evdev::Key::KEY_BRIGHTNESS_AUTO,
        evdev::Key::KEY_DISPLAY_OFF,
        evdev::Key::KEY_WWAN,
        evdev::Key::KEY_RFKILL,
        evdev::Key::KEY_MICMUTE,
        evdev::Key::BTN_0,
        evdev::Key::BTN_1,
        evdev::Key::BTN_2,
        evdev::Key::BTN_3,
        evdev::Key::BTN_4,
        evdev::Key::BTN_5,
        evdev::Key::BTN_6,
        evdev::Key::BTN_7,
        evdev::Key::BTN_8,
        evdev::Key::BTN_9,
        evdev::Key::BTN_LEFT,
        evdev::Key::BTN_RIGHT,
        evdev::Key::BTN_MIDDLE,
        evdev::Key::BTN_SIDE,
        evdev::Key::BTN_EXTRA,
        evdev::Key::BTN_FORWARD,
        evdev::Key::BTN_BACK,
        evdev::Key::BTN_TASK,
        evdev::Key::BTN_TRIGGER,
        evdev::Key::BTN_THUMB,
        evdev::Key::BTN_THUMB2,
        evdev::Key::BTN_TOP,
        evdev::Key::BTN_TOP2,
        evdev::Key::BTN_PINKIE,
        evdev::Key::BTN_BASE,
        evdev::Key::BTN_BASE2,
        evdev::Key::BTN_BASE3,
        evdev::Key::BTN_BASE4,
        evdev::Key::BTN_BASE5,
        evdev::Key::BTN_BASE6,
        evdev::Key::BTN_DEAD,
        evdev::Key::BTN_SOUTH,
        evdev::Key::BTN_EAST,
        evdev::Key::BTN_C,
        evdev::Key::BTN_NORTH,
        evdev::Key::BTN_WEST,
        evdev::Key::BTN_Z,
        evdev::Key::BTN_TL,
        evdev::Key::BTN_TR,
        evdev::Key::BTN_TL2,
        evdev::Key::BTN_TR2,
        evdev::Key::BTN_SELECT,
        evdev::Key::BTN_START,
        evdev::Key::BTN_MODE,
        evdev::Key::BTN_THUMBL,
        evdev::Key::BTN_THUMBR,
        evdev::Key::BTN_TOOL_PEN,
        evdev::Key::BTN_TOOL_RUBBER,
        evdev::Key::BTN_TOOL_BRUSH,
        evdev::Key::BTN_TOOL_PENCIL,
        evdev::Key::BTN_TOOL_AIRBRUSH,
        evdev::Key::BTN_TOOL_FINGER,
        evdev::Key::BTN_TOOL_MOUSE,
        evdev::Key::BTN_TOOL_LENS,
        evdev::Key::BTN_TOOL_QUINTTAP,
        evdev::Key::BTN_TOUCH,
        evdev::Key::BTN_STYLUS,
        evdev::Key::BTN_STYLUS2,
        evdev::Key::BTN_TOOL_DOUBLETAP,
        evdev::Key::BTN_TOOL_TRIPLETAP,
        evdev::Key::BTN_TOOL_QUADTAP,
        evdev::Key::BTN_GEAR_DOWN,
        evdev::Key::BTN_GEAR_UP,
        evdev::Key::KEY_OK,
        evdev::Key::KEY_SELECT,
        evdev::Key::KEY_GOTO,
        evdev::Key::KEY_CLEAR,
        evdev::Key::KEY_POWER2,
        evdev::Key::KEY_OPTION,
        evdev::Key::KEY_INFO,
        evdev::Key::KEY_TIME,
        evdev::Key::KEY_VENDOR,
        evdev::Key::KEY_ARCHIVE,
        evdev::Key::KEY_PROGRAM,
        evdev::Key::KEY_CHANNEL,
        evdev::Key::KEY_FAVORITES,
        evdev::Key::KEY_EPG,
        evdev::Key::KEY_PVR,
        evdev::Key::KEY_MHP,
        evdev::Key::KEY_LANGUAGE,
        evdev::Key::KEY_TITLE,
        evdev::Key::KEY_SUBTITLE,
        evdev::Key::KEY_ANGLE,
        evdev::Key::KEY_ZOOM,
        evdev::Key::KEY_FULL_SCREEN,
        evdev::Key::KEY_MODE,
        evdev::Key::KEY_KEYBOARD,
        evdev::Key::KEY_SCREEN,
        evdev::Key::KEY_PC,
        evdev::Key::KEY_TV,
        evdev::Key::KEY_TV2,
        evdev::Key::KEY_VCR,
        evdev::Key::KEY_VCR2,
        evdev::Key::KEY_SAT,
        evdev::Key::KEY_SAT2,
        evdev::Key::KEY_CD,
        evdev::Key::KEY_TAPE,
        evdev::Key::KEY_RADIO,
        evdev::Key::KEY_TUNER,
        evdev::Key::KEY_PLAYER,
        evdev::Key::KEY_TEXT,
        evdev::Key::KEY_DVD,
        evdev::Key::KEY_AUX,
        evdev::Key::KEY_MP3,
        evdev::Key::KEY_AUDIO,
        evdev::Key::KEY_VIDEO,
        evdev::Key::KEY_DIRECTORY,
        evdev::Key::KEY_LIST,
        evdev::Key::KEY_MEMO,
        evdev::Key::KEY_CALENDAR,
        evdev::Key::KEY_RED,
        evdev::Key::KEY_GREEN,
        evdev::Key::KEY_YELLOW,
        evdev::Key::KEY_BLUE,
        evdev::Key::KEY_CHANNELUP,
        evdev::Key::KEY_CHANNELDOWN,
        evdev::Key::KEY_FIRST,
        evdev::Key::KEY_LAST,
        evdev::Key::KEY_AB,
        evdev::Key::KEY_NEXT,
        evdev::Key::KEY_RESTART,
        evdev::Key::KEY_SLOW,
        evdev::Key::KEY_SHUFFLE,
        evdev::Key::KEY_BREAK,
        evdev::Key::KEY_PREVIOUS,
        evdev::Key::KEY_DIGITS,
        evdev::Key::KEY_TEEN,
        evdev::Key::KEY_TWEN,
        evdev::Key::KEY_VIDEOPHONE,
        evdev::Key::KEY_GAMES,
        evdev::Key::KEY_ZOOMIN,
        evdev::Key::KEY_ZOOMOUT,
        evdev::Key::KEY_ZOOMRESET,
        evdev::Key::KEY_WORDPROCESSOR,
        evdev::Key::KEY_EDITOR,
        evdev::Key::KEY_SPREADSHEET,
        evdev::Key::KEY_GRAPHICSEDITOR,
        evdev::Key::KEY_PRESENTATION,
        evdev::Key::KEY_DATABASE,
        evdev::Key::KEY_NEWS,
        evdev::Key::KEY_VOICEMAIL,
        evdev::Key::KEY_ADDRESSBOOK,
        evdev::Key::KEY_MESSENGER,
        evdev::Key::KEY_DISPLAYTOGGLE,
        evdev::Key::KEY_SPELLCHECK,
        evdev::Key::KEY_LOGOFF,
        evdev::Key::KEY_DOLLAR,
        evdev::Key::KEY_EURO,
        evdev::Key::KEY_FRAMEBACK,
        evdev::Key::KEY_FRAMEFORWARD,
        evdev::Key::KEY_CONTEXT_MENU,
        evdev::Key::KEY_MEDIA_REPEAT,
        evdev::Key::KEY_10CHANNELSUP,
        evdev::Key::KEY_10CHANNELSDOWN,
        evdev::Key::KEY_IMAGES,
        evdev::Key::KEY_DEL_EOL,
        evdev::Key::KEY_DEL_EOS,
        evdev::Key::KEY_INS_LINE,
        evdev::Key::KEY_DEL_LINE,
        evdev::Key::KEY_FN,
        evdev::Key::KEY_FN_ESC,
        evdev::Key::KEY_FN_F1,
        evdev::Key::KEY_FN_F2,
        evdev::Key::KEY_FN_F3,
        evdev::Key::KEY_FN_F4,
        evdev::Key::KEY_FN_F5,
        evdev::Key::KEY_FN_F6,
        evdev::Key::KEY_FN_F7,
        evdev::Key::KEY_FN_F8,
        evdev::Key::KEY_FN_F9,
        evdev::Key::KEY_FN_F10,
        evdev::Key::KEY_FN_F11,
        evdev::Key::KEY_FN_F12,
        evdev::Key::KEY_FN_1,
        evdev::Key::KEY_FN_2,
        evdev::Key::KEY_FN_D,
        evdev::Key::KEY_FN_E,
        evdev::Key::KEY_FN_F,
        evdev::Key::KEY_FN_S,
        evdev::Key::KEY_FN_B,
        evdev::Key::KEY_BRL_DOT1,
        evdev::Key::KEY_BRL_DOT2,
        evdev::Key::KEY_BRL_DOT3,
        evdev::Key::KEY_BRL_DOT4,
        evdev::Key::KEY_BRL_DOT5,
        evdev::Key::KEY_BRL_DOT6,
        evdev::Key::KEY_BRL_DOT7,
        evdev::Key::KEY_BRL_DOT8,
        evdev::Key::KEY_BRL_DOT9,
        evdev::Key::KEY_BRL_DOT10,
        evdev::Key::KEY_NUMERIC_0,
        evdev::Key::KEY_NUMERIC_1,
        evdev::Key::KEY_NUMERIC_2,
        evdev::Key::KEY_NUMERIC_3,
        evdev::Key::KEY_NUMERIC_4,
        evdev::Key::KEY_NUMERIC_5,
        evdev::Key::KEY_NUMERIC_6,
        evdev::Key::KEY_NUMERIC_7,
        evdev::Key::KEY_NUMERIC_8,
        evdev::Key::KEY_NUMERIC_9,
        evdev::Key::KEY_NUMERIC_STAR,
        evdev::Key::KEY_NUMERIC_POUND,
        evdev::Key::KEY_NUMERIC_A,
        evdev::Key::KEY_NUMERIC_B,
        evdev::Key::KEY_NUMERIC_C,
        evdev::Key::KEY_NUMERIC_D,
        evdev::Key::KEY_CAMERA_FOCUS,
        evdev::Key::KEY_WPS_BUTTON,
        evdev::Key::KEY_TOUCHPAD_TOGGLE,
        evdev::Key::KEY_TOUCHPAD_ON,
        evdev::Key::KEY_TOUCHPAD_OFF,
        evdev::Key::KEY_CAMERA_ZOOMIN,
        evdev::Key::KEY_CAMERA_ZOOMOUT,
        evdev::Key::KEY_CAMERA_UP,
        evdev::Key::KEY_CAMERA_DOWN,
        evdev::Key::KEY_CAMERA_LEFT,
        evdev::Key::KEY_CAMERA_RIGHT,
        evdev::Key::KEY_ATTENDANT_ON,
        evdev::Key::KEY_ATTENDANT_OFF,
        evdev::Key::KEY_ATTENDANT_TOGGLE,
        evdev::Key::KEY_LIGHTS_TOGGLE,
        evdev::Key::BTN_DPAD_UP,
        evdev::Key::BTN_DPAD_DOWN,
        evdev::Key::BTN_DPAD_LEFT,
        evdev::Key::BTN_DPAD_RIGHT,
        evdev::Key::KEY_ALS_TOGGLE,
        evdev::Key::KEY_BUTTONCONFIG,
        evdev::Key::KEY_TASKMANAGER,
        evdev::Key::KEY_JOURNAL,
        evdev::Key::KEY_CONTROLPANEL,
        evdev::Key::KEY_APPSELECT,
        evdev::Key::KEY_SCREENSAVER,
        evdev::Key::KEY_VOICECOMMAND,
        evdev::Key::KEY_ASSISTANT,
        evdev::Key::KEY_KBD_LAYOUT_NEXT,
        evdev::Key::KEY_BRIGHTNESS_MIN,
        evdev::Key::KEY_BRIGHTNESS_MAX,
        evdev::Key::KEY_KBDINPUTASSIST_PREV,
        evdev::Key::KEY_KBDINPUTASSIST_NEXT,
        evdev::Key::KEY_KBDINPUTASSIST_PREVGROUP,
        evdev::Key::KEY_KBDINPUTASSIST_NEXTGROUP,
        evdev::Key::KEY_KBDINPUTASSIST_ACCEPT,
        evdev::Key::KEY_KBDINPUTASSIST_CANCEL,
        evdev::Key::KEY_RIGHT_UP,
        evdev::Key::KEY_RIGHT_DOWN,
        evdev::Key::KEY_LEFT_UP,
        evdev::Key::KEY_LEFT_DOWN,
        evdev::Key::KEY_ROOT_MENU,
        evdev::Key::KEY_MEDIA_TOP_MENU,
        evdev::Key::KEY_NUMERIC_11,
        evdev::Key::KEY_NUMERIC_12,
        evdev::Key::KEY_AUDIO_DESC,
        evdev::Key::KEY_3D_MODE,
        evdev::Key::KEY_NEXT_FAVORITE,
        evdev::Key::KEY_STOP_RECORD,
        evdev::Key::KEY_PAUSE_RECORD,
        evdev::Key::KEY_VOD,
        evdev::Key::KEY_UNMUTE,
        evdev::Key::KEY_FASTREVERSE,
        evdev::Key::KEY_SLOWREVERSE,
        evdev::Key::KEY_DATA,
        evdev::Key::KEY_ONSCREEN_KEYBOARD,
        evdev::Key::KEY_PRIVACY_SCREEN_TOGGLE,
        evdev::Key::KEY_SELECTIVE_SCREENSHOT,
        evdev::Key::BTN_TRIGGER_HAPPY1,
        evdev::Key::BTN_TRIGGER_HAPPY2,
        evdev::Key::BTN_TRIGGER_HAPPY3,
        evdev::Key::BTN_TRIGGER_HAPPY4,
        evdev::Key::BTN_TRIGGER_HAPPY5,
        evdev::Key::BTN_TRIGGER_HAPPY6,
        evdev::Key::BTN_TRIGGER_HAPPY7,
        evdev::Key::BTN_TRIGGER_HAPPY8,
        evdev::Key::BTN_TRIGGER_HAPPY9,
        evdev::Key::BTN_TRIGGER_HAPPY10,
        evdev::Key::BTN_TRIGGER_HAPPY11,
        evdev::Key::BTN_TRIGGER_HAPPY12,
        evdev::Key::BTN_TRIGGER_HAPPY13,
        evdev::Key::BTN_TRIGGER_HAPPY14,
        evdev::Key::BTN_TRIGGER_HAPPY15,
        evdev::Key::BTN_TRIGGER_HAPPY16,
        evdev::Key::BTN_TRIGGER_HAPPY17,
        evdev::Key::BTN_TRIGGER_HAPPY18,
        evdev::Key::BTN_TRIGGER_HAPPY19,
        evdev::Key::BTN_TRIGGER_HAPPY20,
        evdev::Key::BTN_TRIGGER_HAPPY21,
        evdev::Key::BTN_TRIGGER_HAPPY22,
        evdev::Key::BTN_TRIGGER_HAPPY23,
        evdev::Key::BTN_TRIGGER_HAPPY24,
        evdev::Key::BTN_TRIGGER_HAPPY25,
        evdev::Key::BTN_TRIGGER_HAPPY26,
        evdev::Key::BTN_TRIGGER_HAPPY27,
        evdev::Key::BTN_TRIGGER_HAPPY28,
        evdev::Key::BTN_TRIGGER_HAPPY29,
        evdev::Key::BTN_TRIGGER_HAPPY30,
        evdev::Key::BTN_TRIGGER_HAPPY31,
        evdev::Key::BTN_TRIGGER_HAPPY32,
        evdev::Key::BTN_TRIGGER_HAPPY33,
        evdev::Key::BTN_TRIGGER_HAPPY34,
        evdev::Key::BTN_TRIGGER_HAPPY35,
        evdev::Key::BTN_TRIGGER_HAPPY36,
        evdev::Key::BTN_TRIGGER_HAPPY37,
        evdev::Key::BTN_TRIGGER_HAPPY38,
        evdev::Key::BTN_TRIGGER_HAPPY39,
        evdev::Key::BTN_TRIGGER_HAPPY40,
    ]
}

pub fn get_all_relative_axes() -> Vec<RelativeAxisType> {
    vec![
        RelativeAxisType::REL_X,
        RelativeAxisType::REL_Y,
        RelativeAxisType::REL_Z,
        RelativeAxisType::REL_RX,
        RelativeAxisType::REL_RY,
        RelativeAxisType::REL_RZ,
        RelativeAxisType::REL_HWHEEL,
        RelativeAxisType::REL_DIAL,
        RelativeAxisType::REL_WHEEL,
        RelativeAxisType::REL_MISC,
        RelativeAxisType::REL_RESERVED,
        RelativeAxisType::REL_WHEEL_HI_RES,
        RelativeAxisType::REL_HWHEEL_HI_RES,
    ]
}

pub fn get_all_switches() -> Vec<SwitchType> {
    vec![
        SwitchType::SW_LID,
        SwitchType::SW_TABLET_MODE,
        SwitchType::SW_HEADPHONE_INSERT,
        SwitchType::SW_RFKILL_ALL,
        SwitchType::SW_MICROPHONE_INSERT,
        SwitchType::SW_DOCK,
        SwitchType::SW_LINEOUT_INSERT,
        SwitchType::SW_JACK_PHYSICAL_INSERT,
        SwitchType::SW_VIDEOOUT_INSERT,
        SwitchType::SW_CAMERA_LENS_COVER,
        SwitchType::SW_KEYPAD_SLIDE,
        SwitchType::SW_FRONT_PROXIMITY,
        SwitchType::SW_ROTATE_LOCK,
        SwitchType::SW_LINEIN_INSERT,
        SwitchType::SW_MUTE_DEVICE,
        SwitchType::SW_PEN_INSERTED,
        SwitchType::SW_MACHINE_COVER,
    ]
}

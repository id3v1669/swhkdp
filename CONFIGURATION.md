Example configuration elements

```kdl
master {                                     // default mode if not defined is called "master"
  KEY_BRIGHTNESSDOWN "light -U 5"            // single-key command
  BTN_SIDE KEY_LEFTMETA                      // remap mouse side button to act as lmeta
  KEY_LEFTMETA+KEY_LEFTSHIFT+KEY_T wezterm   // launch terminal wezterm
  KEY_RIGHTMETA+KEY_2 "@enter secondary"     // load shortcuts under "secondary" section
}
secondary {                                  // alternative keymap
  KEY_RIGHTMETA+KEY_1 "@enter master"        // load shortcuts under "master" section
  KEY_BRIGHTNESSUP "light -A 5"
}
general {
  default master                             // custom name for default mode
  oneoff #false
  swallow #false
}
```
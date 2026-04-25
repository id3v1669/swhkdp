Example configuration elements

```kdl
master {                                          // mode name; "master" is the built-in default
  KEY_BRIGHTNESSDOWN "light -U 5"                 // single key — run a shell command
  KEY_LEFTMETA+KEY_T "wezterm"                    // modifier + key — run a shell command
  KEY_LEFTMETA+KEY_LEFTSHIFT+KEY_T "wezterm"      // multiple modifiers are allowed

  KEY_LEFTMETA+KEY_C "wezterm" on_release=#true   // fire on key release instead of press
  KEY_LEFTMETA+KEY_V "wezterm" send=#true         // fire command AND forward key to virtual device

  BTN_SIDE KEY_LEFTMETA                           // remap — rewrite one key as another

  KEY_RIGHTMETA+KEY_2 "@enter secondary"          // switch active mode
  KEY_RIGHTMETA+KEY_3 "notify-send hi && @enter secondary"  // run command AND switch mode

  // group expansion — one rule creates one hotkey per key/command pair
  KEY_LEFTMETA+<KEY_1,KEY_2,KEY_3> "swaymsg workspace {1,2,3}"

  // range expansion — expands all keys from KEY_1 to KEY_3 inclusive
  KEY_LEFTMETA+KEY_LEFTSHIFT+<KEY_1-KEY_3> "swaymsg move container to workspace {1,2,3}"
}

secondary {                                       // alternative keymap, activated via @enter
  KEY_RIGHTMETA+KEY_1 "@enter master"             // return to master mode
  KEY_BRIGHTNESSUP "light -A 5"
}

general {
  default master                                  // mode that is active on start and after oneoff
  oneoff #false                                   // if true, return to default after each hotkey fires
  swallow #false                                  // if true, matched key events are not forwarded to the virtual device
}
```

Macro configuration

```kdl
master {
  // Macro syntax: KEY "@macro" <type> { <steps> }
  // <type> is optional; defaults to "simple"

  KEY_LEFTMETA+KEY_N "@macro" {              // simple macro — runs steps once
    KEY_LEFTCTRL "down"                      // key step: hold Ctrl down
    KEY_A "click"                            // key step: press and release A (default action)
    KEY_LEFTCTRL "up"                        // key step: release Ctrl
  }

  KEY_LEFTMETA+KEY_R "@macro" "simple" {    // explicit simple type
    KEY_LEFTMETA "down"
    BTN_LEFT "down"
    move x=0 y=200 duration=1000            // move mouse 200px down over 1 second
    BTN_LEFT "up"
    KEY_LEFTMETA "up"
  }

  KEY_LEFTMETA+KEY_E "@macro" "endless" {   // endless macro — repeats steps in a loop
    BTN_LEFT "click"                         // stopped by pressing ESC
    move x=10 y=0 duration=50
  }

  KEY_LEFTMETA+KEY_H "@macro" "hold" {      // hold macro — repeats while trigger key is held
    move x=5 y=0 duration=16                // stops when any trigger key (modifier or keysym) is released
  }
}
```

Macro step reference

```kdl
// Key action step: KEY_NAME "down"|"up"|"click"
//   down  — press the key and keep it held
//   up    — release a previously held key
//   click — press and immediately release (default when action is omitted)
KEY_LEFTSHIFT "down"
KEY_A "click"
KEY_LEFTSHIFT "up"

// Move step: move x=<px> y=<px> duration=<ms> type=<curve> path=<shape> direction=<arc-dir>
//   x, y       — target offset in pixels (default 0)
//   duration   — total time in milliseconds (default 0 = instant)
//   type       — "constant" (default), "accelerate", "decelerate"
//   path       — "direct" (default), "arc"
//   direction  — "cw" (default, clockwise) or "ccw"; only used when path="arc"
move x=100 y=0 duration=500 type="decelerate"
move x=0 y=150 duration=800 path="arc" direction="ccw"

// Repeat step: repeat <count> { <steps> }
//   count must be >= 2
//   steps inside repeat follow the same rules as top-level macro steps
//   repeat blocks can be nested
repeat 3 {
  KEY_LEFTCTRL "down"
  KEY_C "click"
  KEY_LEFTCTRL "up"
}
```

swhkd(5) "github.com/id3v1669/swhkd" "File Formats Manual"

# NAME

swhkd	- Hotkey daemon inspired by sxhkd written in Rust

# CONFIG FILE

	- A global config can be defined in */etc/swhkd/swhkdp.yml.
	- A local config overrides the global one. Local configs should be placed in the root of the project.

# SYNTAX

Syntax follows yml or json format

For valid keys and modifiers, launch swhkdp with debug -d flag and press key,
in logs key will appear in format: `DEBUG swhkdp] Key: KEY_C`

Hotkeys have 2 formats right now, "command" and "singlecommand".

# EXAMPLE

```
# Modes section containes different modes with "normal" to be default
modes:
  normal:
    # Section for hotkeys
    hotkeys:
      KEY_LEFTMETA+KEY_LEFTSHIFT+KEY_A:
        action: alacritty
        # Action type defaults to "command", so can be skipped
        action_type: command
      KEY_LEFTMETA+KEY_LEFTSHIFT+KEY_K:
        action: kitty
      KEY_MUTE:
        action: pamixer -t
        action_type: singlecommand
    oneoff: false
    swallow: false
remaps:
  BTN_SIDE: KEY_LEFTMETA

```
# AUTHORS

Created by Shinyzenith, EdenQwQ, and Angelo Fallaria.
Maintained by id3v1669 <id3v1669@gmail.com> as this project went different path from original repo.
For more information about development, see <https://github.com/id3v1669/swhkd>.

# SEE ALSO

	- *swhkdp(1)*
	- *swhks(1)*

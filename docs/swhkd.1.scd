swhkd(1) "github.com/id3v1669/swhkd" "General Commands Manual"

# NAME

swhkd - Hotkey daemon inspired by sxhkd written in Rust

# SYNOPSIS

*pkexec swhkd* [_flags_]

# CONFIG FILE

The config file goes in */etc/swhkd/swhkdp.yml*.

This can be avoided as such:
	- Using the *include* statement in your config file.
	- Using the *-c* flag to mention a custom config file at runtime.
	- Symlinking the config file from *~/.config/swhkd/swhkdp.yml* or any other directory of choice to */etc/swhkd/swhkdp.yml*.

More about the config file syntax in `swhkd(5)`

# OPTIONS

*-h*, *--help*
	Print help message and quit.

*-V*, *--version*
	Print version information.

*-c*, *--config* <CONFIG_FILE_PATH>
	Set a custom config file path.

*-C*, *--cooldown* <COOLDOWN_IN_MS>
	Set a custom repeat cooldown duration. Default is 250ms. Most wayland
	compositors handle this server side however, either way works.

*-d*, *--debug*
	Enable debug mode.

*-D, --device* <DEVICE_NAMES>
	Manually set the supported devices to use. Can occur multiple times.

*-I, --ignoredevice* <DEVICE_NAMES>
	Manually set the supported devices to ignore. Can occur multiple times.

# SIGNALS

	- Reload config file: `sudo pkill -HUP swhkd`
	- Pause Hotkey checking: `sudo pkill -USR1 swhkd`
	- Resume key checking: `sudo pkill -USR2 swhkd`

# AUTHORS

Created by Shinyzenith, EdenQwQ, and Angelo Fallaria.
Maintained by id3v1669 <id3v1669@gmail.com> as this project went different path from original repo.
For more information about development, see <https://github.com/id3v1669/swhkd>.

# SEE ALSO

	- *swhkdp(5)*
	- *swhks(1)*

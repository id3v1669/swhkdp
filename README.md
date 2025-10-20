<p align=center>
  <img src="https://github.com/id3v1669/id3v1669/assets/swhkdp.png" alt=swhkdp width=60%>

  <p align="center">A next-generation hotkey daemon for Wayland/X11 written in <a href="https://www.rust-lang.org/">Rust</a>.</p>

  <p align="center">
  <a href="./LICENSE.md"><img src="https://img.shields.io/github/license/id3v1669/swhkdp?style=flat-square&logo=appveyor"></a>
  <img src="https://img.shields.io/badge/cargo-v1.2.1-green?style=flat-square&logo=appveyor">
  <img src="https://img.shields.io/github/issues/id3v1669/swhkdp?style=flat-square&logo=appveyor">
  <img src="https://img.shields.io/github/forks/id3v1669/swhkdp?style=flat-square&logo=appveyor">
  <img src="https://img.shields.io/github/stars/id3v1669/swhkdp?style=flat-square&logo=appveyor">
  </p>
</p>

## swhkdp

**S**imple **W**ayland **H**ot**K**ey **D**aemon **P**olkit

Originally forked from [swhkd](https://github.com/waycrate/swhkd), deattached 
from the original repo due to the desire to keep the Polkit security model and improve repo discoverability.

`swhkdp` is a display protocol-independent hotkey daemon made in
[Rust](https://www.rust-lang.org). `swhkdp` uses an easy-to-use configuration
system inspired by `sxhkd`, so you can easily add or remove hotkeys.

Because `swhkdp` can be used anywhere, the same `swhkdp` config can be used across
Xorg or Wayland desktops, and you can even use `swhkdp` in a TTY.

## Installation and Building

[Installation and building instructions can be found here.](./INSTALL.md)

## Running

```bash
swhks &
pkexec swhkdp
```

## Runtime signals

After opening `swhkdp`, you can control the program through signals:

- `sudo pkill -USR1 swhkdp` — Pause key checking
- `sudo pkill -USR2 swhkdp` — Resume key checking
- `sudo pkill -HUP swhkdp` — Reload config file

## Configuration

`swhkdp` uses configuration files that follows json syntax, [detailed
instructions can be found in CONFIGURATION.md](./CONFIGURATION.md)

The default configuration file is in `/etc/swhkdp/swhkdp.json`. If you don't like
having to edit the file as root every single time, you can create a symlink from
`~/.config/swhkdp/swhkdp.json` to `/etc/swhkdp/swhkdp.json`.

Not sure what key to use, launch swhkdp with -d option and press needed key,
it will be shown in logs as `DEBUG swhkdp] Key: KEY_C`

## Autostart

### To autostart `swhkdp` you can do one of two things

1. Add the commands from the ["Running"
   section](https://github.com/id3v1669/swhkdp#running) to your window managers
   configuration file.
1. Enable the [service
   file](https://github.com/id3v1669/swhkdp/tree/master/contrib/init) for your
   respective init system. Currently, only systemd and OpenRC service files
   exist and more will be added soon including Runit.

## Security

We use a server-client model to keep you safe. The daemon (`swhkdp` — privileged
process) communicates to the server (`swhks` — running as non-root user) after
checking for valid keybindings. Since the daemon is totally separate from the
server, no other process can read your keystrokes. As for shell commands, you
might be thinking that any program can send shell commands to the server and
that's true! But the server runs the commands as the currently logged-in user,
so no extra permissions are provided (This is essentially the same as any app on
your desktop calling shell commands).

So yes, you're safe!

## Contributors

<a href="https://github.com/id3v1669/swhkdp/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=id3v1669/swhkdp" />
</a>

## Thanks to original authors

* Shinyzenith
* Angelo Fallaria
* EdenQwQ
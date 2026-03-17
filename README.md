<p align=center>
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
[Rust](https://www.rust-lang.org). `swhkdp` uses an easy-to-use configuration via [kdl](https://kdl.dev/).

Because `swhkdp` can be used anywhere, the same `swhkdp` config can be used across
Xorg or Wayland desktops, and you can even use `swhkdp` in a TTY.

## Installation and Building

### Warning: installation instructions are tested for nixos only, for other distros they are updated, but changes might be required.

[Installation and building instructions can be found here.](./INSTALL.md)

## Running

Recomended to use 2 services running in background, but you can lauch directly via:
```bash
swhks &
pkexec swhkdp
```

## Runtime signals

After opening `swhkdp`, you can control the program through signals or embed commands(decribed in [configuration](./CONFIGURATION.md)).

Availible signals:

- `sudo pkill -USR1 swhkdp` — Pause key checking
- `sudo pkill -USR2 swhkdp` — Resume key checking
- `sudo pkill -HUP swhkdp` — Reload config file

## Configuration

`swhkdp` uses configuration files that follows [kdl](https://kdl.dev/) syntax, [detailed
instructions can be found in CONFIGURATION.md](./CONFIGURATION.md)

The default configuration file is in `/etc/swhkdp/swhkdp.kdl`. 

It is not recommended to link `~/.config/swhkdp/swhkdp.kdl` to `/etc/swhkdp/swhkdp.kdl`. 
Recently feature of remaping keys was introduced and if shortcuts are stored in user-accesable directory, malicious script might remap all your keys to "none" and lock you out.
Also in the future macros feature will be implemented, so making config file user-accesable is not a safe option by any means.

**FEATURE IN DEVELOPMENT**: Not sure what key to use, launch swhkdp with -w or --what-am-i-pressing. This will launch swhkdp without a config and output in cli key you are pressing

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

## Work being done?

To view upcoming tasks and project development open [TODO](./TODO.md)

## Contributors

<a href="https://github.com/id3v1669/swhkdp/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=id3v1669/swhkdp" />
</a>

## Thanks to original authors

* Shinyzenith
* Angelo Fallaria
* EdenQwQ
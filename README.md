<!-- markdownlint-disable MD033 -->
<!-- markdownlint-disable MD041 -->
<p align="center">
  A next-generation hotkey daemon for Wayland/X11 written in
  <a href="https://www.rust-lang.org/">Rust</a>.
</p>

<p align="center">
  <a href="./LICENSE.md"><img alt="License" src="https://img.shields.io/github/license/id3v1669/swhkdp?style=flat-square&logo=appveyor"></a>
  <img alt="GitHub issues" src="https://img.shields.io/github/issues/id3v1669/swhkdp?style=flat-square&logo=appveyor">
  <img alt="GitHub forks" src="https://img.shields.io/github/forks/id3v1669/swhkdp?style=flat-square&logo=appveyor">
  <img alt="GitHub stars" src="https://img.shields.io/github/stars/id3v1669/swhkdp?style=flat-square&logo=appveyor">
</p>

# SWHKDP

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

The configuration file is at the fixed path `/etc/swhkdp/config.kdl`. Release
builds read **only** this path — there is no `-c`/`--config` flag (it exists only
in debug builds, for local testing).

The config must be **owned by root and writable only by root** (no group/other
write bit), and it must sit under a directory chain that is likewise root-owned
and root-write-only. The daemon refuses to parse anything else — see the
[Security](#security) section for why. Do **not** make the config or its
directory user-writable: keys can be remapped from the config, so a malicious
script with write access could remap every key to "none" and lock you out (and
macros, once implemented, make a user-writable config even more dangerous).

Macros feature is still in development. If you want to test it, build project with `--features macro`.

### Not sure what key to use?

launch swhkdp with -w or --watch. This will launch swhkdp without a config and output in cli key you are pressing.

## Security

swhkdp's security model has three parts: how it is launched, how it treats the
config it parses as root, and how it isolates keystrokes from command execution.

### Runtime: general isolation

We use a server-client model to keep you safe. The daemon (`swhkdp` — privileged
process) communicates to the server (`swhks` — running as non-root user) after
checking for valid keybindings. Since the daemon is totally separate from the
server, no other process can read your keystrokes. As for shell commands, you
might be thinking that any program can send shell commands to the server and
that's true! But the server runs the commands as the currently logged-in user,
so no extra permissions are provided (This is essentially the same as any app on
your desktop calling shell commands).

### Launch: pkexec only

`swhkdp` runs as root. Authorization is governed by the polkit action `com.github.swhkdp.pkexec`.
Install script and the provided NixOS module adds a single narrow rule that grants a passwordless
launch **only** to the configured user's active session, so unattended autostart works while
other local users and inactive/remote (e.g. SSH) sessions get a password prompt.

### Config: fixed path, root-authored

The daemon parses its config as root, which makes an arbitrary, caller-chosen config path dangerous.
Dropping priveledges is not reliable way to check file perms, chainloading shwkdp with user perms
creates another vector of attac, so the most reliable and secure solution was just to make file
root-write only. swhkdp constructs **release** builds without `-c`/`--config` and makes it
availble only for debug builds. There is exactly one config, at the fixed path that is accepted by
daemon: `/etc/swhkdp/config.kdl`.

Before parsing that fixed path, the daemon checks two things:

- The **file** is owned by root and writable only by root (`root_write_only`), so a non-root user
  cannot have altered the key remaps.
- Every **ancestor directory** is likewise root-owned and root-write-only (`chain_is_root_write_only`),
  so a non-root user cannot have planted or swapped a symlink in the path (e.g. making
  `/etc/swhkdp/config.kdl` point at `/etc/shadow`). If the whole chain is root-only, any symlink in it
  was placed by root and is as trusted as the config itself.

## Work being done?

To view upcoming tasks and project development open [TODO](./TODO.md)

## Contributors

<a href="https://github.com/id3v1669/swhkdp/graphs/contributors?selectedMetric=additions">
  <img alt="GitHub contributors" src="https://contrib.rocks/image?repo=id3v1669/swhkdp" />
</a>

## Thanks to original authors

- Shinyzenith
- Angelo Fallaria
- EdenQwQ

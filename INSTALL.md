# Installation instructions

Instructions for different distros. To be updated, for now:

- [NixOS](#nixos)
- [Other distros](#other)

## NixOS

For now flake users only.

This repo contains a NixOS Module for swhkdp service.
To enable module add an input first and import to modules:

```nix
{
  inputs = {
    swhkdp.url = "github:id3v1669/swhkdp";
  }

  outputs = {nixpkgs, swhkdp, ...} @ inputs: {
    nixosConfigurations.HOSTNAME = nixpkgs.lib.nixosSystem {
      specialArgs = { inherit inputs; };
      modules = [
        ./configuration.nix
        swhkdp.nixosModules.default
      ];
    };
  } 
}
```

After importing you should be able to use it in your configuration.nix file:

```nix
{ inputs
, ...
}:
{
  services.swhkdp = {
    enable = true;
    username = "user";
    package = inputs.swhkdp.packages.${system}.default.override { rfkillFeature = true; };
    cooldown = 300;
    # To get device names use command `libinput list-devices | grep -i Device:`
    devices = [ #if device list is not present or empty, automaticly scans among all availible devices
      "device1"
    ];
    ignore = [ # ignoring devices is prioritized over device list
      "device2"
    ];
    settings = {
      modes = {
        normal = {
          swallow = false;
          oneoff = false;
          hotkeys = {
            "KEY_LEFTMETA+KEY_LEFTSHIFT+KEY_K".action = "kitty";
            "KEY_LEFTMETA+KEY_B" = {
              action = "firefox";
            };
            "KEY_MUTE" = {
              action_type = "singlecommand";
              action = "pamixer -t";
            };
          };
        };
      };
      remaps = {
        "BTN_SIDE" = "KEY_LEFTMETA";
      };
    };
  };
}
```

- rfkill is feature related to [this](https://github.com/waycrate/swhkd/pull/254) pr/discussion
- Replace HOSTNAME with your oun

## Other

### Building

`swhkdp` and `swhks` install to `/usr/local/bin/` by default. You can change this behaviour by editing the [Makefile](../Makefile) variable, `DESTDIR`, which acts as a prefix for all installed files. You can also specify it in the make command line, e.g. to install everything in `subdir`: `make DESTDIR="subdir" install`.

### Security requirements (all distros)

`swhkdp` runs as root via `pkexec`, so its integrity depends on the install location.
The `swhkdp` binary and every directory on its path must be owned by root and not writable
by other users** (e.g. `/usr/local/bin` root-owned, mode `0755`).

Install a polkit action for `com.github.swhkdp.pkexec` that allows passwordless start for swhkdp via daemon/autostart.
The NixOS module in this repo does exactly that, on other distros drop an equivalent `.policy` (and, if desired, a `rules.d` rule) into your polkit configuration.

The config file lives at the fixed path `/etc/swhkdp/config.kdl` (release builds have no `-c`/`--config` flag).
It must be owned by root and writable only by root, under a root-owned directory chain.
The daemon refuses to parse anything else, more can be found here: [Security](./README.md#security).

### Dependencies

**Runtime:**

- Policy Kit Daemon ( polkit )
- Uinput kernel module
- Evdev kernel module

**Compile time:**

- git
- scdoc (If present, man-pages will be generated)
- make
- libudev (in Debian, the package name is `libudev-dev`)
- rustup

### Compiling

- `git clone https://github.com/id3v1669/swhkdp;cd swhkdp`
- `make setup`
- `make clean`
- `make`
- `sudo make install`

### Running

```sh
swhks &
pkexec swhkdp
```

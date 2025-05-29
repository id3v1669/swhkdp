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
* rfkill is feature related to [this](https://github.com/waycrate/swhkd/pull/254) pr/discussion
* Do not forget to start/add to autostart swhkdp of your system after login.
* Replace HOSTNAME with your oun

ps. this module will be updated to support devices, but it is already good enough to use

# Building:

`swhkdp` and `swhks` install to `/usr/local/bin/` by default. You can change this behaviour by editing the [Makefile](../Makefile) variable, `DESTDIR`, which acts as a prefix for all installed files. You can also specify it in the make command line, e.g. to install everything in `subdir`: `make DESTDIR="subdir" install`.

# Dependencies:

**Runtime:**

-   Policy Kit Daemon ( polkit )
-   Uinput kernel module
-   Evdev kernel module

**Compile time:**

-   git
-   scdoc (If present, man-pages will be generated)
-   make
-   libudev (in Debian, the package name is `libudev-dev`)
-   rustup

# Compiling:

-   `git clone https://github.com/id3v1669/swhkdp;cd swhkdp`
-   `make setup`
-   `make clean`
-   `make`
-   `sudo make install`

# Running:

```
swhks &
pkexec swhkdp
```

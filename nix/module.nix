self: {
  config,
  lib,
  pkgs,
  ...
}: let
  cfg = config.services.swhkdp;
  format = pkgs.formats.yaml {};
  inherit (pkgs.stdenv.hostPlatform) system;

  inherit (lib) types;
  inherit (lib.modules) mkIf;
  inherit (lib.options) mkOption mkEnableOption;
in {
  options.services.swhkdp = {
    enable = mkEnableOption "Simple Wayland HotKey Daemon";

    package = mkOption {
      description = "The package to use for `swhkdp`";
      default = self.packages.${system}.default;
      type = types.package;
    };

    cooldown = mkOption {
      description = "The cooldown to use for `swhkdp`";
      default = 250;
      type = types.int;
    };

    devices = mkOption {
      description = "The list of devices to use for `swhkdp`";
      default = [];
      type = types.listOf types.str;
    };

    ignore = mkOption {
      description = "The list of devices to ignore for `swhkdp`";
      default = [];
      type = types.listOf types.str;
    };

    settings = mkOption {
      description = "The config to use for `swhkdp` syntax and samples could found in [repo](https://github.com/id3v1669/swhkdp).";
      type = format.type;
      default = {
        modes = {
          normal = {
            swallow = false;
            oneoff = false;
            hotkeys = {
              "KEY_LEFTMETA+KEY_LEFTSHIFT+KEY_T".action = "alacritty";
            };
          };
        };
      };
    };
  };

  config = mkIf cfg.enable {
    environment.systemPackages = [cfg.package];

    systemd.user.services.swhkdp = {
      description = "Simple Wayland HotKey Daemon";
      bindsTo = ["default.target"];
      script = let
        swhkdpcfg = pkgs.writeText "swhkdp.json" (builtins.toJSON cfg.settings);
        swhkdpCmd =
          if cfg.settings != null
          then "--config ${swhkdpcfg}"
          else "";
        devicesCmd =
          if cfg.devices != []
          then "-D \"${lib.concatStringsSep "|" cfg.devices}\""
          else "";
      in ''
        /run/wrappers/bin/pkexec ${cfg.package}/bin/swhkdp ${swhkdpCmd} ${devicesCmd}\
          --cooldown ${toString cfg.cooldown} \
          -I "${lib.concatStringsSep "|" cfg.ignore}"
      '';
      serviceConfig.Restart = "always";
      wantedBy = ["default.target"];
    };
    security.polkit = {
      enable = true;
      extraConfig = ''
        polkit.addRule(function(action, subject) {
            if (action.id == "com.github.swhkdp.pkexec"  &&
                subject.local == true &&
                subject.active == true &&) {
                    return polkit.Result.YES;
                }
        });
      '';
    };
  };
}

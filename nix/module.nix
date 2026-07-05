self: {
  config,
  lib,
  pkgs,
  ...
}: let
  cfg = config.services.swhkdp;
  inherit (pkgs.stdenv.hostPlatform) system;
  inherit (lib) types;
  inherit (lib.modules) mkIf;
  inherit (lib.options) mkOption mkEnableOption;
in {
  options.services.swhkdp = {
    enable = mkEnableOption "Simple Wayland HotKey Daemon Polkit";

    username = mkOption {
      description = "Username to resolve profile PATH for the swhks user service.";
      type = types.nullOr types.str;
      default = null;
    };

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
      description = "Path to config file or config string for `swhkdp`";
      default = ''
        master {
          KEY_LEFTMETA+KEY_LEFTSHIFT+KEY_T wezterm
        }
        general {
          oneoff #false
          swallow #false
        }
      '';
      type = types.either types.str types.path;
    };
  };

  config = mkIf cfg.enable {
    assertions = [
      {
        assertion = cfg.username != null;
        message = "services.swhkdp.username must be set when swhkdp is enabled.";
      }
    ];

    environment.systemPackages = [cfg.package];

    environment.etc."swhkdp/config.kdl".source =
      if builtins.isPath cfg.settings
      then cfg.settings
      else pkgs.writeText "config.kdl" cfg.settings;

    systemd.user.services.swhkdp = {
      description = "Simple Wayland HotKey Daemon";
      bindsTo = ["default.target"];
      script = let
        devicesCmd =
          if cfg.devices != []
          then "-D \"${lib.concatStringsSep "|" cfg.devices}\""
          else "";
      in ''
        /run/wrappers/bin/pkexec ${cfg.package}/bin/swhkdp ${devicesCmd}\
          --cooldown ${toString cfg.cooldown} \
          -I "${lib.concatStringsSep "|" cfg.ignore}"
      '';
      serviceConfig.Restart = "always";
      wantedBy = ["default.target"];
    };

    systemd.user.services.swhks = {
      description = "Simple Wayland HotKey Daemon User Service";
      bindsTo = ["graphical-session.target"];
      after = ["graphical-session.target"];
      serviceConfig = {
        ExecStart = "${lib.getExe' cfg.package "swhks"}";
        Restart = "always";
        KillMode = "process";
        SendSIGKILL = "no";
      };
      path = [
        "/run/wrappers"
        "/etc/profiles/per-user/${cfg.username}"
        "/run/current-system/sw"
      ];
      wantedBy = ["graphical-session.target"];
    };

    security.polkit = {
      enable = true;
      extraConfig = ''
        polkit.addRule(function(action, subject) {
            if (action.id == "com.github.swhkdp.pkexec" &&
                subject.active &&
                subject.user == "${cfg.username}") {
                    return polkit.Result.YES;
            }
        });
      '';
    };
  };
}

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
    enable = mkEnableOption "Simple Wayland HotKey Daemon";

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
      description = "settings";
      default = "";
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

    systemd.user.services.swhkdp = {
      description = "Simple Wayland HotKey Daemon";
      bindsTo = ["default.target"];
      script = let
        swhkdpcfg =
          if builtins.isPath cfg.settings
          then cfg.settings
          else pkgs.writeText "swhkdp.kdl" cfg.settings;
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

    systemd.user.services.swhks = {
      description = "Simple Wayland HotKey Daemon User Service";
      bindsTo = ["graphical-session.target"];
      after = ["graphical-session.target"];
      serviceConfig = {
        ExecStart = "${lib.getExe' cfg.package "swhks"}";
        Restart = "always";
        KillMode = "process";
      };
      path = [
        "/etc/profiles/per-user/${cfg.username}"
        "/run/current-system/sw"
      ];
      wantedBy = ["graphical-session.target"];
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

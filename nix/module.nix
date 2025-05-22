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
        swhkdpyml = pkgs.writeText "swhkdp.yml" "${lib.generators.toYAML {} cfg.settings}";
        swhkdpymlCmd =
          if cfg.settings != null
          then "--config ${swhkdpyml}"
          else "";
      in ''
        /run/wrappers/bin/pkexec ${cfg.package}/bin/swhkdp ${swhkdpymlCmd} \
          --cooldown ${toString cfg.cooldown}
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

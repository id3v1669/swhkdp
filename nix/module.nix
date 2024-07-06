inputs: {
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.services.swhkd;
  inherit (pkgs.stdenv.hostPlatform) system;
  defaultPackage = inputs.self.packages.${system}.default;

  inherit (lib) types;
  inherit (lib.modules) mkIf mkForce;
  inherit (lib.options) mkOption mkEnableOption;

  format = pkgs.formats.ini {};
in {
  options.services.swhkd = {
    enable = mkEnableOption "Simple Wayland HotKey Daemon";

    package = mkOption {
      description = "The package to use for `swhkd`";
      default = defaultPackage;
      type = types.package;
    };

    configPath = mkOption {
      description = "The config path to use for `swhkd`";
      default = "/etc/swhkd/swhkdrc";
      type = types.str;
    };

    target = mkOption {
      description = "The target to use for `swhkd`";
      default = "default.target";
      type = types.str;
    };
    
    cooldown = mkOption {
      description = "The cooldown to use for `swhkd`";
      default = 250;
      type = types.int;
    };
  };

  config = mkIf cfg.enable {
    environment.systemPackages = [ cfg.package ];

    systemd.user.services.swhkd = {
      description = "Simple Wayland HotKey Daemon";
      bindsTo = [ "${cfg.target}"];
      script = ''
        /run/wrappers/bin/pkexec ${cfg.package}/bin/swhkd \
          --config ${cfg.configPath} \
          --cooldown ${toString cfg.cooldown}
      '';
      serviceConfig.Restart = "always";
      wantedBy = [ "${cfg.target}" ];
    };
    security.polkit = {
      enable = true;
      extraConfig = ''
        polkit.addRule(function(action, subject) {
            if (action.id == "com.github.swhkd.pkexec"  &&
                subject.local == true &&
                subject.active == true &&) {
                    return polkit.Result.YES;
                }
        });
      '';
    };
  };
}
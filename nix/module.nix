self:
{ config
, lib
, pkgs
, ...
}:
let
  cfg = config.services.swhkd;
  format = pkgs.formats.yaml { };
  inherit (pkgs.stdenv.hostPlatform) system;

  inherit (lib) types;
  inherit (lib.modules) mkIf;
  inherit (lib.options) mkOption mkEnableOption;
in
{
  options.services.swhkd = {
    enable = mkEnableOption "Simple Wayland HotKey Daemon";

    package = mkOption {
      description = "The package to use for `swhkd`";
      default = self.packages.${system}.default;
      type = types.package;
    };
    
    cooldown = mkOption {
      description = "The cooldown to use for `swhkd`";
      default = 250;
      type = types.int;
    };

    settings = mkOption {
      description = "The config to use for `swhkd` syntax and samples could found in [repo](https://github.com/id3v1669/swhkd).";
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
    environment.systemPackages = [ cfg.package ];

    systemd.user.services.swhkd = {
      description = "Simple Wayland HotKey Daemon";
      bindsTo = [ "default.target" ];
      script = let 
        swhkdrc = pkgs.writeText "swhkd.yml" "${lib.generators.toYAML { } cfg.settings}";
        swhkdrcCmd = if cfg.settings != null then "--config ${swhkdrc}" else "";
      in ''
        /run/wrappers/bin/pkexec ${cfg.package}/bin/swhkd ${swhkdrcCmd} \
          --cooldown ${toString cfg.cooldown}
      '';
      serviceConfig.Restart = "always";
      wantedBy = [ "default.target" ];
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

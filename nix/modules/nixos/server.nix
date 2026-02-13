{
  flake,
  config,
  lib,
  pkgs,
  ...
}: let
  cfg = config.services.pictureframeServer;
in {
  options.services.pictureframeServer = {
    enable = lib.mkEnableOption "pictureframe web server";

    package = lib.mkOption {
      type = lib.types.package;
      description = "The pictureframe package to use.";
      default = flake.packages."${pkgs.system}".default;
    };

    debug = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = "Enable debug logging via RUST_LOG=debug.";
    };
  };

  config = lib.mkIf cfg.enable {
    users.users.pictureframe = {
      isSystemUser = true;
      group = "pictureframe";
      home = "/var/lib/pictureframe";
      createHome = true;
    };

    users.groups.pictureframe = {};

    systemd.services.pictureframe = {
      description = "Pictureframe web server";
      wantedBy = ["multi-user.target"];
      after = ["network.target"];

      environment =
        {
          XDG_CONFIG_HOME = "/var/lib/pictureframe/.config";
          XDG_DATA_HOME = "/var/lib/pictureframe/.local/share";
          XDG_STATE_HOME = "/var/lib/pictureframe/.local/state";
          XDG_CACHE_HOME = "/var/lib/pictureframe/.cache";
        }
        // lib.optionalAttrs cfg.debug {
          RUST_LOG = "debug";
        };

      serviceConfig = {
        ExecStart = "${lib.getExe cfg.package} --dist-dir ${flake.packages.${pkgs.system}.frontend}";
        Restart = "on-failure";
        RestartSec = 5;

        User = "pictureframe";
        Group = "pictureframe";
        StateDirectory = "pictureframe";
        WorkingDirectory = "/var/lib/pictureframe";

        # Hardening
        ProtectHome = true;
        ProtectSystem = "strict";
        PrivateTmp = true;
        NoNewPrivileges = true;
      };
    };
  };
}

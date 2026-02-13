{
  flake,
  config,
  lib,
  pkgs,
  ...
}: let
  cfg = config.services.pictureframeServer;
  cliArgs = lib.cli.toGNUCommandLine {} {
    dist-dir = cfg.distDir;
    host = cfg.host;
    port = cfg.port;
  };
in {
  options.services.pictureframeServer = {
    enable = lib.mkEnableOption "pictureframe web server";

    package = lib.mkOption {
      type = lib.types.package;
      description = "The pictureframe package to use.";
      # TODO: when using this module, I kept getting the error 'no flake input'
      # default = flake.packages."${pkgs.system}".default;
    };

    # TODO: once we figure out why 'flake' input wont work we can get ride of this option
    distDir = lib.mkOption {
      type = lib.types.path;
      description = "Dist dir to serve";
    };

    debug = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = "Enable debug logging via RUST_LOG=debug.";
    };

    host = lib.mkOption {
      type = lib.types.str;
      default = "0.0.0.0";
      description = "Host address to bind to.";
    };

    port = lib.mkOption {
      type = lib.types.port;
      default = 3000;
      description = "Port to listen on.";
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

      path = [
        pkgs.imagemagick # TODO: instead of duplicating, take runtimeInputs from nix/lib/default.nix
      ];

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
        ExecStart = lib.escapeShellArgs ([
          (lib.getExe cfg.package)
        ] ++ cliArgs); # TODO: same issue with flake input from ${flake.packages.${pkgs.system}.frontend}"
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

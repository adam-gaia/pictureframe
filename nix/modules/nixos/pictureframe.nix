{
  config,
  lib,
  pkgs,
  ...
}: let
  cfg = config.services.pictureframe;
in {
  options.services.pictureframe = {
    enable = lib.mkEnableOption "pictureframe kiosk display";

    url = lib.mkOption {
      type = lib.types.str;
      default = "https://pictureframe.gmoff.net";
      description = "URL to display in the kiosk browser.";
    };

    user = lib.mkOption {
      type = lib.types.str;
      default = "pictureframe";
      description = "User account under which the kiosk runs.";
    };
  };

  config = lib.mkIf cfg.enable {
    # Create the dedicated user
    users.users.${cfg.user} = {
      isSystemUser = true;
      group = cfg.user;
      # Cage needs a home directory for the Wayland socket, etc.
      home = "/var/lib/${cfg.user}";
      createHome = true;
    };
    users.groups.${cfg.user} = {};

    # Cage kiosk compositor running Firefox
    services.cage = {
      enable = true;
      user = cfg.user;
      program = "${pkgs.firefox}/bin/firefox --kiosk --private-window ${cfg.url}";
    };

    # Ensure Firefox uses Wayland natively under Cage
    environment.sessionVariables = {
      MOZ_ENABLE_WAYLAND = "1";
    };

    # GPU / graphics support for the Pi
    hardware.graphics.enable = true;
  };
}

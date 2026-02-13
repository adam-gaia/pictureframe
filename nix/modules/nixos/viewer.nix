{
  config,
  lib,
  pkgs,
  ...
}: let
  cfg = config.services.pictureframeViewer;
in {
  options.services.pictureframeViewer = {
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

    environment = {
      # Ensure Firefox uses Wayland natively under Cage
      MOZ_ENABLE_WAYLAND = "1";

      # Needed so cage doesnt block without input devices
      # https://github.com/cage-kiosk/cage/wiki/Troubleshooting#cage-does-not-start-without-any-input-devices
      WLR_LIBINPUT_NO_DEVICES = "1";
    };

    # GPU / graphics support for the Pi
    hardware.graphics.enable = true;
  };

  systemd.services."cage-tty1" = {
    after = [
      "network-online.target"
      "systemd-resolved.service"
    ];
  };
}

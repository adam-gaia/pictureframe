{
  config,
  lib,
  pkgs,
  ...
}: let
  cfg = config.services.pictureframeViewer;

  mkUserHome = username: "/var/lib/${username}"; # TODO: make this match config.users.users.${cfg.user}.home

  # Remove .mozilla to stop crash message
  mk-firefox-kiosk = username: let
    userHome = mkUserHome username;
  in
    pkgs.writeShellScriptBin "firefox-kiosk" ''
      rm -rf ${userHome}/.mozilla
      rm -rf ${userHome}/.cache/mozilla

      exec "${pkgs.firefox}/bin/firefox" --new-instance --kiosk ${cfg.url}
    '';
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

    programs.firefox = {
      enable = true;
      policies = {
        DisableTelemetry = true;
        DisableFirefoxStudies = true;
        DisableFirefoxAccounts = true;
        DisableAccounts = true;
        DontCheckDefaultBrowser = true;
      };
    };

    # Cage kiosk compositor running Firefox
    services.cage = let
      firefox-package = mk-firefox-kiosk cfg.user;
    in {
      enable = true;
      user = cfg.user;
      program = "${firefox-package}/bin/firefox-kiosk";
      environment = {
        # Needed so cage doesnt block without input devices
        # https://github.com/cage-kiosk/cage/wiki/Troubleshooting#cage-does-not-start-without-any-input-devices
        WLR_LIBINPUT_NO_DEVICES = "1";
      };
    };

    environment.sessionVariables = {
      # Ensure Firefox uses Wayland natively under Cage
      MOZ_ENABLE_WAYLAND = "1";
    };

    # GPU / graphics support for the Pi
    hardware.graphics.enable = true;

    systemd.services."cage-tty1" = {
      after = [
        "network-online.target"
        "systemd-resolved.service"
      ];
    };
  };
}

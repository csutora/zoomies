self:
{ config, lib, pkgs, ... }:

let
  cfg = config.services.zoomies;
in
{
  options.services.zoomies = {
    enable = lib.mkEnableOption "zoomies momentum scrolling";

    package = lib.mkOption {
      type = lib.types.package;
      default = self.packages.${pkgs.stdenv.hostPlatform.system}.zoomies;
      defaultText = lib.literalExpression "zoomies.packages.\${system}.zoomies";
      description = "the zoomies package to use";
    };

    deviceName = lib.mkOption {
      type = lib.types.str;
      default = "Magic Trackpad";
      description = "substring of the touchpad device name to match";
    };

    multiplier = lib.mkOption {
      type = lib.types.float;
      default = 0.1;
      description = "scale from touchpad velocity to scroll distance";
    };

    decayMs = lib.mkOption {
      type = lib.types.float;
      default = 325.0;
      description = "momentum decay time constant in milliseconds";
    };

    minVelocity = lib.mkOption {
      type = lib.types.float;
      default = 200.0;
      description = "minimum flick speed required to start momentum";
    };

    stopThreshold = lib.mkOption {
      type = lib.types.float;
      default = 40.0;
      description = "speed below which the glide stops";
    };

    tickMs = lib.mkOption {
      type = lib.types.ints.positive;
      default = 8;
      description = "milliseconds between emitted scroll frames";
    };

    naturalScroll = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = "whether momentum follows the natural scroll direction";
    };

    manageDeviceAccess = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = "grant the active-session user read access to the device via a udev uaccess rule";
    };
  };

  config = lib.mkIf cfg.enable {
    services.udev.extraRules = lib.mkIf cfg.manageDeviceAccess ''
      ACTION=="add|change", SUBSYSTEM=="input", KERNEL=="event*", ATTRS{name}=="*${cfg.deviceName}*", TAG+="uaccess"
    '';

    systemd.user.services.zoomies = {
      description = "zoomies momentum scrolling";
      wantedBy = [ "graphical-session.target" ];
      partOf = [ "graphical-session.target" ];
      after = [ "graphical-session.target" ];
      serviceConfig = {
        ExecStart = lib.concatStringsSep " " ([
          (lib.getExe cfg.package)
          "--device-name ${lib.escapeShellArg cfg.deviceName}"
          "--multiplier ${toString cfg.multiplier}"
          "--decay-ms ${toString cfg.decayMs}"
          "--min-velocity ${toString cfg.minVelocity}"
          "--stop-threshold ${toString cfg.stopThreshold}"
          "--tick-ms ${toString cfg.tickMs}"
        ] ++ lib.optional (!cfg.naturalScroll) "--traditional");
        Restart = "on-failure";
        RestartSec = 3;
      };
    };
  };
}

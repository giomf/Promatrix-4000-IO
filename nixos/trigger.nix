self:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.services.promatrix-trigger;
in
{
  options.services.promatrix-trigger = {
    enable = lib.mkEnableOption "Promatrix-4000-IO input trigger";

    package = lib.mkOption {
      type = lib.types.package;
      default = self.packages.${pkgs.stdenv.hostPlatform.system}.promatrix-trigger;
      defaultText = lib.literalExpression "promatrix-trigger";
      description = "The promatrix-trigger package to use.";
    };

    port = lib.mkOption {
      type = lib.types.str;
      default = "/dev/promatrix";
      description = "Serial port of the board's data interface.";
    };

    panelConfig = lib.mkOption {
      type = lib.types.path;
      description = "AM03127 panel config TOML.";
    };
  };

  config = lib.mkIf cfg.enable {
    # Stable /dev/promatrix for the board's data interface (if00; if02 is the log interface).
    services.udev.extraRules = ''
      SUBSYSTEM=="tty", ATTRS{idVendor}=="16c0", ATTRS{idProduct}=="27dd", ATTRS{product}=="Promatrix-4000-IO", ENV{ID_USB_INTERFACE_NUM}=="00", SYMLINK+="promatrix"
    '';

    systemd.services.promatrix-trigger = {
      description = "React to Promatrix-4000-IO input changes";
      wantedBy = [ "multi-user.target" ];
      serviceConfig = {
        ExecStart = "${cfg.package}/bin/promatrix-trigger --port ${cfg.port} --panel-config ${cfg.panelConfig}";
        Restart = "always";
        RestartSec = 5;
        DynamicUser = true;
        SupplementaryGroups = [ "dialout" ];
      };
    };
  };
}

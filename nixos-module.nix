{ self }:
{
  config,
  lib,
  pkgs,
  ...
}:

let
  cfg = config.services.fetch-rs;
  defaultUser = "fetch-rs";
  defaultGroup = "fetch-rs";
in
{
  options.services.fetch-rs = {
    enable = lib.mkEnableOption "fetch-rs";

    package = lib.mkOption {
      type = lib.types.package;
      default = self.packages.${pkgs.stdenv.hostPlatform.system}.default;
      defaultText = lib.literalExpression "inputs.fetch-rs.packages.${pkgs.stdenv.hostPlatform.system}.default";
      description = "The fetch-rs package to run.";
    };

    gitPackage = lib.mkPackageOption pkgs "git" { };

    user = lib.mkOption {
      type = lib.types.str;
      default = defaultUser;
      description = "User account under which fetch-rs runs.";
    };

    group = lib.mkOption {
      type = lib.types.str;
      default = defaultGroup;
      description = "Group under which fetch-rs runs.";
    };

    onCalendar = lib.mkOption {
      type = lib.types.str;
      default = "*-*-* *:00:00";
      description = "Interval in which fetch-rs runs.";
    };
  };

  config = lib.mkIf cfg.enable {
    users = {
      users = lib.mkIf (cfg.user == defaultUser) {
        ${defaultUser} = {
          description = "fetch-rs service user";
          inherit (cfg) group;
          isSystemUser = true;
        };
      };
      groups = lib.mkIf (cfg.group == defaultGroup) { ${defaultGroup} = { }; };
    };

    # this should probably be handled by the app itself,
    # but it works for now
    systemd.tmpfiles.rules = [
      "d /etc/fetch-rs 0750 fetch-rs fetch-rs -"
      "f /etc/fetch-rs/.gitconfig 0644 fetch-rs fetch-rs -"

      # Traversal permission on private parents.
      "a+ /home/camdenboren     - - - - u:fetch-rs:--x"
      "a+ /home/camdenboren/etc - - - - u:fetch-rs:--x"

      # Existing contents plus inheritance for new contents.
      "A+ /home/camdenboren/etc/nixos - - - - u:fetch-rs:r-X,d:u:fetch-rs:r-X"
    ];

    systemd.services.fetch-rs = {
      description = "fetch-rs service";
      wantedBy = [ ];
      after = [ "network.target" ];
      onSuccess = [ "fetch-rs-rebuild.service" ];
      path = [ "${cfg.gitPackage}" ];
      environment = {
        GIT_CONFIG_GLOBAL = "/etc/fetch-rs/.gitconfig";
      };

      serviceConfig = {
        Type = "oneshot";
        ExecStart = "${cfg.package}/bin/fetch-rs";
        User = cfg.user;
        Group = cfg.group;
        NoNewPrivileges = true;
        PrivateDevices = true;
        PrivateTmp = true;
        ProtectHome = "tmpfs";
        ProtectSystem = "strict";
        ReadWritePaths = [
          "/etc/fetch-rs"
          "/etc/fetch-rs/.gitconfig"
        ];
        BindReadOnlyPaths = [
          "/home/camdenboren/etc/nixos"
        ];
        UMask = "0027";
        WorkingDirectory = "/etc/fetch-rs";
      };
    };

    systemd.timers.fetch-rs = {
      description = "Timer for fetch-rs runs";
      wantedBy = [ "timers.target" ];
      timerConfig = {
        OnCalendar = cfg.onCalendar;
        Persistent = true;
      };
    };

    /*
      systemd.services.fetch-rs-rebuild = {
      description = "fetch-rs rebuild";
      wantedBy = [ "multi-user.target" ];
      after = [ "fetch-rs.service" ];

      serviceConfig = {
        ExecStart = "${cfg.package}/bin/rebuild";
        User = "root";
        NoNewPrivileges = true;
        PrivateDevices = true;
        PrivateTmp = true;
        ProtectHome = "read-only";
        ProtectSystem = "strict";
        ReadWritePaths = [ "/etc/fetch-rs" ];
        UMask = "0027";
        WorkingDirectory = "/etc/fetch-rs";
      };
      };
    */
  };
}

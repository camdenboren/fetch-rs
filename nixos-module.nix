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
  configDir = "/etc/fetch-rs";
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

    flakePath = lib.mkOption {
      type = lib.types.str;
      description = "Absolute path to the directory containig your flake-based Nix configuration (required).";
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

    systemd.tmpfiles.rules = [
      "d ${configDir} 0750 ${cfg.user} ${cfg.group} -"
    ];

    systemd.services.fetch-rs = {
      description = "fetch-rs service";
      wantedBy = [ ];
      after = [ "network.target" ];
      onSuccess = [ "fetch-rs-rebuild.service" ];
      path = [ cfg.gitPackage ];
      environment = {
        F_RS_FLAKE = cfg.flakePath;
        GIT_CONFIG_GLOBAL = "${configDir}/.gitconfig";
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
        ReadWritePaths = [ configDir ];
        BindReadOnlyPaths = [ cfg.flakePath ];
        UMask = "0027";
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

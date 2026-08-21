{ self }:
{
  config,
  lib,
  pkgs,
  ...
}:

let
  cfg = config.services.fetch-rs;
  defaultGroup = "users";
  configDir = ".config/fetch-rs";
  defaultHomeDirRoot = "/home";
  defaultRebuildUser = "root";
  defaultGitConfigPath = "/root/.gitconfig";
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

    rebuildPackage = lib.mkPackageOption pkgs "nixos-rebuild" { };

    user = lib.mkOption {
      type = lib.types.str;
      description = "User account under which fetch-rs runs This should generally be the owner of your Nix config as the main fetch-rs service will interact with it heavily (required).";
    };

    group = lib.mkOption {
      type = lib.types.str;
      default = defaultGroup;
      description = "Group under which fetch-rs runs.";
    };

    rebuildUser = lib.mkOption {
      type = lib.types.str;
      default = defaultRebuildUser;
      description = "User account under which fetch-rs-rebuild runs. This should generally be the root user.";
    };

    homeDirRoot = lib.mkOption {
      type = lib.types.str;
      default = defaultHomeDirRoot;
      description = "Path prefacing the user's home directory.";
    };

    gitConfigPath = lib.mkOption {
      type = lib.types.str;
      default = defaultGitConfigPath;
      description = "Path referenced for rebuildUser's Git config";
    };

    onCalendar = lib.mkOption {
      type = lib.types.str;
      default = "*-*-* *:00:00";
      description = "Interval in which fetch-rs runs. The default schedules fetch-rs to run hourly.";
    };
  };

  config = lib.mkIf cfg.enable {
    systemd.tmpfiles.rules = [
      "d ${cfg.homeDirRoot}/${cfg.user}/${configDir} 0750 ${cfg.user} ${cfg.group} -"
    ];

    systemd.services.fetch-rs = {
      description = "fetch-rs service";
      wantedBy = [ ];
      after = [ "network.target" ];
      onSuccess = [ "fetch-rs-rebuild.service" ];
      path = [ cfg.gitPackage ];
      environment = {
        F_RS_FLAKE = cfg.flakePath;
        F_RS_CONFIG = "${cfg.homeDirRoot}/${cfg.user}/${configDir}";
      };

      serviceConfig = {
        ExecStart = "${cfg.package}/bin/fetch-rs";
        Type = "oneshot";
        User = cfg.user;
        Group = cfg.group;
        NoNewPrivileges = true;
        PrivateDevices = true;
        PrivateTmp = true;
        ProtectSystem = "strict";
        ReadWritePaths = [
          cfg.flakePath
          "${cfg.homeDirRoot}/${cfg.user}/${configDir}"
        ];
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

    systemd.services.fetch-rs-rebuild = {
      description = "fetch-rs rebuild";
      wantedBy = [ ];
      after = [ "fetch-rs.service" ];
      path = [
        cfg.gitPackage
        cfg.rebuildPackage
      ];
      environment = {
        F_RS_FLAKE = cfg.flakePath;
        F_RS_CONFIG = "${cfg.homeDirRoot}/${cfg.user}/${configDir}";
        GIT_CONFIG_GLOBAL = "${cfg.gitConfigPath}";
      };

      serviceConfig = {
        Type = "oneshot";
        ExecStart = "${cfg.package}/bin/rebuild";
        User = cfg.rebuildUser;
        PrivateDevices = true;
        PrivateTmp = true;
        BindReadOnlyPaths = [
          cfg.flakePath
          "${cfg.homeDirRoot}/${cfg.user}/${configDir}"
        ];
        UMask = "0027";
      };
    };
  };
}

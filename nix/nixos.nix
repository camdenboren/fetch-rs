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
  defaultHomeDirRoot = "/home";
  defaultRebuildUser = "root";
  defaultGitConfigPath = "/root/.gitconfig";
  userHome = "${cfg.homeDirRoot}/${cfg.user}";
  configDir = "${userHome}/.config/fetch-rs";
  commonEnvironment = {
    F_RS_CONFIG = configDir;
    F_RS_FLAKE = cfg.flakePath;
  };
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
      type = lib.types.nonEmptyStr;
      description = "Absolute path to the directory containig your flake-based Nix configuration (required).";
    };

    gitPackage = lib.mkPackageOption pkgs "git" { };

    rebuildPackage = lib.mkOption {
      type = lib.types.package;
      default = config.system.build.nixos-rebuild;
      defaultText = lib.literalExpression "config.system.build.nixos-rebuild";
      description = "The package providing nixos-rebuild.";
    };

    user = lib.mkOption {
      type = lib.types.nonEmptyStr;
      description = "User account under which fetch-rs runs This should generally be the owner of your Nix config as the main fetch-rs service will interact with it heavily (required).";
    };

    group = lib.mkOption {
      type = lib.types.nonEmptyStr;
      default = defaultGroup;
      description = "Group under which fetch-rs runs.";
    };

    rebuildUser = lib.mkOption {
      type = lib.types.nonEmptyStr;
      default = defaultRebuildUser;
      description = "User account under which fetch-rs-rebuild runs. This should generally be the root user.";
    };

    homeDirRoot = lib.mkOption {
      type = lib.types.nonEmptyStr;
      default = defaultHomeDirRoot;
      description = "Path prefacing the user's home directory.";
    };

    gitConfigPath = lib.mkOption {
      type = lib.types.nonEmptyStr;
      default = defaultGitConfigPath;
      description = "Path referenced for rebuildUser's Git config";
    };

    secretsFile = lib.mkOption {
      type = lib.types.nullOr lib.types.nonEmptyStr;
      default = null;
      example = "/run/secrets/fetch-rs.env";
      description = ''
        Absolute path to a secrets file loaded by both fetch-rs services. Set F_RS_NTFY_URL in this file to the complete ntfy topic URL. The file itself is not copied to the Nix store.
      '';
    };

    onCalendar = lib.mkOption {
      type = lib.types.nonEmptyStr;
      default = "*-*-* *:00:00";
      description = "Interval in which fetch-rs runs. The default schedules fetch-rs to run hourly.";
    };
  };

  config = lib.mkIf cfg.enable {
    assertions = [
      {
        assertion = lib.hasPrefix "/" cfg.flakePath;
        message = "services.fetch-rs.flakePath must be an absolute path.";
      }
      {
        assertion = lib.hasPrefix "/" cfg.homeDirRoot;
        message = "services.fetch-rs.homeDirRoot must be an absolute path.";
      }
      {
        assertion = lib.hasPrefix "/" cfg.gitConfigPath;
        message = "services.fetch-rs.gitConfigPath must be an absolute path.";
      }
      {
        assertion = cfg.secretsFile == null || lib.hasPrefix "/" cfg.secretsFile;
        message = "services.fetch-rs.secretsFile must be an absolute path.";
      }
    ];

    systemd.tmpfiles.rules = [
      "d ${configDir} 0750 ${cfg.user} ${cfg.group} -"
    ];

    systemd.services.fetch-rs = {
      description = "fetch-rs service";
      wantedBy = [ ];
      after = [ "network.target" ];
      onSuccess = [ "fetch-rs-rebuild.service" ];
      path = [ cfg.gitPackage ];
      environment = commonEnvironment;

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
          configDir
        ];
        UMask = "0027";
      }
      // lib.optionalAttrs (cfg.secretsFile != null) {
        EnvironmentFile = cfg.secretsFile;
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
      environment = commonEnvironment // {
        GIT_CONFIG_GLOBAL = cfg.gitConfigPath;
      };

      serviceConfig = {
        Type = "oneshot";
        ExecStart = "${cfg.package}/bin/rebuild";
        User = cfg.rebuildUser;
        PrivateDevices = true;
        PrivateTmp = true;
        BindReadOnlyPaths = [
          cfg.flakePath
          configDir
        ];
        UMask = "0027";
      }
      // lib.optionalAttrs (cfg.secretsFile != null) {
        EnvironmentFile = cfg.secretsFile;
      };
    };
  };
}

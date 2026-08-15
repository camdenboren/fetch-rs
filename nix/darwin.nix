{ self }:
{
  config,
  lib,
  pkgs,
  ...
}:

let
  cfg = config.services.fetch-rs;
  defaultGroup = "staff";
  defaultHomeDirRoot = "/Users";
  defaultRebuildUser = "root";
  defaultGitConfigPath = "/var/root/.gitconfig";
  userHome = "${cfg.homeDirRoot}/${cfg.user}";
  configHome = "${userHome}/.config";
  configDir = "${userHome}/.config/fetch-rs";
  commonEnvironment = {
    F_RS_CONFIG = configDir;
    F_RS_FLAKE = cfg.flakePath;
  };
  loadSecretsFile = lib.optionalString (cfg.secretsFile != null) ''
    set -a
    . ${lib.escapeShellArg cfg.secretsFile}
    set +a
  '';

  profileBin = "/run/current-system/sw/bin";
  systemPath = [
    "/usr/bin"
    "/bin"
    "/usr/sbin"
    "/sbin"
  ];
  nativePath = lib.concatStringsSep ":" systemPath;
  fetchPath = "${
    lib.makeBinPath [
      cfg.gitPackage
      pkgs.curl
    ]
  }:${nativePath}";
  rebuildPath = "${
    lib.makeBinPath [
      cfg.gitPackage
      cfg.rebuildPackage
      pkgs.curl
    ]
  }:${nativePath}";
  fetchLabel = "org.nixos.fetch-rs";
  rebuildLabel = "org.nixos.fetch-rs-rebuild";
  fetchService = pkgs.writeShellScriptBin "fetch-rs-service" ''
    set -eu

    ${loadSecretsFile}
    export PATH=${lib.escapeShellArg fetchPath}
    ${cfg.package}/bin/fetch-rs
    exec /usr/bin/sudo -n -- /bin/launchctl kickstart system/${rebuildLabel}
  '';
  rebuildService = pkgs.writeShellScriptBin "fetch-rs-rebuild-service" ''
    set -eu

    ${loadSecretsFile}
    export PATH=${lib.escapeShellArg rebuildPath}
    exec ${cfg.package}/bin/rebuild
  '';
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
      description = "Absolute path to the directory containing your flake-based nix-darwin configuration (required).";
    };

    gitPackage = lib.mkPackageOption pkgs "git" { };

    rebuildPackage = lib.mkOption {
      type = lib.types.package;
      default = config.system.build.darwin-rebuild;
      defaultText = lib.literalExpression "config.system.build.darwin-rebuild";
      description = "The package providing darwin-rebuild.";
    };

    user = lib.mkOption {
      type = lib.types.nonEmptyStr;
      description = "Trusted normal user under which fetch-rs runs. This user must own the Nix configuration (required).";
    };

    group = lib.mkOption {
      type = lib.types.nonEmptyStr;
      default = defaultGroup;
      description = "Group under which fetch-rs runs.";
    };

    rebuildUser = lib.mkOption {
      type = lib.types.nonEmptyStr;
      default = defaultRebuildUser;
      description = "User under which fetch-rs-rebuild runs. nix-darwin activation requires root.";
    };

    homeDirRoot = lib.mkOption {
      type = lib.types.nonEmptyStr;
      default = defaultHomeDirRoot;
      description = "Path prefacing the user's home directory.";
    };

    gitConfigPath = lib.mkOption {
      type = lib.types.nonEmptyStr;
      default = defaultGitConfigPath;
      description = "Path referenced for the rebuild user's Git config.";
    };

    secretsFile = lib.mkOption {
      type = lib.types.nullOr lib.types.nonEmptyStr;
      default = null;
      example = "/etc/fetch-rs.env";
      description = ''
        Absolute path to a shell-compatible secrets file loaded by both fetch-rs services. Set F_RS_NTFY_URL in this file to the complete ntfy topic URL. The file itself is not copied to the Nix store and must be readable by the configured user.
      '';
    };

    startCalendarInterval = lib.mkOption {
      type = lib.types.either lib.types.attrs (lib.types.listOf lib.types.attrs);
      default = {
        Minute = 0;
      };
      defaultText = lib.literalExpression "{ Minute = 0; }";
      example = lib.literalExpression "{ Minute = 15; }";
      description = ''
        launchd calendar interval in which fetch-rs runs. Missing fields are wildcards, so the default runs at the start of every hour.
      '';
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

    # Only unique wrappers enter the system profile, avoiding collisions from
    # the generic `rebuild` binary or custom service dependencies. Keeping the
    # wrapper paths stable also stops routine package updates from changing the
    # plists and unloading the job that is rebuilding them.
    environment.systemPackages = [
      fetchService
      rebuildService
    ];

    system.activationScripts.launchd.text = lib.mkBefore ''
      if [[ ! -d ${lib.escapeShellArg configHome} ]]; then
        /usr/bin/install -d -m 0755 -o ${lib.escapeShellArg cfg.user} -g ${lib.escapeShellArg cfg.group} ${lib.escapeShellArg configHome}
      fi
      /usr/bin/install -d -m 0750 -o ${lib.escapeShellArg cfg.user} -g ${lib.escapeShellArg cfg.group} ${lib.escapeShellArg configDir}
    '';

    # launchd has no equivalent to systemd's OnSuccess. Permit the normal-user
    # fetch job to kickstart only this fixed, root-owned rebuild job.
    security.sudo.extraConfig = lib.mkAfter ''
      ${cfg.user} ALL = (root) NOPASSWD: /bin/launchctl kickstart system/${rebuildLabel}
    '';

    launchd.daemons = {
      fetch-rs = {
        command = "${profileBin}/fetch-rs-service";
        path = [ profileBin ] ++ systemPath;
        environment = commonEnvironment // {
          HOME = userHome;
          LOGNAME = cfg.user;
          USER = cfg.user;
        };
        serviceConfig = {
          Label = fetchLabel;
          UserName = cfg.user;
          GroupName = cfg.group;
          InitGroups = true;
          ProcessType = "Background";
          RunAtLoad = false;
          StartCalendarInterval = cfg.startCalendarInterval;
          WorkingDirectory = cfg.flakePath;
          Umask = 23;
          StandardOutPath = "${configDir}/fetch.log";
          StandardErrorPath = "${configDir}/fetch.log";
        };
      };

      fetch-rs-rebuild = {
        command = "${profileBin}/fetch-rs-rebuild-service";
        path = [ profileBin ] ++ systemPath;
        environment = commonEnvironment // {
          GIT_CONFIG_GLOBAL = cfg.gitConfigPath;
          HOME = "/var/root";
          LOGNAME = cfg.rebuildUser;
          SUDO_USER = cfg.user;
          USER = cfg.rebuildUser;
        };
        serviceConfig = {
          Label = rebuildLabel;
          UserName = cfg.rebuildUser;
          InitGroups = true;
          ProcessType = "Background";
          RunAtLoad = false;
          WorkingDirectory = cfg.flakePath;
          Umask = 23;
          StandardOutPath = "/var/log/fetch-rs-rebuild.log";
          StandardErrorPath = "/var/log/fetch-rs-rebuild.log";
        };
      };
    };
  };
}

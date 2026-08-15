# Building `.#checks.aarch64-darwin.darwinModule` evaluates and builds this configuration.
# CI additionally activates it on an ephemeral macOS runner and exercises both launchd jobs
# via `testScript`
{
  nix-darwin,
  module,
  system,
  activate ? false,
}:

nix-darwin.lib.darwinSystem {
  inherit system;
  modules = [
    module
    (
      { lib, pkgs, ... }:
      {
        system.stateVersion = 5;
        system.primaryUser = "runner";
        nix.enable = !activate;

        services.fetch-rs = {
          enable = true;
          package = import ./package.nix { inherit pkgs; };
          user = "runner";
          flakePath = "/Users/runner";
          secretsFile = "/etc/fetch-rs-test.env";
        };

        system.activationScripts.launchd.text = lib.mkAfter ''
          /usr/bin/printf '%s\n' 'F_RS_NTFY_URL=https://ntfy.invalid/darwin-test' > /etc/fetch-rs-test.env
          /usr/sbin/chown root:staff /etc/fetch-rs-test.env
          /bin/chmod 0640 /etc/fetch-rs-test.env
        '';

        environment.systemPackages = [
          (pkgs.writeShellScriptBin "testScript" ''
            set -euo pipefail
            trap 'cat /Users/runner/.config/fetch-rs/fetch.log || true; sudo cat /var/log/fetch-rs-rebuild.log || true' ERR

            sudo launchctl print system/org.nixos.fetch-rs
            sudo launchctl print system/org.nixos.fetch-rs-rebuild
            sudo launchctl kickstart -k system/org.nixos.fetch-rs

            for _ in {1..30}; do
              if sudo grep -Fq "fetch-rs test rebuild ran" /var/log/fetch-rs-rebuild.log 2>/dev/null; then
                break
              fi
              sleep 1
            done

            test "$(cat /Users/runner/.config/fetch-rs/ntfy-url)" = "https://ntfy.invalid/darwin-test"
            sudo grep -Fq "fetch-rs test rebuild received ntfy URL" /var/log/fetch-rs-rebuild.log
            sudo grep -Fq "fetch-rs test rebuild ran" /var/log/fetch-rs-rebuild.log
          '')
        ];
      }
    )
  ];
}

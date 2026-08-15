{
  nixosModule,
  pkgs,
}:

let
  testPackage = import ./package.nix { inherit pkgs; };
in
pkgs.testers.runNixOSTest {
  name = "fetch-rs-module";

  nodes.machine = {
    imports = [ nixosModule ];

    users.users.fetcher = {
      isNormalUser = true;
      createHome = true;
      group = "users";
    };

    # a normal user will already have these created
    systemd.tmpfiles.rules = [
      "d /home/fetcher/.config 0755 fetcher users -"
      "d /var/lib/fetch-rs-flake 0755 fetcher users -"
      "f+ /run/fetch-rs.env 0640 root root - F_RS_NTFY_URL=https://ntfy.invalid/test"
    ];

    services.fetch-rs = {
      enable = true;
      package = testPackage;
      user = "fetcher";
      flakePath = "/var/lib/fetch-rs-flake";
      secretsFile = "/run/fetch-rs.env";
    };
  };

  testScript = ''
    machine.start()
    machine.wait_for_unit("multi-user.target")

    machine.wait_for_unit("fetch-rs.timer")
    fetch_user = machine.succeed(
        "systemctl show fetch-rs.service --property User --value"
    ).strip()
    assert fetch_user == "fetcher", fetch_user

    owner_mode = machine.succeed(
        "stat -c '%U:%G:%a' /home/fetcher/.config/fetch-rs"
    ).strip()
    assert owner_mode == "fetcher:users:750", owner_mode

    machine.succeed("systemctl start fetch-rs.service")
    ntfy_url = machine.succeed(
        "cat /home/fetcher/.config/fetch-rs/ntfy-url"
    ).strip()
    assert ntfy_url == "https://ntfy.invalid/test", ntfy_url
    machine.wait_until_succeeds(
        "journalctl -u fetch-rs-rebuild.service --no-pager "
        "| grep -Fq 'fetch-rs test rebuild ran'"
    )
    machine.succeed("test -f /home/fetcher/.config/fetch-rs/fetched")
  '';
}

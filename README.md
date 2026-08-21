# fetch-rs

Like [deploy-rs](https://github.com/serokell/deploy-rs), except you just fetch and rebuild the most recent commit

This tool may be generalized via configuration, and even integrates w/ [ntfy.sh](https://ntfy.sh) to enable push notifications on build failure

## Usage

Assuming a fairly standard desktop user setup (e.g., your Nix configuration is editable by your user while rebuilds are relegated to root), I recommend using the included NixOS module

The `user` and `flakePath` attributes are **required** (and enable auto-discovery of your flake repo's owner and name!)

```nix
services.fetch-rs = {
  enable = true;
  user = "camdenboren";
  flakePath = "/home/camdenboren/etc/nixos"
};
```

I provide several user and path related options for other user setups, but YMMV. If you're doing something drastically different from the above, opting for [deploy-rs](https://github.com/serokell/deploy-rs) (or similar) will probably be the right move as it's infinitely more mature

fetch-rs can also be invoked manually via

```shell
nix shell github:camdenboren/fetch-rs --command bash -c "fetch-rs && sudo rebuild"
```

_As `fetch-rs` exits w/ values > 0 for any condition other than "proceed with rebuilding", `rebuild` is only called as needed_

## Implementation

fetch-rs simply grabs the date of the most recent commit on the branch of your choosing (that passed CI builds) and compares it to the date of your current local commit (regardless of the branch). If the remote branch's commit is more recent, it rebuilds your system

If polled somewhat frequently (e.g., via the default hourly systemd timer) then your machines will be kept in sync w/ successfully built `main` commits. Otherwise, you can remain on whatever feature branch you prefer

This tool currently only works on NixOS, but I plan on targeting nix-darwin as well

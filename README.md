# fetch-rs

Like [deploy-rs](https://github.com/serokell/deploy-rs), except you just fetch and rebuild the most recent commit passing CI

This tool may be generalized via configuration, and even integrates w/ [ntfy.sh](https://ntfy.sh) to enable push notifications on build failure

## Usage

Assuming a fairly standard desktop user setup (e.g., your Nix configuration is editable by your user while rebuilds are relegated to root), I recommend using the included NixOS or nix-darwin module

The `user` and `flakePath` attributes are **required** (and enable auto-discovery of your flake repo's owner and name!)

### NixOS

```nix
imports = [ inputs.fetch-rs.nixosModules.default ];
services.fetch-rs = {
  enable = true;
  user = "camdenboren";
  flakePath = "/home/camdenboren/etc/nixos";
};
```

### nix-darwin

> [!NOTE]
> On your first attempted rebuild on nix-darwin, you'll need to grant fetch-rs App Management permissions via the System Settings app (Privacy & Security -> App Management)

> [!NOTE]
> Relatedly, fetch-rs is currently unable to manipulate nix-darwin options requiring Full Disk Access (e.g., `system.defaults.universalaccess.reduceTransparency`)

```nix
imports = [ inputs.fetch-rs.darwinModules.default ];
services.fetch-rs = {
  enable = true;
  user = "camdenboren";
  flakePath = "/Users/camdenboren/etc/nixos";
};
```

### Notifications

To enable notifications, set `notify = true` in `config.toml` and put the complete ntfy topic URL in a secrets file:

```shell
F_RS_NTFY_URL=https://ntfy.sh/my-secret-topic
```

Then point the module at it (e.g., `services.fetch-rs.secretsFile = "/run/secrets/fetch-rs.env";`). The secrets file must be readable by the configured `user`. Pass its path as a string, rather than a Nix path literal, so Nix does not copy it into the store.

## Implementation

fetch-rs simply grabs the SHA of the most recent commit on the branch of your choosing (that passed CI builds) and compares it to the SHA of your current local commit (regardless of the branch). If the SHAs disagree, it rebuilds your system

If polled somewhat frequently (e.g., via the default hourly timer) then your machines will be kept in sync w/ successfully built commits on your preferred branch

### Limitations

The primary limitation of the current approach is that fetch-rs implicitly assumes that your latest local commit matches your system's current configuration revision (i.e., `system.configurationRevision`), which may not be the case for several reasons

I plan on decoupling the management of the local flake repo from the local configuration revision, but that will require additional effort in drawing the associated behavioral boundaries

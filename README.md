# fetch-rs

Like [deploy-rs](https://github.com/serokell/deploy-rs), except you just fetch and rebuild the most recent commit

This tool may be generalized to a degree via configuration (created w/ user input on first run), and even integrates w/ [ntfy.sh](https://ntfy.sh) to enable push notifications on build failure

## Usage

The eventual goal is to automate the usage of this project via system services on both NixOS and Nix-Darwin, but in the meantime, it can be invoked manually via

```shell
nix shell github:camdenboren/fetch-rs --command bash -c "fetch-rs && sudo rebuild"
```

_As `fetch-rs` exits w/ values > 0 for any condition other than "proceed with rebuilding", `rebuild` is only called as needed_

## Implementation

fetch-rs simply grabs the date of the most recent commit on the branch of your choosing (that passed CI builds) and compares it to the date of your current local commit (regardless of the branch). If the remote branch's commit is more recent, it rebuilds your system

If polled somewhat frequently (e.g., via an hourly systemd timer) then your machines will be kept in sync w/ successfully built `main` commits. Otherwise, you can remain on whatever feature branch you prefer

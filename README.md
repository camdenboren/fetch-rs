# fetch-rs

Like deploy-rs, except you just fetch and rebuild the most recent commit

This tool may be generalized to a degree via configuration (created w/ user input on first run), and even integrates w/ ntfy.sh to enable push notifications on build failure

## Implementation

fetch-rs simply grabs the date of the most recent commit on `main` (that passed CI builds) and compares it to the date of your current local commit (regardless of the branch). If the `main` commit is more recent, it rebuilds your system via nh

If polled somewhat frequently (e.g., via an hourly systemd timer) then your machines will be kept in sync w/ successfully built `main` commits. Otherwise, you can remain on whatever feature branch you prefer

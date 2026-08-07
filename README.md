# fetch-rs

Like deploy-rs, except you just fetch and rebuild the most recent commit

This is currently unusable for anyone other than myself, but I might generalize it in the future via configuration. I'm also planning on integrating w/ ntfy.sh to enable push notifications

## Implementation

fetch-rs simply grabs the date of the most recent commit on `main` (that passed CI builds) and compares it to the date of your current local commit (regardless of the branch). If the `main` commit is more recent, it rebuilds your system via nh

If polled somewhat frequently (e.g., via an hourly systemd timer) then your machines will be kept in sync w/ successfully built `main` commits. Otherwise, you can remain on whatever feature branch you prefer

I currently default to doing nothing when errors occur, but this behavior may change as the project matures

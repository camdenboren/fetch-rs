{ pkgs }:

pkgs.symlinkJoin {
  name = "fetch-rs-test-package";
  paths = [
    (pkgs.writeShellScriptBin "fetch-rs" ''
      set -eu

      test -d "$F_RS_FLAKE"
      test -d "$F_RS_CONFIG"
      command -v ssh >/dev/null
      : > "$F_RS_CONFIG/fetched"
      printf '%s\n' "''${F_RS_NTFY_URL-}" > "$F_RS_CONFIG/ntfy-url"
      printf '%s\n' "fetch-rs test fetch ran"
    '')
    (pkgs.writeShellScriptBin "rebuild" ''
      set -eu

      test -d "$F_RS_FLAKE"
      test -f "$F_RS_CONFIG/fetched"
      command -v ssh >/dev/null
      case "''${F_RS_NTFY_URL-}" in
        https://ntfy.invalid/test|https://ntfy.invalid/darwin-test) ;;
        *) exit 1 ;;
      esac
      printf '%s\n' "fetch-rs test rebuild received ntfy URL"
      printf '%s\n' "fetch-rs test rebuild ran"
    '')
  ];
}

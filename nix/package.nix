{ pkgs }:

{
  default = pkgs.rustPlatform.buildRustPackage {
    pname = "fetch-rs";
    version = "0.1.0";
    src = ../.;
    cargoHash = "sha256-xzGKOrKmlD4DKOfISdkQ8s5jfUj+TdgUGMMyG3Lhm04=";
  };
}

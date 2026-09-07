{ pkgs }:

{
  default = pkgs.rustPlatform.buildRustPackage {
    pname = "fetch-rs";
    version = "0.1.0";
    src = ../.;
    cargoHash = "sha256-tMM8Onvjp3csO4NoAOtkbeL2czfSgk7ZY1LtC0U6D4c=";
  };
}

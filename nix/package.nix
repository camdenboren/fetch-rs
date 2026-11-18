{ pkgs }:

{
  default = pkgs.rustPlatform.buildRustPackage {
    pname = "fetch-rs";
    version = "0.1.0";
    src = ../.;
    cargoHash = "sha256-6t1KsAoZFCNyHZ9mCRHW/5u1Sm94ZVhmk/+1brj4TZc=";

    # link aws-lc-sys against nixpkgs' AWS-LC instead of building/
    # vendoring its own
    buildInputs = [ pkgs.aws-lc ];
    nativeBuildInputs = [ pkgs.pkg-config ];
  };
}

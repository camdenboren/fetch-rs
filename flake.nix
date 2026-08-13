{
  description = "fetch-rs Development Environment via Nix Flake";

  nixConfig.bash-prompt = ''\n\[\033[1;31m\][devShell:\w]\$\[\033[0m\] '';

  inputs = {
    nixpkgs = {
      url = "github:nixos/nixpkgs/nixos-unstable";
    };
  };

  outputs =
    { nixpkgs, ... }:
    let
      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
      ];
      forEachSupportedSystem =
        function:
        nixpkgs.lib.genAttrs supportedSystems (
          system:
          function {
            pkgs = nixpkgs.legacyPackages.${system};
          }
        );
    in
    {
      devShells = forEachSupportedSystem (
        { pkgs }:
        {
          default = pkgs.mkShell {
            packages = with pkgs; [
              rustc
              cargo
              rust-analyzer
              rustfmt
              clippy
            ];

            shellHook = ''
              echo -e "\nfetch-rs Development Environment via Nix Flake\n"

              echo -e "┌───────────────────────────┐"
              echo -e "│      Useful Commands      │"
              echo -e "├────────┬──────────────────┤"
              echo -e "│ Init   │ cargo init       │"
              echo -e "│ Run    │ cargo run        │"
              echo -e "│ Check  │ cargo check      │"
              echo -e "│ Test   │ cargo test       │"
              echo -e "│ Clippy │ cargo clippy     │"
              echo -e "│ Format │ rustfmt fileName │"
              echo -e "└────────┴──────────────────┘"
            '';
          };
        }
      );

      packages = forEachSupportedSystem (
        { pkgs }:
        {
          default = pkgs.rustPlatform.buildRustPackage {
            pname = "fetch-rs";
            version = "0.1.0";
            src = ./.;
            cargoHash = "sha256-4MWfKflB6i07iiQs7IgIMcHmvBTuYHK//eHbmmzc2XE=";
          };
        }
      );
    };
}

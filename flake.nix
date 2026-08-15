{
  description = "fetch-rs Development Environment via Nix Flake";

  nixConfig.bash-prompt = ''\n\[\033[1;31m\][devShell:\w]\$\[\033[0m\] '';

  inputs = {
    nixpkgs = {
      url = "github:nixos/nixpkgs/nixos-unstable";
    };
    nix-darwin = {
      url = "github:nix-darwin/nix-darwin";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      nix-darwin,
    }:
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
      devShells = forEachSupportedSystem (import ./nix/shell.nix);
      packages = forEachSupportedSystem (import ./nix/package.nix);
      nixosModules = rec {
        default = fetch-rs;
        fetch-rs = import ./nix/nixos.nix { inherit self; };
      };
      darwinModules = rec {
        default = fetch-rs;
        fetch-rs = import ./nix/darwin.nix { inherit self; };
      };
      checks = forEachSupportedSystem (
        { pkgs }:
        nixpkgs.lib.optionalAttrs pkgs.stdenv.isLinux {
          nixosModule = import ./nix/tests/nixos.nix {
            inherit pkgs;
            nixosModule = self.nixosModules.default;
          };
        }
        # darwin testing approach from https://github.com/Mic92/fast-nix-gc
        // nixpkgs.lib.optionalAttrs pkgs.stdenv.isDarwin {
          darwinModule =
            (import ./nix/tests/darwin.nix {
              inherit nix-darwin;
              module = self.darwinModules.default;
              system = pkgs.stdenv.hostPlatform.system;
            }).system;
        }
      );
      darwinConfigurations.ci = import ./nix/tests/darwin.nix {
        inherit nix-darwin;
        module = self.darwinModules.default;
        system = "aarch64-darwin";
        activate = true;
      };
    };
}

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

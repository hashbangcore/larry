set unstable

fmt:
    just --fmt
    cargo fmt
    nixfmt flake.nix

commit hint="":
    netero commit {{ hint }} | git commit --edit -F -

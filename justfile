set unstable

fmt:
    just --fmt
    cargo fmt
    nixfmt flake.nix

hint := ""
commit *files="":
    gareth repo commit {{ if hint != "" { "--hint " + quote(hint) } else { "" } }} {{ files }} | git commit {{ files }} -F - --edit

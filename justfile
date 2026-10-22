install:
    cargo install --path .

hint := ""
commit *files="":
    gareth repo commit {{ if hint != "" { "--hint " + quote(hint) } else { "" } }} {{ files }} | git commit {{ files }} -F - --edit

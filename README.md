# Larry

Larry is a small Unix-style wrapper that
executes commands and captures pipeline input,
emitting a structured, concatenated text output
with labeled sections.

Its goal is observability, not orchestration.

Larry does not try to infer, reconstruct, or guess commands it cannot know.
It prints exactly what it receives and executes.

---

## Installation

### Via Nix profile

```bash
nix profile add github:hashbangcore/larry
```

To update:

```bash
nix profile upgrade larry
```

### Declarative (NixOS)

Add the input to your `flake.nix`:

```nix
{
  inputs.larry.url = "github:hashbangcore/larry";

  outputs = { self, nixpkgs, larry, ... }@inputs: {
    nixosConfigurations.my-machine = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      specialArgs = { inherit inputs; };
      modules = [ ./configuration.nix ];
    };
  };
}
```

Then add to your `configuration.nix`:

```nix
{ config, pkgs, inputs, ... }:
{
  environment.systemPackages = [
    inputs.larry.packages.${pkgs.system}.default
  ];
}
```

### From source

```bash
git clone https://github.com/hashbangcore/larry.git
cd larry
cargo build --release
```

---

## Usage

```
Capture piped stdin and execute commands, producing structured output with [section] markers.

Usage: larry [OPTIONS] [COMMAND]...

Arguments:
  [COMMAND]...  Commands to execute via sh -c

Options:
  -r, --repeat <N>  How many times to repeat the corresponding command
  -v, --version     Show version and exit
  -h, --help        Print help (see more with '--help')
```

A repeat count defaults to 1, accepts up to 100, and 0 repeats the command
forever with a 5 second pause between runs.

Each command and each repetition produces its own [section] in the output.
Use quotes to pass flags to commands: larry "ls -la"

Repeat counts are paired with commands in the order they appear, so
`larry -r 2 pwd -r 3 date` runs `pwd` twice and `date` three times.

---

## What Larry Does

Larry accepts:

* Standard input (stdin) from a pipeline
* One or more commands passed as arguments

It then:

1. Captures any incoming stdin
2. Executes each provided command
3. Produces a single, concatenated text stream

This makes Larry ideal for:

* System inspection
* Diagnostics
* Tooling pipelines
* LLM-assisted analysis
* Retrying flaky commands

---

## Output Format

Larry emits a simple, explicit output format:

```text
[section]
[stdin]
<text received from stdin>
[end section]

[section]
[command]
<command string>
[output]
<command stdout + stderr>
[end section]
```

Each `[section]` block is self-contained, making it easy to parse programmatically
or feed into LLMs.

---

## Examples

### Basic command execution

```bash
larry pwd whoami
```

Output:

```text
[section]
[command]
pwd
[output]
/home/user/work/larry
[end section]

[section]
[command]
whoami
[output]
user
[end section]
```

---

### Using stdin (pipeline input)

```bash
ls -a | larry pwd "echo $USER"
```

Output:

```text
[section]
[stdin]
.
..
.git
Cargo.toml
src
[end section]

[section]
[command]
pwd
[output]
/home/user/work/larry
[end section]

[section]
[command]
echo $USER
[output]
user
[end section]
```

---

### Repeat a command multiple times

```bash
larry -r 3 whoami
```

Output (3 sections, one per execution):

```text
[section]
[command]
whoami
[output]
user
[end section]

[section]
[command]
whoami
[output]
user
[end section]

[section]
[command]
whoami
[output]
user
[end section]
```

Each repetition produces its own independent section,
useful when results may vary between runs.

---

### Retry on failure (e.g. flaky network)

```bash
larry -r 3 "curl -s https://example.com"
```

If the first attempt fails due to a network hiccup,
the next attempt runs automatically.

```bash
larry -r 3 "nixos-rebuild switch --flake ."
```

Same idea — useful for builds interrupted by transient errors.

---

### Mixed repeat counts

```bash
larry -r 4 pwd -r 2 "echo hello"
```

This runs `pwd` 4 times, then `echo hello` 2 times.

---

### Verify command equivalence

These two produce identical output:

```bash
larry ls ls pwd
larry -r 2 ls pwd
```

Because `-r 2 ls` is equivalent to passing `ls` twice.

---

### Ping with multiple repeat counts

```bash
larry -r 2 "ping -c5 8.8.8.8" "ping -c10 1.1.1.1"
```

This runs `ping -c5 8.8.8.8` twice and `ping -c10 1.1.1.1` once.

---

### Infinite loop (monitoring)

```bash
larry -r 0 "date"
```

Repeats `date` every 5 seconds until interrupted with Ctrl-C.
Useful for watching changing state.

---

### LLM-assisted diagnostics

```bash
lsblk | larry df free | llm "analyze the system state"
```

Larry outputs structure; the LLM interprets.

---

## Why Larry Exists

The idea for this wrapper emerged while I was working on [netero](https://github.com/hashbangcore/netero).

During that project, I needed LLMs to be able to observe the output of multiple commands,
especially when they are chained together in pipelines. I initially explored
similar solutions using aliases and Bash functions,
which only solved the problem partially.

## License

BSD 0-Clause

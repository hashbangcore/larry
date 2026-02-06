# Larry

Larry is a small Unix-style wrapper that
executes commands and captures pipeline input
emitting a structured, concatenated text output
designed to be easily consumed by humans and
Large Language Models (LLMs).

Its goal is observability, not orchestration.

Larry does not try to infer, reconstruct, or guess commands it cannot know.
It prints exactly what it receives and executes.

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

---

## Output Format

Larry emits a simple, explicit output format:

```text
[stdin output]
<text received from stdin>

[command]
<command string>
[output]
<command stdout + stderr>
```

---

## Examples

### Basic command execution

```bash
larry "pwd" "whoami"
```

Output:

```text
[command]
pwd
[output]
/home/user/work/larry

[command]
whoami
[output]
user
```

---

### Using stdin (pipeline input)

```bash
ls -a | larry pwd "echo $USER"
```

Output:

```text
[stdin output]
./
../
.git/
Cargo.toml
src/

[command]
pwd
[output]
/home/user/work/larry

[command]
echo user
[output]
user
```

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

However, beyond that functional need, I also had another goal: learning Rust.
Separating this functionality into a small, standalone tool turned out to be 
a good excuse to implement it in Rust, while keeping the design simple and reusable.

## License

BSD 0-Clause

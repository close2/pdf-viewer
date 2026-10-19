# 2044 — A command under the wrapper reads its caller's standard input

Session 1604, slot 6 of batch ninety-five. Status: **accepted** and **built** in `tools/bounded.sh`.
Context: trap 207 (round 1596's census read an empty pipe and reported 0 of 0 files); ADR 1674 (the
subshell that keeps the lanes' descriptors); ADR 1710 (`--build` inside the hold).

## 1. The cause, read in bash rather than in the wrapper

The wrapper starts its command inside `( … ) &`, so that `watch_tree` can sample the tree while it
runs. Bash gives an asynchronous list `/dev/null` for its standard input where job control is off,
which it is in every script, unless the list redirects its input itself. Nothing in the wrapper read
the caller's input or closed it: the command was handed `/dev/null` by the shell. Reproduced at HEAD:
`printf 'piped\n' | tools/bounded.sh --tree 1 -- cat` printed nothing and exited 0.

## 2. Decision

- **The input passes through**: the subshell is started as `( … ) <&0 &`, so the command reads what
  its caller gave the wrapper, a pipe, a file or a terminal. Of the brief's two remedies this is the
  one that makes the piped probe correct rather than refused; a refusal of a non-terminal input
  would also have refused `< /dev/null` and every redirect from a file, which are not mistakes.
- **A `--build` reads `/dev/null`**, so a piped input reaches the command whole and no build can take
  a byte of it.
- **The self-test holds it** (case 15): a two-line pipe into the wrapper is the two lines `cat`
  prints, in order. Planted with the subshell's `<&0` taken out, the case fails with
  `stdin: the command read "" of a two-line pipe` and the self-test exits 1.

## 3. What this does not change

A run started by `tools/wait.sh --start` is detached and reads `/dev/null` by design (ADR 2025), so a
pipe into `wait.sh --start … -- tools/bounded.sh …` still reaches nothing; a run long enough to need
the wait takes its input as a file argument, as trap 207's rule already says. The trap stands: a probe
that can answer "nothing found" is first shown something it must find, wherever its input comes from.

# 2011 — A wait takes its options on either side of its log, and refuses anything else in a sentence

Session 1587, slot 3 of batch ninety-three. Status: **accepted** and **built** in `tools/wait.sh`.
Context: ADRs 1976 and 1977 (a long command is waited for in calls inside the cache's lifetime),
`doc/history/1583` "Found, not fixed".

## 1. What went wrong

`tools/wait.sh <log> --wall 30` stopped reading options at the log. It then found three words where
it wanted one, printed its header and exited 64. The header is the script's usage text: forty lines
that look like output. A round reading the call's tail took that for a wait that had been made and
went on. Round 1577 lost several waits this way.

## 2. Decision

- **Every word that is not an option is a log, wherever it stands**, so `<log> --wall 30` waits as
  `--wall 30 <log>` does. After `--start <log>` the first such word still begins the command, as it
  always did.
- **Anything else is refused on one line that starts `wait:`**, with exit 64. That covers two logs,
  a log beside `--start` or `--stop`, a command without `--start`, and an option missing its value.
  The line names the form the caller meant. The usage text is printed only for `-h`.
- `--self-test` carries the three cases. Planted against the previous script, the first of them
  exits 64 where the new one waits.

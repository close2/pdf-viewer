# 2120 — A window's settings are read once in `viewer-host`, and cross the C ABI as a handle

Session 1642, batch one hundred's slot 4, a host round. Status: **accepted** and **built**.
Context: ADRs 0247 (the ABI's shapes and what it refused), 1144, 1155, 1227, 1331, 1527, 1228 (the
words), 1581 (`ReaderWords`, the precedent), 2088 (the window opened at the defaults), 2092 and habit
131; RFC 0010 section 5.

## 1. What was copied

`quorra`, `quorra-gtk`, `quorra-qt` and `quorra-confined` each matched the same prefixes —
`--restrictions=`, `--ignore-restrictions`, `--links=`, `--remote-documents=`,
`--embedded-documents=`, `--submissions=`, `--separations=` — and each called the same parser per
word: four copies of one decision, `quorra-confined` taking three of the seven and reading the other
four as a document's path. `quorra_window` had no way to be told any of them and opened at
`viewer_host::Settings::default()`. The brief's `--scripts=` does not exist: `--scripts` takes the
word after it and `ReaderWords` reads it with the reader's other words, so it is left there.

## 2. Decisions

1. **`viewer_host::Settings::take(word)`** reads the seven words onto the settings and answers which
   constant it was (`viewer_host/src/policy/settings.rs`); each parser's sentence is unchanged. The
   four binaries call it; each keeps `scripts` from `ReaderWords`. `quorra-confined` refuses by name
   the four it has no level for, where it used to take the first as a path and print its usage.
2. **`Settings::set(Policy, Level)`** is the reading for a host with no command line: six policies —
   a link, a remote document, an embedded one, a submission, the scripts and the simulation — on one
   scale, `Refuse`, `Ask`, `Warn`, `Proceed`, which is each policy's own `ALL` in its order, so the
   word a person reads stays the policy's. The simulation takes the two ends and refuses the middle.
3. **The restrictions are not a `Policy`**: they are what a document asserts over its reader, the
   other direction `LINKS`' documentation draws, and they already cross as `QUORRA_RESTRICTED_*` and
   `QUORRA_RESTRICT_*`, which `quorra_settings_restrict` takes as `quorra_restrict_operation` does.
4. **A handle, not a struct** (ADR 0247 section 4's last row): `quorra_settings_new`, `_free`,
   `_set`, `_restrict`, `_take` (the command-line word, so a .NET host with a command line takes the
   same words with the same refusals) and `_refusal` (the sentence), and
   `quorra_window_open_under`, which takes a copy. `QUORRA_POLICY_*` (6) and `QUORRA_LEVEL_*` (4) are
   argument enumerations and refuse an unknown number with `QUORRA_WRONG_KIND`; `QUORRA_LEVEL_*` are
   `QUORRA_SCRIPTS_*`' numbers. `quorra_window_open` is unchanged and opens at the defaults.

## 3. Refused

- **One entry point per policy** (`quorra_window_links` and five more): six functions saying one
  thing, and a seventh for the next policy, where a constant is free (ADR 0247 section 1's cost
  table, a new argument value).
- **The words alone** (`quorra_settings_take` with no numbers): a settings page would build a string
  to be parsed back, and a word misspelt is found at run time rather than by the compiler.
- **A `quorra_settings` struct by value**: the one change ADR 0247 says a compiled caller cannot
  survive unknowing.

## 4. Costs

Seven entry points and ten constants; one enum pair in `viewer-host` the ABI maps by number.

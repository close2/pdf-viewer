# 1616 — The `Scripts` level in every window, and the runner a host supplies

Status: accepted and **built**. Session 1390. Builds on RFC 0008 sections 6.3, 6.6 and 11 item 3
(e) (accepted, `doc/questions/A193`), ADR 1591 (the policy hook is `ViewState::run_scripts_with`),
ADR 1602 (one realm per document), ADRs 1608 and 1609 (the worker), ADR 1581 (one reading of the
reader's words) and ADR 1155 (a machine act's levels run permissive-last).
Code: `crates/viewer-host/src/policy.rs` (`Scripts`, `SCRIPTS`, `scripting`, `Warning`,
`asked_to_run_scripts`, `SCRIPTS_PINNED`), `restriction.rs` (`Act::RunningScripts`, `told`),
`reader.rs` (`--scripts`), `crates/viewer-core/src/scripting.rs`, `viewer.rs`
(`supply_scripts`, `say_what_scripts_said`, `ask_about_scripts`), `crates/pdf-model/src/view/scripts.rs`
(`recalculate_with_runner`), `crates/viewer-confined/src/protocol.rs`, the three windows' question
arms, `crates/viewer-ui/src/bin/quorra-confined.rs`, `crates/viewer-ffi/src/kinds.rs`.
Tests: `crates/viewer-core/tests/script_levels.rs`, `viewer-host`'s `restriction` and `reader`
tests; the drive's `scripts_levels` step in `tools/drive-windows.sh`.

## 1. The level, and where it is read

`viewer_host::Scripts` is RFC 0008 section 6.3's four, spelled as the RFC spells them — `off`,
`ask`, `warn`, `on` — in ADR 1155's direction, permissive last. **`off` is the default**, the
owner's answer to the RFC's first question. It is global, like `Submissions`; what *ask* asks is
per document. It is set by `--scripts <level>`, read by `ReaderWords` for all four windows (ADR
1581), and by a third act under the menu's machine group, `Act::RunningScripts`, in all three
menus. `viewer_host::scripting` is the one place a level becomes what the viewer is handed, and
`viewer_host::told` is the one place a menu choice becomes a command: the other two acts are the
host's own and send the viewer nothing, while a script runs inside the view state the viewer holds.

## 2. What the viewer is handed: a maker of runners, one runner per document

`Command::Scripts(Scripting)`, with `Scripting::Off`, `Ask(Arc<dyn ScriptRunners>)` and
`Run(..)`. A runner keeps one document's realm (ADR 1602), so a host supplies a maker and every
document is handed a runner of its own when it opens, and every open document when the level is
sent again. `off` hands `None` — no engine, no process, every script reported not run, exactly as
before. `warn` and `on` are both `Run`: `warn`'s difference is what the runner says, so
`viewer_host::Warning` wraps the worker and adds one sentence per run. **A runner arriving late runs
what it missed**: the open sequence again where it has run, then Table 224's `/CO` over the values
already there, through `ViewState::recalculate_with_runner` — a five-line addition to round 1389's
file, named in this round's report.

## 3. *Ask*: one question at the first script, and an answer that holds

Until a document is answered its view state holds `scripting::Withheld`, a runner that runs nothing
— `rc` true, no value, no edit — says it is waiting, and remembers the first script it was handed.
After the command that handed it one, the viewer sends `Event::AskingToRunScripts` with that
script's name and first line, once; `Command::AnswerScripts` answers. A `yes` hands the document a
real runner and runs what was withheld (section 2); a `no` keeps `Withheld` in its declined form
until the document closes. Either holds when the level is sent again. **The cost, written down**:
the trigger that raised the question does not run, and a keystroke or format script withheld before
the answer is not replayed — the next one runs. What a `yes` recomputes is what can be recomputed
from state: the document-level scripts and the calculations. A face that cannot ask answers `no`.

## 4. Every script's sentence goes out, after every command

The view state kept every script sentence and the viewer sent only the open sequence's; a commit's
`/CO` walk, a keystroke and a format said nothing. `say_what_scripts_said` sends what is new after
every command, per document, so the *off* level's sentence — Tier 0's naming of the script it does
not run — reaches the window, and so do `warn`'s and *ask*'s. Table 200's moments, which round 1389
built (ADR 1614), are called from the viewer at the close, around a save and around a print, and a
script's `setFocus` (ADR 1615) is carried out after each command, held to four hops.

## 5. The runner is always built; the engine is never in a window

**No feature on the host crates.** `viewer-host` depends on `pdf-script-worker` without `engine`:
the host's half is the wire and a client that spawns a program, and links no engine. A feature would
make `--scripts on` a word one build obeys and another refuses — ADR 1581's levelness broken — and
the engine linked into a window would be untrusted text evaluated in the unconfined host, which
principle 3 forbids. **What `tools/batch.sh install` must carry**: the worker program, built in a
cargo run of its own so that feature unification puts no engine in a window — `cargo build
--release -p pdf-script-worker --features engine --bin pdf-script-worker` — and installed beside the
windows, where `confined_transport::program_beside_executable` finds it; `install_featured` is that
line (ADR 1625). Without it, the first trigger at `on` says the worker was not found and how to
build it.

## 6. `quorra-confined` is pinned to `off`, and says so

Its viewer is in its worker, and a runner is a process the window would have to start and hand
across: the wire carries `Scripting::Off` and refuses the other two as `Uncarried`. `--scripts`
other than `off` is said with `SCRIPTS_PINNED` and set to `off`. The C ABI's sessions supply no
runner either; its event kind 22 is named for a caller's default arm and never sent.

## 7. `quorra`'s card takes keys before a field

*Ask* is put at a commit, as the focus moves into the next field, so the question card can be up
while a field holds the keyboard. A question is modal: it now takes every key first, and the typing
resumes once it is answered.

## 8. Driven

`scripts_levels` under Xvfb, in all four windows: `off`, `on`, `ask` answered `yes` and `no` in
`quorra`, `quorra-gtk` and `quorra-qt`, each witnessed by the saved file (`12.5 7 None` at `off` and
after a `no`, `12.5 7 19.5` drawn `$19.50` at `on` and after a `yes`) and the window's own line; the
confined window pinned. Thirteen verdicts, all `works`, about 6.5 s each; and ADR 1617's push-button, drawn red by its
script at `on` and grey at `off` in the three windows.

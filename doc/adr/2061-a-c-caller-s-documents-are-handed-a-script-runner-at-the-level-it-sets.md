# 2061 — A C caller's documents are handed a script runner at the level it sets

Session 1612. Status: **accepted** and **built**. Context: RFC 0008 section 6.3, ADRs 1616 and
1702 (the C ABI's sessions supplied no runner), RFC 0010 section 3.1, `CLAUDE.md`'s JavaScript
exclusion and its four levels.

## 1. The question

RFC 0010 section 3.1 priced `Scripts`, `AnswerScripts` and `AnswerScript` as needing "a runner
made from `pdf-script-worker` spawned beside" the library, and until then a C host pinned
`Scripting::Off`. ADR 1616 and ADR 1702 wrote the pin into `kinds.rs` and the header: the script
kinds were "sent to no caller of this ABI".

## 2. Decision

**`quorra_scripts` takes `viewer_host`'s four levels and makes the runner where every window
makes it**, `viewer_host::policy::scripting`. The library is a C caller's host, as it is for a
conversion (ADR 1954), so it supplies what a window supplies: at `ask`, `warn` and `on` each
document is handed a `pdf_script_worker::ScriptWorker`, which starts the worker program at the
document's first trigger and never before, found beside the running executable or where
`PDF_SCRIPT_WORKER` names it. `off` is the default and constructs nothing, so a caller that never
calls this is unchanged.

The confinement is the worker's, the same process every window starts; nothing is evaluated in
the caller's process, and every call that leaves the process is refused by name at every level
(the JavaScript exclusion). A caller that ships no worker beside its executable is told "scripts
stopped running for this document" and why, as a window is.

## 3. Consequences

The five script kinds (22–25, 27) now reach a C caller at a level it chose, and the header and
`kinds.rs` say so instead of "sent to no caller"; `quorra_timer_due` answers a timer a script set.
The C program sets `ask` and `off` and answers a document that is not asking; the session's tests
show the open script asked about at `ask` and not at `off`. A worker driven from C is the .NET
host's to show (RFC 0010 N3a); `tools/drive-windows.sh` drives the four windows, which this does
not touch.

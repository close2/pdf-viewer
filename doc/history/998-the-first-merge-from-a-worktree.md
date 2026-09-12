# 998 — The first merge from a worktree, and what the layout taught in one afternoon

Date: 2026-09-12. Rounds merged: 992 (RFC 0007's configuration, sites and the first departure,
ADRs 1012/1018), 993 (the date check on the ISO 8601 working draft, ADR 1013), 994 (the third
amendment to the authoring exclusion and RFC 0007 accepted, ADR 1014), 995 (six ungated walks into
the sequence and the population checked, ADR 1015), 996 (a golden of our own output held by name,
ADR 1016), 997 (the kind of an alpha carried where it is decided, ADR 1017). The first batch run
in a worktree (`/home/AI/pdf-viewer-rounds`, branch `batch-992-997`) and fast-forwarded into
`main`, which is the working branch since session 991.

Files the merge itself changed: `doc/todo/02-every-round.md` and `tools/state.sh` (996's gate line
and `section_golden`, and `run()` now prints `✗ the gate exited N` beside a failing section — a
forty-seven-section run had returned 101 with no line saying where), `crates/pdf-model/src/signature.rs`
(the excuse line 995's census asked for), `crates/pdf-vfs-ffi/tests/the_kio_worker.rs` (a CMake
build directory per checkout — the shared build root held a cache baked with the main checkout's
source path and refused the worktree's, trap 15's shape one tool over), `doc/todo/23` (ADR 1017's
closure recorded in the two paragraphs that had said what would close it).

**What the layout taught.** A worktree needs every gitignored resource of the main checkout —
the two submodules, the four corpora inside `doc/corpora/` (themselves submodules, empty until
linked), `doc/md/`, `doc/pdfa/`, the specification PDFs, `corpus-cache`, `tmp` — and the honest
way to find that list is `git ls-files --others --ignored --exclude-standard --directory` in the
main checkout, minus the three it must *not* carry (`.claude`, `fuzz/__pycache__`, and a token
file). The pointer sweep does not resolve paths through those links, so it reports the linked
documents as absent from a worktree; the count settles on `main`. The shared build directory is
warm and right for Cargo and wrong for anything that bakes a source path, which was one program.
And a monitor that watches the pid `$!` of a `setsid` wrapper watches nothing — the script's own
pid is what `pgrep -f "bash .*script.sh"` returns.

Six rounds were cut off by a session limit at their first step and relaunched with the same
briefs; one (994) had already written its amendment paragraph and was relaunched on top of it,
and it declined a correction I had briefed from a stale reading of `A58` — the owner's own session
had rewritten the answer files into the header-plus-verbatim form, and the round quoted the file
at HEAD. That was the right refusal.

The merged sequence, run whole: fmt (both) clean after one indentation fix, clippy --workspace
--all-targets under `-D warnings` exit 0, nextest 4424 passed, doctests exit 0, both fuzz lines
exit 0, conformance exit 0, `tools/state.sh` whole exit 0 on its second run (the first returned
101 from a section that passed alone and printed nothing — hence the marker), pointers 193 absent
in the worktree (187 on `main` plus the linked resources the sweep cannot see plus one retired
probe), quotations 49 / 5 diverging unchanged.

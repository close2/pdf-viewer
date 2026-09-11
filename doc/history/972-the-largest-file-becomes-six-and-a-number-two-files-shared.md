# 972 — The largest file in the reading set becomes six, and a number two files shared

Date: 2026-09-11. ADRs: 0983, 0993.
Files: `doc/habits.md` and the six new `doc/habits/*.md`, `doc/todo/02-every-round.md`,
`doc/todo/README.md`, `doc/HANDOVER.md`, `tools/round.sh`, `doc/running-the-viewer.md`,
`doc/state-of-play.md`, `doc/verify.md`, `doc/todo/36-a-frame-every-refresh.md`, and the
retrieval-API item, renamed out of `doc/todo/36` into `doc/todo/63-a-retrieval-api.md`.
`CLAUDE.md` was read and not changed. Nothing under `crates/` was touched.

This is the fifth round over the documents every round reads (ADRs 0232, 0281, 0428, 0974), and the
first whose job was the two *structural* proposals the fourth wrote down and would not take.

**`doc/habits.md` is six files now and an index of forty-five lines.** Each section
became `doc/habits/<subject>.md` in `doc/traps/`'s style, verbatim; the check that nothing was
tidied while it was carried is mechanical — every non-blank line of the old file, sorted, against
every non-blank line of the six, sorted, with the six section headings and the six new header
blocks as the only difference. The six `###` headings stay headings in the index, because
`doc/habits.md` is cited **235 times from 162 files** and several of those citations name a section
by its *title* — which is ADR 0232 §2's rule, and the reason the fourth compaction did not do this.
`tools/round.sh` now names the file a round actually opens for four of its ten kinds.

**§2's change → gate map is a rule and then a lookup.** Six numbered rules, then the table under a
heading a round that got its answer never reaches. Five of the six were already in the section, in
three different places, including two of the table's own rows — the seven crates under everything,
and the documents-only row — which are rules 2 and 6 now with every clause of their text kept. Rule
2 keeps the phrase *first row* in it deliberately: about fifteen files under `doc/history/` record a
gate choice as "`pdf-model` is the change→gate map's first row", and a restructuring that made that
idiom unfindable would cost more than the ordering buys.

**The corpus is longer, and that is what a reordering costs**: 6307 → 6438 lines, 84 210 → 85 395
words. Two duplications came out under ADR 0974's heading 1 — `doc/running-the-viewer.md`'s second
copy of §5's session-142 incident, and §2's opening restating the rules it points at — and one
session number came out under heading 3. **A round that reported this as a compaction would be
measuring the wrong thing**, which is the fourth compaction's own lesson in the other direction: it
found an index that had doubled in words while its line count stood still.

**One of the three live duplicate todo numbers is resolved and two are not, and the reason is the
finding.** `36` went to the frame-cadence item, on evidence rather than taste: it is cited by ten
ADRs, two documents and — the half that decides it — by every one of the four citations of
`doc/todo/36` written from `crates/`. The retrieval API is `doc/todo/63` and says in its own header
what it used to be called. **`46` and `47` could not be moved at all**: all four files are cited *by
full filename* from ADRs 0766, 0767 and 0770 and from four `doc/history/` files, none of which is
ever edited to follow a file that moved underneath it — so a rename would turn live pointers into
absent ones in files nothing may repair, permanently, to fix an ambiguity a reader resolves from the
sentence in front of them. `doc/todo/README.md` carries a four-row disambiguation by *subject*
instead; the two subjects in each pair are disjoint.

**And each of the three numbers has a third referent that is deleted**, which is ADR 0730's defect
rather than this one: eight ADRs cite a `doc/todo/47` that was the cold document-wide search.
`doc/todo/README.md` now states the *sweep* rather than the answer — `ls doc/todo/ | grep -oE
'^[0-9]+' | sort | uniq -d` — because the round that found these found two by reading and the third
only once that command was run.

**A decayed claim, corrected rather than dropped**: `doc/todo/README.md` said "The number prefix
**is** the priority", and `60`, `61` and `62` have carried `Priority: 50-band` in their own headers
since the 50s filled. It says *wherever the band has room* now, and the paragraph under the table
says what takes over when it does not.

**Kept, with the reasons in ADR 0983**: every habit; the whole of §2's remaining seven lookup rows;
`doc/running-the-viewer.md`'s key-by-key paragraphs; and the four *ledger* habits sitting at the end
of *Measuring*, which the split made visible for the first time — a 392-line section has no visible
end and a file does — and which are written up in `doc/habits.md` for whoever takes the move rather
than moved here on this round's judgement.

**One habit was handed over and placed.** ADR 0984 §5 flagged a lesson for this session by name: a
quotation in a ledger note is checked by a report rather than by a gate, so a paraphrase wearing
quotation marks lives there until somebody reads the report — §7.6.4.1's two hosts quoting a
`should` as a `shall` for two hundred and eighty-five sessions, and twenty-three Annex F rows
arguing from a phrase that is ISO 32000-1's vocabulary. It is in
`doc/habits/the-ledger-and-claims-about-this-tree.md` now.

**Gates.** `--bin pointers` 185 absent and 14 undefined before and after; `--bin quotations` 3709
verbatim and 49 diverging in documents before and after, in six more documents, with one *unrelated*
quotation gone with the sentence that quoted `doc/HANDOVER.md`'s own section title. Both exit 0.
`cargo test -p conformance` runs six targets: `bounded`, `sandbox_gates`, `workspaces`, `questions`
and `submodules` pass, and `conformance.rs`'s
`the_ledger_agrees_with_the_standard_and_with_the_tree` fails on §6.1, §6.3.2 and §6.3.2.2 being
`unreviewed` — rows a sibling round is adding to
`doc/conformance/ledger.toml` in this shared tree as this round ran, and clauses cited only from
`crates/`. The core four could not be completed either: `cargo fmt --all --check` fails in
`crates/pdf-font/src/predefined.rs`, also a sibling's uncommitted work. `cargo fmt --manifest-path
fuzz/Cargo.toml --check` is clean. No file under `crates/` was touched here.

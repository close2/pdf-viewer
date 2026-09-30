# 1439 — The fuzz workspace's lock is tracked and held to the root lock, and every target arrives with its line and its seeds

Session 1302. Status: accepted. Context: principle 3 (fuzzing from the first parser commit); ADR
0742 (a target fuzzed from nothing reaches almost none of what it exists for); ADR 1024
(`tools/fuzz.sh` refuses a target `doc/verify.md` does not name); ADR 1423 (the campaign, and the
lock that did not build); ADR 1416 (a done todo file is cut to its header only where an ADR holds
every paragraph). Code: `.gitignore`, `fuzz/Cargo.lock`, `fuzz/seeds.sh`, `fuzz/seed_streams.py`,
`tools/conformance/tests/fuzz_workspace.rs`, `tools/worktree.sh`, `tools/batch.sh`.

## 1. The lock

`fuzz/Cargo.toml` is a workspace of its own, so cargo resolves it apart from the root, and
`fuzz/Cargo.lock` was gitignored — by a bare line under the corpus and artefact lines, with no
reason written for the lock itself. A checkout therefore had whatever lock its own last resolution
left, and the two worktree scripts copied the main checkout's. That copy is the one no merge ever
updates: on this round's first read it still pinned `hybrid-array` 0.4.13 and `wnaf` 0.14.0 against
the root lock's 0.4.15 and 0.14.1, a day after a campaign had to `cargo update --precise` its way to
a build, and it lacked `pdf-ccitt` entirely.

Two ways were open. **Sharing the root lock is not one cargo offers**: a lock belongs to the
workspace that resolves it, a symbolic link would have the fuzz workspace rewrite the root lock
with only its own packages, and making `fuzz/` a member is what `fuzz/Cargo.toml` refuses in its
own comment — the sanitiser profile would apply to the whole tree. So **the lock is tracked**, the
line leaves `.gitignore` with the reason in its place, and the copy lines leave `tools/worktree.sh`
and `tools/batch.sh`, because a checkout now brings the lock with it. `cargo-fuzz`'s own
`fuzz/.gitignore` template ignores `target`, `corpus`, `artifacts` and `coverage` and not the lock,
and a fuzz workspace builds binaries, which is the case cargo's guidance says to commit a lock for;
nothing here depended on it being ignored.

**A tracked lock still drifts**, since the two resolutions are independent. So
`tests/fuzz_workspace.rs` fails when the lock is ignored again, and when it pins any version of a
package the root lock also holds that the root lock does not pin. A package the root lock holds at
two versions may be at either; `libfuzzer-sys` and what it brings belong to the fuzz workspace
alone and are not asked about. The locks are read by line, the conformance crate taking no TOML
dependency. What it found today: 137 packages in the fuzz lock, 134 shared, and the two
disagreements above, now aligned with `cargo update --offline -p hybrid-array --precise 0.4.15` and
`-p wnaf --precise 0.14.1`; `cargo +nightly fuzz build -O -s none` built all 23 targets against it.
A planted pair calibrates the comparison (trap 13).

## 2. Every target has its line and its seeds

`tools/fuzz.sh --list` exited 1 because `ccitt` had no `doc/verify.md` line, found by the round
that tried to run it, and most targets' seed recipes were prose a round had to find and copy. The
same test fails on a file under `fuzz/fuzz_targets/` that is not a `[[bin]]`, has no
`cargo +nightly fuzz run <target>` line, or has no arm in `fuzz/seeds.sh`. **The recipe is a `case` arm**, so the test reads a structure rather than
a phrase: `fuzz/seeds.sh [<root> [<target>...]]` now seeds every target or the ones named, and the
prose about each population stays under the target's line in `doc/verify.md`.

Eighteen targets had a recipe only in prose or none. The arms carry `doc/verify.md`'s own commands
for `page`, `serialize`, `cms`, `revocation`, `x509`, `confined_wire` and `display_list`;
`document`, `lexer` and `object` take whole documents, `seed_page.py`'s recipe; `fragment` takes the
fragments the reader and its tests state, since a fragment arrives with a request and never in a
file. Six targets read one object out of a document, and `fuzz/seed_streams.py` takes exactly that
object, in the framing each target reads: a metadata packet (`xmp`), an embedded TrueType program
(`sfnt`), an embedded CMap (`cmap`), a fax stream behind the six head bytes `ccitt.rs` takes Table
11's parameters from, derived from its `/DecodeParms` (`ccitt`), the inside of a standard security
handler's dictionary (`crypt`), and a field's `/DA` and `/V` as the two halves `variable_text.rs`
splits (`variable_text`). Over `doc/pdf.js/test/pdfs` it found 390, 402, 20, 160, 25 and 51 seeds;
the `INITED` coverage of those against the main checkout's corpus of each, grown by every campaign
since, is 689/1584, 290/383, 360/368, 122/223, 1064/837 and 5042/5985 — a fresh clone's starting
point rather than a replacement, and on `crypt` ahead of the grown one.

## 3. Todo 12 is its header

The file `doc/todo/12` kept three paragraphs no header ADR held: the two re-run commands and the two
neighbouring questions. The commands are `doc/verify.md`'s now, beside the oracle's other variables;
ADRs 0575, 0616 and 0617 are named in the header; so ADR 1416's condition holds and the file is its
header. Every citer in `crates/`, `tools/` and the owed documents points at the ADR that answers
what it cited; the ledger note and the trap that name the question by the file's number keep it,
and the file stays for them.

## Costs

A lock a person must update in two places: a `cargo update` at the root that moves a shared
package fails this test until `fuzz/` follows, which is the point. `seed_streams.py` reads raw bytes,
so an object inside an object stream is not seen and each figure above is a lower bound; it says so
in its header.

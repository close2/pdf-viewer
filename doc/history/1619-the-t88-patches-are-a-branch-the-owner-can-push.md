# 1619 — The three T.88 patches are a branch the owner can push

Slot 5 of batch ninety-seven, 2026-10-20, a clause round: ADR 2074 (2075 unused), `doc/questions/Q514`, no `crates/` file.

**Premise.** Held: the fork `close2/hayro` lists `main` and three branches, and none carries any of
the three patches. It was read from a bare clone in a fresh scratch directory, nothing run from it.
No commit of the fork's own touches `hayro-jbig2/` or `hayro-ccitt/`. **Hypothesis.** Half held.
The fork's `main` does not track upstream: it is upstream's `cc9c4024`, 31 commits before the
pinned `ea9c81dc`, plus the owner's two JPEG 2000 commits. Its manifests say `hayro-jbig2` 0.3.0
and `hayro-jpeg2000` 0.4.0. A branch on it would take the JPEG 2000 codec back past 0.4.1, so the
branch is built on the pin, as each patch's own header asks (ADR 2074 section 2). Upstream's `main`
has implemented the extended template itself in #1485 (`e7c49ef5`, 2026-10-13). That commit is
unreleased, and it is evidence about the reading, nothing more.

**Built.** `/home/AI/hayro-fork/` is a no-checkout clone of the fork. Branch `quorra/t88-patches`
holds the three patches as three commits on `ea9c81dc`, tip `7e39af41`. They were made through a
temporary index, so no fork file was written to disk. Each commit message cites its ADR (1447,
1459, 1485) and says that no upstream report was filed. `quorra-t88-patches.bundle` (13 791 bytes)
carries the same commits to the owner's own clone with their hashes kept. Q514 has the push
commands and the six steps that follow it. §7.4.7 stays `partial`. Its note now names the branch
and Q514, and its stale "the fork's commit" for `Cargo.toml`'s pin now says upstream's.

**Checked.** The base's `hayro-jbig2/`, `hayro-ccitt/` and `hayro-jpeg2000/` are byte-identical to
cargo's checkout of `ea9c81dc`, apart from the licence symlinks, which were not hashed. The
branch's `hayro-jbig2/` equals that checkout with the three patches applied. In a copy of the
checkout, not the fork, `cargo test -p hayro-jbig2 --lib` gives exit 0, 3 of 3, Annex H.2's test
among them. The `asset_suite` has no inputs until upstream's `sync.py` downloads them, so it was not
run.

**Gates.** `cargo test -p conformance`: exit 101 at first, `names` alone, on two paths into slot
8's open `transparency` split; exit 0 on the re-run with this record in place, 59 binaries. `tools/state.sh --round 1619 questions`: exit 0, 91 questions, 2 open
(Q501, Q514). For `pdf-sandbox`, whose code did not change, `cargo fmt --check` gives exit 0,
clippy `-D warnings --all-targets` exit 0, and `cargo nextest run` exit 0 with 50 of 50, the
EXTTEMPLATE refusal and `t88_conformance` among them. The row's `pdf-model` tests (`jbig2`,
`fixed_documents`) give exit 0, 5 passed and 1 skipped. Owed only if the codec changes, and it did
not: the six arms, `cargo deny` and the JBIG2 fuzz target.

**Files.** `doc/conformance/ledger.toml` (§7.4.7's note), `doc/todo/01`, `doc/todo/65`, ADR 2074,
`doc/questions/Q514`, this record. Outside the tree: `/home/AI/hayro-fork/`. 1 600 s.

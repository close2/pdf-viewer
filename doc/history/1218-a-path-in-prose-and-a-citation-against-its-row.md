# 1218 — A Rust path written in prose, and a citation read against its row

The instruments round of batch thirty-three: two sweeps built, four stale sentences rewritten,
three `code` lists closed.

## What moved

No ledger status. §12.7.3 and §7.9.6 now name `crates/pdf-transform/src/archive/{sites,rewrite}.rs`
in `code`, §14.13.2 names `rewrite.rs`.

## Findings

**A Rust path written in prose was read by nothing** (ADR 1273). The conformance gate checks a `§`,
`--bin pointers` a file path, the ledger its own arrays; `` `Interpretation::text` `` was checked by
none, and `rustdoc` resolves only the linked form and only under `cargo doc`, which no tier runs.
`--bin names`, `tools/state.sh names`, `cargo test -p conformance --test names`. **Where a prefix
is looked for decided the instrument**: workspace-wide and crate-local were both measured and both
worse than the comment's own crate plus the crates its manifest depends on. Thirty findings were
fixed in prose by naming the library — `wgpu::Surface::configure`, `peniko::Compose::DestIn` — and
the residue is a **named population** in `tests/names.rs`, one line per path, failing both ways.

**A `code` list decays in one direction only** (ADR 1274). A round adding a reader writes the
citation because principle 5 requires it and forgets the row in another file, so the citation is the
live half. `--bin cited`, `tools/state.sh cited`, calibrated by plucking a file the ledger names out
of its own row in memory, the pair chosen by the sweep. Of its first ten hits, eight were the rule;
one a validator citing a clause it checks rather than implements, one a crate whose listed site is
its C++ half, which the member rule then fixed.

**Four sentences outlived `verdict.rs` and `trust.rs`.** `signature.rs` said "nothing this program
prints uses the word"; `revision.rs` said §12.8.1's third question "has no trust store behind it in
this program at all", three times. ADR 1076 made `Valid` a type with no public constructor and ADR
1039 the anchors a host's input rather than an absence; each is now what is.

## Handed over

- **`STANDING` in `tests/names.rs` is a reading list**: each line is a renamed item, a name a round
  meant to write, or a type named without its crate. The day it empties the gate becomes a zero.
- **`--bin cited`'s top rung is a campaign's reading list**, one crate at a time: eight of the
  first ten read were real gaps in rows this round did not own.
- `names.rs` reads a `#[derive]`d method and a `pub use` re-export, not a macro-generated item.

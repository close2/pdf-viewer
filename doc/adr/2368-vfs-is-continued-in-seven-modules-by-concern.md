# 2368 — `Vfs` is continued in seven modules, one per concern

Session 1767, the standing code-quality slot of batch 116. Status: implemented; no behaviour changed, no bug fixed. ADR 2369 is unused.

`tools/state.sh gods` named `Vfs` in `crates/pdf-vfs/src/lib.rs` after `Host` (ADR 2352): one impl block of about 1 180 lines
and 38 methods in a file of 2 415 lines. **It is not the protocol's one `match`.** The block is thirty-eight methods of five
kinds and the file also held the error types and the worker-consent seam, so `Vfs` was taken and `Carry` left.

**What it is now.** `lib.rs` (566 lines) keeps the crate's module header, `Config`, the `Vfs` type with `new`,
`with_password`, `layout`, `write_meaning` and `shortfalls`, the structs one generation is made of (`Serving`, `Current`,
`Embedded`), `Verb` and the three small helpers every child shares (`refusal_for`, `canonical_path`, `mismatch`). Beside it:

- `mount.rs` — which generation is served (`current`, `held`, `current_in`) and the counters (`generated`, `questions`,
  `asked_twice`, `forgotten`, `provenance`, `generation`, `pages`);
- `directory.rs` — `list`, `entries_of`, `stat`;
- `reading.rs` — `consult`, `answer`, `open`, `generate`, `document_text`;
- `writing.rs` — `write`, `create`, `write_at`, `truncate`, `flush`, `release`, `pending`, `remove`,
  `create_directory`, `rename` and the helpers a commit uses;
- `inventory.rs` — the free functions that answer what the document has: `locate_in`, `attachments`, `metadata_stream`,
  `image_names`, `extract_images` and their neighbours;
- `error.rs` — `VfsError` and `Refused`, re-exported from the root, and `Write::name`;
- `consent.rs` — `Consenting`, `Counted`, `Totals` and `Questions`, the seam every question to a worker crosses.

**Hypothesis that did not hold.** The slot hoped for four to six modules and a root under 500 lines; the split is seven
and the root is 566, because `Config`, the public data types (`DirEntry`, `Attributes`, `Handle`), the generation's structs
and the crate documentation are what the root is for.

**Proof.** `proof.py` takes the multiset of tokens of the non-comment lines, `pub(crate) ` stripped, HEAD's `lib.rs`
against the root and its seven children: nothing is lost, and once `use` and `mod` statements are removed the only
additions are four `impl Vfs {` heads with their braces and two trailing commas that `rustfmt` wrote when it rewrapped
a long line. The compiler found the visibility: a private method or function a sibling reads became `pub(crate)`, and
`Totals::twice` is the one field widened. Nothing was renamed, so no document that names `Vfs::…` was edited; the seven
ledger rows that named `lib.rs` also name the child that now holds the clause's code. Twenty doc links that resolved
through the root's imports were written `crate::…` (section 1 of the brief; rustdoc with `-D warnings` is clean on the
crate).

**Gates.** See `doc/history/1767`.

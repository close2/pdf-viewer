# 1928 — A packet no page can lay out is removed and stated

Session 1928, a single archive round in the rounds worktree. ADRs 2678 and 2679. No Q, trap or habit.

**Why.** R14 of `doc/pdfa-blocking-issues.md`: `241573_ftp` was refused with `NOT_TEXT`, and
`state-every-loss`'s header promised a second step at the information sites that only the
embedded-file rows carried out.

**The premise, measured.** Object 67 is 532 bytes under a dictionary stating no `/Filter`; they
inflate to a 1603-byte packet. Every shipped profile refused the document at 2b and 4. Under
`state-every-loss` the refusal was `NOT_TEXT` at the packet row (and the property row at 2b); at 4
the two section 6.1.3 document-information rows refused too, for custom `/Info` keys. §7.4 makes
the `Filter` entry the only source of a stream's filters, so the remedy of inflating is declined
(ADR 2679).

**Built** (ADR 2678). One predicate, `keeps_per_object`, now decides where a keeping step hands
an object on: the embedded-file rows' `preserve:append` and the packet rows' `preserve` with
`original = "page"`. The step list, `no_page_carries` and the `append-statement` check all ask it.
A replaced packet whose decoded data is not UTF-8 is removed, not laid out; that is exactly the
case that refuses `NOT_TEXT`. The statement page lists the requirement, the loss `metadata-packet`,
the object, its stored length, the filters stated and the reason. The report names it per stream
(JSON `replaced_metadata_packets[].removed`, text `removed:`), and so does `xmpMM:History`.
`state-every-loss`'s three packet rows at 2b and 4 gain `on-failure = "append-statement"`.
`no-data-loss` is unchanged and still refuses. The page was rendered and looked at.

**Not taken.** The part 4 document-information rows: their second step is the `unmapped` key, which
has no `append-statement`, and the profile names neither row. Building it is a separate change.

**Tests.** `tests/archive/packet_removed.rs`, three: removal and statement at 2b and 4 (page, report,
history), `no-data-loss` refusing at both, and `original = "attach"` refused a second step. The
predicate planted back without the packet rows failed the first.

**Census.** ADR 2669's command against round 1927's: 1 new output, none gone, and the 4346 shared
outputs match once masked. The new output is `241573_ftp` under `state-every-loss` at 2b, and it
passes veraPDF 1.30.3. Ledger: §7.4.1 stays `implemented`, with its note naming ADR 2679.

**Gates.** `cargo fmt --check`, `clippy -D warnings --all-targets` (`pdf-transform`): exit 0.
`nextest` of `pdf-transform` and `pdf-archive`: 1353 passed, exit 0. Archive tier behind the lock
(`corpus`, `archive_corpus`, `witnesses`): exit 0 each. `cargo test -p conformance`, `state.sh docs`: exit 0.

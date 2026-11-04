# 1925 — The parent tree a source loses is rebuilt from the structure tree

Session 1925, a single archive round in the rounds worktree. ADR 2674; no question, trap or habit.

**Why.** Round 1924's row refused three pdf.js documents that converted before; ADR 2673 decided
the remedy: rebuild the entries, which are the structure tree read backwards.

**The premise, measured.** `mutool show` held it for three of four. `issue20516`'s root *does*
state `/ParentTree 11 0 R`, but object 11 is `null`, which §7.3.10 makes no entry; `tagged.rs` had
refused it as a tree that is not a dictionary.

**Built (ADR 2674).** `archive/parent_tree.rs` gathers each claimed item's claimants in `/K` order.
A lost holder (no key, a key the tree does not hold, a value of the wrong shape, an entry naming no
claimant, or Table 359's other key, which is taken out) takes a new key from `/ParentTreeNextKey`,
past every key any page, annotation or `XObject` states, and an entry naming its first claimant. A
right producer key is never changed, and a sequences entry keeps the producer's right and unclaimed
slots: the structure tree is the authority, by §14.7.5.4's direction. `NumberTree` keeps its
entries; the rebuilt pairs ride ahead of the appended pages' in one extension
(`Planned::rebuilding`), else on their own (`Rebuilt::alone`). `Rewrite::ParentTreeRebuilt` answers
the row, counted per entry, and `xmpMM:History` states the count, the packet's refusal withdrawing
it. Refused by name: a structure tree too deep or too large, an object both item and container, an
item claimed only inline, a document past 2^20 keyed objects.

**Tests.** `tests/archive/parent_tree.rs` at 2a, 2b and 4: `issue15340`'s and `issue20516`'s shapes;
`wikipedia`/`f1040`'s, a right key kept and three annotations keyed in a new leaf under `/Kids`; an
unclaimed page stating the first key, which is not handed out again; two refusals. Each output is
read back through `pdf_model::structure::ParentTree` and `Conforms`. `statement.rs`: a rebuilt entry
and the statement page share one new leaf. Planted controls: with the keys withheld the conversions
write nothing; without the key walk the unclaimed page's key is reused.

**Census.** ADR 2669's command against round 1924's after-run: `issue15340`, `issue20516` and
`pdfjs_wikipedia` convert under both stating profiles at 2b and 4 (3 more in each of the four), and
the 12 outputs pass veraPDF 1.30.3; `prefilled_f1040` stays refused by its other rows. No corpora
count moves, nothing is newly refused, and the 4318 shared outputs match once masked. Ledger:
14.7.5.4 stays `implemented`; its code, tests and note name ADR 2674.

**Gates.** `cargo fmt --check` and `clippy -D warnings` of `pdf-transform`: exit 0. `nextest` of
`pdf-transform` and `pdf-archive` (`CARGO_INCREMENTAL=0`, an ICE): 1345 passed, exit 0. Archive
tier: exit 0. `cargo test -p conformance`: exit 0. After the key-walk fix: census unchanged, 12
outputs pass veraPDF, every item reads back via `ParentTree`.

# 1149 — No object may name an appearance; Table 43's last two entries are read

2026-09-16. `batch-1147-1152`. `crates/pdf-model/src/{file_spec,thumbnail,attachment,view}.rs`,
`tests/named_appearances.rs` and `examples/name_dictionary_and_file_spec_census.rs` (both new),
`doc/conformance/ledger.toml` (§7.7, §7.7.2, §7.7.4, §7.11.3), this file. No pixel moved.

## The census
Over **3427 documents** (963 of the pdf.js 974, 489 of the curated corpora, 1975 of a 1-in-45 crawl
sample): **25 state a `/Names /AP` tree** (48 names, 47 streams); **11 file specifications state Table
43's `/Thumb`**, all streams; **none states an `/EP`**; **not one annotation states an `/AP` that is a
name**. Calibrated (trap 13) by planting all four in a §7.6.7 wrapper: 1, 2, 2, 1.

## §7.7.4 — the brief asked for a lookup the standard forbids → `implemented`
Table 32 sends `/AP` to §12.5.5, **not** §12.7.4.3, which states nothing about the tree. §12.5.5 ends:
"The name strings have no standard meanings; no PDF objects may refer to appearance streams by name."
Resolving a widget's `/AP` by name would implement what that sentence denies exists (principle 5). What
it owes is a **reader**, and `pdf_syntax::tree` already is one general enough: rounds 1121 and 1145's
shape, no consumer owed. `tests/named_appearances.rs` witnesses that, calibrated against a document
without the entry. The other five are `out-of-scope` or `inapplicable` by their own rows — all ten read
or decided.

## §7.11.3, and §7.7.2 on exclusion (round 1131's shape) → both `implemented`
Table 43's `/Thumb` *cites §12.3.4*, correcting this row's "only defined for a *page*":
`thumbnail::of_file_spec` is `thumbnail::read` over one shared `decode`. `/EP` is Table 28:
`file_spec::EncryptedPayload` names the required `/Subtype` filter, keeps `/Version` as the name its
NOTE forbids reading as a number, records whether `/Type` is permitted. `Attachment::thumbnail` carries
the stream **undecoded** (principle 2) and `Attachment::payload` the dictionary; naming the filter is
all §7.6.7 owes a reader with none. §7.7.2's four are decided — `/SpiderInfo` (§14.10), `/PieceInfo`
(§14.5), `/NeedsRendering` (Annex K), and catalog `/AA`, **every Table 200 entry being an ECMAScript
action** which §12.6.3's row already excluded and `pdf_archive` reads. §7.7 stays `partial`, §7.7.3.3
its last child. No ADR — both are clause readings.

## Gates
`conformance` caught two quotations of mine attributed to the clause a neighbouring sentence named last,
and passes on my files once re-anchored; its 2 remaining failures are siblings', as are `nextest -p
pdf-model`'s 2 of 1425 (**1423 passed**), five clippy errors and `fuzz/`'s. `rustfmt --check` on my six
files, `--doc` and `fuzz/` `fmt` clean. `pdf-model --test corpus` **exit 0**, ratchets 0/10/1/5/61 at
ceiling. `raster_golden` read **exit 101** first — held 8, moved 966, all "list only (an interpreter
change no pixel shows)", 1148's new `DisplayList::transfers` inside a `format!("{:?}", list)` digest —
then **exit 0**, held 974 moved 0, once they regenerated. `batch.sh check` **exit 0**.

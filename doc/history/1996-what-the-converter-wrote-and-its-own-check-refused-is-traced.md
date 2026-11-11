# 1996 — What the converter wrote and its own check refused is traced

Session 1996, a single round, PDF/A lane's worktree. `doc/pdfa-blocking-issues.md` section 8 (`D-defect`); ADRs 2803, 2804.

**The premise.** Held for all eight lines at HEAD, every main profile at both parts: `T02-02` failed the page-count row
the source met (every profile, both parts); `hesr13613` and `poppler-90-0-fuzzed` the operator row under `as-if-printed`
and `state-every-loss`; `027613` the transfer-function row at 4 under `as-if-printed`; `digitally_signed_3D_Portfolio`
the `DeviceCMYK` row under `as-shown` and `as-if-printed`; `issue14200` the group row at 2b under every profile;
`auth-event-ef-open` the crypt-filter row at 4 under `no-data-loss`; `bug1365930` carried the stand-in default.

**Traced** (each a place where two readers disagreed). `T02-02`: `/Kids [1 0 R]` names the root; the reader draws the
page by `/Type`, the copied closure holds none. `hesr13613`: `> Tj` in a widget's appearance; the photograph keeps
`/Annots`. `poppler-90-0-fuzzed`: page 4's run `0..9172` in `ASCII85Decode` data with a group past 2^32; the run
writer needs the whole decode and skipped the stream silently. `027613`: a second `/TR` state (3516) found by its
header past the damaged table; the source failed the row, so ADR 2795's trigger did not fire. `digitally_signed`: the
pages laid out from its embedded PDFs, the same third trigger. `issue14200`: the mask's `/G` states no `/Resources`.
`auth-event-ef-open`: the embedded-file row passed an `/EFOpen` stream it could not decrypt. `bug1365930`: the default
went to a page group the photograph removes.

**Built.** ADR 2803: a page tree reaching none of the pages drawn is declined and rebuilt under
`recovered-structure`; an operator an annotation's appearance reaches keeps the fence's refusal; a run in a stream that
does not decode whole is photographed; the second pass also runs on a requirement the first pass changed and the
output still fails; the check fails an embedded file its own encryption withholds. ADR 2804: a form naming no resource
gets an empty `/Resources` for its default (§7.8.3); a page group the photograph removes gets no default.

**Freed** (output-fails to written): `T02-02` (every profile, both parts); `poppler-90-0-fuzzed`, `027613` (4),
`digitally_signed` (`as-shown`, `as-if-printed`, both parts; `state-every-loss` 2b); `issue14200` (2b, every profile).
**Refused by name instead**: `hesr13613` (fence), `auth-event-ef-open` under `no-data-loss`, `T02-02` without the loss.
**Left**: the `uniXXXX` rows (a `pdf-font` reading owing tier 2), the check's enumeration of header-found objects.

**Pixels** (CPU, `/OutputIntents` renamed): every changed census output 0 px old against new; `poppler` pages 1-4 0 px
against the source at 300 dpi, its other unphotographed pages 0 px at 72; `digitally_signed` page 1 moves by the
stated `DeviceN` default at 2b (64 levels) and the named shipped press at 4 under `as-if-printed` (14).
**Tests.** Seven, in `pdf-transform/tests/archive.rs`, `archive/recovered.rs` and `archive/photographed.rs`, each
failing under its plant.

**Gates.** fmt 0 and clippy `-D warnings --all-targets` 0 on `pdf-transform`, `pdf-archive`; nextest of both 0 (1471
run, after a citation in `pdf-archive` was respelled); `cargo test -p conformance` 0; archive tier 0 and 0; censuses
against 1992: shipped 13 NEW, 6 CHANGED, 0 GONE, stating 8 NEW, 5 CHANGED, 0 GONE, 0 newly refused in either;
veraPDF 1.30.3 73 of 73 PASS; `tools/state.sh --round 1996 docs` 0.

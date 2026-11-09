# 1974 — A fault of a content stream's own bytes is reported as the stream's

Session 1974, a single round, PDF/A lane's worktree. R03 and R08's "does not draw whole"; ADRs 2759
(the reader) and 2760 (the gate).

**The premise.** Not true as stated. The census's line named seven documents, refused at both parts
under `as-if-printed` and `state-every-loss`: govdocs `150277`, `219789`, `427669`, `492484`,
`504076`, `507676` and pdf.js's `poppler-90-0-fuzzed`. None holds a filter, image or shading gap of
this reader's. Every page the gate stopped has a `/Contents` whose Flate data fails its own Adler-32
(Python's `zlib` rejects the same streams), and what stopped it was the decoded garbage reported as
this reader's: `sc` with 2 to 27 operands as `Shading`, `sh` on an undefined `/` as `Shading`, inline
images with a number where a key belongs or no `/W` as `Image`, and `Do` with no name, a stray `EI`,
a stream ending after `W` and a bare `true` as operators the fence's survey does not list. The
other rows each holds (`only-operators`, at 2b `values-written`) the photograph carries.

**Built.** ADR 2759: `ColourOperands` (§8.6.8) and `MalformedInlineImage` (§8.9.7) are the stream's
faults; `sh`'s undefined name is `MissingResource`; `StreamSyntaxFault` names a nameless `Do`, a
pending clip and a stray `ID`/`EI`; `true`, `false` and `null` are operands (§7.3.2, §7.3.9); a number
past f32's range is no number (an infinity on `150277`'s page 57 made the CPU backend refuse the
page). ADR 2760: the gate admits the two kinds, and its refusal says which kept cause holds a page:
a bound of this reader's, an image or shading it could not build, a part of `/Contents` drawn of
nothing.

**Freed** (`as-if-printed`): `504076` at 2b and 4, `492484` at 4 (2b stops at the hard limits). Under
`state-every-loss` both at 4. **Moved on**: `150277`, `219789` to the photographed-pixel bound;
`poppler-90-0-fuzzed` to the output check (a `/A85` contents stream the conversion restates decodes).
**Kept by name**: `427669` (`MAX_OPERANDS`), `507676` (`R73` and two inline images do not decode).

**Pixels** (CPU, 72 dpi): both sources draw identically before and after the reader change on all 33
and 32 pages. Outputs differ from source on the photographed pages (4, 12, 32; 2, 8, 14, 31) and where
R08 substitutes a face, identically with `/OutputIntents` renamed.

**Tests.** `pdf-model/tests/stream_faults_by_name.rs` (six) fails whole against the old reader; the
gate's tests fail without the two kinds and under one sentence. **veraPDF** 1.30.3 passes all five
outputs written.

**Gates.** fmt 0 and clippy `-D warnings --all-targets` 0 on pdf-model, pdf-transform, viewer-core;
nextest of pdf-model, viewer-core, pdf-archive, pdf-transform 0 (4294 passed); `cargo test -p
conformance` 0. Censuses against round 1971: shipped 3 NEW, stating 2 NEW, 0 GONE, 0 CHANGED; veraPDF passes
all five. Archive tier 0; tier 2 as at HEAD (16 moved, 62 incomplete); `state.sh docs` 0.

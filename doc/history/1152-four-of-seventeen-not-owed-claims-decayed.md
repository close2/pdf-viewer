# 1152 — Four of the seventeen not-owed claims decayed, and two more lost their reason

2026-09-16. Files: `doc/conformance/ledger.toml` (17 notes, no status moved), `doc/todo/65`, this
record; other `git status` paths are siblings'. No ADR: nothing was decided, seventeen claims were
re-read, with `spec-errata emit` over `ISO_32000-2_sponsored_EC3.pdf` filed by heading.
## Decayed — four
- **§12.7.8.3.1** — `/EmbeddedFDFs` was carried as *deprecated in PDF 2.0*. Table 246's cell is
  "( Optional; PDF 1.4 ) An array of file specifications … representing other FDF files embedded
  within this one", with no deprecation marker; only Table 247's `/EncryptionRevision` has one, and
  the prose the word came off ("Although deprecated in PDF 2.0, embedded FDF files … may be
  encrypted.") is ambiguous about what the participle governs. **Errata #173 (`Review`/`Completed`)
  settles it**: that opening becomes *Although FDF file encryption is deprecated in PDF 2.0,*. The
  array is an ordinary PDF 1.4 entry and its import **owed**; the RC4 form stays refused. Not built
  — it needs §7.11.4 streams, a bound and a fixture. → bucket 4.
- **§12.8.3.4.4** — "nothing in a document supplies it". ETSI EN 319 122-1 clause 5.2.10's unsigned
  attribute carries the policy *inside* the signature and `cms.rs` knows its OID; what is missing is
  the specification its `spDocSpec` names. → bucket 2, beside §8.6.5.9.
- **§8.9.6.4** — filed as a bit depth Table 87 leaves undefined; the same row says "The bit depth is
  determined by the PDF processor in the process of decoding the JPEG 2000 image.", ADR 1121 settled
  it 44 sessions ago, and the retired sentence still stood beside its replacement. Struck → bucket 4.
- **§12.7.8.3.2** — the note has said since ADR 0907 that the unapplied `/AP`, `/APRef`, `/IF`, `/A`,
  `/AA` are requirements unmet, not permissions declined; the membership was stale. → bucket 4.
## Kept the disposition, lost the reason — two, plus one candidate
**§12.5.6.2**: `/ExData` was "outside this project's scope"; geospatial is §12.10's and *is* in
scope, and Table 173's own next sentence disposes of `MarkupGeo`. **§12.11.3**: the residue quotes
across a join — weighting "among other features in the same document requirements array" needs no
second document, and `requirements::penalty_total` performs it. **§10.4.2.3**, the candidate: "the
formula has no caller" is false — `colour::rgb_to_cmyk` runs §10.4.2.4's steps on every colour
reaching `rgb_to_ink`, and for a grey they *are* this clause's formula, asserted by a test §10.4.2.5's
row cites and this one did not. The residue is §10.4.2.5's departure on §10.4.2.1's ranking; left
`partial` because `departed` is a decision and needs an ADR.
## Held — ten, each note now saying it was checked
§12.6.4.9, §12.6.4.10 (hand-over sentence verbatim, no erratum, clause 13 unamended); §12.5.6.11
(#524, #608 reach `/RD` — #608 *strengthens* the refusal); §12.5.6.12 (`NotForPublicRelease` appears
once in the standard, Table 184, no artwork); §10.7 (held; its note called §10.7.5 "neither
implemented nor reported" while that row is `departed` — struck); §9.8.3.3; §12.7.4.1 (#28/#313/#618
reach `/T` and `/AA`, not the bound); §12.7.8.3.3; §12.7.5.4 (#191 types `/TI`); §12.11. Gate:
`cargo test -p conformance` exit 0, 259 passed; no code changed, so `raster_golden` is not owed. 22
bare `"` I introduced were caught by a scan of every `note` line and escaped — `HEAD` had none, and
the ledger reader had accepted them silently.

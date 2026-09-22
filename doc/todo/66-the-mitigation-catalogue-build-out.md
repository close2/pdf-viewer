# 66 — The mitigation catalogue, site by site: the remedies a profile names and this version does not carry out

Status: **open**, written in the round after 1156 when the owner asked where the PDF/A converter
stood and whether the "as-if-printed" profile worked. A 20-band item (a real document asks and the
converter refuses) numbered past its band because the band is full.
Priority: **high** — it is the bulk of RFC 0007, which its own section 4.7.5 says: *the catalogue
is the bulk of this feature, not the mechanism*. The mechanism is built (the format, the sites, the
departures, the tool executor, the appended page, the relocation); what a profile answers is only
carried out at the sites that have code behind the answer.
Companions: `doc/rfc/0007` (the design), `doc/pdf-a-mitigations.md` (the catalogue: 118
requirements, four answers each, and the 29 whose honest answer is *none*), `doc/profiles/` (the
six shipped profiles), `doc/todo/62` (the exemption), `doc/questions/A54`–`A60`.
Corpus: the archive corpus walk, `cargo test --profile gates -p pdf-transform --test archive_corpus
-- --ignored --nocapture` under the heavy-walk lock, prints per target how many documents convert
by default and with everything authorised, and the ten sites that refuse most.

## The instrument, and the sentence that names the gap

```sh
quorra-transform archive --remedy-sites --to 2b      # every site the target binds, and what this version carries out
quorra-transform archive --to 2b --config doc/profiles/as-if-printed.toml -o out.pdf in.pdf
```

The second prints, for each answer the profile gives at a site without code behind it:

> note: the configuration answers "<site>" with `discard`, which this version does not carry out
> yet; that site stays refused with the sentence it names

**The item is done when a shipped profile produces no such note at any target.** Both halves of
that condition are now instruments rather than numbers in this file. `--remedy-sites` prints its own
two totals — `N of M sites not built yet`, and, given `--config <file>`, every answer that profile
gives which this version does not carry out and `N of M sites answered with a remedy not carried out
yet` — and `tools/state.sh remedies` filters those two sentences per target and per shipped profile.
It runs in `quick`; `section_archive` calls it, so one section prints both halves.

**The profile half needs no corpus, which is a property of the question rather than a shortcut.**
The note comes from `Configuration::unbuilt`, which reads the answers a profile gives and asks which
of them have code behind them; both are properties of the profile and the target, so a conversion
prints the same notes whatever document it is handed, and walking a corpus for them would count the
corpus. The corpus question is a different one and is `section_archive`'s: how many documents
convert, by default and with everything authorised, and which sites refuse most.

## What each family needs, so that a round can take one

Grouped by the code one build unlocks, not by clause; every site's own entry in
`doc/pdf-a-mitigations.md` carries the argument and the per-target answer.

- **Actions** (`actions/*`, eight sites): **built** — `crates/pdf-transform/src/archive/actions.rs`,
  under one `Loss::InteractiveBehaviour`, with `forms/no-action-on-widget-or-field` beside them
  because one routine answers all nine. A removed action's §12.6.2 `/Next` subtree is promoted into
  its place, so the permitted actions behind a forbidden one still run (ADR 1175).
- **Forms** (`forms/*`, four): widget actions **built** with the Actions routine above;
  `/NeedAppearances` cleared and the XFA packet removed or, at 4f, attached (catalogue section 7's
  `preserve`) are what is left.
- **Metadata** (`metadata/*`, eight): a fresh packet written by the existing XMP writer with the
  old one attached at 4f/4e or appended as a page (catalogue section 9; the owner's *append or
  prefix the packet as a page*). The appended page is built, including the structure entries it
  owes a tagged document (ADRs 1025, 1163). The extension-schema container site is the top refusal
  at 2b.
- **File structure and encryption** (`file-structure/no-encryption`, `crypt-filter-is-identity`,
  `permissions-dictionary-keys`): **built** — `Loss::Encryption`, the word `encryption`, with
  `crates/pdf-transform/src/archive/protection.rs` carrying the producer's Table 22 flags into the
  report and the output's own `xmpMM:History` (catalogue item 5, ADR 1187). The third row was
  already `Mechanical` (ADR 1007).
- **Colour** (`graphics/separations-of-one-name-agree`): **built** as a `supply` —
  `winner = "first" | "most-used"` names which of the file's own definitions of an ink the archive
  keeps, so every byte written is the producer's (ADR 1188). What is left in this family is
  `no-overprint-mode-one-under-icc-cmyk`'s `discard`.
- **External stream data** (`file-structure/no-external-stream-data`): **built** for the bytes a
  caller can resolve — `crates/pdf-transform/src/archive/external.rs`, ADR 1199. The conversion
  names what it needs in `Conversion::external_data`, `quorra-transform archive
  --resolve-external-data` reads a plain file name beside the document under ADR 1155's rule, the
  plan carries the bytes, and §7.3.8.2's Table 5 is the rewrite; a stream stating the filter keys
  and no `/F` needs nothing from outside and is `Mechanical` on its own. What is left is the
  `tool = "resolve-external"` route, which is what §7.11.5's URL needs and what every corpus
  witness turns out to be — the archive sweep counts, per target, how each such stream names its
  data. **One wrinkle the build left**: `--remedy-sites` lists a site while the census classes it
  `Refused` or a `Loses` remedy, and this one is now `Mechanical`, so it has dropped off the
  listing although a URL-named stream still refuses and `keep-everything.toml`'s answer for it is
  still counted as not carried out. A `Mechanical` answer conditional on something the *caller*
  supplies wants to stay enumerable; the classification that decides is `archive::census`.
- **Graphics state keys** (`graphics/no-transfer-function-*`, `no-halftone-*`,
  `second-transfer-function-is-default`, `rendering-intent-*`): a key removed from an `ExtGState`
  or a halftone dictionary, each a `discard`; at 4f the sampled function may be attached.
- **Annotations** (`printable-and-visible`, `appearance-dictionary-holds-only-normal`): the `/F`
  bits set as the part requires, the `/D` and `/R` appearances dropped.
- **Optional content** (two sites): `/AS` removed, configuration names supplied.
- **Embedded files** (four): the 4f `preserve` (catalogue section 1.2) for anything a byte string
  can hold, and the media-type `supply` already built.
- **Fonts, content-stream marks**: the catalogue's *none* — leave them, and say so; ADR 0816's
  fence is the reason, and ADR 1124's content-stream splice is the precedent to cite if a later
  round argues the fence should move. Two have a narrower reading than the family's:
  `fonts/cid-system-info-agrees-with-the-cmap` keeps its *none* for the file whose CMap and font
  genuinely belong to different collections, and the catalogue entry names the buildable part —
  correcting a `/CIDSystemInfo` **from the program it describes**, which §9.7.4.2 makes a copy —
  with the two readers it needs; `fonts/embedded-programs-define-every-glyph-shown` and
  `fonts/no-notdef-glyph-shown` hold their *none* on a re-reading of both clauses (ADR 1200), and
  the difference between them is ISO 19005-2 section 6.2.11.8's *regardless of text rendering
  mode*, which its neighbour's NOTE 2 does not say.
- **Fonts, the supply this tree describes and does not accept**: five messages tell a user to
  supply a face with `--font`, and `quorra-transform` has no such flag. ADR 1200 section 4 is the
  reading — the licence ISO 19005-2 section 6.2.11.4.1 demands is a fact only an operator can
  state, so the flag is `doc/rfc/0007`'s `supply` in its oldest form — and the recommendation is
  `--font <base-font>=<path>` carried in the plan, reported and recorded in `xmpMM:History` beside
  the substitution already there, with `pdf_font::restate` applied so no glyph moves.
- **Implementation limits**: *none* for nine of the ten, and **not** for
  `implementation-limits/page-boundary-sizes` — §7.7.3.3's Table 31 makes four of §14.11.2's five
  boxes optional and §14.11.2.1 gives each a default that is another box in the file, so removing
  an out-of-range optional entry moves no mark. ADR 1200 section 1 has the predicate that decides
  whether the removal is mechanical or a loss; not built.

What every site needs alike: the answer carried out at rewrite, the report naming what left and
where it went, a fixture per site in `crates/pdf-transform/tests/archive.rs`, the output validated
clean, and the mitigation catalogue's entry updated from *catalogued* to *built*.

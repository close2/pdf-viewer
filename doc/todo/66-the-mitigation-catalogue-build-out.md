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
- **External stream data** (`file-structure/no-external-stream-data`): **not built**, and the
  catalogue entry now says what stands between it and a build — the fetch cannot live inside `apply`
  without costing RFC 0002 section 9's determinism claim, so either the caller resolves the `/F`
  names under ADR 1155's rule and hands the bytes in the plan, or the `A54` two-pass tool request
  carries it. The key removal itself is written down there.
- **Graphics state keys** (`graphics/no-transfer-function-*`, `no-halftone-*`,
  `second-transfer-function-is-default`, `rendering-intent-*`): a key removed from an `ExtGState`
  or a halftone dictionary, each a `discard`; at 4f the sampled function may be attached.
- **Annotations** (`printable-and-visible`, `appearance-dictionary-holds-only-normal`): the `/F`
  bits set as the part requires, the `/D` and `/R` appearances dropped.
- **Optional content** (two sites): `/AS` removed, configuration names supplied.
- **Embedded files** (four): the 4f `preserve` (catalogue section 1.2) for anything a byte string
  can hold, and the media-type `supply` already built.
- **Fonts, implementation limits, content-stream marks**: the catalogue's *none* — leave them,
  and say so; ADR 0816's fence is the reason, and ADR 1124's content-stream splice is the
  precedent to cite if a later round argues the fence should move. One of them has a narrower
  reading since session 1175: `fonts/cid-system-info-agrees-with-the-cmap` keeps its *none* for the
  file whose CMap and font genuinely belong to different collections, and the catalogue entry now
  names the buildable part — correcting a `/CIDSystemInfo` **from the program it describes**, which
  §9.7.4.2 makes a copy — with the two readers it needs.

What every site needs alike: the answer carried out at rewrite, the report naming what left and
where it went, a fixture per site in `crates/pdf-transform/tests/archive.rs`, the output validated
clean, and the mitigation catalogue's entry updated from *catalogued* to *built*.

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

**The item is done when a shipped profile produces no such note at any target.** Count them rather
than reading a number here: the first command's `not built yet` lines per target, and the second's
`does not carry out yet` notes over the veraPDF corpus, are what `tools/state.sh archive` should
grow to print (an instrument gap of its own).

`tools/state.sh remedies` prints the first command's `not built yet` count per target and runs in
`quick`; `section_archive` calls it, so one section prints both halves. The second command's
`does not carry out yet` notes are still uncounted — they come from the `--config` reader rather
than from the corpus walk — and the cheapest way to close that half is for `--remedy-sites` to
print its own `N of M sites not built yet` trailer, which `state.sh` can then filter instead of
count.

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
  `permissions-dictionary-keys`): decrypt on the way out — RFC 0006 section 5.4 calls it the
  easiest requirement — with the producer's permissions kept as a statement (catalogue item 5).
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
  precedent to cite if a later round argues the fence should move.

What every site needs alike: the answer carried out at rewrite, the report naming what left and
where it went, a fixture per site in `crates/pdf-transform/tests/archive.rs`, the output validated
clean, and the mitigation catalogue's entry updated from *catalogued* to *built*.

# 1029 — The roots are the manifest's, and a blockquote is not always the standard's

Session 1010. Status: **accepted**.

ADR 1024 §4 found that `tools/conformance`'s scan read a hand-written list of directories beside a
workspace manifest whose members are a glob, and that **1,884 clause citations in 243 of
`raster/`'s 283 Rust files had been outside the citation and quotation gate since 2026-09-06** —
producing no findings, because a directory nobody reads produces none. That round could only
report it from `tools/round.sh`; this one owned the crate, derived the roots, and read what came
in.

## 1. The roots, derived

`SOURCE_ROOTS` is gone. [`conformance::roots`](../../tools/conformance/src/roots.rs) derives the
scan's population from two rules:

- **every member the workspace manifest's `members` globs expand to** — `crates/*`, `tools/*`,
  `raster/crates/*` — read from `Cargo.toml` rather than restated beside it;
- **every crate at the top of the tree that holds a `Cargo.toml` of its own**, which is what
  `fuzz/` is. It declares its own `[workspace]` on purpose, so the root manifest does not mention
  it anywhere and no derivation from that manifest alone could find it. Dropping it would have
  traded one silently unread directory for another.

The second rule is bounded to depth one deliberately: a walk deep enough to find a second excluded
workspace is also deep enough to walk into `doc/corpora`, which is a submodule in a primary
checkout and a symlink into one in a worktree round.

`pointers.rs`'s `ROOTED_HEADS` had the same shape one file along — five names, `doc, crates, tools,
fuzz, data`, so no pointer written under `raster/` or `kio/` resolved at all. It is now
`Tree::is_a_head`, derived from the walk `pointers` already does, which knows the tree's top level
directly and answers for `doc/` and `data/` as well as for the crates.

**The check moved with it.** `tools/round.sh`'s check 7 is now
`tools/conformance/tests/workspaces.rs::every_workspace_member_is_scanned`, and `round.sh` prints a
pointer at it rather than a second copy of it. Its two sides are derived *differently* on purpose —
the scan's population from the manifest, the gate's from `git ls-files "*Cargo.toml"` — because a
check whose two sides are one derivation is a tautology. Calibrated: with one member filtered out
of `source_roots` it fails naming that member, and it also fails if a root returns no sources at
all, which is the same silence one step on.

## 2. What the 1,884 citations produced

259 citation findings and 60 quotation findings, none of which anything had ever printed.

**Every one of the 259 was a `§` naming a section of a document that is not ISO 32000-2.** The
project's rule is that a bare `§` is a clause of this standard and that another document's section
is written with the word `section` (`ISO 19005-2 section 6.2.2`, which this tree writes 1,462
times). `raster/` arrived under its own convention — `raster-gpu/src/error.rs` states it outright —
in which a bare `§` is `raster/doc/RENDER_LIBRARY.md`, the brief. Split by the document each names:

| naming | count | what was done |
|---|---|---|
| the brief, `raster/doc/RENDER_LIBRARY.md` | 204 | `§N` → `section N`, with `brief` in front where the line named no document |
| the caller's `doc/QUORRA_FEEDBACK.md` and `doc/HAYRO_ISSUES_FOR_QUORRA.md` | 26 | the same, with `the caller's` |
| WGSL | 26 | the same, with `WGSL` — another *standard*'s section, which is the shape `ForeignCitation` exists for |
| `raster/doc/research-function-paint-arithmetic.md` | 3 | the same |
| a clause of ISO 32000-2 that does not exist | **0** | — |

**Zero wrong clause numbers, and that is not the good news it looks like.** The 259 are the
citations whose *number* proves they are not clauses. A brief section whose number ISO 32000-2 also
has resolved in silence and still does: `§5`, `§3`, `§7`, `§11.4`, `§6.1` and their neighbours.
`raster/` holds 1,884 `§` of which 189 are spelled `ISO 32000-2 §N`; the rest are the two
conventions interleaved, twice on one line in
`raster-gpu/tests/degenerate_subpaths.rs` — "§4.5 places §8.5.3.2's disc upstream", one brief
section and one clause, both bare. **A per-directory default would therefore be wrong**, which is
why this ADR does not propose one. §5 states the remainder; it needs a round of `raster/`'s
own, reading line by line.

The quotation findings, which is where the resolving ones surfaced:

- **Six wrong clause numbers**, each a correct quotation of the standard attributed to the wrong
  clause — the class principle 5 exists for, and the first six this tree has found outside its own
  crates. "The existence of the knockout feature…" is §11.4.6 and was cited as §11.3.7.2 and as
  §11.6.4.4; "The shape of a group object shall be the union […]" is §11.3.7.2 and was cited as
  §11.6.4.2, twice; "The base image and the image mask need not have the same resolution" is
  §8.9.6.3 and was cited as §8.9.6.4; "The correspondence between image space and user space is
  constant" is §8.9.4 and was cited as §8.9.5.1; "Every pattern has a pattern matrix" is §8.7.2 and
  was cited as §8.7.4.1, twice; and the *alpha source* row is **Table 51 in §8.4.1**, not Table 57,
  in two files. Table 57's `AIS` is the key that sets the parameter; Table 51 is the parameter.
- **Two blockquotes carrying their own attribution inside the quote** — `(ISO 32000-2 §10.7.4)` as
  the quotation's last words, which can never match.
- **One paraphrase** presented as a quotation: §11.5.3's compositing joined to §11.6.5.1's `BC`
  entry in one sentence and quoted as though it were one clause's.
- **Eleven quotations of PLRM3, the caller's issue list, ADR 0053 and raster's own notes**, correct
  writing that this gate had no way to classify.

## 3. Three things the gate was wrong about, and they were the instrument's

Half the divergences were not raster's at all. Each fix is a *tolerance with its cost written
down*, never a weakening of what verbatim means.

- **`doc/md/` breaks words.** §8.7.4.5.2 reads "Points wi thin the shading's bounding box" where
  `doc/ISO_32000-2_sponsored_EC3.pdf` reads "Points within" — checked in the PDF, as the gate's own
  message says to. `quote::occurs_in` now tries a second time with every space removed. **12 of the
  43 divergences were this and nothing else.**
- **`doc/md/` does not hold the standard's equations**, leaving `<!-- formula-not-decoded -->`
  where the PDF sets one. §11.4.6's knockout computation is three of those markers in nine lines, so
  a comment quoting the equation exactly and a comment paraphrasing it fail identically.
  `ClauseIndex::dropped_a_formula` answers the narrow question; the gate pairs it with the
  quotation carrying an `=` and counts those as unverifiable rather than reporting them as wrong.
  **8 today.**
- **Typography is the renderer's, not the standard's.** Square brackets round a single letter are
  the writer's editorial case change (`[a] semicircular arc`, and `CLAUDE.md`'s own `[t]he
  implementation`); en dashes, minus signs and hyphens are one character in two renderings;
  variables are set in Mathematical Alphanumeric Symbols and indices below the line. `normalise`
  folds all three, and `occurs_in` folds case, which the bracket convention exists to change.

And one thing the gate was wrong about in a different direction. **A blockquote is not always the
standard's words.** `prose.rs` had already stated the problem about the ledger — a note "quotes the
standard constantly and also quotes this project's own past conclusions, and it has no blockquote
syntax to tell the two apart" — and a Rust doc comment has the same two kinds with the same syntax.
`citation::Quotation` now carries the *other* document the nearest attribution names, and the gate
counts those instead of checking them. The rule that keeps it from being an escape hatch: **a line
that cites a clause attributes to the standard even if it also names a document.** A quotation
reaches the other side only through an attributing sentence that cites no clause at all. 16 today.

## 4. The population question, asked of the sweeps

1004 asked it of four candidates outside `tools/conformance`. Asked of the twenty `--bin` sweeps
inside it, almost every hand-written constant is a *vocabulary* — the words a claim is made in —
and a vocabulary is the rule rather than the population. One is not.

**`ledger::NORMATIVE_ANNEXES` and `INFORMATIVE_ANNEXES` are a classification the standard states
itself.** `ledger::check` reports an annex in *neither* list, so the pair cannot lose one; nothing
asked whether a letter was in the **right** one, and the standard prints the answer on every annex's
own title line — `Annex D (normative) Character sets and encodings`, `Annex B (informative)
Operators in Type 4 Functions`. A transposition would move an annex's requirements out of the
ledger's scope in silence: `SOURCE_ROOTS`' defect one instrument further in.
`every_annex_is_classified_as_the_standard_classifies_it` now reads all 17 headings against the two
lists. **It agrees today** — the classification was right — and it is calibrated: moving `D` to the
other list fails by name. Two annexes carry the word on the line below their heading, where the
conversion broke the title; a heading whose marker cannot be found at all is a loud failure rather
than a pass.

## 5. What is not done

- **`raster/`'s remaining bare `§`.** 1,884 minus the 259 corrected minus the 189 already spelled
  out. Each needs reading to say whether it means the brief or the standard, and the two are
  interleaved line by line. `tools/conformance`'s gate cannot see them, by construction: each
  resolves.
- **`--bin quotations`' 49 diverging** did not move, and that is measured rather than assumed: with
  §3's `quote.rs` swapped back to its previous version the sweep prints the identical line, so the
  typography folds resolve none of the 49. They diverge by *words*, which is the reading nobody has
  done.

## 6. Five of `--bin quotations`' forty-nine, read

The figure has stood for many rounds and nobody had answered one of them. Five, each against
`doc/md/` and against `doc/ISO_32000-2_sponsored_EC3.pdf` where the conversion was in doubt.

- **`doc/conformance/ledger.toml`'s §7.6.6 note — not verbatim, fixed.** It wrote "[a]ny keys in
  the CF dictionary that are listed in Table 26 shall be ignored by a PDF processor"; Table 20's
  own words are `…that are listed in "Table 26 - Standard crypt filter names" shall be…`. The note
  abbreviated the caption inside the quotation marks. Now elided with `[…]`, which is this tree's
  device for exactly that.
- **The §12.3.3 note — not verbatim, fixed.** `( Required if there are any open or closed outline
  entries )` closes a parenthetical the standard continues: `…entries; shall be an indirect
  reference )`. Truncating at a semicolon and supplying the closing bracket makes the requirement
  look complete; `[…]` now says it is not.
- **The §11.3.6 note — verbatim, and the sweep is wrong.** It quotes `control[s] the influence of
  the backdrop and source colours`, and §11.3.7.3's sentence is "the backdrop and source alphas
  control the influence of the backdrop and source colours, respectively". The sweep matched the
  five-word prefix against a *different* sentence — "controls the influence of the blend function" —
  and reported the divergence from that one. **Its matcher takes the first candidate rather than the
  best**, which is a finding about the instrument and is left standing here because fixing it is a
  change to how `prose::Conversion` searches, not a citation.
- **The §8.4.4 note — a false positive by construction.** The diverging span is inside the
  sentence "**The quotation said \"a PDF processor may ignore this parameter\" until the
  four-hundred-and-thirteenth session**". A correction quotes the wrong quotation it retired, which
  the pointer sweep already names as its oldest false positive; the quotation sweep has the same
  one and does not say so.
- **`doc/errata-read.md`'s §8.5.3.2 row — a false positive of a second kind.** It quotes
  "**In the opaque imaging model, this** rule …", which is Errata Collection 3's *replacement*
  text. `doc/md/` is the published text the erratum amends, so every quotation in that file of a
  sentence an erratum changed diverges by definition. That is most of what `doc/errata-read.md`
  contributes to the forty-nine.

Two of five were real and are fixed; three were the instrument's, in three different ways. **A
divergence is a question rather than a verdict** was the right label, and this is the first round
that answered any.

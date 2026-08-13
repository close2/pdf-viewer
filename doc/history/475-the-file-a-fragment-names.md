# 475 — The file a fragment names, and the sentence that was about the ones after it

ADR 0310. Branch `worktree-agent-a3fa42a8635e772cd`.

## What the instrument said first

`tools/state.sh annex-o`, before anything was touched:

```
carried out: Page NamedDestination StructureElement Comment Zoom View ViewRect Search
reported:    EmbeddedFile Highlight Fdf
```

and `EmbeddedFile`'s reason, verbatim from the arm: "opening an embedded file is the host's
decision, and every parameter after this one applies to that file rather than to this document".

## What was taken, and why that one

`ef`. Two claims joined by an "and" — `doc/habits.md`'s fifth shape — and splitting them is the
whole finding. The second is about the parameters *after* `ef`; the first is not a blocker at all
but the *mechanism*, because `Event::Extracted` is how this crate hands a host a decision and it has
existed since the hundred-and-sixty-ninth session. Table Annex O.3's own `shall` is "the PDF
processor shall open the embedded file contained within the EmbeddedFiles name tree identified by
name", and the security sentences after it are a **should** and a **may**.

`doc/todo/39` had recorded the blocker as `doc/todo/38`'s four levels. That is a third reading and
the annex settles it: prompting is permitted, not required.

## What landed

- `Parameter::unhonoured` returns `None` for `EmbeddedFile`. Nine of eleven carried out, two
  reported.
- `Open::opening_file` records the tree key; `Viewer::open` runs the extraction as the document
  opens, before the first page's events. `apply_fragment` still stops after `ef` and says how many
  parameters it did not apply — moved from the refusal branch to the applied one.
- **`Event::Extracted` gained `asked: Extraction`**, and this is the half that was not bookkeeping.
  All three hosts wrote an extracted file to disk unasked, because until this session only a person
  could produce one. `pdf-viewer report.pdf#ef=x` would have written a file with nobody pressing
  anything. §O.1 is why the annex singles this parameter out — fragment identifiers are "useful
  primarily when referring to them from external to the PDF such as a web page or web API".
- `viewer_host::may_write_extracted` is the decision, once, beside §12.7.6.4's import policy;
  `viewer-ui` took a dependency on `viewer-host` for it rather than keeping a third copy.
- `highlight`'s reason re-derived from the annex and *not* changed sides: the annex says "selected"
  for `comment` and `search` and "highlighted" here, so this program's selection is not it.
- `fdf` checked against `CLAUDE.md`'s exclusions, which do not cover a network at all; the reason
  stands unchanged, and that is now written down so the next round does not re-check it.

## The trap this round nearly walked into

**A `..` pattern defeats the "every consumer fails to compile" rule.** Nothing in `viewer-core` is
`#[non_exhaustive]` exactly so that a new message breaks every host's build — and every host matched
`Event::Extracted { name, bytes, .. }`, so adding a field broke *no* build. The compiler enforces
the rule for a new variant and not for a new field on a struct variant. The three hosts were changed
by hand. Written into `doc/ui-boundary.md`.

## Populations, counted rather than assumed

**10 of the corpus's 964 documents carry an `/EmbeddedFiles` tree, 23 files between them** — a
throwaway example over `pdf_model::attachment::attachments`, deleted after it printed. That agrees
with `Query::Attachments`'s standing "964 of the 974 … state none".

## Gates

`cargo fmt --all --check` clean; `cargo clippy --workspace --all-targets` silent; `cargo nextest run
--workspace` **1699 passed, 11 skipped**; `cargo test --workspace --doc` clean (one spurious
`pdf-spec` rlib failure from five rounds sharing `/home/AI/cargo-target/pdf-viewer`, passing on its
own); `cargo build --profile gates -p pdf-sandbox --bins`; corpus gate **974 documents, 0
unopenable, 65 incomplete** — unchanged, as it must be, since nothing on the interpretation path
moved; `cargo test -p conformance` **6970 citations, 688 quotations, all verbatim**; `tools/state.sh`
ledger 875 rows. Two fuzz targets for the two parsers touched: `fragment` 1 769 632 runs in 91 s,
`confined_wire` 10 962 728 runs in 91 s, both clean.

The oracle, quorra and text-extraction gates were not run: they render pages, and no fragment
reaches them.

## Seen

Under `Xvfb :175`, `pdf-viewer --cpu 'attachment.pdf#ef=foo.txt'` says the file is 9 bytes and then
"the URI's fragment asked for this embedded file rather than a person, so it was not written to disk
— open it from the files panel to extract it (ISO 32000-2 §O.2.1)", and the directory afterwards
holds only the document. With `#ef=nothing.txt&page=1` it says the parameter after `ef` was not
applied and that the tree holds no such name.

## Left open

§O.2.1 stays `partial` on one sentence — the parameters after `ef` apply to the *embedded* file, and
composing that needs a second document the core cannot open for itself. `highlight` wants a `Query`
no host has asked for. `fdf` wants a host with a URI.

## Worktree note

This worktree's branch was 98 commits behind `main` when the round started (`tools/state.sh` did not
exist in it). Fast-forwarded to `main` before any work; the merge is the first commit on the branch
and carries no changes of its own.

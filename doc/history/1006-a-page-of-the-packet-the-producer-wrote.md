# 1006 — A page of the packet the producer wrote

Date: 2026-09-13. Branch: `batch-1006-1011`, worktree `/home/AI/pdf-viewer-rounds`, shared with
five sibling rounds (1007–1011). ADR:
[1025](../adr/1025-a-page-of-the-packet-the-producer-wrote.md).

## What was asked, and what was built

RFC 0007's `preserve`, in both of §4.6.1's mechanisms, under the permission the owner's `A58` gave
and `doc/adr/1014` records: a page composed solely of content the document already holds is on the
near side of `CLAUDE.md`'s authoring exclusion.

1. **`preserve` by attachment**, wired to the derivation `derive` already runs rather than written
   twice: `placement = "attach"` at either embedded-file site reaches `Configuration::derivation`,
   and the report says *derived, not original*, because at a target admitting only conforming
   attachments that is what keeping one means. At 4f and 4e the requirement does not bind, so the
   row is inert and the original stays as it is. No tool named at a target that binds it is a
   configuration error naming both the site and the target.
2. **`preserve` by appended page**: `crates/pdf-transform/src/archive/preserve.rs`, the composer,
   plus `Rewrite::PreservedAsPage` (the page tree's root node and the page-label tree),
   `Prepared::preserved`, `Conversion::preserved`, and one `xmpMM:History` event.
3. **The first content preserved**: the XMP packet of a document whose properties ISO 19005-2
   section 6.6.2.3.1 rejects — the biggest single refusal the corpus meets, and until now the one
   built loss. The properties still come out of the packet; the packet the producer wrote is set on
   pages appended to the document.
4. **Every placement decision as a documented choice**, ADR 1025 §4: page size, boxes, margin, type
   size, line breaking, tabs and byte-order marks, padding, colour, where the pages go, and the
   budget — each with what it costs.

## Three things worth keeping

**The strictest reading of ADR 1014 costs most of the remedy's reach, and the ADR does not ask for
it.** The composer's first choice of face is one the *document* embeds — addressed by
`LoadedFont::code_for`, checked against its own `/ToUnicode` — because a page set that way puts
nothing whatever on the document from outside it. Measured over the pdf.js corpus, that answered
one of the nine documents that fail the rule: a subset face embedded for body text has no glyph for
`<`, and a document with no text has no face at all. ADR 1014 §5 had already said what to do — *for
text, a face, which must be embedded under every target, and which `A47`'s permission and condition
already govern* — so the shipped face is the second choice, and the count went from one to seven.
The strict rule stays as the *preference*, which is what it is good for.

**§9.8.1's Table 122 has a value for "we did not measure it", and this tree had read the clause the
other way.** `fonts.rs` refuses to write a descriptor for a producer's font, "`/StemV` above all,
which is a measurement of a face's stems that nothing in this file states" — and the table's own
next sentence is "A value of 0 indicates an unknown stem thickness." That does not lift that
refusal, which is about describing somebody else's font; it is what makes a descriptor for a face
*this program chose* writable without inventing a number. Every other entry is read off the
program's `head`, `hhea`, `OS/2`, `post` and `name`.

**A fixture whose page is 200 units square is not a small version of a real document, it is a
different question.** The first composition put a fourteen-page packet into the test's output,
because a 72-unit margin on each side of a 200-unit page leaves a seventh of the width. The margin
is now capped at an eighth of the page's own sides, which changes nothing on any page anybody
prints — and the fixture states a letter page, substituted over the *bytes*, because
`String::from_utf8_lossy` over a file holding a font program replaces every byte of it that is not
UTF-8 (the fixture's font then failed to load with "units per em is zero", which is what sent this
round looking).

## What it does, measured

Over 974 pdf.js documents at PDF/A-2b with `doc/profiles/only-metadata-loss.toml`: nine fail the
rule, seven are preserved on a page, one is refused for carrying a structure tree and one for a
character `/WinAnsiEncoding` cannot address. Two of the seven convert to a written file; the other
five are refused by *other* requirements this converter still owes. Both written files gained
exactly one page, and page 2 of one of them was rendered and read: the producer's packet, legible,
one inch inside a letter page.

## Gates

`cargo clippy --workspace --all-targets` and `cargo fmt --all --check` exit 0; nextest 4484 passed,
36 skipped; doctests exit 0; both `fuzz/` lines exit 0; `cargo test -p conformance` exits 0 (the new
quotations of Table 51, Table 122 and Table 30 verify against `doc/md/`); `pdf-transform`'s whole
row under `tools/bounded.sh` — `gate`, the seven walks and `archive_corpus` — exit 0, one at a time,
with the corpus figures unchanged. `archive_unconsidered.txt` is byte-identical, so the census's
`Unconsidered` stays 0 at all six targets.

## What is left

The structure entries a Level A file's tree owes an appended page (the fence today); appending an
embedded file's content, which waits on the rewrite that takes a file specification out of the name
tree; a signature's statement as a page; and characters outside `/WinAnsiEncoding`, which want a
composite font written for the page. ADR 1025 §6 has what each needs.

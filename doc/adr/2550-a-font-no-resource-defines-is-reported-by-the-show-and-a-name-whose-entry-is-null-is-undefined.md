# 2550 — A font no resource defines is reported by the show under it, and a name whose entry is null is undefined

Status: implemented. Session 1858, Slot 8 of batch one hundred and twenty-six (two sessions). Q724 is not asked.
Context: ISO 32000-2 §7.3.5, §7.3.9, §7.3.10, §7.5.2, §9.3.1 (Table 103); ISO 19005-2 sections 6.1.7.1 and
6.2.2, cited by section; ADRs 0255, 2490 (R06a) and 2502; trap 258. ADR 2551 holds the third
document's decision. Code: `crates/pdf-model/src/content/font.rs` (`font`, `load_font`, `Absent::detail`),
`crates/pdf-model/src/content.rs` (`owed_font_report`), `crates/pdf-model/src/content/text/show.rs`,
`crates/pdf-archive/src/survey/records.rs` (`peek`), `crates/pdf-transform/src/archive/rewrite/reader_resources.rs`
(`insert_name`).

The SafeDocs and corpora residue batch 125 left had four documents. Each is taken to its cause below,
the clause read before any other reader's output.

## 1. `2145704`: a `Tf` with the empty name and no text shown

Stream 2136 of the SafeDocs document writes `BT/ -1 Tf ET`. §7.3.5 admits the name: "The token SOLIDUS
(a slash followed by no regular characters) introduces a unique valid name defined by the empty sequence
of characters." The page's resources define no `/Font` entry under it, and the interpreter reported "no
/Font resource named /" at the `Tf`, so the survey counted the page incomplete and the archive census
counted the document.

Table 103 in §9.3.1 gives `Tf` two text state parameters to set and nothing else: "There is no initial
value for either font or size; they shall be specified explicitly by using Tf before any text is shown."
The font is what a later show draws with. A `Tf` whose name reaches no font dictionary therefore costs
a mark only where text is shown under it, and a text object that selects such a font and shows nothing
is drawn exactly as its producer wrote it.

**Decided.** `font` and `load_font` no longer note a name that reaches no dictionary. They return the
sentence beside the font, the text state carries it as `owed_font_report` (set by `Tf` and by Table 57's
`/Font` alike), and the first show that draws nothing for the name says it, once a page as before. A
show under a layer that is off still says nothing, because not drawing it is correct there. A
dictionary that is present and whose program does not load is still reported at its load, since that
sentence is about the font and not about a show. The empty name now prints as "no /Font resource named
/ (the empty name, §7.3.5)", because a bare `/` reads as no name at all; ADR 0255's population is counted
by the sentence's unchanged prefix. That one document goes from 1 incomplete page to 0 in the survey. The corpus gate's figure did not
move: 2 of its 974 documents report a `/Font` name the resources do not define, each under a show, and
its population of 59 incomplete documents is unchanged.
50 000 `Tf` of distinct undefined names with no show now report nothing where HEAD printed 50 000
reports, at 15 MiB of high-water memory where HEAD's was 26 MiB; with a show under each name, both
print the same 50 001 lines and draw the same page.

## 2. `poppler-742-0-fuzzed`: a page dictionary that does not parse whole

Page 8's dictionary stops at byte 678, where a `(` inside `/TrimBox` opens a string that never closes.
Seven entries were read whole. ADR 2551 decides it: the page is written as those entries, and is named.

## 3. `issue9105_other`: the header `%PDF-1.`

The file's first bytes are `%PDF-1.` and an end of line. §7.5.2: "The file header shall consist of
'%PDF1. n ' or '%PDF2. n ' followed by a single EOL marker, where ' n ' is a single digit number between
0 (30h) and 9 (39h)." The minor digit is missing, so the file-header row's refusal is a correct reading
and nothing in this tree is at fault. The remedy is R06a's (ADR 2490: the target's header written, the
source's bytes recorded in `xmpMM:History`), which is Slot 4's to build and named to it.

## 4. `veraPDFHiResWrongObjectID`: the premise did not hold

The brief's premise was that the object headers disagree with the cross-reference table and that the
writer copied a stated `/Length`. Neither held. Every object header agrees with both cross-reference
sections at the offset each states. The "wrong object ID" is page 9's `/XObject << /Im0 99 0 R >>`, and
object 99 does not exist (the image is object 11). HEAD refused the document at 2b and 4 under both
profiles with `graphics/named-resources-are-defined`, "which this document met, so the conversion broke
it", and wrote no output, so the `/Length 8700`, packet and `/ID` description in
`doc/pdfa-blocking-issues.md` section 8 did not reproduce; the writer restates every `/Length` from the
bytes it writes, and a stream planted with `/Length 8700` over 62 bytes is written with its true length.

The cause is trap 258's: `survey/records.rs::peek` counted the entry present without resolving it.
§7.3.9: "An indirect object reference (see 7.3.10, "Indirect objects") to a nonexistent object shall be
treated the same as a null object. Specifying the null object as the value of a dictionary entry (7.3.7,
"Dictionary objects") shall be equivalent to omitting the entry entirely." So `/Im0` is undefined in the
source, which the survey did not see and the output's check did.

**Decided.** `peek` resolves the entry before it counts as found and returns it as the file wrote it.
`insert_name` defines a name whose entry resolves to null in place of that entry, except where the entry
already holds the edit's own value, which a second edit of a shared table finds. The document converts at
2b and at 4 under both shipped profiles; veraPDF passes all four outputs, and its verdict on the source
names the same defect ("Undefined XObject /Im0", rule 6.6.4-2 alone). A page naming 50 000 dangling
references converts in 2.4 s at 75 MiB of high-water memory, one empty form a name, where HEAD refused
it at 52 MiB (both measured as the child's maximum resident set, the export's `dev` build).

## Consequences

- The survey and the interpreter now agree with §7.3.9 and Table 103 at the two places that read past
  them. A page that selects a font nobody defines and shows nothing is no longer incomplete.
- `doc/pdfa-blocking-issues.md` section 8's row is replaced by its cause, and R01's list loses
  `poppler-742-0-fuzzed` (ADR 2551).

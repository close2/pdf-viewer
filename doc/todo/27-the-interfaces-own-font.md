# The interface's own font, and the text it cannot set

Status: **measured in the three-hundred-and-sixteenth session, and no longer silent.** What is
open is coverage, which is a decision the project owner has not been asked for.
Priority: 27
Corpus: **74 documents** state something in the sidebar this font cannot set; 9 strings would be
drawn as nothing at all. 45 documents state a popup and 6 of the 7 open ones are Chinese.
Clauses: §9.6.2.2, §12.3.3, §12.5.6.14, §14.3.3
Code: `crates/viewer-ui/src/chrome.rs` (`Chrome::set`, `Chrome::text`, `Chrome::width`),
`crates/viewer-ui/examples/chrome_coverage.rs`

## What it is

Everything this program draws for itself is set in §9.6.2.2's fourteen, compiled into the binary
(ADR 0133) — which is what makes an interface reproduce on a machine with no fonts installed, and
is the right default. The fourteen are Latin; a document's own text is not.

## What is answered

**Nothing is dropped in silence any more** (ADR 0195). A character the face states no code for is
drawn as a **box**, 0.6 em wide, and it advances — so five characters of Japanese are five boxes
rather than an empty row, and `Chrome::width` measures them exactly as they are drawn. Whitespace
with no code is a space; a control character is nothing, because it has no visible form to be
missing. §12.5.6.14's popup window still says *how many* under its note
(`Chrome::without_a_code`, ADR 0191), which a picture cannot.

**And the population is measured**, by `examples/chrome_coverage`, which opens every corpus
document through `viewer-core` with a zero-pixel viewport and asks the four queries the sidebar
asks. 964 documents open:

| population | documents | of those, short | strings | short | drawn as nothing | characters |
|---|---|---|---|---|---|---|
| §12.3.3 outline titles | 150 | **7** | 343 | 24 | 3 | 46 |
| §8.11.4.3 layer names | 21 | **1** | 91 | 2 | 0 | 61 |
| §7.11.4 file names and descriptions | 10 | **0** | 64 | 0 | 0 | 0 |
| §14.3.3 `/Info` values | 492 | **45** | 1293 | 80 | 4 | 195 |
| §14.3.2 XMP properties | 317 | **21** | 1437 | 38 | 2 | 72 |

The nine strings that would have been drawn as nothing are Japanese, Thai and Chinese:
`issue2884_reduced.pdf`'s outline is あいち電子調達共同システム, `issue16176.pdf`'s is
ローカルディスク, `issue13211.pdf`'s `/Info` is a Thai sentence of 46 characters of which
Helvetica sets one.

**The largest single loss is a malformed file rather than a language.** `bug1146106.pdf` loses 51
characters of one layer name, and they are U+FFFD: it writes its text strings as UTF-16
little-endian, which is none of §7.9.2.2's three encodings, so `text_string` reads Table D.3 and
every second byte is the clause's undefined code point. Correct reading, malformed file — and
`text_string`'s own comment had already said a caller "reports it rather than dropping it
silently", which nothing did until ADR 0195.

## What is not answered: coverage

A box says a character is there and that this program cannot set it. It does not say *what*, and
for a person reading a Japanese outline that is most of what they wanted. Three answers, in rising
order of what they cost, and none is obviously right:

1. **Fall back to a face on the machine**, which is `pdf_font::substitute`'s whole subject. It
   works and it costs ADR 0133's argument: the interface stops looking the same on two machines,
   and every assertion about it becomes an assertion about which fonts this one has.
2. **Compile in a face with the coverage** — a licence question, a size question (a CJK face is
   megabytes against the standard fourteen's 804 KB) and a decision the project owner has not been
   asked for.
3. **Ask the host.** A native host draws these rows in a `QTreeView` or an `NSOutlineView` with the
   platform's own font stack, which has the coverage and did not have to ship it — so on the hosts
   `doc/todo/30` is about, this whole item is `viewer-ui`'s alone. That is an argument for not
   spending a megabyte on it here.

The 74 documents above are the demand side of that decision, and they are what this file was
missing.

# 631 — Three samples, and the ranges nobody asked them about

`doc/todo/03`'s chunk again, over the SafeDocs crawl, for the sixth round running. Ten whole
archives, and three defects that are one question asked in three places: **what range does a sample
run over?** Not one of them is about decoding. Every one is about the arithmetic between a decoded
byte and a colour or an extent — and all three were silent or wrongly worded about it.

Date: 2026-08-21.
ADR: [0464](../adr/0464-three-samples-and-the-ranges-nobody-asked-them-about.md).

Touched: `crates/pdf-model/src/image.rs`, `crates/pdf-model/src/inline_image.rs`,
`crates/pdf-model/tests/jpx_channels.rs` (new), `crates/pdf-model/tests/dct_components.rs`,
`crates/pdf-model/tests/inline_images.rs`, `doc/conformance/ledger.toml` (§7.4.2, §7.4.3, §7.4.9,
§8.9.7), `doc/errata-read.md`, `doc/checks/fixed-documents.toml`, `doc/todo/03-more-corpora.md`
§22, the ADR and this file.

## The chunk

**`0792`, `1038`, `1776`, `2145`, `2883`, `3621`, `4359`, `5097`, `5835`, `6573` — 10 000
documents**, none of the thirty-two archives sessions 603, 613, 615, 619 and 625 ranked. An archive
is a hash bucket (ADR 0261), so any set is unbiased. 603's instrument reused rather than rewritten,
at **15 minutes** for the ten thousand on sixteen workers.

**Checked before it was trusted.** Both binaries built (619's lesson), `target/release/examples/`
confirmed to hold no worker of its own (624's), and **§20's own check run first**:
`cargo test --profile gates -p pdf-model --test fixed_documents -- --ignored` — **25 checked, 0
absent, 25 rows, green** — which is what 623 paid for and this round's first command. The
instrument then reproduced ADR 0459's three recorded documents to the thousandth before a new row
was read.

## The three defects

**`5097148.pdf` −43.503**, the deepest row of the ten thousand and a blank sheet at 0.092 against
44.214 / 43.596 / 44.042. Its inline image is `/W 2951 /H 178 /CS /RGB /F [/A85 /Fl]` with no
`/L`, so `inline_image`'s answer 3 ran — the forward search its own comment calls "the one guess" —
and the first `EI` standing as a token is **69 598 bytes** into 1.29 MB of base-85. One command
drawn, several hundred `Operator` reports whose names are runs of base-85. §8.9.7 makes the bytes
"a stream object's data" and §7.4.2 and §7.4.3 give two of Table 92's filters an end-of-data marker
*in* the data, over alphabets that cannot contain it; Table 5 says which filter is asked. The
clause's own EXAMPLE is the arrangement, ending its `/F [/A85 /LZW]` image `…2HCqC~> EI`.
−43.503 → **−0.323**, 328 commands, nothing reported.

**`4359750.pdf` +32.097** is the first chunk's finding to sit on the *positive* side, and it could
not have been found any other way: a page that is otherwise perfect, silent, with one photograph
drawn as a **solid black rectangle**. The image is `/DCTDecode` in a `/Lab` space, and
`convert_three` divided every eight-bit channel by 255 — where §8.9.5.2's map takes Table 88's
default pair per *space*, and a `Lab` space's first component is a percentage running to **100**.
ADR 0448 fixed the `Indexed` half of the identical hole one arm along in the six-hundred-and-thirteenth,
by writing that space's Table 88 default as a constant; the three- and four-component arms were
never touched and had no witness until now. All three arms index the map now.
+32.097 → **+0.307**.

**`0792405.pdf` −8.329** loses both its `/JPXDecode` photographs to "the colour space takes 4
components but the codestream has 3". `opj_dump` says the codestream has four. What it does not
have is any JP2 box: it is a **bare** codestream, so `hayro-jpeg2000` synthesises sRGB for it and
then, finding a fourth channel beside a three-channel space, concludes in its own comment
"[a]ssume that we have an alpha channel in this case". §7.4.9 makes the dictionary's `/ColorSpace`
determine the interpretation and the JPEG 2000 colour space specifications ignored — and which
channel is an *opacity* channel is read off those same specifications — while Table 87 makes it
moot from the other side, `/SMaskInData` 0 meaning encoded soft-mask information is ignored. The
three conditions on the fix are the point rather than the fix: a non-zero `/SMaskInData` is
believed, and a space whose ordinary channels already match is left alone.
−8.329 → **+0.576**.

## The erratum

`spec-errata emit` over the clause family before writing — `doc/errata-read.md`'s own standing rule
— found **Issue #293**, a whole sentence added to §7.4.3: *"If the ASCII85Decode filter encounters
the character ~ in its input, the next character shall be > and the filter will reach EOD. Any
other characters shall cause an error."* `check` had never named it and could not, because it
compares the tree's *quotations* against struck passages and this is a pure addition over text
nobody had quoted — ADR 0187's §5.1.3 lesson one clause family along. It is what turns the marker
from a convention into a rule, which is exactly what an extent derived from it needs.

## What moved

**The reach is measured over our own panel**, before and after, across all **42 archives / 42 000
documents** any chunk round has ranked — because a reference renderer's panel cannot depend on our
build, and ADR 0459 measured that one *does* differ between two runs with nothing changed. That
removes the noise by construction and costs a quarter of the wall clock.

**Ten rows move. Six are the fixes:**

| document | before → after | fix |
|---|---|---|
| `5097/5097148.pdf` | 0.0924 → 43.2732 | the extent |
| `4359/4359750.pdf` | 72.3370 → 40.5470 | the decode range |
| `4482/4482885.pdf` | 67.7896 → 55.6610 | the decode range |
| `0792/0792405.pdf` | 13.3155 → 22.2199 | the channels |
| `0423/0423614.pdf` | 43.1913 → 42.7474 | the extent |
| `7311/7311510.pdf` | 26.5206 → 26.5402 | the decode range |

**Three are in archives an earlier chunk took** — `4482` and `0423` are 615's, `7311` is 625's —
which is the sixth round running that a fix has reached back, and put in front of the three
references afterwards each moves toward agreement: `4482885.pdf` **+11.288 → −0.840** against
56.501 / 56.943 / 56.634, `0423614.pdf` +0.487 → +0.043, `7311510.pdf` −0.278 → −0.259.

**The other four are the instrument**: `4482567.pdf`, `5589666.pdf`, `0546114.pdf` and
`0546365.pdf` produced no number in the before pass and one in the after, and on a quiet machine
both binaries give byte-identical numbers on all four. They are renders that lost their
ninety-second budget while three other rounds were compiling — 626's lesson arriving in a sweep
instead of in a gate.

**The channels fix's population is one document of 65 944**, counted by a walk that reads each
`/JPXDecode` stream's own SIZ marker and its dictionary with a regular expression (trap 8): one
document, eight images, every one under a stated `/ColorSpace`. Narrow, and said so.

Each fix is pinned by a test **run against the defect first** (trap 13) — the base-85 arm, the
hex arm, a window twin that must ask for more bytes rather than search, a generated four-component
bare codestream with two negative twins, and a `Lab` JPEG whose sample of 128 is mid grey where a
division by 255 drew **(4, 1, 1)**. Three rows appended to `doc/checks/fixed-documents.toml`, which
then runs 28 and passes.

## Gates

The full §2 sequence, because the change is in `pdf-model`. `RUSTFLAGS="-D warnings"` on the clippy
line, which caught two `doc_markdown` lints this round's own quotations introduced — both answered
with an `#[expect]` naming the reason rather than by adding backticks to a quotation. The
conformance gate caught a blockquote whose clause number was written as "Table 5" with no `§`
before it in the same comment. **The sweeps ran before the sequence and nothing ran beside it**,
which is 626's rule and matters here more than usual: this round had two whole-population sweeps
and a 93 GB census of its own.

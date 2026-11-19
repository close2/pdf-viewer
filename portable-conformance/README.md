# A portable conformance suite for ISO 32000-2

Test files whose expected outcomes are derived from ISO 32000-2 (PDF 2.0) and from nothing else —
not from a reference rendering, not from what any processor does — runnable against any PDF
processor through a small adapter. RFC 0016 (`doc/rfc/0016-a-portable-conformance-suite-expected-outcomes-from-the-clause-alone.md`)
is the design and its argument; this file is how to use it.

## What a test is

One TOML manifest under `tests/`, named `<clause>-<what>.toml`, holding everything:

- `id`, `clause`, `title`, and `kind` — `pixels`, `text`, `structure` or `refusal`;
- `quote` — the sentences of the clause the test rests on, verbatim from ISO 32000-2:2020;
- `[pdf]` — the file itself as readable source: its indirect objects in order, numbered from 1, the
  first the catalog, each `body` written in PDF syntax by hand and each stream's data beside it;
- `[[check]]` — the expected outcomes, each with `why`: the arithmetic or the clause sentence that
  fixes the value and its tolerance.

`tools/generate.py <dir>` writes each manifest's PDF; it adds the header, every `/Length`, the
cross-reference table's offsets and the trailer, and nothing else. The output is byte-for-byte
reproducible (`tools/generate.py --check <dir>`), so no binary file is committed.

## Checks

| type | asks | passes when |
|---|---|---|
| `marked` | a probe point, `at` in default user space or `pixel` in the raster | every component is at most 64 |
| `unmarked` | a probe point | every component is at least 191 |
| `neutral` | a probe point | the three components differ by at most `tolerance` |
| `between` | a probe point | every component is strictly between 64 and 191 |
| `size` | the raster | its width and height are exactly `width` and `height` pixels |
| `text` | the extracted text of `page` | it equals `equals`, leading and trailing white space aside |
| `answer` | `question`, one of the questions below | the one-line answer equals `equals` |

A pixel test paints black on white and puts every probe at least five device pixels from any edge
the clause fixes, at the `dpi` the manifest states, so the probe sees paint or paper. The two
thresholds are a quarter of the range from each end. **No test compares against an image**: where the
standard fixes a region, the probes are inside or outside it; where it fixes an equality, the check
is the equality; where it fixes nothing (how a fractional page becomes whole pixels, the artwork of
an icon, anti-aliasing), there is no check.

Questions an `answer` check asks: `page-count`; `page-label <index>`; `destination <name>`, answered
`page <index>`; `field-value <fully qualified name>`. Page indices are zero-based (§12.4.2).

## The adapter protocol

An adapter is an executable that answers:

```
<adapter> render <pdf> <page> <dpi> <out.png>   draw one page to an 8-bit, non-interlaced PNG
<adapter> text   <pdf> <page>                    print the page's text as UTF-8
<adapter> query  <pdf> <question ...>            print the one-line answer
```

It exits 0 with an answer, 3 for a request its processor cannot be asked (shown as `n/a`, not as a
failure), and anything else for an error. `adapters/` holds three: `quorra` (this program),
and `poppler` and `mupdf`, which are **evidence only** — a processor that agrees with the
expected value raises confidence that the clause was read right, and one that disagrees is a
question to take back to the clause, never a reason to change an expected value.

## Running

```sh
tools/generate.py /tmp/suite
tools/run.py --pdfs /tmp/suite --adapter adapters/quorra --adapter adapters/poppler --adapter adapters/mupdf
```

The table is printed per clause; `--json <file>` keeps every verdict and its detail. The `quorra`
adapter finds `quorra-transform` and `portable-adapter` on `PATH`, or in `$QUORRA_BIN`.

## Licence

The manifests and the PDFs generated from them are dedicated to the public domain under CC0 1.0
(`SPDX-License-Identifier: CC0-1.0`); the generator, the runner and the adapters are under the
repository's Apache-2.0 licence. The quoted sentences are ISO's, quoted as citation: a sentence or
two per test, each attributed to its clause of ISO 32000-2:2020, and never the clause entire. The
standard itself is not in this directory and is not needed to run the suite. `doc/questions/Q948`
asks the owner to confirm both licences.

## Version

`suite.toml` holds the suite's version. A change to an expected value or to a check type is a new
minor version and names its clause; a new test is a new patch version.

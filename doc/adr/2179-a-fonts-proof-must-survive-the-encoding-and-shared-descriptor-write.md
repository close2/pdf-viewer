# 2179 — A font's proof must survive the encoding and shared descriptor write

Session 1671, archive. Status: **accepted**. Context: ADRs 2147, 2162 and 2163.

The ten real-corpus embedding refusals were traced through the existing read-back
proof. They are not hinting differences: this proof compares advances and outline
paths before rasterisation or hinting. Its tolerance stays unchanged.

Eight first failing fonts lose the viewer's horizontal fitting when embedded:

| Document | Font | Source stretch (embedded stretch is 1) |
| --- | --- | --- |
| `427669.pdf` | F5 Helvetica-Bold | 0.9999765 |
| `me450f12project10_report.pdf` | T1_5 Helvetica-Bold | 0.9997446 |
| `E3_Mayhew_et_al_2009.pdf` | HelvCDDEI~1287712280 Helvetica | 0.9999765 |
| `PDF 2.0 image with BPC.pdf` | F1 Helvetica | 0.99999064 |
| `PDF 2.0 via incremental save.pdf` | F1 Helvetica | 0.99999064 |
| `PDF 2.0 with offset start.pdf` | F1 Helvetica | 0.99999064 |
| `PDF 2.0 with page level output intent.pdf` | F1 Helvetica | 0.99999064 |
| `Simple PDF 2.0 file.pdf` | F1 Helvetica | 0.99999064 |

These are the first failing fonts in each document, not a bound on every font or
on rendered pixel error. Advances can remain identical while outlines change.
The existing `machine-face` loss concerns replacing a machine's face; it does not
authorise changing a shipped face's fitting. The refusal now names horizontal
fitting. Q566 asks about a separate explicit loss; implementing it first requires
a decision to preserve every loss owed by one rewrite. An exact program outline
transformation is another possible future answer. Neither is silently assumed.

The two remaining documents expose a code-to-name omission. In govdocs1's
`error_set_2/020806.pdf`, source font 18, F2, Symbol Italic selects code 110 as
`nu` (glyph 74). In `492484.pdf`, source font 57, F4, Symbol selects code 101 as
`epsilon` (glyph 67). Both source TrueType dictionaries omit Encoding. When the
shipped Symbol face becomes an embedded Type1/CFF program, §9.6.5.1 makes an
absent Encoding use that program's built-in table. It is not the source reader's
Annex D.5 table; the converted selections were missing. The converter now states
the source's selected names as Differences for this absent-Encoding case. Every
existing advance and shown-outline check still runs. Explicit encodings retain
their existing treatment. The synthetic regression checks both targets and both
names. Both real documents clear the embedding refusal after this correction;
independent content-stream and implementation-limit fences still stop conversion.

The second verdict of govdocs1 `error_set_1/316267.pdf` fails TT2 at output object
4682: for example I is stated as 0.483 but the embedded program says 0.27783203;
S is 0.633 versus 0.6669922. These match source font 602, Tahoma-Bold. It shares
FontDescriptor 596 with fonts 590, 597 and 607, each stating only space at 293.
Preparation proved each program independently, then inserted each at the same
descriptor key. The later space-only program overwrote the earlier prepared
program, discarding widths that had already passed proof. The writer's second
verdict correctly refused the assembled document.

Every simple font receiving a program now receives its own descriptor copy. Its
original descriptive entries are retained, its program entry points to that
font's proved bytes, and the font dictionary points to its own descriptor.
This is unconditional: a survey of selected simple fonts cannot prove the absence
of unused aliases or CIDFont descendants. Copying avoids changing those aliases
and avoids a quadratic scan. The same `Embedding::write_into` helper serves both
the copied descriptor and the existing writer, removing other null program keys
and stale CharSet/CIDSet descriptions exactly as before. Existing object
allocation and font-entry rewriting support both direct and indirect font
dictionaries. No advance, glyph selection or loss authorisation changes.

The focused two-simple-font regression reproduces the overwrite before the
change and requires both targets' complete second verdict to conform afterward.
Composite fonts still use the existing descriptor attachment route. A second
prepared program at that descriptor must have the same key and bytes as the
first, or preparation refuses before overwriting it. A two-descendant fixture
with different widths holds this refusal to both targets. Private CIDFont
descendant descriptors remain future work; this round does not claim that
composite isolation is implemented.

The broader lesson is a scope mismatch: local proof belongs to a font/program
pair, while the old write key belonged to a descriptor shared by several pairs.
A map insertion cannot silently substitute for that ownership decision. The
second verdict is essential, but should retain full findings rather than only
failed requirement IDs; its discarded evidence required temporary diagnostic
logging to recover the concrete font and codes in this investigation.

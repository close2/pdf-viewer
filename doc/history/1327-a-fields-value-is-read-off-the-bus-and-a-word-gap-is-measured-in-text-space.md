# 1327 — A field's value is read off the bus, and a word gap is measured in text space

The HOST-UI round of batch fifty-one. ADRs 1489 and 1490. No ledger row moved; no question.

**Value (ADR 1489).** The premise was half stale: a check box's state and a choice's selection
already crossed in `Control`; a text field's text did not. `AccessibilityNode::value` (a
`ShownValue`, `Answer::Fields`' reading) crosses the confined pipe through `encode_shown`; greeting
`PDFVCF05` → `06`. AT-SPI exposes no string value, so a text field and an editable combo get a
`TextRun` (`Text`), a choice its options as `ListBoxOption`s, chosen ones selected (`Selection`).
Census floor "fields with a value published" 545. The drive's golden is gone: `24-reopened` reads
`quorra`'s nodes off the bus, `1 2 3 True Blue`, as GTK's and Qt's.

**Text space (ADR 1490).** `separate_text` compared a user-space step with a text-space threshold.
Now the step goes back through `Tm`'s linear part and out of `Th`, forward by the sign of `Tfs`. Found
beside it: a font with no space got `0.25 × Tfs` as threshold, equal to its producer's `-250` gap
(`tracemonkey.pdf` read "Trace-basedJust-in-Time"); now 0.6 of a nominal quarter em, as a stated
space. A/B on one build: 1205 of 2809 pages in 109 documents change (spaces 494 913 → 447 862).
Word agreement unmoved; word-box judged set 503 → 509, pairs 11 131 → 12 513, floors raised; census
caret floors +269 elements, +260 lines, +5 characters. Fixtures: five in `content/text.rs`,
`headless.rs` three-ways under the mirroring `Tm` (3, old reading 2), drive `25-find-mirrored`.

**Driven** (release, Xvfb :127, lavapipe):

| windows | works | wrong | not offered | to look at |
|---|---|---|---|---|
| `quorra`, `quorra-confined` | 31 | 0 | 0 | 0 |
| `quorra-gtk`, `quorra-qt` | 59 | 0 | 3 | 0 |

**Gates.** fmt (12 files); clippy `pdf-model viewer-core viewer-accessibility viewer-confined pdf-vfs`
`--all-targets`: 0; nextest those five: 2274 passed, 0; `conformance`: 0; `text_extraction`: 0;
`selection_census`: 0; `accessibility_census`: 0; `launch_path`: 0.

**Left.** A field's characters have no positions (`doc/todo/31`); `issue1453.pdf`'s 0.14 em gaps in a
display face now read as one word — the quarter-em choice, not tuned.

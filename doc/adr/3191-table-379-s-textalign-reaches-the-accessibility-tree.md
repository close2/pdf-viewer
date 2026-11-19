# 3191 — Table 379's `/TextAlign` reaches the accessibility tree

Session 2122. Status: accepted. Rows: §14.8.5.4.3 (`implemented`; `/TextAlign` moves from
inapplicable to exported). Context: ADRs 2384, 3177; `doc/questions/A634`.

## Why it is in scope

A634 adopted Q634's recommendation and its owing line: "the accessibility node gains what Table 380
states and a sighted reader sees drawn", through the platform counterpart that exists, with "no
reflowing view". The question named Table 380, but its reasoning is the export's: the row's old reason
"exports to no format" decayed when `viewer_core`'s accessibility tree began handing the structure to
assistive technology. `/TextAlign` is "[t]he alignment, in the inline-progression direction, of text
and other content within lines of the BLSE" — a centred heading is drawn centred — and
`accesskit::TextAlign`, already in the tree at 0.24.1, carries it to AT-SPI's `justification`. No
question was needed: A634 decides it. The reflowing view stays out.

## Decision

- `pdf_model::structure::TextAlign` is the cell's four names; `Tree::text_align` reads the element's
  own statement under §14.8.5.4.1's owners.
- `viewer_core`'s walk carries it in `Inline` beside Table 380's attributes: "inheritable", plain, and
  an inline element's statement is passed over, because the entry "applies only to BLSEs containing
  text"; a block inside a span still inherits its block's value.
- `viewer_core::LineAlignment::of` turns start and end into left and right by Table 378's
  `/WritingMode` (§14.8.3.3: "inline from start to end"); under a vertical mode an edge maps to
  nothing, since no platform alignment names the top. `Center` and `Justify` cross in every mode.
- **Choice: no default.** Where nothing states the entry, nothing is set, although the cell's default
  is `Start`: the default parameterises a layout process this program does not run, and a heading
  its producer centred without stating it is not left-aligned text.
- `AccessibilityNode::alignment` crosses the confined protocol as one byte, and
  `viewer_accessibility` sets it on the element's runs, where Table 380's attributes already go.

## Cost and what is left

A public field on `AccessibilityNode` and a protocol byte. The C interface (`viewer-ffi`) does not
carry the alignment yet. Table 378's `/BackgroundColor`, which `accesskit` can also carry, is
§14.8.5.4.2's and is not built here.

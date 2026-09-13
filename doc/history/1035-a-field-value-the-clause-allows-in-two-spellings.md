# 1035 — a field value the clause allows in two spellings, and a refusal the parser outlived

Date: 2026-09-13. No ADR. Touched `variable_text.rs`, `action.rs`, `tests/variable_text.rs`, ledger.

**§12.7.5.3 says where a text field's value lives, and this tree read half the sentence.** "The
field's text shall be held in a text string (or, beginning with PDF 1.5, a stre am) in the V
(value) entry of the field dictionary. The contents of this text string or stream shall be used to
construct an appearance stream for displaying the field" — and `variable_text::value_text` matched
`Object::String` and an array of them, so a `/V` held in §7.9.3's text stream resolved to *no value
at all*: nothing drawn, nothing reported, every caller below answered with an empty string — trap
5's silence, inside a feature otherwise built.

**It hid where the row that owns the type could not see it.** §7.9.3's note censuses "exactly eight
entries in the standard typed `text string or text stream`", and it counts *table cells*: §12.7.5.3
states the type in prose while Table 226 types `/V` as `(various)`. Both rows now say so. **The
corpus states one, and only one**: over the 1452 curated documents that open, **2 of 2026 widgets**
resolve a `/V`, `/DV` or `/RV` to a stream, both on `AcroFormsBasicFields.pdf`'s
`LongRichTextField` — a Multiline, RichText `/Tx` whose `/V` and `/RV` are each a stream of the same
Lorem ipsum, `/RV` being XFA and still declined.

**No page moves today** (`raster_golden` held 974, moved 0): that widget has an `/AP` and no
`/NeedAppearances`, so the construction is reached by a regeneration — typing, §12.7.6.3's reset,
§12.7.8's import, or the flag. What changes without one is what a *host* is told, since
`field_text_value` feeds `form::fields`. The stream is decoded first and §7.9.2.2's prefixes looked
for in the result — §7.9.3's own *unencoded* — under the document's own `decoded_stream_data`
budget, a longer value staying `Owed::Truncated`. Calibrated (trap 13): without the
`Object::Stream` arm the new test fails with an empty inked-column list against the string
fixture's 37, and both fixtures were rendered and looked at (trap 1).

**§12.7.6.4's refusal of XFDF rested on a decision already taken.** Row and `action.rs` both said
"an XML parser is a dependency and a decision rather than a clause" — false since ADR 0186 took it
for §14.3.2 and made `xmlparser` this crate's own dependency, which `popup::rich_text` uses:
`doc/habits/the-ledger-and-claims-about-this-tree.md`'s third shape. What declines XFDF is the
standard — ISO 19444-1 is not on this disk, and principle 5 makes a grammar taken from another
reader not a reading. That sentence is the debt, bought rather than built.
**§12.7.5.3's `partial` is now one flag**: Table 231 bit 21's `FileSelect` names a submission,
which is §12.7.6.2's and `reported` there; bit 23 is a prohibition nothing here can break, and bit
26's `shall`s address the file. **§12.7.4.3's one reported edge has an empty population**: of
55 403 `/DA`s crawled and 2117 curated, 3 in each state a `Tm` and none of the six scales, rotates
or skews (calibrated on planted files).

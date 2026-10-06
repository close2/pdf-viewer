# 1578 — The `AF*` form library is re-implemented in Rust, and every rule of it is a choice

Status: accepted and **built**. Session 1371. Builds on RFC 0008 sections 2.8, 4.1 and 11 item 2,
accepted by the owner in `doc/questions/A193`; ADR 1579 is the dispatch that runs it.
Context: `CLAUDE.md` principle 5 ("Where the specification genuinely defines nothing … make a
deliberate choice, and document it *as a choice*"); `doc/todo/56`; `doc/third-party-data.md`.
Code: `crates/pdf-model/src/aform/` (`mod.rs`, `call.rs`, `number.rs`, `date.rs`, `mask.rs`,
`site.rs`). Fixtures: `crates/pdf-model/tests/aform.rs`.

## 0. Where the definitions come from — and the premise that did not hold

The brief asked for "every one Adobe's *JavaScript for Acrobat API Reference* lists". **It lists
none.** `doc/todo/56` measured it in August (no `AF*` heading in either page), and the reference
re-read at `adobe/dc-acrobat-sdk-docs` commit `ab3b42a7` agrees. Adobe's only statement about the
functions is the *Interapplication Communication API Reference*, Acrobat Forms plug-in,
`Field.SetJavaScriptAction`: an argument menu, no algorithm. What the JavaScript reference *does*
document is the machinery the functions are written over — `util.printd`'s place-holder table with
an example per row, `util.scand`'s two-digit-year horizon, `util.printx`'s masking characters and
its telephone example, `util.printf`'s `nDecSep` table and its `%.2f` of `Math.PI * 100`. Those
examples, and the menu's four time examples, are the fixtures marked *Adobe's example*; every other
expected value is one of the choices below. Both texts are cited by name and section, never quoted.
pdf.js's `aform.js` was read as evidence of the work's size; where it differs from a choice here,
the difference is ours on purpose (section 8).

The twenty-one functions built: `AFNumber_Format`, `AFNumber_Keystroke`, `AFPercent_Format`,
`AFPercent_Keystroke`, `AFDate_Format`, `AFDate_FormatEx`, `AFDate_Keystroke`, `AFDate_KeystrokeEx`,
`AFTime_Format`, `AFTime_FormatEx`, `AFTime_Keystroke`, `AFTime_KeystrokeEx`, `AFSpecial_Format`,
`AFSpecial_Keystroke`, `AFSpecial_KeystrokeEx`, `AFSimple_Calculate`, `AFRange_Validate`,
`AFMakeNumber`, `AFExtractNums`, `AFMergeChange`, `AFParseDateEx`.

## 1. Reading a value

`AFMakeNumber` reads a number only where the whole trimmed text is one: optional sign, digits with
at most one decimal point written `.` or `,`, optional exponent. No currency, no grouping — a field
holding `$1,234.50` states no number, because reading one means guessing which mark is the point.
**A format over a value it cannot read shows the value as it stands** (never blank): the format
describes values of its kind. `AFExtractNums` is the maximal digit runs, a leading `.`/`,` giving a
`0` run first. Rounding is half away from zero **on the shortest decimal digits**, never the
double: `1.005` at two places is `1.01`. A value that rounds to zero has no sign.

## 2. `AFNumber_Format` and `AFPercent_Format`

Separator styles 0–3 are `util.printf`'s `nDecSep` table; style 4, `1'234.56`, is Acrobat's dialog's
and documented nowhere — taken as the convention it is. Any other style, a fractional or negative
`nDec`, or `nDec` over 64 is refused by name rather than clamped. The negative styles are the
menu's names: `MinusBlack` a minus; `Red` red **with no sign**, since the name states a colour and
nothing else; `ParensBlack` parentheses; `ParensRed` both. Red is drawn by appending `1 0 0 rg`
after the `/DA`'s own operators; a non-negative value keeps the `/DA`'s colour. The currency string
goes before the digits or after them as `bCurrencyPrepend` says, the minus before the currency.
`AFPercent_Format` writes a hundredfold value, `%` after it or, with a true third argument, before.

## 3. Number keystrokes

Typing: the text must be a prefix of a number with one decimal point — `.` for the period styles,
whose grouping mark `,` is refused, and `,` or `.` for styles 2 and 3. Commit: a whole number or
empty. A comma style's committed value is stored with a period, which is why the comma styles also
take one: `script_corpus`'s first run found comma-style fields holding `5.25` (`bug1811510.pdf`,
`bug1918115.pdf`) that their own keystroke script refused when the value was read back for editing.

## 4. `AFRange_Validate`

Both bounds inclusive. An empty value and a value stating no number stand: a range judges numbers.

## 5. `AFSimple_Calculate`

`cFunction` is exactly `AVG`, `SUM`, `PRD`, `MIN` or `MAX`. `cFields` is an array literal or one
comma-separated string. A name names its field, or every terminal field below it. A value stating
no number is zero; no values at all is zero; the mean counts blanks. The result is rounded to
fifteen significant digits and written as ECMAScript's `Number.prototype.toString` writes it — `0.1 + 0.2`
is `0.3`, because a double carries fifteen digits faithfully and the sixteenth is binary showing.

## 6. Dates and times

A picture is a **template read left to right**: a fixed-width numeric place-holder followed directly
by another reads exactly its width (`yyyymmdd`), otherwise one or two digits (four for a year); a
month name is three or more letters of an English month; `tt` may be absent; any separator in the
picture matches any run of non-alphanumerics. Two digits under any year go through `scand`'s
horizon (documented); one or three refuse. No year in the picture reads against 2000, a leap year,
so `2/29` in an `m/d` field exists. An hour past twelve under `h` with no marker is a 24-hour hour.
Names are English — the reference's examples are. `j`/`jj` are refused (deprecated for XFA's
picture clause). `AFDate_Format`/`AFTime_Format` take the menu's index or a picture. **The two Adobe
texts disagree on the 12-hour marker** — the menu's examples write `PM`, `printd`'s table `am` — and
each function follows its own page: `AFTime_Format(1)` writes `2:30 PM`, `AFTime_FormatEx("h:MM
tt")` writes `2:30 pm`. Date and time keystrokes judge nothing while typing and the whole text at
commit.

## 7. Masks

`printx` stops at the first place-holder the source cannot fill, dropping literals pending since the
last filled one. The four special formats are the United States masks their names denote; phone
takes the area-code mask at ten digits. `AFSpecial_Keystroke` admits digits and the masks'
punctuation, at most the largest digit count while typing and exactly a complete count at commit,
keeping what was typed. The arbitrary mask's letters — `9`, `A`, `O`, `X` — are documented nowhere
and taken as convention; a commit that filled it without its literals has them put back.

## 8. Where pdf.js differs, read once and not followed

It blanks a value a number format cannot read, clamps `sepStyle` into 0–4, rounds a calculation to
six decimal places and falls back to `Date.parse`. Each would be a second, unstated rule; none is
taken.

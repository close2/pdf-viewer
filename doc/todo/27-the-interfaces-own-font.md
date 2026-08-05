# The interface's own font, and the text it drops in silence

Status: **found in the three-hundred-and-twelfth session, half of it answered.**
Priority: 27
Corpus: 45 documents state a popup and 6 of the 7 open ones are Chinese; the outline half is
unmeasured
Clauses: §9.6.2.2, §12.3.3, §12.5.6.14, §14.3.3
Code: `crates/viewer-ui/src/chrome.rs` (`Chrome::text`, `Chrome::width`)

## What it is

Everything this program draws for itself is set in §9.6.2.2's fourteen, compiled into the binary
(ADR 0133) — which is what makes an interface reproduce on a machine with no fonts installed, and
is the right default. `Chrome::text` walks a string, asks the face for each character's code and
**skips the character where there is none**. `Chrome::width` gives it no advance either, so a line
containing nothing the face can set measures zero and draws nothing.

That is correct for the case it was written for: a title being elided to a panel's width, where a
missing glyph is one character of a label a person can see the rest of. It is wrong for **text the
document states and this program is showing on purpose**, and the difference is trap 5's:

> a person shown an empty window has been told the note is empty.

## What is answered

§12.5.6.14's popup window counts what it could not set and says so, in the window
(`Chrome::without_a_code`, ADR 0191). Six of the corpus's seven open popups are in Chinese, so the
first thing this feature drew would have been six blank windows.

## What is not

The same silence is still there for every other string this host draws from a document:

- §12.3.3's outline titles — a Japanese document's outline is an empty tree of rows;
- §8.11.4.3's layer names;
- §7.11.4's embedded file names and descriptions;
- §14.3.3's `/Info` values and §14.3.2's XMP properties, both of which are document text.

**Unmeasured**, and the measurement is one sweep: how many corpus documents state an outline title,
a layer name or an `/Info` value with a character §9.6.2.2's Helvetica has no code for.

## What the fix is, and why it is not obvious

Three answers, in rising order of what they cost:

1. **Say so, everywhere** — the popup's answer, applied to rows. Cheap, honest, and it turns a
   panel of Japanese headings into a panel of apologies.
2. **Fall back to a face on the machine**, which is `pdf_font::substitute`'s whole subject. It
   works and it costs ADR 0133's argument: the interface stops looking the same on two machines,
   and every assertion about it becomes an assertion about which fonts this one has.
3. **Compile in a face with the coverage** — which is a licence question, a size question (a CJK
   face is megabytes against the standard fourteen's 804 KB) and a decision the project owner has
   not been asked for.

None is obviously right, which is why this is a todo rather than a patch. What is *not* acceptable
is what was there before: drawing part of a sentence, or none of it, without a word.

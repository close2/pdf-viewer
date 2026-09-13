# 1050 — A name §12.3.5.2 restricts is supported, and the support is said out loud

Session 1033. Status: **accepted**. Takes the choice §12.3.5.2 offers a processor, once, in the
place a host can reach. Changes `pdf_syntax::text_string`, `pdf_model::collection`,
`viewer_confined`'s panel codec, `viewer_host::panel` and `viewer_ui::chrome`.

`§N` is ISO 32000-2 and nothing else.

## 1. The sentence nobody had taken

§12.3.5.2 bounds a collection's names with five bullets and one further sentence, and then hands
the reader a choice:

> An interactive PDF processor may choose to support invalid names or not. If not, an appropriate
> error message shall be provided.

That is a conditional `shall` with exactly two lawful outcomes. Until this session `collection.rs`
read a folder's `/Name` through `pdf_syntax::text_string` and applied none of the six
restrictions, so the program was in the third state the sentence does not offer: it supported
invalid names *by never having looked*, which is not a choice and cannot be read back as one.
`is_file_name` existed and was called by nothing but its own tests — a capability that reached the
crate and never reached the program, which is `doc/habits/`'s fourth refusal shape.

## 2. The decision

**This program supports invalid names.** Deliberately, and here rather than at the point of any
operation.

The argument is that the alternative loses content the same clause requires to be shown. §12.3.5.1
makes presenting the collection a `shall`; §12.3.5.2 makes every key in the `/EmbeddedFiles` tree a
member of the folder structure — "all files in the EmbeddedFiles name tree … shall be treated as
members of the folder structure by an interactive PDF processor" — and states the remedy for a
*key* it cannot parse as placement at the root, never as refusal. A reader that dropped a folder
because its name ends in a full stop would answer a producer's mistake by hiding a producer's
document. The restrictions are what makes a name *valid*; the sentences that oblige a reader are
the ones about showing the files.

**And the support is stated.** The clause asks for no message once the choice is *support*, so the
sentence under the file list is this project's rule and not the standard's: a row saying `a:b` is
drawn as the document wrote it, and a person is told that is what happened rather than left to
wonder. `viewer_host::panel::restricted_names` is that sentence, once, for all three windows —
ADR 0711's reason for the rest of this clause.

## 3. What that makes the code owe

- **The six are applied, not just decidable.** `Collection::invalid_names` carries one entry per
  restriction per name, filled by `Collection::read`. An empty vector is the only thing that
  distinguishes a conforming collection from one nobody asked about, which is why the choice has a
  value attached rather than a comment.
- **The first restriction is about bytes.** "The string shall be a PDF text string" cannot be
  decided from a decoded `String`: §7.9.2.2's three encodings can each be contradicted by the bytes
  that claim them, and `text_string` answers U+FFFD for all three. `pdf_syntax::is_text_string` is
  the predicate, and it lives beside the decoder because §7.9.2.2 is a syntax clause.
- **The sixth is about two names.** "[T]wo file names in the same folder do not map to the same
  string following case normalization" puts subfolders and files in one namespace: the clause
  defines *file name* for "[a] folder, as well as its associated files", and Table 159's "Two
  sibling folders shall not share the same name following case normalization" is then a
  restatement rather than a second rule with a gap beside it.
- **`Special` is raised once per name.** The sentence restricts the name rather than each character
  of it, and one statement per name is what keeps the report proportional to the tree instead of to
  its content — a folder named with 255 solidi is one defect, not 255.

## 4. The one limit, stated rather than hidden

§12.3.5.2 sends case normalization to Unicode Standard Annex #21, whose caseless matching is
`toCasefold`. `case_normalised` uses `char::to_lowercase`, Unicode's full lowercase mapping, which
agrees with folding everywhere except the handful of characters that fold to a *different* string
from the one they lowercase to — U+00DF and U+017F chief among them. A pair it misses is a
collision reported as none, which is the safe direction for a program that supports the names
anyway; closing it means carrying `CaseFolding.txt` for one sentence of one clause, and that is a
dependency decision rather than this one.

## 5. What a later round must not re-litigate

The choice in §2. A round that finds an invalid name and wants to refuse it is reopening a decision
made against two `shall`s, and owes an argument against them rather than a patch. What is open is
§4's limit, and whether the sentence in §2's second paragraph should become one of `CLAUDE.md`'s
four levels — it is a statement about somebody else's file, and every such statement in this tree
is on its way to being a policy a host supplies.

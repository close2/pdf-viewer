# 1196 — A redaction enters a form, and copies what the page does not own

Status: accepted. Session 1179. Amends ADR 1126 §2, whose single-referrer guard refused a shared
image; it is now the test that decides *replace or copy* rather than *clear or refuse*.
Context: `crates/pdf-transform/src/redact.rs` (`Walk::resources`, `run_stream`, `run_form`,
`Frame`, `FormEdit`, `Owned`, `build_form`, `private_copies`, `privatise`, `reaches`,
`reaches_in`, `build_page`, `exclusively_owned`, `ImageClear::private`, `ClearedImage::private`),
`crates/pdf-transform/tests/redact.rs`.
Builds: ADR 1124, ADR 1126, ADR 0255 (a form's resource lookup), ADR 1195 (the path cut, which is
what a form's own content is usually edited *by*).
Clauses: ISO 32000-2 §12.5.6.23, §8.10.1, §8.10.2 (Table 93), §7.8.3, §7.7.3.3, §7.7.3.4.

## 1. A form is entered, and always

§8.10.1 makes a form XObject "a self-contained description of any sequence of graphics objects",
which is §7.8.2's content stream under another name. So the marks a form draws under the region are
described in *its* stream, and removing them is an edit of that stream rather than of the page's.

The walk therefore enters the form, with §8.10.1 step b)'s `/Matrix` in front of the transform at
the `Do` and Table 93's `/Resources` as what its names resolve in — the page's dictionary where it
states none, which is the direction §7.8.3's NOTE 3 gives and the reading ADR 0255 fixed. Nothing
else of the graphics state is reset, because §8.10.1 inherits it.

**It enters a form the region misses too, and that is not thoroughness.** ADR 1124 calibrates the
walk against the interpreter's placed-code count and refuses the page on any disagreement, and the
interpreter runs a form's content inline — so every code a form shows is in that count. A walk that
skipped the form refused **every page whose text is inside one**, for a reason that named nothing
real. `text_inside_a_form_clear_of_the_region_does_not_refuse_the_page` is that defect held.

Refused by name: a form whose content does not decode, one that draws itself (the walk would not
end), one nested deeper than the walk goes, and one the resources name directly rather than by
reference, since there is then no object to replace.

## 2. An object the page does not own is copied, never replaced

ADR 1126 refused a shared image, on the sound observation that overwriting an object's samples
changes bytes every placement of it shares. The refusal was the safe answer to the right question;
it was not the only one.

§12.5.6.23 asks for the content **the annotation identified** to be removed. An object drawn on the
redacted page *and* somewhere else carries marks the annotation did not identify, so destroying them
would remove content nobody asked for — the same failure the clause's own "clipping … shall not be
used" sentence is the mirror of, in the other direction. The answer that removes exactly what was
asked for is a **copy**: the removal is written into an object the redacted page's own resources
name, and the original stays byte for byte as the producer wrote it, holding the picture every
unredacted placement draws.

Nothing is hidden by this. The unredacted placement is visible in the output because the user did
not redact it; what is gone is the redacted page's copy of it.

`exclusively_owned` keeps its whole job and changes what it decides: an object referenced once,
reached by a resource path the page does not share, is the page's alone and is **replaced** — its
slot reserved before the closure walk, so the original is copied by nothing. Anything else is
**copied**.

## 3. Copying stops where it stops mattering

A copy is reached through dictionaries that are themselves shared — the `/XObject`
subdictionary first, a nested form's `/Resources` next — so carrying those by reference would point
every other page at the copy. `privatise` rebuilds exactly the objects that *reach* a copied one
and carries everything else by reference, so the output shares what it always shared. The redacted
page is then given a `/Resources` of its own (§7.7.3.3 permits it; §7.7.3.4's inheritance is what
`Page::resources` has already resolved), which is also how a page whose resources were an
ancestor's gets a private answer without rewriting the page tree.

`reaches` terminates on a cycle by remembering the chain it is inside, which is what a `/Parent` or
a form that names itself would otherwise do to it.

## 4. What measured it

`a_shared_image_is_copied_for_the_redacted_page`: two pages draw one image, one is redacted; the two
pages name two objects afterwards, the redacted page's left half is zero and its right half is the
producer's, and the other page's picture is exactly the producer's and still draws.
`a_shared_form_is_copied_for_the_redacted_page` is the same shape for a form, byte for byte.
`a_form_the_page_owns_has_its_own_content_redacted` is the replacement case, and
`a_path_inside_a_form_is_cut_in_the_form_s_own_space` proves the region arrives through the
`/Matrix` rather than beside it.

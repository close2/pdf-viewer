# 1285 — A preserve that moves nothing, and a profile that stops where it has nowhere to keep

Status: accepted and **built**.
Context: `crates/pdf-transform/src/archive/{in_place,config,mod,prepare,rewrite,report}.rs`,
`crates/pdf-archive/src/table/graphics.rs` (`reference_xobjects`),
`crates/pdf-transform/src/bin/quorra-transform.rs` (`--remedy-sites`),
`doc/profiles/keep-everything.toml`, `crates/pdf-transform/tests/{archive,profiles}.rs`.
Answers: `doc/todo/66`, the unbuilt answers `Configuration::unbuilt` printed for
`keep-everything` at PDF/A-2b, in the order it printed them.
Builds on: `doc/adr/1234` (the flag site's removal), `doc/adr/1245` and `doc/adr/1270` (the packet
site's two mechanisms), `doc/rfc/0007` section 4.6.1 (a page and an attachment are two mechanisms).
Clauses: ISO 32000-2 §12.5.3 (Table 167), §8.10.4.1, §7.3.8.1; ISO 19005-2 sections 6.3.2,
6.2.9.2, 6.8; ISO 19005-4 sections 6.3.2, 6.2.8.2, 6.9 and Annexes A and B.

## 1. `preserve` at the flag site shows the annotation

ISO 19005-2 section 6.3.2 requires an annotation's flags to set Print and clear Hidden, Invisible,
NoView and ToggleNoView, and offers nothing to write in place of flags that say otherwise.
`doc/pdf-a-conversion-limits.md` section 3.7 gives the document two futures and ADR 1234 built the
default one, removal. This builds the other: the annotation stays and its `/F` becomes
`pdf_archive::flags_permitting`'s value, so the five bits of Table 167 the requirement is not about
(NoZoom, NoRotate, ReadOnly, Locked, LockedContents) stay as the producer wrote them. The
population is the removal's own preparation, so an annotation the requirement passed is untouched.

**What it costs is on the page, and the report says so**: a mark kept off the screen or off paper
is now on both. `Conversion::shown_annotations` names each annotation with the flags it had and the
flags it has. Nothing is written into `xmpMM:History`: the permissions that make a record their
condition (`doc/questions/A55`, ADR 1014) are about content this program made or placed, and here
it placed nothing.

## 2. `preserve` at the reference XObject site keeps the proxy

§8.10.4.1 makes a form carrying `Ref` the thing a processor draws in the imported page's absence:

> This form XObject shall serve as a proxy that should be processed by a PDF processor when the
> referenced content is not available.

An archive holds no other file, so the proxy is what its reader draws already. The `Ref` entry
goes, the form stays with the producer's bytes, and `Conversion::proxied_references` names the file
and page Table 95 named. The cost is for a reader that *could* have reached the other file: it now
draws the proxy too. `pdf_archive::reference_xobjects` is the requirement's walk read as a list; a
form dictionary that is not a stream of its own refuses by name, since §7.3.8.1 makes every stream
indirect and a guess about which dictionary the finding meant would be this converter's.

## 3. The word is the whole answer

Neither site keeps anything *elsewhere*, so neither takes a `placement`: a row stating one is
`ConfigError::NotBuiltThatWay`, because a key read and ignored is the failure this format exists
to remove. `--remedy-sites` prints what the word does at each site.

## 4. Where a profile's mechanism cannot exist, the profile says `stop`

`keep-everything.toml` named an attachment at six sites for every target: the rollover and down
appearances, XFDF for a widget's actions, the XFA resource, and the three transfer-function rows.
ISO 19005-2 section 6.8 and ISO 19005-4 section 6.9 admit an embedded file only where it is itself
PDF/A, and ISO 19005-4's Annexes A and B lift that for 4f and 4e alone (XFDF for 4f alone,
`doc/pdf-a-mitigations.md` section 7). The profile's own rule is *keep it, else stop*, and its
comments already said *elsewhere this site stops*. So those rows are now `stop` unqualified and
the attachment is a 4f/4e row: the intent is kept where it can mean something, and the count of
unbuilt answers at part 2 no longer includes six answers no version could carry out.
`tests/profiles.rs` holds the property rather than a count.

**Not every attachment row went**: the three halftone rows keep their unbuilt `preserve`, because
the catalogue names a part 2 preserve for them — record the halftone in `xmpMM:History` — which is
buildable and unbuilt. Withdrawing those would have been hiding work, not stating a clause.

**The packet rows were already built**: `prefer = ["attach", "page"]` was the one key ADR 1245's
reader never read, so the rows counted as unbuilt. They are `original = "page"` unqualified and
`original = "attach"` at 4f and 4e, which is that preference written in the format as it is.

# 1233 — An answer that needs no authorisation is already carried out

Status: accepted and **built**.
Context: `crates/pdf-transform/src/archive/config.rs` (`Configuration::unbuilt`),
`crates/pdf-transform/src/archive/decision.rs` (`answered_without_authorisation`).
Answers: `doc/todo/66`'s done-condition — *a shipped profile produces no `does not carry out yet`
note at any target* — by removing the notes that were false.
Builds on: `doc/adr/1199` (the enumerability wrinkle a conditional answer opens), `doc/rfc/0007`
section 4.7 (what each remedy word means).

## 1. The note said a site stays refused, and at four sites it does not

`quorra-transform archive --remedy-sites --to <target> --config <profile>` prints, for every answer
a profile gives that this version does not carry out:

> "<site>" is answered with `discard`, which this version does not carry out yet

and the sentence the tool's own help attaches to it says *that site stays refused with the sentence
it names*. `Configuration::unbuilt` decided which answers get that note, and for a `discard` it
asked one question: is this site in the loss table? The loss table is built from `REMEDIES`' rows
whose answer is `Answer::Loses`, because a `discard` is exactly the authorisation such a row wants.

A row answered `Answer::Mechanical` is not in that table, and it is not in it for the right reason:
nothing about it needs authorising. It is also not in `--remedy-sites`' own listing, which lists
the sites that refuse. So a profile answering such a site with `discard` was counted as an answer
this version does not carry out **and** reported at a site the listing does not name — the
`N at a site the listing does not name` column, which `tools/state.sh remedies` prints and
`doc/todo/66` reads.

The four sites are `file-structure/permissions-dictionary-keys` (`Rewrite::ForeignPermissionHandlers`,
ADR 1007), `metadata/extension-schema-container-fields` (`Rewrite::ExtensionSchemaPrefixes`),
`metadata/xmp-packet-header-attributes` (`Rewrite::PacketHeaderAttributes`) and
`forms/no-needs-rendering` (`Rewrite::NeedsRendering`). Every one of them is a rewrite this version
performs on every document that fails the requirement, whatever the caller authorised.

## 2. What `discard` means decides the question

`doc/rfc/0007` section 4.7 gives the word one meaning: *do not stop the conversion here; I accept
what it costs*. At a row that costs nothing the instruction is satisfied a fortiori — the
requirement is met, the conversion proceeds, and there is no residue for a later version to build.
Reporting it as owed is not conservative, it is wrong in the direction that matters: it tells an
operator their archive will be refused when it will not, and it makes `doc/todo/66`'s
done-condition unreachable by counting work that does not exist.

So `Configuration::unbuilt` now asks a second question of a `discard`, and
`decision::answered_without_authorisation` is it: does this requirement have a row whose answer is
`Answer::Mechanical`, `Answer::Stated` or `Answer::CmykUnderPartTwo`? Each of those three is a
decision `decide` reaches without consulting `Authorisations` at all.

## 3. Three shapes are deliberately left named, and each for its own reason

- **`Answer::Loses`** is what a `discard` authorises, and the loss table already answers it. Nothing
  changes there.
- **`Answer::AsUnderlying`** routes a compound requirement to the rules it names, whose answers are
  their own. Whether a `discard` at the compound reaches them is a question about *those* rows, and
  answering it here would state a fact this predicate has not established. It keeps its name in the
  listing.
- **`Conditional::CallerSuppliesTheBytes`** is a `Mechanical` answer that waits on bytes no part of
  this program can fetch (`doc/adr/1199`). The rewrite loses nothing and still does not happen, so
  the site does stay refused and the note is true. It is excluded explicitly rather than by
  accident.

The other conditional shape, `Conditional::PerDocument(loss)`, was already carried into the loss
table by ADR 1209 and needs nothing here.

## 4. What this is not

It is not a change to what any conversion does. No document converts that did not convert before,
no rewrite is added, and no refusal is lifted: the only thing that moves is what the enumeration
says about a configuration's answers. That is why the evidence for it is the decision table read
against `decide`, and the guard against it becoming a way to make a number smaller is the shape of
the predicate — it names the three answers that reach `Decision::Mechanical` or `Decision::Stated`
without an authorisation, and a row that stops the conversion cannot enter it.

# 1199 — The bytes a stream points at are the caller's to fetch

Status: accepted and **built**. Session 1181.
Context: `crates/pdf-transform/src/archive/{external,prepare,rewrite,decision,report,mod}.rs`,
`crates/pdf-transform/src/bin/quorra-transform.rs`,
`crates/pdf-archive/src/table/{file_structure.rs,table.rs}`,
`crates/pdf-transform/tests/{archive.rs,archive_corpus.rs}`.
Answers: `doc/todo/66`'s external-stream-data family, and `doc/pdf-a-mitigations.md` section 2's
`file-structure/no-external-stream-data`, which this moves from *catalogued* to *built for the
shape this program can resolve*.
Builds on: `doc/questions/A54` (`apply` returns a request, the caller performs the act),
RFC 0002 section 9 (a pass is a pure function of its inputs), `doc/adr/1155` (what a name a
document wrote may reach), `doc/adr/0947` (nothing changes that no failed requirement asked to
change).
Clauses: ISO 32000-2 §7.3.8.2 (Table 5), §7.11.1, §7.11.3, §7.11.5; ISO 19005-2 section 6.1.7.1,
ISO 19005-4 section 6.1.6.1.

## 1. What the requirement forbids, and what Table 5 says the keys do

Both parts forbid a stream dictionary the keys that put its data outside the file, and the
requirement's own NOTE says the list is the keys that point at data external to the file. Table 5
is the base standard's side of it, and the construction falls straight out of three sentences: `/F`
is

> The file containing the stream data. If this entry is present, the bytes between stream and
> endstream shall be ignored.

`/FFilter` is "[t]he name of a filter to be applied in processing the data found in the stream's
external file", and `/FDecodeParms` the parameter dictionary "used by the filters specified by
FFilter". So embedding is one act with four parts: the external file's bytes go where the stream's
own were, `/Filter` and `/DecodeParms` become the `F`-prefixed pair — they are what described the
data now written — `/Length` is restated, and the forbidden keys go.

## 2. The third key has two spellings, and both are forbidden

Both parts spell the third key `FDecodeParams`. **That names nothing in ISO 32000**: Table 5's key
is `FDecodeParms`, and both parts' section 5.1 makes the base standard what a conforming file is
read against. Two readings were open and the purposive one wins on the requirement's own terms.

The rule forbids the **presence** of names, not the reaching of data — which is why it lists
`FFilter` at all, a key that is inert without `/F` in exactly the way the misspelling is inert
without a meaning. Read literally, a stream stating `FDecodeParms` would pass a requirement whose
NOTE says the list is about data external to the file, while carrying the one key that actually
holds an external file's decode parameters. So the validator reports **both** spellings: Table 5's
because it is the key the part means, and the part's own because the part forbids that name as it
wrote it. `pdf_archive::table::EXTERNAL_DATA_KEYS` is the one list, exported so the converter's
removal cannot drift from the judgement's population.

Nothing in the corpus states either spelling without also stating `/F`, so the added spelling moved
no verdict; `crates/pdf-archive/src/table/file_structure.rs` carries the reasoning and
`the_third_key_is_reported_under_the_spelling_iso_32000_gives_it` is the fixture.

## 3. The decision: the plan carries the bytes, and the CLI resolves what its own rule permits

Session 1175 left two shapes open — the caller resolves and hands the bytes in the plan, or the
`A54` two-pass tool request carries the fetch. **The first is built.** Three reasons, in order:

1. **It needs no declared program.** The catalogue said so and it holds: the command-line program
   already opens the document, so the whole of the addition is a switch and a rule.
2. **The rule is this program's rather than the operator's.** A tool handed a document-derived
   string is RFC 0007 section 4.5's sharpest case: the operator's fetcher decides what a document's
   own bytes may reach. `doc/adr/1155`'s rule — one path component, resolved against the directory
   the document is in — is a bound a reviewer can check once and rely on, which is what
   `CLAUDE.md` principle 3 asks of anything that reads untrusted input.
3. **`apply` stays pure either way, and that is the constraint neither shape may lose.** The fetch
   lives in the caller, the bytes reach `apply` as data in `ArchivePlan::external_data`, and the
   second pass is a function of its inputs — RFC 0002 section 9, the same seam
   `--output-intent-profile` sits on.

**The two-pass shape is the report's, not a second mechanism.** A conversion names what it needs in
`Conversion::external_data` — where each stream's data is, in §7.11's own two forms — exactly as it
names tool invocations in `Report::requested`; `quorra-transform archive` reads that, resolves what
the rule permits, puts the bytes in the plan and applies again, inside the loop `A54` already built.
So the operator types one command.

`beside_the_document` restates `doc/adr/1155`'s two sentences in the command-line program rather
than calling `viewer_host::policy::resolve_import`. That is a layer boundary rather than a
preference: `pdf-transform` is a batch job over documents no window has open, and its manifest says
one type of the viewer's vocabulary crosses and no more. What the two share is a *rule*, and it is
two sentences, stated in both places with this ADR's number beside it.

## 4. A filter key with no file names nothing, and needs no fetch

A stream stating `/FFilter` or `/FDecodeParms` and **no `/F`** has its data inside the file already.
Table 5 gives those keys meaning only through `/F` — they describe the filters of "the stream's
external file" — so a conforming reader decodes such a stream by `/Filter` and never consults them.
Removing them changes nothing any reader computes: the decision is `Mechanical`, no switch is
needed and nothing is resolved. This is `doc/adr/1176`'s shape — the standard has already decided
what the bytes mean, and the rewrite transcribes it — and it is half of what this requirement's
failures actually are.

## 5. What a URL is, and why it is named rather than resolved

§7.11.1 makes a file specification either a string or a dictionary, and §7.11.5 makes `/FS` `/URL`
the one file system PDF defines: under it the `/F` entry "is not a file specification string, but a
uniform resource locator". `ExternalData::AtUrl` is that case named as itself. Applying a rule about
path components to an RFC 3986 URI would be applying the wrong rule, and fetching one would be a
network operation this program does not have and `CLAUDE.md` principle 3 will not acquire. The
report says which form each stream used, the switch prints the refusal, and the conversion keeps the
requirement refused.

**The corpus says this is the case that matters, and the sweep counts it rather than this file.**
`archive_corpus` prints, per target, how each stream that keeps its data outside the file names it —
a plain file name this program's own rule would read, a file specification string of more than one
component, a URL, or no file at all. When this was built not one witness named a plain file: at
PDF/A-4 the three state `/FS /URL`, and at PDF/A-2b the three state `/FS` as a *string*, which
Table 43 makes no file system at all, leaving an `http:` URL to be read as a §7.11.2 string of
several components — refused by the rule either way. So the built route moved no corpus count, and
that is a fact about the producers rather than about the build.

**What is left, and it is the tool route, not a bigger rule.** `doc/profiles/keep-everything.toml`
answers this site with `tool = "resolve-external"`, and that row stays unbuilt deliberately: a
fetcher is the operator's program under the operator's trust, and it is what reaches the URL case.
The seam it would deliver into is the one this ADR built — the bytes land in
`ArchivePlan::external_data` whoever resolved them — so what that build adds is a request
population and a config word, not a second mechanism. Widening `beside_the_document` to reach a URL
is the thing this ADR rules out.

# The predefined CMaps, and where these bytes came from

ISO 32000-2 §9.7.5.2's Table 116 names the `CMap`s a PDF may reference by name without carrying
them, and the clause states the obligation plainly:

> A PDF processor shall support Adobe-CNS1-7, Adobe-GB1-5, Adobe-Japan1-7 and Adobe-KR-9 character
> collections.

and, as plainly, that the data is not in the standard:

> The CMap programs that define the predefined CMaps are available through a variety of online
> sources.

Until the hundred-and-fifty-sixth session this tree had none of them, so a font naming one was
refused and reported — thirteen documents of the pdf.js corpus. That was honest and it was not the
clause. This directory is the clause.

## What is here

239 files: every `CMap` Adobe publishes for the six character collections, so that the
`usecmap` chains inside them are transitively closed by construction rather than by a pruning
rule somebody would have to keep right.

| directory it came from | files | what it covers |
|---|---|---|
| `Adobe-CNS1` | 55 | Traditional Chinese |
| `Adobe-GB1` | 43 | Simplified Chinese |
| `Adobe-Japan1` | 92 | Japanese |
| `Adobe-Japan2` | 1 | Japanese, deprecated by ISO 32000-2 |
| `Adobe-Korea1` | 34 | Korean, deprecated by ISO 32000-2 |
| `Adobe-KR` | 14 | Korean, added in ISO 32000-2 |

12 MB of PostScript, which `crates/pdf-font/build.rs` deflates one file at a time into a 1.5 MB
blob and an index. **Nothing is decompressed at startup**; a document that names
`90ms-RKSJ-H` inflates that one entry and touches no other.

The two deprecated collections are here because deprecation is a statement about what a *producer*
should write and this program reads what exists: `UniKS-UCS2-H` names Adobe-Korea1 and three corpus
documents use it.

## Where they came from

`/usr/share/poppler/cMap/`, from Arch Linux's **`poppler-data 0.4.12-2`**, which redistributes
Adobe's own files unmodified. The licence beside them there is `COPYING.adobe`, copied here
verbatim as `LICENSE_ADOBE`, and **every file also carries the same notice inline** in its own
`%%Copyright` lines, which is how Adobe published them.

**`poppler-data` is two data sets under two licences and says so.** The `cMap` directory is
Adobe's, BSD-3-Clause. The other half — `cidToUnicode`, `nameToUnicode`, `unicodeMap` — is Glyph &
Cog's under GPL-2-or-3, and **none of it is here**. What that half would have answered is §9.10.2's
CID-to-Unicode question, and Adobe's own `Adobe-CNS1-UCS2`, `Adobe-GB1-UCS2`, `Adobe-Japan1-UCS2`
and `Adobe-KR-UCS2` answer it under the permissive licence; they are in this directory and
`pdf_font::predefined::cid_to_unicode` is what reads them.

They are copied rather than read from the system at build time, for the reason
`data/standard-fonts/PROVENANCE.md` gives about the submodule: a page's appearance must not be a
property of which packages happen to be installed on the machine that built the binary.

## Checking them

`SHA256SUMS` is a digest of each file as committed. `cd data/cmaps && sha256sum --check
SHA256SUMS` verifies that what is here is what was vetted. As with the fonts, it is not a security
boundary — anyone who can change the data can change the sums — it is a record of which bytes were
read, licensed and measured.

## Why the files as they are

`pdf_font::cmap::CMap::parse` already reads this syntax, because §9.7.5.4's *embedded* `CMap`
streams are written in it, and the `-UCS2` files state `/CMapType 2` and use `beginbfchar`, which
`pdf_font::tounicode::ToUnicode` already reads because that is §9.10.3's `/ToUnicode` form. So
Adobe's bytes go in unconverted: no second format to keep in step with the first, and no
opportunity to mistranslate a mapping while compacting one.

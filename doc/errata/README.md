# The errata sources, held

Everything in this directory except this file and `pdfa.md` (this tree's list of the PDF/A errata) is fetched by `tools/fetch-errata.sh`, and
`MANIFEST.tsv` says, for each file, its SHA-256, the URL it came from and the day it was fetched.
Nothing here is edited by hand. `tools/fetch-errata.sh --check` fetches again and fails, naming the
file, when a hash has moved; the post-merge train runs it daily, so an erratum published or a
resolution recorded after this was written is noticed rather than missed (ADR 3583).

**Why this is held at all.** Where an erratum agreed by the working group that owns a standard —
published on pdf-issues.pdfa.org, or recorded as agreed in `pdf-association/pdf-issues` — corrects
a clause, the corrected text is the standard and the printed one is retired (`CLAUDE.md` principle 5,
ADR 3581). A source this project reads the standard through belongs on its own disk.

## Attribution

The content is the PDF Association's, from
[github.com/pdf-association/pdf-issues](https://github.com/pdf-association/pdf-issues), the
source of [pdf-issues.pdfa.org](https://pdf-issues.pdfa.org/), and is held under that repository's
licence, Creative Commons Attribution 4.0 International (CC-BY-4.0), whose text is
`site/LICENSE`. The issue and comment texts are their authors' contributions to that repository.
The files are unchanged except that the issues and comments are reduced to the fields named below
and written one record a line.

## What is here

| file | what it is |
|---|---|
| `site/` | the Jekyll source of pdf-issues.pdfa.org at the commit `MANIFEST.tsv`'s URLs name: every Markdown page under `docs/`, the site's `_data/data.yml`, the grammar files, and the repository's README, licence and publication process. The figures are not held. |
| `issues.jsonl` | every issue and pull request of `pdf-association/pdf-issues`, open and closed, one a line: number, title, state, labels, author, dates, URL, body |
| `comments.jsonl` | every comment on them, one a line, keyed by issue number: where a working group's resolution is recorded |
| `MANIFEST.tsv` | path, SHA-256, source URL, fetch date |

## Which standards, and which this tree holds

The site publishes errata pages for these documents (`site/docs/<directory>/`). The standard's
own text is in `doc/` for the first group only.

| site directory | document | held in `doc/` |
|---|---|---|
| `32000-2-2020` | ISO 32000-2:2020, PDF 2.0 | yes (`ISO_32000-2_sponsored_EC3.pdf`, `doc/md/`) |
| `19005-4-2020` | ISO 19005-4:2020, PDF/A-4 | yes (`doc/pdfa/`) |
| `14289-1-2014` | ISO 14289-1:2014, PDF/UA-1 | yes |
| `32001-2022` | ISO/TS 32001, hash algorithm extensions | yes |
| `32002-2022` | ISO/TS 32002, digital signature extensions | yes |
| `32003-2023` | ISO/TS 32003, AES-GCM | yes |
| `32005-2023` | ISO/TS 32005, PDF 1.7 and 2.0 inclusion rules | yes |
| `wtpdf-1` | Well-Tagged PDF 1.0 | yes |
| `pdf-declarations` | PDF Declarations | yes |
| `16684-2-2014` | ISO 16684-2:2014, XMP RELAX NG schema | no (part 1 only) |
| `21757-1-2020` | ISO 21757-1:2020, ECMAScript for PDF | no |
| `14739-1-2014` | ISO 14739-1:2014, PRC | no |
| `15930-9-2020` | ISO 15930-9:2020, PDF/X-6 | no |
| `16612-3-2020` | ISO 16612-3:2020, PDF/VT-3 | no |
| `32007-2024` | ISO/TS 32007, RichMedia glTF assets | no |

**ISO 19005-2, which `doc/pdfa/` also holds, has no errata page on the site.** Its corrections
are found in the issues: PDF/A issues are labelled by part, and where an erratum to part 4 restates
a sentence part 2 prints in the same words (#77's supplement comparison is the instance), the
working group's resolution names both.

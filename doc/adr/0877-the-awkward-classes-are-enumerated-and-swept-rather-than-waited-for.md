# 0877 — The awkward classes are enumerated and swept rather than waited for

Session 917. Status: **accepted**. The second of this round's two records: an instrument that puts
a document of every awkward class through the confinement on purpose, because the three deaths this
tree has had were each found by somebody taking a path no test had taken.

## Context

`crates/pdf-vfs/tests/awkward_classes.rs`, new. The argument for it is a pattern rather than a
requirement:

| session | what died | why nothing saw it |
|---|---|---|
| 902 | `available_parallelism` reading `/proc/self/cgroup` | the render path was not taken confined |
| 911 | `glibc` sizing an arena from `/sys` on a pool thread (ADR 0865) | a second image on a page put a second thread in the pool |
| 914 | `pdf_font::substitute` walking `/usr/share/fonts` (ADR 0870) | the four committed fixtures embed their fonts |

Each was found by a person doing something else. ADR 0876 is the same defect a fourth time, found
by a fourth person doing a fifth thing, and misattributed to encryption on the way. The population
that produces these is not a clause and not a corpus: it is **the set of documents that make the
worker do something rare**, and nothing enumerated it.

## Decision

**Enumerate the classes, derive a population for each from every corpus on the disk, and walk the
whole layout of each through the confined transport, asking only whether the worker survives.**

Ten classes, of which the first nine are awkward and the tenth is the control: encrypted (opens
under §7.6.4.1's default user password), locked (does not), an encryption this reader does not
implement, pageless, damaged (the cross-reference table was rebuilt by scanning), unopenable, huge
(a hundred pages or eight mebibytes), `/JBIG2Decode`, `/JPXDecode`, and plain. The vocabulary is
`safedocs::survey::Outcome`'s for the five it already has, which is why those five are not
re-argued here.

Three properties are the design, and each is a trap this tree has paid for:

- **The population is derived, never named** (trap 25). Every corpus root on the disk — `doc/pdf.js`,
  the four `doc/corpora` submodules, the three `corpus-cache` collections — is sampled at a fixed
  stride, each sampled document is classified by *opening* it, and each class takes the first few
  that fall into it, per root, so that one large collection cannot fill every bucket. A class no
  corpus fills prints as empty rather than passing silently. A document is in as many classes as it
  satisfies: an encrypted, damaged, thousand-page scan states three things about itself.
- **The tree is walked from `/` rather than from a list.** Every directory is listed, the first two
  of its entries are `stat`ed and read, and a directory recurses — so a row added to
  `pdf_vfs::layout::LAYOUT` is swept without this file being edited. Two entries per directory is
  what keeps the *huge* class affordable: a `stat` generates (RFC 0003 §5.5), and session 911
  measured `ls -l pages/` on a 1023-page document at 2 min 45 s.
- **The question is survival, not agreement.** A refusal is counted by reason and printed (trap 11);
  what fails the run is a `WorkerError::Transport` whose sentence names a signal. The mount is then
  asked one more question, because session 902's recovery — a dead worker is thrown away and the
  next operation gets a fresh one — is a claim until something exercises it.

### Why this is not `tests/read_corpus.rs`, and how the two should end up

They ask different questions over different populations and the difference is worth one sentence
each. **`read_corpus.rs` (ADR 0871) asks whether the two transports agree**: every file of the
layout, over the 974 `doc/pdf.js` documents, held byte for byte against the generator
`crate::layout` names. **This asks whether the worker survives at all**, over a population drawn
from all eight corpus roots — 65 944 SafeDocs files and 23 075 from the Tika tracker among them,
which is where damaged, huge and JPEG 2000 documents actually live and which the pdf.js corpus
under-populates.

The honest reading of that is that **one instrument should eventually do both**, and the merge is
`read_corpus.rs`'s: widen its population beyond `doc/pdf.js` and its byte comparison covers these
classes too. What stops that today is cost — 974 documents already take 324 s at sixteen pages
each, and the byte comparison needs the *same* plan computed in-process for every file — and cost
is a reason to keep two instruments, not to keep two designs. `doc/todo/58` §4 carries it so that
the round that widens the walk deletes this file rather than inheriting it.

## What it found

```
vfs-awkward: 8 root(s), 3916 document(s) classified, 258 chosen
vfs-awkward:   encrypted                33 document(s), 306 answered (32.8 MiB), 0 refused, 0 killed
vfs-awkward:   locked                   12 document(s),   0 answered,            12 refused, 0 killed
vfs-awkward:   encryption unimplemented  2 document(s),   0 answered,             2 refused, 0 killed
vfs-awkward:   pageless                  9 document(s),  45 answered,             0 refused, 0 killed
vfs-awkward:   damaged                  39 document(s), 360 answered (33.2 MiB),  0 refused, 0 killed
vfs-awkward:   unopenable                8 document(s),   0 answered,             8 refused, 0 killed
vfs-awkward:   huge                     33 document(s), 354 answered (89.7 MiB),  6 refused, 0 killed
vfs-awkward:   jbig2                    27 document(s), 258 answered (38.0 MiB),  3 refused, 0 killed
vfs-awkward:   jpeg 2000                32 document(s), 284 answered (74.6 MiB), 11 refused, 0 killed
vfs-awkward:   plain (control)          63 document(s), 567 answered (58.8 MiB),  3 refused, 0 killed
vfs-awkward: killed: 0, did not recover: 0, in 25.0s
```

**Nothing dies**, and the 44 refusals are all four of the shapes the design expects: twelve
passwords, nine documents that are not PDFs or have no usable cross-reference table, two
encryptions §7.6 states and this reader does not implement (`/Encrypt` not a dictionary; `/Filter
/Adobe.PubSec`), and twenty-one pages past the walk's own pixel ceiling — one of them asking for
3 678 693 350 pixels. Every one of those is a sentence a face can show, which is what
`doc/todo/58` §4's fourth requirement asks for.

## Trap 13: the sweep was run against the defect before it was believed

A sweep that finds nothing has proved nothing until it has been shown finding something.
`pdf_vfs::confine`'s `no_machine_fonts()` line was commented out, `pdf-vfs-worker` rebuilt, and the
same sweep run again over the same 258 documents:

```
vfs-awkward: killed: 76, did not recover: 0, in 25.9s
```

and the kills land in **six of the ten classes**: huge 26, damaged 16, plain 14, jbig2 8, encrypted
6, jpeg 2000 6. That distribution is the strongest single statement this round produced. **The
control class has more kills than the encrypted class.** ADR 0876's misattribution was not a
careless reading of one document; it is what any reading of one document produces, and the only
cure is a population wide enough for the classes to disagree.

The `did not recover: 0` beside 76 deaths is session 902's recovery measured for the first time:
every one of those mounts answered again afterwards.

## Consequences

- One `--ignored` line in `doc/todo/02` §2, under the `pdf-vfs` `--bins` build that trap 10 makes
  mandatory, and a sentence in the map beside `read_corpus`'s.
- **The recovery check was wrong on its first writing and is worth recording as such** (trap 11).
  It asked for `Vfs::pages()` to answer `Ok`, and reported eleven failures that were a locked
  document, an unopenable one and an encryption this reader does not implement — each of which
  answers `Err` for ever and is right to. What it asks now is that the answer is not a *corpse*.
- The classification opens 3916 documents to choose 258, which is most of the run's 30 s. It is
  bounded by `SAMPLE_PER_ROOT` and it is deliberately a stride rather than the first N: a corpus
  directory's first hundred names are one contributor's and one generator's.

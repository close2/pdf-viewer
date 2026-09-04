# 0879 — The other confined program is swept, over the same population, and the population is a crate

Session 919. Status: **accepted**. The second of this round's two records: `pdf-view-worker` gets
the sweep `pdf-vfs-worker` has had since session 917, and the two share one population because two
would drift.

## Context

There are **two** confined workers in this tree, and they are the same posture over different work:
`pdf_sandbox::lockdown::Profile::Interpreter` and `confined-transport`'s supervision, around
`pdf-vfs`'s generators in one case and `viewer-core`'s whole viewer in the other. A system call one
is killed for is one the other is killed for — session 914 found `pdf_font::substitute` walking
`/usr/share/fonts` and had to state `no_machine_fonts()` in **both** `confine()`s (ADR 0870).

What the two did *not* share was measurement. `pdf-vfs`'s worker has had a corpus walk since
session 914 and a class sweep since 917; `pdf-view-worker` has had four committed fixtures and one
probe. `doc/todo/61` §2 recorded the asymmetry and named the consequence: session 914's sentence
that "the confined viewer loses the page rather than a glyph" was **read off the code**, not
measured.

## Decision

**`crates/viewer-confined/tests/awkward_classes.rs`**: the same ten classes, from every corpus root
on this disk, opened as a descriptor (ADR 0812 — which is what a host does with a file on disk) and
drawn through `pdf-view-worker`, with three page turns and a frame asked for afterwards. What fails
it is a death and nothing else; a locked document, a page past a budget and an image whose codec is
a program the confinement forbids are each counted and printed, because a refusal is a sentence a
host can show (trap 11).

**And the population is `crates/corpus-classes`, a crate.** Session 917's classifier — the roots on
this disk, the fixed stride, the ten classes, and the sentence that tells a death from a refusal —
was a hundred and eighty lines inside one test file. Two sweeps needing it is what makes it a
crate rather than a copy:

- a copy is two populations, and then a difference between the two sweeps is a difference between
  their samples rather than between their workers, which is the one thing this pair is for;
- `test-scenes` is the precedent and the argument is the same one it makes: "if each backend's
  tests built their own scenes … a difference could just as easily mean the scenes differed as that
  a backend was wrong";
- it is a dev-dependency of `pdf-vfs` and of `viewer-confined` and a dependency of neither, so
  nothing a person runs takes it.

`is_a_death` lives there too, which is the one judgement both sweeps make and the one place either
could have got it wrong on its own: `confined_transport::supervision` words a signal death as
`killed by signal N`, and the predicate is on that sentence rather than on an error variant, because
the two sweeps meet it through different types (`pdf_vfs::VfsError` and
`viewer_confined::ConfinedError`) and through whichever question was being asked when the worker
went.

## What it found

Nothing dies:

```
view-awkward: 8 root(s), 3916 document(s) classified, 198 swept, 24 threads
view-awkward:   encrypted 28 documents, 168 answered, 28 frames … 0 killed
view-awkward:   damaged   39 documents, 234 answered, 32 frames, 16342 reported, 0 killed
view-awkward:   huge      30 documents, 180 answered, 31 frames … 0 killed
view-awkward:   plain (control) 48 documents, 288 answered, 48 frames … 0 killed
view-awkward: killed: 0, in 13.5s
```

Two things in that output are worth keeping. **The reports are the viewer's own sentences**, and
16 342 of them fall on 39 damaged documents — a broken cross-reference table rebuilt by scanning,
an unimplemented operator, a font program whose Adler-32 disagrees with its bytes — which is
§9.10.2's and §7.5.7's machinery doing its job through the confinement rather than a defect. And
**a locked document is answered rather than refused**: `Event::PasswordRequired` crosses the
boundary as the event it is, so the sweep counts ten refusals and no deaths where a person would be
asked for a password.

## Trap 13

`no_machine_fonts()` commented out of `viewer_confined::worker::confine`, `pdf-view-worker` rebuilt,
the same 198 documents:

```
view-awkward: killed: 28, in 13.5s
```

in **six of the ten classes**: huge 10, damaged 5, plain 5, jbig2 3, jpeg 2000 3, encrypted 2. The
same shape ADR 0877 recorded for the other worker — the *control* class is not the safe one — and
the same lesson: one document cannot say which of its properties killed a worker.

## Consequences

- **It is not a `doc/todo/02` §2 gate, deliberately.** `pdf-vfs`'s read walk asks a wider set of
  questions of the same class of defect every round, and a second corpus-scale line would cost
  every future round for a narrower question. It is in `doc/verify.md` as the run a round that
  touches the confinement, the interpreter's dependencies or `pdf-sandbox`'s allow-list owes.
- **Trap 10 is one line above it**: `cargo build --profile gates -p viewer-confined --bins`, because
  a `--profile gates --test` line builds one test target and nothing else, and a sweep that cannot
  start a worker would report every document as answered. It says so rather than passing.
- `doc/todo/61` §2 is closed by this and §1 by ADR 0878; what is left of that item is the standing
  rule, which is that a round taking a new dependency into either confined worker runs both sweeps.

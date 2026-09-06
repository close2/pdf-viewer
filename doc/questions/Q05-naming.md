# Q05 — Naming: the application, the library, and how many executables

Source: RFC 0002 §13 question 6, first asked 2026-09-03. **Answered 2026-09-06 in
`A05-naming.md`, and the owner marked that answer "not yet a qualified answer" and asked for the
question to be put again with their text and our response in it.** This is that second asking.
Status: **open** — answered when `A05-naming.md` says so, replacing what it says today.

## What the owner said

> We will rename the whole app to quorra.
>
> The currently named quorra library should be renamed (my first instinct was to use quorra-xxx;
> i.e. "quorra-" as prefix; but I am open to suggestions).
>
> I don't want to create too many exes. So unless there is a good reason, I think we should make
> subcommands, like `quorra transform`...
>
> This is not yet a qualified answer. integrate this into the question, and let me decide again.

## What the tree looks like today, so the decision is priced rather than guessed

**Executables that exist**: `pdf-viewer`, `pdf-viewer-confined`, `pdf-transform`, `pdf-retrieve`,
`pdffs`, `pdf-vfs-worker`, `pdf-sandbox-worker`, `pdf-view-worker`, plus the conformance and
corpus tools under `tools/`. Twelve binaries and two libraries are installed by `doc/todo/02` §5.

**They are not all the same kind of thing**, and that is the first sub-question:

- **Three are user-facing verbs** — the viewer, the transform suite, the retrieval tool — and those
  are what "too many exes" is about. Subcommands fit them.
- **Four are workers a broker spawns**, never typed by a person: the sandbox worker, the view
  worker, the vfs worker, and the confined viewer's own. They are found by path at run time
  (`worker_program`), and a stale one beside a build has now cost six rounds a wrong measurement.
  Folding these into subcommands of one executable is possible and would *reduce* that trap, but it
  makes the sandboxed process and the unsandboxed one the same file, which is a security-relevant
  change and wants its own argument.
- **One is a mount helper** (`pdffs`), which by convention is named for the filesystem it mounts and
  is invoked by `mount` as well as by hand.

**What a rename touches**: 26 crate names and their directories, both workspaces' manifests, the C
interface's symbol prefix (`pdfv_*`) and its header, the KIO plugin's protocol registration and its
installed filename, `pdffs`'s mount type as it appears in `/proc/mounts`, the environment variables
(`PDF_VFS_MACHINE_FONTS`, `PDFVIEWER_LAUNCH_CLOCKS`, and eight more), every `doc/` path that names
a crate, and the other repository, which is currently *called* quorra.

**The collision is the awkward part.** The rendering library at `/home/cl/projects/render-lib` is
`quorra` today, on crates.io-style names `quorra-gpu` and `quorra-scene`, and this tree depends on
it by those names. If the application takes the name, the library must give it up, and the owner's
own instinct — a `quorra-` prefix — is the shape that *keeps* the collision rather than resolving
it, because `quorra-gpu` would then read as a component of the application.

## Our response, and a recommendation

**On the executables: yes to subcommands for the three user-facing tools, no for the workers.**
`quorra view`, `quorra transform`, `quorra retrieve` is one executable a person types. The workers
stay separate files, because a broker that spawns *itself* with a different argument is a
well-known way to lose a confinement boundary by accident, and because the whole point of
`pdf-sandbox-worker` is that it is a different program with a different system-call policy. That is
the "good reason" the owner's sentence invites. `pdffs` keeps its own name for the mount convention
and can also be `quorra mount`.

**On the library's name, three options rather than a prefix:**

1. **A different character from the same film.** The library is a rasteriser; the application is a
   reader. `quorra` was a good name for the library and would be a good name for the application,
   and the cheapest resolution is that the library takes another name of the same flavour — for
   instance `flynn`, `clu`, `sark`, `bit`. No prefix, no collision, and the two projects stay
   visibly related without one reading as a part of the other.
2. **A name that says what it does** — `raster`, `vello`-shaped, e.g. `tessera`, `scanline`. This
   is what an outside reader would find clearest, and it gives up the family resemblance.
3. **The owner's `quorra-` prefix**, which we would rather not recommend for the reason above: once
   the application is `quorra`, `quorra-gpu` reads as the application's GPU crate rather than as an
   independent library.

**Sub-questions the owner may want to answer separately**, because they can be decided apart:

- (a) Does the application's *crate* prefix change too — `pdf-model` → `quorra-model` — or do the
  internal crates keep their descriptive names and only the product is renamed? Renaming 26 crates
  conflicts with every open branch; renaming only the binaries is a day's work.
- (b) Does the C interface's symbol prefix change? It is a frozen ABI with drift tests, and renaming
  it breaks every existing host.
- (c) When? This is a mechanical change that touches nearly every file, so it wants a moment with no
  other work in flight, not a slot beside three other rounds.

**Recommendation**: subcommands for the three verbs and separate files for the workers; the library
renamed to another name of the same family rather than to a prefix; the internal crate names left
alone for now and only the product, the binaries and the documents renamed; the C prefix left alone
until someone asks for it.

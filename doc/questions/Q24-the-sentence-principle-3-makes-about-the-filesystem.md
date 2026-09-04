# Q24 — The sentence principle 3 makes about the filesystem, now that the port exists

Asked by session 920, which built `doc/todo/59`'s resource port. **Provisional, not a blocker**:
the port is built, measured and shipped off-by-default, and nothing waits on this answer except the
wording of one bullet.

## The question

`CLAUDE.md` principle 3's second bullet reads, today:

> - Multi-process sandbox: renderer runs unprivileged under seccomp-BPF + Landlock, with no
>   filesystem and no network access.

`doc/todo/59` §5 says an amendment to it is owed "**as a clarification of what the broker may do,
not a weakening of what the worker may** — once the port exists". It exists. **What exact wording
does the owner want?**

The proposed replacement, for acceptance or redrafting:

> - Multi-process sandbox: renderer runs unprivileged under seccomp-BPF + Landlock, with no
>   filesystem and no network access. **It may be *given* things, and that is not the same
>   sentence**: the broker — which already opens the document and passes it across as a descriptor
>   — may answer a description the renderer sends with a resource it opened itself. The renderer's
>   system-call set does not change, no host may change it, and the renderer can still name no path.
>   Every such port is off by default and turned on by a host, never by a document.

## Why it cannot be settled without the owner

Three reasons, and the first is the one that matters:

1. **The sentence is the owner's own statement of the boundary**, and it is the sentence every
   future round will read when it is tempted to widen the allow-list. `doc/todo/61` exists because
   that temptation has arrived four times already. A round that rewrites the boundary's own wording
   to accommodate the thing it just built is exactly the failure that item is about, whatever the
   round's reasoning was.
2. **"No filesystem" is currently true without qualification, and the amendment makes it a sentence
   with a clause.** That is a real loss of sharpness even if the property it describes is unchanged,
   and whether it is worth the accuracy is a judgement about the document rather than about the code.
3. The owner asked for the layer in the words "a clean layer, which every implementation must (can?)
   overwrite"; whether the principle should mention the layer at all, or stay silent and let
   `doc/todo/59` and ADR 0880 carry it, is the same judgement.

## What the tree does meanwhile

**Principle 3 is unamended.** This round did not touch `CLAUDE.md`, on the item's own instruction and
the owner's sentence behind it. What is true of the tree today, and is what the wording above would
describe:

- the allow-list did not move — `git diff crates/pdf-sandbox/` for session 920 is empty, and the
  worker's traced system calls after the filter is installed are a subset of the ones it already
  made;
- the port is off in every host: `pdffs` needs `--machine-fonts`, `pdf-viewer-confined` needs
  `--machine-fonts` or `PDF_VIEWER_MACHINE_FONTS=on`, the KIO face needs `PDF_VFS_MACHINE_FONTS=on`,
  and a host that says nothing gets the worker session 914 left;
- a worker whose host offers nothing still renders, still substitutes from the compiled-in faces,
  and still reports the shortfall under §9.10.2.

So a reader of `CLAUDE.md` today is told the renderer has no filesystem, and that is still exactly
true of every default configuration.

## Recommendation

**Take the wording above, or a shorter form of it.** The bullet should say something, because the
gap between "no filesystem" and "may be handed a file the broker opened" is precisely where a future
round would argue itself into `openat`, and a principle that does not name the port leaves that
argument open. If a shorter form is wanted:

> - Multi-process sandbox: renderer runs unprivileged under seccomp-BPF + Landlock, with no
>   filesystem and no network access. What it may be *given* by its broker — a document, a face — is
>   a port a host opens, never a capability the renderer holds.

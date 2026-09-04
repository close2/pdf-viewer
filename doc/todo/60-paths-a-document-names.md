# 60 — Paths a document names: external file specifications, and the tier that must ask

Status: **accepted** by the project owner on 2026-09-04, as the second half of the resource
question ([59](59-the-resource-port.md)) — *"GUIs could either have a setting, or ask the user"*.
Priority: 50-band. Companions: [59](59-the-resource-port.md), `doc/todo/38` (the four levels),
ADRs 0874 and 0875 (the consult/consent round trip that already crosses the confinement).
Clauses: §7.11.2 file specifications, §7.11.3's `/F` naming a file **outside** the document,
§7.11.4's embedded alternative, §7.3.8.2's external streams, §12.5.6.15's file attachment
annotation, §7.6.4.2 Table 22.

## The distinction this item exists to hold

[59](59-the-resource-port.md) is about a resource the document names as a **hint** — a family name
that some face may satisfy. This item is about a path the document **states**, which a reader would
then open. They look alike and they are not alike: the first cannot be aimed, and the second is
aimed by whoever wrote the file. Making a reader open a stated path is the oldest trick in the
format, and it is why this tier's default is off and its answer is a question rather than a
setting.

## What binds

- **Default off**, and the refusal names the clause and the path rather than failing quietly.
- **When on, it is asked** — per document, through the machinery sessions 916 built:
  `pdf_transform::consult` out and `Query::Consented` back, so the policy is still asked once in
  `pdf_model::restriction::decide` and the *question* is what crosses the confinement.
- **The broker opens, never the worker**, exactly as in [59](59-the-resource-port.md): the worker
  receives a descriptor or a refusal, and the path resolution happens in code that is not parsing
  untrusted bytes.
- A host with no way to ask gets `Refusal::Unanswered` and says so, which is what `pdf-fuse`
  already does for the restriction levels.

## What is owed

1. A census first: how many corpus documents state a `/F` with no `/EF`, an external stream, or an
   annotation pointing outside the file. The population decides whether this is a feature or a
   refusal that should simply be sharpened. **Do not implement before measuring.**
2. The tier in the port, with its own operation in `pdf_model::restriction` if Table 22 has one that
   fits, and a stated answer if it does not.
3. Tests for all four levels, both answers, and for the case a face cannot ask.

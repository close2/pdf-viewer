# Q07 — `pdf:/` for the KIO worker, or something format-neutral?

Source: RFC 0003 §9 question 1.
Status: **open** — answered when `A07-the-scheme-name.md` exists beside this file.

## Why it needs the owner

The protocol name appears in every URL a person or a script writes, and changing it later breaks them.

## What the tree does meanwhile

`pdf` is declared in the plugin's metadata and is what session 913 drove through real KIO.

## Recommendation

Keep `pdf:/`. It is honest about what it opens, and a neutral name would promise a generality this core does not have.

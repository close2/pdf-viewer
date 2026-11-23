# 3390 — The page group's knockout flag is honoured

Session 2172. Status: accepted and built (`pdf-model`: `content/transparency/knockout.rs`'s
`page_knockout`, `content/transparency.rs`'s `composite_group`, `open_page_knockout` and
`composite_page_knockout`; `content.rs`; `content/divide.rs`). Answers finding 2171-B02.

## The clauses

Table 145's `/K`: "if true , they shall be composited with the group's initial backdrop and shall
overwrite ( "knock out") any earlier overlapping objects." Its `/I` and `/CS` rows say how they are
read "[i]n the group attributes dictionary for a page"; `/K` has no such sentence, so it is read on
a page as on a form. §11.4.7 makes the page group P "consisting of all elements E 1 ,…, E n in the
page's top -level stack", treated as an isolated group on the medium.

## Decision

The close of a form's group — `knockout_construction`, the reports, the `Command::Group` — is one
method, `composite_group`, which the page group now shares. A page whose `/Group` is
`/Transparency` with `/K true` runs with the enclosing knockout kind `Isolated` from its first
operator (so a nested group knows it is a knockout element, as inside a knockout form), and once the
annotations are drawn — they are elements of the top-level stack too — every command on the list
becomes the elements of one isolated knockout group composited over the medium. §14.11.2.2's
guidelines are drawn after it, being this reader's. A knockout page is not divided across threads,
since each piece would have to carry the open group (the division declines, by name).

ADR 2817 put a mask group's `/K` through the same construction; this is the third place it runs.
The group's `/CS` is not carried: `note_page_blending_space` already judges the page's space.

## Cost

A knockout page reads on one thread. The flag is rare; the pages that state it drew wrong before.

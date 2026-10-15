# 1946 — The clause 13 entry excludes the player and admits the page, and its rows are read again

Session 1555, slot 4 of batch eighty-eight. Status: **accepted, built**. Context: `doc/questions/A254`
(question 2); RFC 0009 §3.1, §3.3, §3.7, §11 question 2 and §12 item 3; ISO 32000-2 §12.5.6.17,
§12.5.6.25, §13.2.1, §13.2.2, §13.2.4.2, §13.4, §13.6.2, §13.7.2.1, §14.9.2.4, Tables 285, 286 and
306; PLAN §5a's statuses; ADRs 0906, 1119, 1535, 1548, 1561, 1599, 1622.

## 1. The entry

`CLAUDE.md`'s exclusion read "Clause 13, multimedia and 3D", one line covering the whole clause. A254
adopted RFC 0009 §11 question 2's sentence as written, and the entry now is that sentence: what is
excluded is the playing of multimedia and the rendering of 3D artwork, and what a still page shows
of a media object is in scope by name — a movie's stream-form poster, every producer-supplied
appearance, §13.2.2's viability computed and reported, and the media file saved or handed to this
machine's player at the reader's level. Each exclusion that is left names the trigger that reopens it.
The ledger's key `clause-13-multimedia` and the checker's `Exclusion::Multimedia` keep their names,
because the closed list did not grow or shrink. Only what the one entry covers changed, and the
enum's doc comment now says that.

## 2. How a row is read against the narrower entry

The entry is narrower now, so "clause 13" no longer settles a row by itself. **A row stays
`out-of-scope` where the subject of its quoted sentence is the engine**: deciding how, where or with
which player to play, decoding, presenting, or the 3D scene. **It leaves where its requirement is one
of the four things the entry admits.** Leaving takes it to whatever its reading earns, so a clause
the amendment admits but nothing implements is owed. Calling it `out-of-scope` would be the graveyard
PLAN §5a refuses. Seven leaves move. Their headings follow by ADR 1599's rule.

| row | from → to | why |
|---|---|---|
| §13.4 | `departed` → `implemented` | The poster is drawn (ADR 1561). The playing, and the boolean `/Poster` (a frame decoded from the movie file), are now the closed list's. A departure is a `shall` this row decided against (ADR 1119), and no such `shall` is left. |
| §13.6.2, §13.7.2.1 | `out-of-scope` → `implemented` | Each makes its normal appearance its inactive state, and the inactive state is the default. That appearance is drawn, or refused by the clause's own requirement when it is absent (ADR 0906). The active state's rendering is excluded. §12.5.6.16's sound icon set this precedent. |
| §12.5.6.25 | `out-of-scope` → `implemented` | It is a pointer that states no requirement of its own, so it takes the status of the two rows above (ADR 1535). |
| §13.2.2 | `out-of-scope` → `reported` | The entry names viability. Nothing computes it. The rendition action is where §13.2.1 says playing begins, and `action.rs` refuses it there by name. |
| §13.2.4.2 | `out-of-scope` → `reported` | Its /MH, /BE and /P feed viability, and its /D, /CT and Table 286's /TF are the hand-off. The same refusal is said at the same site. |
| §14.9.2.4 | `out-of-scope` → `reported` | A clip's /Alt is text "in case it cannot be played". RFC 0009 §3.3 names the Tier A report as the array's first reader, and that report is admitted. Until it is built, the rendition refusal is what is said. |

So §13.2 and §13.2.4 become `partial`, and so do §14.9.2 and §14.9 above §14.9.2.4. All four are
aggregates in `doc/todo/65`'s paragraph. The three owed leaves go into the map: §13.2.4.2 in bucket 1,
since its hand-off is a `Media` level in each window's menu, and §13.2.2 and §14.9.2.4 in bucket 6.
The other clause 13 rows stay `out-of-scope`, and their notes name the entry as it now reads.

## 3. What is not settled here

- **Table 285's form-XObject clip.** "If D references a form XObject, the associated player is
  implicitly the interactive PDF processor, and the form XObject shall be rendered as if it were any
  other data type". That is content this program can draw, presented as playing. Neither of the
  entry's two lists names it. The round that builds RFC 0009 §12 item 3 reads it, and §13.2.4.2's
  note says so.
- **The refusal's sentence.** `action.rs` still says "Rendition: clause 13's multimedia, excluded by CLAUDE.md principle 5".
  That is true of the playing and silent about viability. Item 3's report replaces it. Changing the
  string in this round would change what the windows print without the build behind it.
- **The cost.** Three leaves that owed nothing now owe, and four aggregates go to `partial`. That is
  the price of the scope the owner admitted, and it is paid by item 3's build, not by this reading.

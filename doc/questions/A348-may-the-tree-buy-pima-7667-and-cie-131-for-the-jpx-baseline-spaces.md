Status: complete
Given: 2026-10-14, in conversation — transcribed by the round
Owes: the held PIMA 7667 extracted under `doc/md/` and entered in `doc/third-party-data.md`, then `EnumCS` 20 (e-sRGB) and 24 (e-sYCC, PIMA 7667's Annex B) drawn as the held text defines them and §7.4.9's note narrowed to CIEJab (19) alone with its sentence (acting round)

> i have bought doc/pima7667-2001.pdf
>
> I won't buy the cei document.

The earlier word this supersedes (given 2026-10-08, transcribed then):

> Leva PIMA 7667 and CIE 131 open for now.

Reading: the question's recommendation was *"Buy PIMA 7667; buy CIE 131 only if
it is sold as the publication."* The owner's word takes the first half and
answers the second outright:

- **PIMA 7667:2001 is bought and held** at `doc/pima7667-2001.pdf` — 277 252
  bytes, the 271 KB the question recorded;
  SHA-256 `05653ec7b3b42ad3d8b1a023dc21cfb7af69a7b8ba30c88003d20c46c40d9c20`;
  `PIMA7667-2001.PDF`, created 2001-05-24, IS&T's legacy-standard copy. **The
  gitignore note the owner asked for is settled by a line that already
  exists**: `/doc/*.pdf` (`.gitignore:46`) excludes the file, verified with
  `git check-ignore` — the same pattern that keeps every other held
  specification out of the commit, so no change was owed.
- **CIE 131-1998 will not be bought.** CIEJab (`EnumCS` 19) keeps §7.4.9's own
  device fallback, the row's note narrows to it alone with its sentence naming
  the unheld text, and only the owner reopens that half — by a new answer here
  or a new question.

What the held text closes: e-sRGB (20) and e-sYCC (24) become implementable
from the publication rather than the device fallback, and with them the row's
reason is 19 alone.
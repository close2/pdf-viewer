# 2532 — A page gets the press it names, and pays for it from the memory its list is held to

Status: accepted. Session 1849. ADR 2533 is not used and Q715 is not asked.
Context: ISO 32000-2 §11.4.7, §11.5.3, §11.6.6, §11.7.2, §11.7.5.3, §8.6.5.9, Table 69, Annex C; the owner's
"Having to implement something is not a blocker, even if expensive"; trap 38; ADRs 0417, 1254, 1507, 1622.
Code: `crates/pdf-colour/src/colour/press.rs` (`Presses::within`, `PRESS_BYTES`, `press_charge`),
`crates/pdf-colour/src/icc.rs` (`Profile::owned_bytes`), `crates/pdf-model/src/content/list_budget.rs`
(`ListBudget::passes`), `crates/pdf-model/src/content.rs`.
Documents: §11.5.3, §11.6.6 and §11.7.2's ledger rows, `doc/todo/65`, `doc/verify.md`.

## 1. What the premise got right and what it got wrong

The brief's hypothesis was "the bound is one constant and one fallback branch; the budget already charges a press's
tables". **The first half held and the second did not.** `colour::MAX_PRESSES` was `RENDERINGS_OF_ONE_PROFILE * 2`,
fourteen, tested in `Presses::press_for_profile` before and after the sampling, and the one fallback was
`NoPress::Spent`, which `transparency::press_or_beyond` turns into `PagePress::Beyond`. But the page's memory bound,
`MAX_LIST_BYTES` (ADR 1507), charged the display list and the readback only, and its module says image samples are not
charged either: a press was counted, never weighed. A page that held fourteen presses held 15 MB the bound never saw.

## 2. The decision

**A count of presses is replaced by a charge in bytes, made against the bound the page's list is already held to.**

- Every distinct press — a profile under a rendering (ADR 1054) — a page names is sampled. ISO 32000-2 states no number
  of profiles a page may name; Annex C is informative and gives none, and ADR 1254's floor of seven presses for one
  profile is still the only number the standard supports.
- A press is charged `colour::PRESS_BYTES` (the grid of `PRESS_SIDE`⁴ colours, the luminance grid of §11.5.3 and the
  ink table, 1 414 944 bytes at the sides built) plus `Profile::owned_bytes` for the profile it keeps to convert in
  (§8.6.5.5). The two lazy parts are charged at admission, so that the refusal is made before any sampling and is a
  function of the file alone.
- `Presses::within(bound)` takes the bound; the interpreter passes `MAX_LIST_BYTES`, half the confined worker's address
  space (ADR 1507). `ListBudget::passes` adds `Presses::bytes_held` to the list's charge, so a page whose list and presses
  together pass the bound is refused by the same `Unsupported::ListBytes`, whose `charged` then counts both.
  `Interpretation::list_bytes` stays the list's own.
- A press that does not fit is `NoPress::Spent`, now worded as the memory it may keep (§11.7.2), and the group is
  painted in the parent's space with `PagePress::Beyond` naming why — a report, never a panic and never a silent
  fallback. The refusal is a resource limit under `CLAUDE.md` principle 3, like the list's, and no `shall` the program
  declines: ADR 1622 kept the three rows `departed` because a count was a bound the standard did not state; there is
  no count left. The rows are `implemented`.
- `MAX_RGB_ROUTES` (the routes kept per interpretation) was `MAX_PRESSES` by coincidence of value and is its own
  constant, fourteen: a cache bound whose overflow is slower and never different.

## 3. What it costs

At the sides built a bound of 2 GiB (`MAX_LIST_BYTES`, 2 147 483 648 bytes) holds 1 517 presses of the fixtures'
one-way profile at 1 414 944 bytes each, of which a page can name 1 516 and still have its list drawn (section 5), so a hostile page may now make this tree hold the bound where it held at most
fourteen presses, and sample every one of them. Measured, one process each, the child's peak resident set and its CPU
time (`user`), the machine at a load average of 15 to 34 on 24 cores with the heavy-walk lock's lanes held by other rounds' walks
(the figures are memory and CPU seconds, not a clock gate, and CPU seconds stretch under that load):

| page | HEAD | this tree |
|---|---|---|
| 20 distinct presses | refused after 14: 23 MiB, 0.3 s | drawn in all twenty: 28 MiB, 0.8 s |
| 1 516 distinct presses (the most that leave the list its room) | refused after 14: 36 MiB, 0.3 s | drawn in all: 1 472 MiB, 38.4 s user, 40.8 s wall |
| 1 516 then 1 518 (the probe, both pages) | refused after 14: 35 MiB | 1 475 MiB peak, 74.6 s user, 75.8 s wall |

- **Memory** is the bound's and no more: 1 472 MiB resident against the 2 047 MiB charged, because the charge is what a
  press may hold (a grid of `PRESS_SIDE`⁴ colours, a luminance grid and the ink table are charged whole and the table is
  touched where a colour reaches). The probe's two pages peak at the larger one's, **provided the first is dropped
  before the second is read**: the display list holds its groups' presses, so a host that keeps two hostile pages'
  lists alive holds two bounds (2 927 MiB resident in the probe's first form, before the test dropped the first page). That is ADR 1507's
  design, `MAX_LIST_BYTES` being half the confined worker's address space so that a second interpretation has the
  other half; a host that keeps more lists alive than that is the owner of the multiplication.
- **Lookups** are a `HashMap` keyed by the press's identity, so a page naming a thousand presses does not search a list
  a thousand times per colour.
- `Profile::owned_bytes` counts everything the profile a press keeps can hold, and not the colorimetric table alone.
  A press keeps a clone of a two-way profile (§8.6.5.5: the profile it converts *in*), and the clone deep-copies the
  tables an intent has parsed and carries the tags' own copies of the perceptual and saturation bytes. The first
  version of the charge counted the colorimetric table only: 24 two-way CMYK profiles of 33 points a side were charged
  358 MiB against 1 033 MiB resident under `/Perceptual`, which at the bound is about 5.9 GiB and kills the 4 GiB
  worker (ADR 1507: refused by name *before* the kill). It now charges the colorimetric and the "from CIE" tables, and
  for each alternate tag its bytes and the largest table they can parse into (one `f32` per byte of the tag, so four
  times the tag), whichever intent has drawn: a function of the profile alone. The same 24 profiles are charged
  1 229 MB against 927 MB resident (a probe in `colour/press.rs`), a press 51 MB where it holds 39 MB.
- **Time is not budgeted here.** A press costs 22 to 25 ms of CPU at the sides built (the quieter and the busier of
  the machine's loads), 38 s for a page of 1 516, and a page that names no four-component profile samples none. No
  count of operators or seconds refuses a page: ADR 1809 decides that no count of operators refuses one, ADR 0657 that
  no automatic deadline stops one and the time is the person's to stop, and ADR 0241 that a cancel is a kill. The
  memory is what a page can exhaust before anyone can stop it, which is why that is the bound.
- A page can end past the bound by what the operators since the last asking added (`list_budget`'s cadence), and
  presses join the bound at admission, where the list is not in view. `finished` therefore asks `list_past(0)` once
  more with everything charged, so that a page which ends past the bound says so as `ListBytes`.
- A poisoned table (a sampling that panicked) reads as no presses held in `list_past`, so a poisoned page is refused by
  the group's `PagePress::Beyond` and not by the list's charge.

## 4. What moved

Against HEAD's six arms (compute, cpu and gpu, at 1x and 4x): 968 pages at 1x and 965 at 4x in each lane, **0 pages
differ** in any of the six. The pdf-model corpus gate and `raster_golden` passed unchanged. Pixels could move only on
pages that named more than the old fourteen presses, and `examples/press_depth` finds none: over the 963 documents
(1 283 pages) of the pdf.js corpus every page names 0 presses, and over every eleventh document of the SafeDocs crawl
(5 977 documents, 13 188 pages, three pages each) 13 105 pages name none, 83 name one, and **no page names more than
fourteen** (`--over 14` lists none). The behaviour change is therefore reachable by hostile or unusual documents only;
no document the project holds draws differently.

The three ledger rows move from `departed` to `implemented`: §11.5.3, §11.6.6 and §11.7.2, whose departure was the
count (ADR 1622). The refusal that remains is a memory bound, by name, under `CLAUDE.md` principle 3.

## 5. Evidence

- `cargo test -p pdf-colour` (155 passed, 1 ignored) and `cargo test -p pdf-model --no-fail-fast` (2 343 passed, 22
  ignored, none failed) in an export of HEAD plus the files this round touched, with the sandbox and transform workers
  built; `cargo clippy -p pdf-colour -p pdf-model --all-targets` with `-D warnings`, `cargo fmt --check` and
  `cargo doc --no-deps` with `-D warnings` clean.
- Tests: in `pdf-colour` the bound exact, one byte short, an empty table and 21 presses on a budget of their charges;
  `a_two_way_profile_is_charged_at_least_what_the_perceptual_table_adds_to_its_clone` (`icc.rs`: the charge is the same
  before and after the perceptual table is parsed, and is at least every table the clone then holds); two in
  `list_budget.rs` for the combined charge; in `tests/transparency_groups.rs` sixteen documents in one process and
  twenty presses on a page that spends nothing of the next. Planted: `Presses::within(14 * PRESS_BYTES)` fails the
  twenty-press test, and the colorimetric-only charge fails the `icc.rs` test (78 KB charged against 157 KB of two
  grids).
- The resident-size probe `large_two_way_profiles_are_charged_at_least_their_resident_size` (`#[ignore]`d, run alone
  by hand): 24 two-way CMYK profiles with `A2B0`, `A2B1` and `B2A1`, 33 points a side, drawn under `/Perceptual`, are
  charged 1 229 383 728 bytes (51 224 322 a press) and grew the process by 926 892 032 resident bytes (38.6 MB a
  press). At the bound that is about 41 presses and 1.6 GB resident, inside the worker's 4 GiB, where the colorimetric
  charge admitted about 137 and 5.9 GiB.
- The probe `a_hostile_page_naming_profiles_to_the_bound_is_refused_by_name_past_it` (`#[ignore]`d, a memory probe run by
  hand): a page of 1 516 presses, the most that leave its list its room, is drawn in all of them; a page of 1 518 was
  refused in the list's voice, `Unsupported::ListBytes { charged: 2 147 501 860, bound: 2 147 483 648 }`, and not as a
  group painted in its parent's space: the headroom between 1 517 presses and the bound is about one mebibyte and the
  list of that many groups is larger than that, so the combined charge trips before a 1 518th press is admitted. Either
  voice is a refusal by name; a page whose list is leaner reaches the press's own (`NoPress::Spent`, tested in
  `pdf-colour`). The probe asserts that one of the two appears and that no more presses than the bound admits are held.
  The page of exactly 1 517 now ends in the list's voice too, because the final asking of `finished` sees the list
  the cadence had not charged yet.
- The HEAD figures above are the same test, built in an export of HEAD with the one `PRESS_BYTES` use replaced by its
  literal; HEAD fails at its first assertion, so its peak is the fourteen presses it held before refusing.

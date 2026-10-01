# 1324 — A contradicted group says whose departure it holds

Date: 2026-10-01. ADR 1483. Batch fifty, the robustness slot.

**The ranking.** Oracle before: 1024 agree, 47 contradicted, 836 ambiguous, 46 not comparable,
9 no render. Every page is held by name. The ten highest are held by `ON_A_PAGE_WE_REPORT`,
`LUMINOSITY_OF_A_CIE_BASED_MASK`, `SHARED_JBIG2_DECODER`, `DEVICE_CMYK_CONVERSION`,
`LINK_BORDER` and `SUBSTITUTED_FONT`. None of those six notes names a departure of ours:

- Three name the references': §11.5.3, §7.4.7 "defines decoder behaviour", and §12.5.4 with
  Table 167.
- Three name a choice the clause leaves open: §7.4.8 for a self-contradicting file, §10.3.2's
  NOTE under §10.4.2.1's ranking, and §9.5 NOTE 5.

The CMYK pages are on §10.3's route on both sides. The formula would draw the bottle black. ADR
1441 does not reach a standard-14 name. Each verdict, with its sentence: ADR 1483 section 2.

**The instrument.** `oracle.rs` now has `WHOSE_DEPARTURE`, one row per non-empty group: whose
departure it is, and the clause. Each ranked row prints it, and `name_the_next_departure_of_ours`
names the page. Of the 47 pages, 2 are ours, 12 the references' and 33 choices. The next page is
`issue7891_bc1.pdf` at 1.11x, which is §10.7.4's departure (1) at a clip edge, already recorded on
that row. `every_held_page_names_whose_departure_it_is` holds the table against the groups and the
groups' notes. The eight notes involved each gained a short section quoting their deciding
sentence.

**No defect found, no pixel moved.** The oracle after the change matches the run before it, so
`raster_golden` and the corpus gates were not owed.

Files: `crates/pdf-model/tests/oracle.rs`, `doc/adr/1483-a-contradicted-group-says-whose-departure-it-holds.md`,
this file.

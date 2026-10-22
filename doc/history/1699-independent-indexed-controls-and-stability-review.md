# 1699 — Independent Indexed controls and stability review

Session 1699, 2026-10-22. ADRs 2232/2233.

**Change.** Add ten independent Indexed/ICC controls before the implementation;
retain the thirteen original direct-ICC controls and unchanged pixel tolerance.
Review the distinct lifetimes of selected source, declared space and paint context.
Use immutable resolved source identity and the existing complete-context cache;
avoid another general cache or resource-name resolution during painting.

**Gates.** Fixed baseline 9cd2c665: 14 passed, 9 intended assertion failures,
0 skipped; original thirteen all pass. Candidate: 23 pass. Checkpoint 9e426dd5
records the tests before the fix. Text/Gray assertion ordering was improved without
changing expectations; helper parameter rename is lexical only. Five mutation
controls and the 2311-pass model/colour suite are recorded by session 1700.

**Limits.** Gray cold comparison is supplemental, not a complete independent
conversion oracle. Other Indexed bases and calibrated/Lab/spot source families
remain open. The review does not establish a single cause for all regressions.
Full batch checks and final renderer comparison are integration-owned.

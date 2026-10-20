# 2105 — What a document asks is carried out once in `viewer-host`, and a window keeps its dialogue

Session 1634, batch ninety-nine's slot 4, a host round. Status: **accepted** and **built**.
Context: ADRs 1155, 1227, 1291, 1331, 1527, 1616, 1738 (the requests and their levels), 1920 and
1972 (`NativeWindow`), 2088 (the third copy), 2092 (the precedent), habit 131.

## 1. What was copied

`viewer_host` decided every request — `may_submit`, `link`, `remote`, `may_open_extracted`,
`may_fetch_import`, the signature policy's level — and three windows carried the answer out line for
line: `viewer-qt`'s `requests.rs` and `questions.rs`, `viewer-gtk`'s `requests.rs`, `questions.rs`
and `saving.rs`, and `viewer-ffi`'s `window/asked.rs`. Qt and the C window held the question as a
`Pending` enum of eight subjects, written twice; GTK held it as a closure per subject, so the third
copy of every answer was spread through eight closures.

## 2. Decisions

1. **`viewer_host::native_window::Asked`** is the one enum of the eight subjects, with
   `subject()` for the dialogue's title, and the carrying-out is `native_window/requests.rs`:
   free functions generic over the trait, which `follow_the_link`, `submit`, `needs_file`,
   `fetch_policy`, `extracted`, `ask_whether_to_proceed` and `ask_whether_to_run_scripts` now call
   as provided methods, and `answer_the_question` for the answer.
2. **Two required methods are what is left a toolkit's**: `put_the_question`, the dialogue, and
   `question_waiting`, the one fact the hosts differ in — Qt and the C window have one dialogue, so a
   signature policy asked about while another question waits is declined with its reason, and GTK
   puts each question in a window of its own and answers `false`. A host's exception is a fact the
   shared function decides on (habit 139), and GTK's behaviour is unchanged.
3. **`links` and `remote_documents` move into `WindowState`**, set from `Settings` as the
   restriction levels are, because a provided method reads them.
4. **GTK keeps its closure, holding the `Asked`** in a `Cell` it takes, so a window answered and then
   closed answers once — the guarantee Qt and the C window keep by taking their `Option`.

## 3. What held and what did not

Behaviour is held: the three windows said the same sentences in the same order before, and
`start_fetch`'s first-request check, which GTK made and Qt and the C window did not, arms
`look_for_answers`, a no-op in those two. The brief's hypothesis — one function of about two
hundred lines — did not hold: it is one enum and eleven functions, 484 lines with their clauses,
against about 1 500 removed from the three hosts.

## 4. Costs

Two required methods where seven were, and one enum the C ABI never sees.

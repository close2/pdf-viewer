# 1967 — A conversion opens an encrypted file with the password the window kept

Session 1565, on `doc/todo/72` section 1 ("a password-protected document converts only where its
user password is empty"). Status: **accepted** and **built**. Context: ADRs 0545 (the `Secret`
type), 1954, 1161; ISO 32000-2 §7.6.4.1.

## 1. The premise, as found

The brief said the window "already holds" the password. It did not: a host builds a `Secret` from
the prompt, moves it into `Command::Open`, and the core drops it with the command — `Secret`'s
own documentation says "no copy of it exists in this host at any point". A conversion opens the
file again in `pdf_transform`, so an encrypted document reached `Refusal::PasswordRequired` there.

## 2. Decisions

1. **The window keeps one copy, per document, for as long as the document is open in it**:
   `viewer_host::password::Kept`, a map from `DocumentId` to `Secret`. `keep` is called where a
   host sends `Command::Open` with a password (the first document's second attempt and an
   arriving document's, in `quorra`, both toolkit hosts and the C library's `Session::open`);
   `forget` where a document closes or its prompt is cancelled; `lend` copies it for the
   conversion, whose `Source::with_password` drops it when the run ends. The core still keeps none.
2. **Re-asking was weighed and not taken.** §7.6.4.1 says a processor "should prompt for a
   password", and a prompt for a password given to this window minutes earlier states nothing new;
   it also cannot be answered by a C caller that holds a `Session` and no prompt.
3. **The cost is written down in `Kept`'s documentation**: the password is in this process's memory
   while its document is open, where it was there only while the open was tried. The decrypted
   document and its file encryption key are in the same memory for the same time and either reads
   the file without the password; what keeping it adds is the password itself, which a person may
   use elsewhere. It stays a `Secret`, so it prints as its length and its buffer is cleared.
4. **A copy is a fresh `Secret::new` buffer** filled by `push_str`, so copying leaves no
   reallocation behind; `Secret` stays not `Clone`, because every copy is this one named site.

## 3. Consequences

`viewer-host`'s test encrypts the one-rectangle document under §7.6.4's handler and shows the
conversion refused at the open without the password and past it with one; the drive's step 88
converts `drive-password.pdf` after its prompt. What the converter does about the encryption
itself is the converter's and unchanged: ISO 19005-2 section 6.1.3 forbids an `/Encrypt` key, so
no output is encrypted, and what the source asserted is kept as a statement (ADR 1187).

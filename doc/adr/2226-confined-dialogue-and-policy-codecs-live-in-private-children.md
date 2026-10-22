# 2226 — Confined dialogue and policy codecs live in private children

Session 1696, docs. Status: implemented; scoped gates pass; final conformance pending.

The confined protocol root contains routing, primitives, codecs and their tests.
Two already independent vocabularies can move without decomposing its exhaustive
command and event matches: script questions/answers and published signature
policies. The root keeps every router and test, along with Reader, Writer,
ProtocolError, MAGIC, wire tags and allocation/error ordering.

Private script_dialogues holds write_script_question, read_script_question,
write_script_answer and read_script_answer. Private published_policy holds
write_published and read_published. The parent imports exactly these six names.
Each child imports only the original parent primitives and changes its helpers
to pub(super); no primitive visibility or external API is widened. Existing
qualified/local imports and inverse-function documentation remain intact.

This is a concern extraction only. Wire greeting PDFVCF18 and every original
codec body remain unchanged; no bug fix or protocol version change is justified.
Root size falls from 8169 to 7951 lines, with children of 145 and 91 lines.
The large routing root remains a separate design concern. Shared documentation
pointers may follow the move without changing any conformance status.

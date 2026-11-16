#!/usr/bin/env python3
import re,sys
from datetime import datetime,timezone
from pathlib import Path
from cryptography.x509 import ocsp

def stamp(s):
    # OpenSSL prints RFC 3161 genTime as e.g. "Nov 16 17:03:14 2026 GMT".
    line=next(x for x in s.splitlines() if x.startswith('Time stamp: '))
    return datetime.strptime(line.split(': ',1)[1], '%b %d %H:%M:%S %Y GMT').replace(tzinfo=timezone.utc)
info=Path(sys.argv[1]).read_text()
t=stamp(info)
responses=[ocsp.load_der_ocsp_response(Path(p).read_bytes()) for p in sys.argv[2:]]
early,late=responses
for r in responses:
    assert r.response_status==ocsp.OCSPResponseStatus.SUCCESSFUL
    assert r.certificate_status==ocsp.OCSPCertStatus.GOOD
    assert r.this_update_utc < t < r.next_update_utc, (r.this_update_utc,t,r.next_update_utc)
assert early.produced_at_utc < t < late.produced_at_utc, (early.produced_at_utc,t,late.produced_at_utc)
print('early producedAt < trusted genTime < late producedAt; both thisUpdate/nextUpdate windows bracket genTime')
print('trusted genTime', t.isoformat())
Path('trusted-time-unix.txt').write_text(str(int(t.timestamp()))+'\n')

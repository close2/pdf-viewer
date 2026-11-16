#!/usr/bin/env python3
"""Extract actual signed ranges from the public test PDFs and independently verify them.

Run from repository root. Requires OpenSSL and qpdf. Creates only temporary files;
never reads a private key. Success proves syntax/digest/key checks, not all PDF semantics.
"""
from pathlib import Path
import hashlib
import re
import subprocess
import tempfile

root = Path('crates/pdf-signature/tests/fixtures/docmdp_dts')
with tempfile.TemporaryDirectory(prefix='docmdp-verify-') as tmp:
    out = Path(tmp)
    for cert in ['signer', 'tsa']:
        subprocess.run(['openssl', 'x509', '-inform', 'DER', '-in', str(root / f'{cert}-cert.der'),
                        '-out', str(out / f'{cert}.pem')], check=True)
    for name in ['dts-only', 'dts-plus-page-change']:
        pdf = root / f'{name}.pdf'
        b = pdf.read_bytes()
        print(name, 'sha256', hashlib.sha256(b).hexdigest(), flush=True)
        ranges = list(re.finditer(rb'/ByteRange\s*\[([\d ]+)\]', b))
        assert len(ranges) == 2
        for i, match in enumerate(ranges):
            a, n, c, k = map(int, match[1].split())
            assert a == 0 and c + k <= len(b)
            gap = b[n:c]
            assert gap[:1] == b'<' and gap[-1:] == b'>'
            der = bytes.fromhex(gap[1:-1].decode())
            assert der[0] == 48
            hlen = 2 if der[1] < 128 else 2 + (der[1] & 127)
            size = der[1] if der[1] < 128 else int.from_bytes(der[2:hlen], 'big')
            assert set(der[hlen + size:]) <= {0}
            payload, token = out / 'input.bin', out / 'token.der'
            payload.write_bytes(b[:n] + b[c:c + k])
            token.write_bytes(der[:hlen + size])
            if i == 0:
                command = ['openssl', 'cms', '-verify', '-binary', '-inform', 'DER', '-in', str(token),
                           '-content', str(payload), '-CAfile', str(out / 'signer.pem'), '-out', '/dev/null']
            else:
                assert c + k == len(b), 'timestamp covers the complete updated file'
                command = ['openssl', 'ts', '-verify', '-token_in', '-in', str(token),
                           '-data', str(payload), '-CAfile', str(out / 'tsa.pem')]
            subprocess.run(command, check=True)
            print('verified signature', i, 'range', a, n, c, k, flush=True)
        subprocess.run(['qpdf', '--check', str(pdf)], check=True)

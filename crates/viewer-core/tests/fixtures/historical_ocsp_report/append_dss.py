#!/usr/bin/env python3
import re,sys
from pathlib import Path
base=bytearray(Path(sys.argv[1]).read_bytes())
ocsp=Path(sys.argv[2]).read_bytes(); crl=Path(sys.argv[3]).read_bytes()
# Preserve the first revision and update its catalog/DSS in one incremental revision.
prevs=list(re.finditer(rb'(?m)^startxref\s+(\d+)\s+%%EOF\s*$',base))
if not prevs: raise SystemExit('base PDF has no terminal startxref')
prev=int(prevs[-1].group(1))
starts={}
def obj(n,body):
    starts[n]=len(base)
    base.extend(f'{n} 0 obj\n'.encode()+body+b'\nendobj\n')
obj(1,b'<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [5 0 R] /SigFlags 3 >> /DSS 6 0 R >>')
obj(6,b'<< /Type /DSS /CRLs [7 0 R] /OCSPs [8 0 R] >>')
obj(8,f'<< /Length {len(ocsp)} >>\nstream\n'.encode()+ocsp+b'\nendstream')
xref=len(base)
base.extend(b'xref\n1 1\n'+f'{starts[1]:010d} 00000 n \n'.encode())
base.extend(b'6 1\n'+f'{starts[6]:010d} 00000 n \n'.encode())
base.extend(b'8 1\n'+f'{starts[8]:010d} 00000 n \n'.encode())
# IDs inherited from the signed revision.
m=re.search(rb'/ID\s*\[\s*(<[^>]+>)\s*(<[^>]+>)\s*\]',base[:xref])
if not m: raise SystemExit('no trailer ID in signed base')
first,second=m.group(1),m.group(2)
base.extend(b'trailer\n<< /Size 9 /Root 1 0 R /Prev '+str(prev).encode()+b' /ID ['+first+second+b'] >>\n')
base.extend(f'startxref\n{xref}\n%%EOF\n'.encode())
Path(sys.argv[4]).write_bytes(base)

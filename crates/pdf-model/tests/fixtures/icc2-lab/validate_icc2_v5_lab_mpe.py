"""Independent structural check for make_icc2_v5_lab_mpe.py's ICC.2 profile.

Checks the primary ICC.2:2023 §7.2 header, §8.2/8.4 required tags, §9.2 transform
tag-type permissions, §10.2.17 MPE container and §11.2.10 matrix element. This is a
source-guided structural check, not an ICC CMM or full conformance validator.
"""
import math, struct
from pathlib import Path
p=Path(__file__).with_name('icc2_v5_lab_mpe.icc'); b=p.read_bytes()
assert struct.unpack_from('>I',b,0)[0] == len(b), 'profile size must equal byte length'
assert b[8:12] == bytes((5,0,0,0))
assert b[12:16] == b'mntr' and b[16:20] == b'Lab ' and b[20:24] == b'XYZ ' and b[36:40] == b'acsp'
assert b[84:100] == bytes(16), 'zero profile ID is valid: not calculated'
n=struct.unpack_from('>I',b,128)[0]; assert n == 5
entries=[]
for i in range(n): entries.append(struct.unpack_from('>4sII',b,132+12*i))
assert [e[0] for e in entries] == [b'desc',b'cprt',b'wtpt',b'A2B0',b'B2A0']
for sig,off,size in entries:
 assert off%4 == 0 and 0 <= off <= len(b) and size <= len(b)-off, (sig,off,size)
assert b[entries[0][1]:entries[0][1]+4] == b'mluc'
assert b[entries[1][1]:entries[1][1]+4] == b'mluc'
for sig in (b'desc',b'cprt'):
 _,off,size=next(e for e in entries if e[0]==sig)
 assert size>=28
 typ,res,count,recsize=struct.unpack_from('>4sIII',b,off)
 assert typ==b'mluc' and res==0 and count==1 and recsize==12
 lang=b[off+16:off+18]; country=b[off+18:off+20]
 strlen,stroff=struct.unpack_from('>II',b,off+20)
 assert lang==b'en' and country==b'US' and stroff>=28 and strlen%2==0 and stroff+strlen<=size
assert b[entries[2][1]:entries[2][1]+4] == b'XYZ '
assert entries[2][2] == 20
for sig,off,size in entries[3:]:
 assert sig in (b'A2B0',b'B2A0') and b[off:off+4] == b'mpet'
 assert b[off+4:off+8] == bytes(4)
 inputs,outputs,elements=struct.unpack_from('>HHI',b,off+8)
 assert (inputs,outputs,elements)==(3,3,1)
 elt_off,elt_size=struct.unpack_from('>II',b,off+16)
 assert elt_off==24 and elt_off%4==0 and elt_size==60 and elt_off+elt_size==size
 mat=off+elt_off
 assert b[mat:mat+4]==b'matf' and b[mat+4:mat+8]==bytes(4)
 assert struct.unpack_from('>HH',b,mat+8)==(3,3)
 values=struct.unpack_from('>12f',b,mat+12)
 assert all(math.isfinite(v) for v in values)
 assert values==(1.,0.,0.,0.,1.,0.,0.,0.,1.,0.,0.,0.)
print('PASS: header, ICC.2 display required tags, profile-size/tag bounds, and both 3x3 MPE matrix transforms match the checked source tables')
print('LIMIT: structural self-check only; no independent ICC.2 CMM/validator is installed')

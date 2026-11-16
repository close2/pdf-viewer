# Independent inversion of ISO §8.7.4.5.8's tensor Bernstein surface; no renderer calls.
from math import hypot
net=[[(10,10),(10,30),(10,50),(10,70)],[(30,10),(0,0),(0,100),(30,70)],[(50,10),(100,100),(100,0),(50,70)],[(70,10),(70,30),(70,50),(70,70)]]
def basis(t):return [(1-t)**3,3*t*(1-t)**2,3*t*t*(1-t),t**3]
def db(t):return [-3*(1-t)**2,3*(1-t)*(1-3*t),3*t*(2-3*t),3*t*t]
def sumvec(a,b):return [sum(a[i]*b[j]*net[i][j][axis] for i in range(4) for j in range(4)) for axis in range(2)]
def roots():
 found=[]
 for ui in range(21):
  for vi in range(21):
   u,v=ui/20,vi/20
   for _ in range(100):
    x,y=sumvec(basis(u),basis(v));dx,dy=x-39.5,y-48.5
    if hypot(dx,dy)<1e-9:break
    xu,yu=sumvec(db(u),basis(v));xv,yv=sumvec(basis(u),db(v));det=xu*yv-xv*yu
    if abs(det)<1e-12:break
    u-= (dx*yv-dy*xv)/det;v-= (dy*xu-dx*yu)/det
    if not(-2<u<3 and -2<v<3):break
   if 0<=u<=1 and 0<=v<=1 and hypot(*[a-b for a,b in zip(sumvec(basis(u),basis(v)),[39.5,48.5])])<1e-7 and not any(hypot(u-a,v-b)<1e-6 for a,b in found):found.append((u,v))
 return [(u,v,v*v*255) for u,v in found]
print('correct',roots());net[1][1],net[1][2]=net[1][2],net[1][1];print('swapped',roots())

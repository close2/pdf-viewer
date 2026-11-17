"""Independent arithmetic for the scratch literal oracle; reads no production helper."""
from itertools import product
from math import prod

D50 = (0.9642, 1.0, 0.8249)
ABSORPTION = ((0.60, 0.10, 0.10, 0.80),
              (0.10, 0.60, 0.20, 0.80),
              (0.10, 0.20, 0.70, 0.80))

def pcs(inks):
    return tuple(D50[axis] * prod(1.0 - ABSORPTION[axis][i] * inks[i]
                                  for i in range(4))
                 for axis in range(3))

# The fixture writes two points per input and linear 0..1 curves. Decode its actual
# u1Fixed15 CLUT integers and multilinearly interpolate the 16 corners at (1/2)^4.
points = []
for bits in product((0.0, 1.0), repeat=4):
    points.append(tuple(int(value * 32768.0) for value in pcs(bits)))
xyz = tuple(sum(point[axis] for point in points) / 16.0 / 32768.0
            for axis in range(3))
profile_xyz = xyz

# The selected profile output path compensates its darkest CMYK vertex (all inks); the
# profile has no B2A table, so the local-black endpoint is the least-L* of its four CMYK
# vertices. This fixture's all-ink corner is that minimum and has Y=0.0576.
black_y = pcs((1.0, 1.0, 1.0, 1.0))[1]
black_lstar = 116.0 * black_y ** (1.0 / 3.0) - 16.0
black_luminance = ((black_lstar + 16.0) / 116.0) ** 3
scale = 1.0 / (1.0 - black_luminance)
xyz = tuple(scale * xyz[i] + (1.0 - scale) * D50[i] for i in range(3))

# IEC sRGB D50-adapted display matrix used by this renderer, stated here numerically;
# gamma/quantization are independently applied below rather than via Profile::to_rgb.
matrix = ((3.134136, -1.617036, -0.490662),
          (-0.978755, 1.916142, 0.033454),
          (0.071950, -0.228988, 1.405386))
linear = tuple(sum(row[i] * xyz[i] for i in range(3)) for row in matrix)
def encode(c):
    c = min(1.0, max(0.0, c))
    return 12.92 * c if c <= 0.0031308 else 1.055 * c ** (1.0 / 2.4) - 0.055
rgb = tuple(encode(c) for c in linear)
print('profile PCS midpoint XYZ:', profile_xyz)
print('post-BPC XYZ:', xyz)
print('linear sRGB:', linear)
print('encoded sRGB:', rgb)
print('nearest 8-bit:', tuple(int(c * 255.0 + 0.5) for c in rgb))

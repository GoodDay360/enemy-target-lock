#!/usr/bin/env python3
"""Draw the mod icon as 32x32 pixel art and save it scaled up, with hard pixels, to icon.png.

A lock-on reticle in a blocky, game-like style: corner brackets closing in on a red target
ring around the white lock dot, on a dark speckled stone-like panel with a bevelled edge.
Everything is drawn here pixel by pixel (no game textures).

Usage: python3 docs/icon.py   (needs Pillow; writes icon.png in the project root)
"""

import os
import random

from PIL import Image

SIZE = 32
SCALE = 16  # 32 x 16 = 512 pixels
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# Palette
STONE = [(43, 40, 48), (50, 46, 56), (38, 35, 43), (56, 52, 62)]
BEVEL_LIGHT = (86, 80, 94)
BEVEL_DARK = (20, 18, 24)
OUTLINE = (8, 7, 10)
RED = (232, 58, 66)
RED_DARK = (150, 28, 38)
RED_GLOW = (78, 36, 46)
WHITE = (244, 241, 234)
WHITE_SHADE = (178, 172, 164)
SHADOW = (14, 12, 17)


def draw():
    image = Image.new("RGB", (SIZE, SIZE))
    px = image.load()
    rng = random.Random(7)  # the same speckles every time

    # Speckled stone panel.
    for y in range(SIZE):
        for x in range(SIZE):
            px[x, y] = rng.choice(STONE)

    # Faint red glow behind the target.
    for y in range(SIZE):
        for x in range(SIZE):
            d = ((x - 15.5) ** 2 + (y - 15.5) ** 2) ** 0.5
            if d < 6.5 and rng.random() < 0.35:
                px[x, y] = RED_GLOW

    # Bevelled edge: 1px outline, light top-left, dark bottom-right.
    for i in range(SIZE):
        for (x, y) in ((i, 0), (i, SIZE - 1), (0, i), (SIZE - 1, i)):
            px[x, y] = OUTLINE
    for i in range(1, SIZE - 1):
        px[i, 1] = BEVEL_LIGHT
        px[1, i] = BEVEL_LIGHT
        px[i, SIZE - 2] = BEVEL_DARK
        px[SIZE - 2, i] = BEVEL_DARK

    def put(points, colour, shadow=True):
        """Pixels with a one-pixel drop shadow down and to the right, as game icons have."""
        if shadow:
            for (x, y) in points:
                if (x + 1, y + 1) not in points and px[x + 1, y + 1] != OUTLINE:
                    px[x + 1, y + 1] = SHADOW
        for (x, y) in points:
            px[x, y] = colour

    # Red target ring (radius 7.5 around the center), lit from the top left: darker red on
    # its lower right instead of a shadow, which would fall inside the ring.
    ring = set()
    for y in range(SIZE):
        for x in range(SIZE):
            d = ((x - 15.5) ** 2 + (y - 15.5) ** 2) ** 0.5
            if 6.6 <= d < 8.2:
                ring.add((x, y))
    put(ring, RED, shadow=False)
    for (x, y) in ring:
        if (x - 15.5) + (y - 15.5) > 4:
            px[x, y] = RED_DARK

    # Ticks just outside the ring, two pixels wide and two long.
    ticks = set()
    for t in (0, 1):
        for w in (15, 16):
            ticks |= {(w, 5 + t), (w, 25 + t), (5 + t, w), (25 + t, w)}
    put(ticks, RED)

    # White corner brackets, two pixels thick, closing in.
    brackets = set()
    for (cx, cy, dx, dy) in ((4, 4, 1, 1), (27, 4, -1, 1), (4, 27, 1, -1), (27, 27, -1, -1)):
        for t in range(6):
            for w in range(2):
                brackets.add((cx + dx * t, cy + dy * w))
                brackets.add((cx + dx * w, cy + dy * t))
    put(brackets, WHITE)
    for (x, y) in brackets:  # shade the inner side of each bracket
        if (x in (5, 26) and y not in (4, 27)) or (y in (5, 26) and x not in (4, 27)):
            px[x, y] = WHITE_SHADE

    # The lock dot: white 2x2 with a dark outline, as in the mod.
    for y in range(14, 18):
        for x in range(14, 18):
            px[x, y] = OUTLINE
    put({(15, 15), (16, 15), (15, 16), (16, 16)}, WHITE, shadow=False)
    px[16, 16] = WHITE_SHADE

    return image


if __name__ == "__main__":
    out = os.path.join(ROOT, "icon.png")
    draw().resize((SIZE * SCALE, SIZE * SCALE), Image.NEAREST).save(out, optimize=True)
    print(f"wrote {out}")

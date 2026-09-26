"""Generate the view's woodcut-style sprites into assets/sprites/.

1850s newspaper engravings: ink outlines and hatching over white paper.
Fills are white so Bevy can tint them (paper, sepia, faction colors);
ink stays black under any tint. Drawn large and downsampled for smooth ink.

    python3 tools/woodcut.py
"""

import math
import os

from PIL import Image, ImageChops, ImageDraw

OUT = os.path.join(os.path.dirname(__file__), "..", "assets", "sprites")
S = 4  # supersampling
INK = (18, 14, 10, 255)
PAPER = (255, 255, 255, 255)


def canvas(w, h):
    return Image.new("RGBA", (w * S, h * S), (0, 0, 0, 0))


def pts(points):
    return [(x * S, y * S) for x, y in points]


def poly(d, points, fill=PAPER, width=1.2):
    d.polygon(pts(points), fill=fill)
    d.line(pts(points + [points[0]]), fill=INK, width=int(width * S), joint="curve")


def line(d, a, b, width=0.8):
    d.line(pts([a, b]), fill=INK, width=max(1, int(width * S)))


def hatch(img, points, spacing=2.2, angle=45, width=0.5):
    """Diagonal hatching clipped to a polygon."""
    w, h = img.size
    mask = Image.new("L", (w, h), 0)
    ImageDraw.Draw(mask).polygon(pts(points), fill=255)
    lines = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    ld = ImageDraw.Draw(lines)
    step = spacing * S
    rad = math.radians(angle)
    dx, dy = math.cos(rad), math.sin(rad)
    diag = w + h
    k = -diag
    while k < diag * 2:
        x0, y0 = k, 0
        ld.line(
            [(x0 - dx * diag, y0 - dy * diag), (x0 + dx * diag, y0 + dy * diag)],
            fill=INK,
            width=max(1, int(width * S)),
        )
        k += step
    empty = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    img.alpha_composite(Image.composite(lines, empty, mask))


def save(img, name, size):
    img = img.resize(size, Image.LANCZOS)
    img.save(os.path.join(OUT, name + ".png"))


def cabin():
    img = canvas(24, 20)
    d = ImageDraw.Draw(img)
    poly(d, [(3, 9), (21, 9), (21, 19), (3, 19)])
    for y in range(11, 19, 2):
        line(d, (3, y), (21, y), 0.5)
    roof = [(1, 10), (12, 2), (23, 10)]
    poly(d, roof)
    hatch(img, roof, spacing=1.6, angle=60)
    d = ImageDraw.Draw(img)
    poly(d, [(16, 2), (19, 2), (19, 7), (16, 7)])
    poly(d, [(10, 13), (14, 13), (14, 19), (10, 19)], fill=INK)
    save(img, "cabin", (48, 40))


def barn():
    img = canvas(26, 22)
    d = ImageDraw.Draw(img)
    body = [(2, 10), (24, 10), (24, 21), (2, 21)]
    poly(d, body)
    hatch(img, body, spacing=2.6, angle=90, width=0.4)
    d = ImageDraw.Draw(img)
    roof = [(0, 11), (5, 5), (13, 1), (21, 5), (26, 11)]
    poly(d, roof)
    hatch(img, roof, spacing=1.5, angle=30)
    d = ImageDraw.Draw(img)
    door = [(9, 13), (17, 13), (17, 21), (9, 21)]
    poly(d, door)
    line(d, (9, 13), (17, 21), 0.8)
    line(d, (17, 13), (9, 21), 0.8)
    save(img, "barn", (52, 44))


def ruin():
    img = canvas(26, 22)
    d = ImageDraw.Draw(img)
    for x in (4, 12, 21):
        poly(d, [(x, 9 + (x % 5)), (x + 2, 9 + (x % 5)), (x + 2, 21), (x, 21)], fill=INK)
    line(d, (3, 15), (23, 12), 1.5)
    line(d, (6, 21), (19, 16), 1.5)
    ash = [(1, 19), (25, 19), (25, 22), (1, 22)]
    poly(d, ash)
    hatch(img, ash, spacing=1.0, angle=20)
    save(img, "ruin", (52, 44))


def rick():
    img = canvas(16, 14)
    d = ImageDraw.Draw(img)
    dome = [(1, 13)] + [
        (8 + 7 * math.cos(math.radians(a)), 13 - 11 * math.sin(math.radians(a)))
        for a in range(0, 181, 12)
    ][::-1] + [(15, 13)]
    poly(d, dome)
    hatch(img, dome, spacing=1.4, angle=80, width=0.4)
    save(img, "rick", (32, 28))


def school():
    img = canvas(24, 24)
    d = ImageDraw.Draw(img)
    poly(d, [(3, 12), (21, 12), (21, 23), (3, 23)])
    roof = [(1, 13), (12, 6), (23, 13)]
    poly(d, roof)
    hatch(img, roof, spacing=1.6, angle=60)
    d = ImageDraw.Draw(img)
    poly(d, [(10, 2), (14, 2), (14, 8), (10, 8)])
    poly(d, [(9, 2), (12, 0), (15, 2)], fill=INK)
    for x in (6, 16):
        poly(d, [(x, 15), (x + 3, 15), (x + 3, 19), (x, 19)], fill=INK)
    save(img, "school", (48, 48))


def church():
    img = canvas(22, 30)
    d = ImageDraw.Draw(img)
    poly(d, [(3, 15), (19, 15), (19, 29), (3, 29)])
    roof = [(1, 16), (11, 9), (21, 16)]
    poly(d, roof)
    hatch(img, roof, spacing=1.6, angle=60)
    d = ImageDraw.Draw(img)
    poly(d, [(8, 7), (14, 7), (14, 15), (8, 15)])
    spire = [(8, 7), (11, 0), (14, 7)]
    poly(d, spire)
    hatch(img, spire, spacing=1.2, angle=70)
    d = ImageDraw.Draw(img)
    poly(d, [(9, 21), (13, 21), (13, 29), (9, 29)], fill=INK)
    save(img, "church", (44, 60))


def lyceum():
    img = canvas(26, 24)
    d = ImageDraw.Draw(img)
    poly(d, [(2, 8), (24, 8), (24, 23), (2, 23)])
    ped = [(0, 9), (13, 2), (26, 9)]
    poly(d, ped)
    hatch(img, ped, spacing=1.4, angle=15)
    d = ImageDraw.Draw(img)
    for x in (5, 10, 15, 20):
        line(d, (x, 9), (x, 23), 1.0)
    save(img, "lyceum", (52, 48))


def bridge():
    img = canvas(30, 12)
    d = ImageDraw.Draw(img)
    deck = [(0, 3), (30, 3), (30, 6), (0, 6)]
    poly(d, deck)
    hatch(img, deck, spacing=1.2, angle=90, width=0.4)
    d = ImageDraw.Draw(img)
    for x in (4, 12, 20, 27):
        line(d, (x, 6), (x, 12), 1.2)
    line(d, (0, 1), (30, 1), 0.8)
    save(img, "bridge", (60, 24))


def figure(name, skirt=False, small=False):
    img = canvas(10, 20)
    d = ImageDraw.Draw(img)
    # head
    d.ellipse(pts([(3.5, 3), (6.5, 6)]), fill=PAPER, outline=INK, width=int(0.8 * S))
    if skirt:
        poly(d, [(2.5, 3.5), (5, 1.5), (7.5, 3.5)], fill=INK)  # bonnet
        poly(d, [(3.5, 6), (6.5, 6), (9, 19), (1, 19)])
        hatch(img, [(3.5, 6), (6.5, 6), (9, 19), (1, 19)], spacing=2.4, angle=100, width=0.4)
    else:
        line(d, (1.5, 3), (8.5, 3), 1.0)  # hat brim
        poly(d, [(3.5, 1), (6.5, 1), (6.5, 3), (3.5, 3)], fill=INK)
        poly(d, [(3, 6), (7, 6), (7.5, 13), (2.5, 13)])
        line(d, (4, 13), (3.5, 19), 1.2)
        line(d, (6, 13), (6.5, 19), 1.2)
    size = (10, 20) if small else (14, 28)
    save(img, name, size)


def lamp():
    img = Image.new("RGBA", (32, 32), (0, 0, 0, 0))
    for r in range(16, 0, -1):
        a = int(255 * (1 - r / 16) ** 2)
        ImageDraw.Draw(img).ellipse([16 - r, 16 - r, 16 + r, 16 + r], fill=(255, 255, 255, a))
    img.save(os.path.join(OUT, "glow.png"))


def puff():
    img = Image.new("RGBA", (32, 32), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    for r in range(14, 0, -1):
        a = int(200 * (1 - r / 14))
        d.ellipse([16 - r, 16 - r, 16 + r, 16 + r], fill=(255, 255, 255, a))
    img.save(os.path.join(OUT, "puff.png"))


def tree():
    img = canvas(20, 28)
    d = ImageDraw.Draw(img)
    poly(d, [(9, 18), (11, 18), (11.5, 28), (8.5, 28)], fill=INK)
    crown = [(10, 1), (17, 7), (19, 13), (15, 19), (5, 19), (1, 13), (3, 7)]
    poly(d, crown)
    hatch(img, crown, spacing=1.4, angle=35, width=0.45)
    save(img, "tree", (40, 56))


def stump():
    img = canvas(12, 8)
    d = ImageDraw.Draw(img)
    poly(d, [(2, 3), (10, 3), (11, 8), (1, 8)])
    d.ellipse(pts([(2, 1), (10, 5)]), fill=PAPER, outline=INK, width=int(0.8 * S))
    line(d, (6, 2), (6, 4), 0.5)
    save(img, "stump", (24, 16))


def well():
    img = canvas(14, 16)
    d = ImageDraw.Draw(img)
    poly(d, [(2, 9), (12, 9), (12, 15), (2, 15)])
    hatch(img, [(2, 9), (12, 9), (12, 15), (2, 15)], spacing=1.6, angle=0, width=0.4)
    d = ImageDraw.Draw(img)
    line(d, (3, 9), (3, 3), 1.0)
    line(d, (11, 9), (11, 3), 1.0)
    poly(d, [(1, 4), (7, 0), (13, 4)])
    line(d, (7, 4), (7, 8), 0.5)
    save(img, "well", (28, 32))


def shed(name, stripes):
    img = canvas(16, 16)
    d = ImageDraw.Draw(img)
    body = [(2, 6), (14, 6), (14, 15), (2, 15)]
    poly(d, body)
    if stripes:
        for x in range(4, 14, 2):
            line(d, (x, 6), (x, 15), 0.5)
    roof = [(1, 7), (8, 1), (15, 7)]
    poly(d, roof)
    hatch(img, roof, spacing=1.4, angle=60)
    d = ImageDraw.Draw(img)
    poly(d, [(6, 10), (10, 10), (10, 15), (6, 15)], fill=INK)
    save(img, name, (32, 32))


def cellar():
    img = canvas(16, 10)
    d = ImageDraw.Draw(img)
    mound = [(0, 10)] + [(8 + 8 * math.cos(math.radians(a)), 10 - 7 * math.sin(math.radians(a))) for a in range(0, 181, 15)][::-1] + [(16, 10)]
    poly(d, mound)
    hatch(img, mound, spacing=1.2, angle=10, width=0.4)
    d = ImageDraw.Draw(img)
    poly(d, [(6, 5), (10, 5), (10, 10), (6, 10)], fill=INK)
    save(img, "cellar", (32, 20))


def rail():
    img = canvas(24, 8)
    d = ImageDraw.Draw(img)
    for i, x in enumerate(range(0, 24, 6)):
        y = 1 if i % 2 == 0 else 3
        line(d, (x, y + 4), (x + 7, y), 1.0)
        line(d, (x, y), (x + 7, y + 4), 1.0)
    save(img, "rail", (48, 16))


def corn(name, height, tassel):
    img = canvas(8, 16)
    d = ImageDraw.Draw(img)
    base = 16
    line(d, (4, base), (4, base - height), 0.9)
    for k in range(2, height, 3):
        line(d, (4, base - k), (1 if k % 2 else 7, base - k - 2), 0.6)
    if tassel:
        line(d, (4, base - height), (3, base - height - 2), 0.6)
        line(d, (4, base - height), (5, base - height - 2), 0.6)
    save(img, name, (16, 32))


def shock():
    img = canvas(10, 14)
    d = ImageDraw.Draw(img)
    sh = [(5, 0), (9, 14), (1, 14)]
    poly(d, sh)
    hatch(img, sh, spacing=1.0, angle=85, width=0.4)
    save(img, "shock", (20, 28))


def facade(name, stories, false_front, awning):
    w, h = 28, 12 + 9 * stories
    img = canvas(w, h)
    d = ImageDraw.Draw(img)
    top = 2 if false_front else 6
    body = [(1, top), (27, top), (27, h - 1), (1, h - 1)]
    poly(d, body)
    for y in range(top + 3, h - 1, 3):
        line(d, (1, y), (27, y), 0.35)
    if not false_front:
        roof = [(0, 7), (14, 0), (28, 7)]
        poly(d, roof)
        hatch(img, roof, spacing=1.4, angle=60)
        d = ImageDraw.Draw(img)
    for st in range(stories):
        y = top + 4 + 9 * st
        for x in (4, 11, 18, 23):
            poly(d, [(x, y), (x + 3, y), (x + 3, y + 4), (x, y + 4)], fill=INK)
    poly(d, [(12, h - 8), (16, h - 8), (16, h - 1), (12, h - 1)], fill=INK)
    if awning:
        aw = [(0, h - 10), (28, h - 10), (26, h - 8), (2, h - 8)]
        poly(d, aw)
        hatch(img, aw, spacing=1.0, angle=90, width=0.4)
    save(img, name, (w * 3, h * 3))


def sign():
    img = canvas(12, 16)
    d = ImageDraw.Draw(img)
    line(d, (6, 4), (6, 16), 1.2)
    poly(d, [(0, 2), (11, 2), (12, 4.5), (11, 7), (0, 7)])
    save(img, "sign", (24, 32))


def letter():
    img = canvas(30, 20)
    d = ImageDraw.Draw(img)
    env = [(1, 1), (29, 1), (29, 19), (1, 19)]
    poly(d, env)
    line(d, (1, 1), (15, 11), 1.0)
    line(d, (29, 1), (15, 11), 1.0)
    d.ellipse(pts([(13, 9), (17, 13)]), fill=INK)
    save(img, "letter", (120, 80))


def cross():
    img = canvas(16, 24)
    d = ImageDraw.Draw(img)
    poly(d, [(7, 1), (9, 1), (9, 22), (7, 22)])
    poly(d, [(3, 6), (13, 6), (13, 8), (3, 8)])
    mound = [(0, 24), (4, 21), (12, 21), (16, 24)]
    poly(d, mound)
    hatch(img, mound, spacing=1.0, angle=20, width=0.4)
    save(img, "cross", (64, 96))


def rifles():
    img = canvas(28, 24)
    d = ImageDraw.Draw(img)
    for a in (-0.35, 0.0, 0.35):
        x0 = 14 + 10 * math.sin(a)
        line(d, (14 - 6 * math.sin(a), 23), (x0, 1), 1.4)
        line(d, (14 - 6 * math.sin(a), 23), (14 - 6 * math.sin(a) + 2, 18), 2.4)
    save(img, "rifles", (112, 96))


def ballot():
    img = canvas(24, 20)
    d = ImageDraw.Draw(img)
    box = [(2, 6), (22, 6), (22, 19), (2, 19)]
    poly(d, box)
    hatch(img, box, spacing=1.6, angle=90, width=0.4)
    d = ImageDraw.Draw(img)
    poly(d, [(8, 5), (16, 5), (16, 7), (8, 7)], fill=INK)
    poly(d, [(9, 0), (15, 0), (15, 6), (9, 6)])
    save(img, "ballot", (96, 80))


if __name__ == "__main__":
    os.makedirs(OUT, exist_ok=True)
    cabin()
    barn()
    ruin()
    rick()
    school()
    church()
    lyceum()
    bridge()
    figure("man")
    figure("woman", skirt=True)
    figure("child", small=True)
    lamp()
    puff()
    tree()
    stump()
    well()
    shed("smokehouse", False)
    shed("crib", True)
    cellar()
    rail()
    corn("sprout", 5, False)
    corn("corn", 13, True)
    shock()
    facade("store", 1, True, True)
    facade("saloon", 1, True, False)
    facade("hotel", 3, False, True)
    facade("office", 1, False, False)
    sign()
    letter()
    cross()
    rifles()
    ballot()
    print("wrote", sorted(os.listdir(OUT)))

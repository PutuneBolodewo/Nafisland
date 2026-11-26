"""Draw Nafisland (PulauH) brand files: app icon, bull mascot, installer pictures, and docs assets.

Writes:
  crates/poni/res/poni.ico           app, window and tray icon; every size is drawn on its own
  installer/art/wizard-small-*.png   Inno Setup header logo (WizardSmallImageFile)
  installer/art/wizard-*.png         Inno Setup welcome and finish picture (WizardImageFile)
  installer/art/nsis-*.bmp           the same two for installer/poni.nsi
  docs/brand/app-icon.svg            Horizon Island app icon vector
  docs/brand/app-icon-512.png        512px PNG render of the app icon
  docs/brand/mascot.svg              Bull mascot vector
  docs/brand/mascot-512.png          512px PNG render of the bull mascot

Run: python3 tools/make_brand.py  (needs Pillow)
"""
import math
import re
import sys
from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
BRAND = ROOT / "docs" / "brand"
RES = ROOT / "crates" / "poni" / "res"
ART = ROOT / "installer" / "art"

# App icon (Option 1 — Horizon Island) palette
SKY_STOPS = [(0.0, "#0B132B"), (0.55, "#1C3D72"), (1.0, "#F59E0B")]
SUN_STOPS = [(0.0, "#FEF08A"), (1.0, "#F97316")]
ISLAND_DARK = "#090D16"
NOTCH_BG = "#09090B"
MEDIA_ACCENT = "#38BDF8"
EQ_COLOR = "#FBBF24"
STAR_COLOR = "#E2E8F0"

# Bull mascot palette
BULL_DARK = "#1C1C21"
BULL_STROKE = "#32323A"
HORN_WHITE = "#F8FAFC"
HORN_STROKE = "#111115"
EYE_WHITE = "#FFFFFF"
PUPIL_INK = "#171717"
SNOUT_PEACH = "#E89B6C"

ICO_SIZES = [16, 20, 24, 32, 40, 48, 64, 96, 128, 256]
WIZARD_SIZES = [(202, 386), (336, 643), (430, 824)]
WIZARD_SMALL_SIZES = [58, 77, 97, 116, 124, 143, 159]


def flatten(d: str, steps: int = 20) -> list[list[tuple[float, float]]]:
    """Absolute M/L/Q/Z path data to polygons."""
    toks = re.findall(r"[MLQZ]|-?\d*\.?\d+(?:e-?\d+)?", d)
    polys, pts, i = [], [], 0
    num = lambda k: float(toks[k])
    while i < len(toks):
        t = toks[i]
        if t == "M":
            if pts:
                polys.append(pts)
            pts, i = [(num(i + 1), num(i + 2))], i + 3
        elif t == "L":
            pts.append((num(i + 1), num(i + 2)))
            i += 3
        elif t == "Q":
            (x0, y0), (cx, cy), (x1, y1) = pts[-1], (num(i + 1), num(i + 2)), (num(i + 3), num(i + 4))
            for s in range(1, steps + 1):
                u = s / steps
                pts.append(((1 - u) ** 2 * x0 + 2 * (1 - u) * u * cx + u * u * x1, (1 - u) ** 2 * y0 + 2 * (1 - u) * u * cy + u * u * x1 if False else (1 - u) ** 2 * y0 + 2 * (1 - u) * u * cy + u * u * y1))
            i += 5
        elif t == "Z":
            polys.append(pts)
            pts, i = [], i + 1
        else:
            raise ValueError(f"unexpected path token {t!r}")
    if pts:
        polys.append(pts)
    return polys


def gradient(w: int, h: int, stops: list[tuple[float, str]]) -> Image.Image:
    rgb = [(u, Image.new("RGB", (1, 1), c).getpixel((0, 0))) for u, c in stops]
    col = Image.new("RGB", (1, max(1, h)))
    for y in range(h):
        u = y / max(1, h - 1)
        (u0, a), (u1, b) = next((rgb[k], rgb[k + 1]) for k in range(len(rgb) - 1) if u <= rgb[k + 1][0] or k == len(rgb) - 2)
        t = 0 if u1 == u0 else min(1, max(0, (u - u0) / (u1 - u0)))
        col.putpixel((0, y), tuple(round(a[i] + (b[i] - a[i]) * t) for i in range(3)))
    return col.resize((max(1, w), max(1, h)))


def draw_logo(size: int, ss: int = 8) -> Image.Image:
    """Option 1 (Horizon Island) rendered onto a transparent square."""
    s = size * ss
    k = s / 512.0
    p = lambda v: v * k

    tile_mask = Image.new("L", (s, s), 0)
    ImageDraw.Draw(tile_mask).rounded_rectangle([p(24), p(24), p(488), p(488)], radius=p(112), fill=255)

    # Sky gradient inside tile bounds
    canvas = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    sky_h = max(1, round(p(464)))
    sky_w = max(1, round(p(464)))
    sky = gradient(sky_w, sky_h, SKY_STOPS).convert("RGBA")
    canvas.paste(sky, (round(p(24)), round(p(24))))

    draw = ImageDraw.Draw(canvas)

    # Subtle stars (skip below 32px so icon stays crisp)
    if size >= 32:
        for cx, cy, r, alpha in [(118, 186, 5.0, 180), (394, 168, 6.0, 204), (348, 214, 3.5, 128)]:
            overlay = Image.new("RGBA", (s, s), (0, 0, 0, 0))
            ImageDraw.Draw(overlay).ellipse(
                [p(cx - r), p(cy - r), p(cx + r), p(cy + r)],
                fill=(0xE2, 0xE8, 0xF0, alpha),
            )
            canvas = Image.alpha_composite(canvas, overlay)
            draw = ImageDraw.Draw(canvas)

    # Sun gradient circle
    sun_r = round(p(88))
    sun_grad = gradient(2 * sun_r, 2 * sun_r, SUN_STOPS).convert("RGBA")
    sun_mask = Image.new("L", (2 * sun_r, 2 * sun_r), 0)
    ImageDraw.Draw(sun_mask).ellipse([0, 0, 2 * sun_r - 1, 2 * sun_r - 1], fill=255)
    canvas.paste(sun_grad, (round(p(256)) - sun_r, round(p(340)) - sun_r), sun_mask)
    draw = ImageDraw.Draw(canvas)

    # Island silhouette at bottom
    isl_pts = flatten("M24 388 Q156 320 256 356 Q364 394 488 334 L488 488 L24 488 Z")[0]
    draw.polygon([(p(x), p(y)) for x, y in isl_pts], fill=ISLAND_DARK)

    # Hanging notch at top edge
    notch_pts = flatten("M104 24 Q122 24 122 42 L122 86 Q122 120 156 120 L356 120 Q390 120 390 86 L390 42 Q390 24 408 24 Z")[0]
    draw.polygon([(p(x), p(y)) for x, y in notch_pts], fill=NOTCH_BG)

    # Media tile + equalizer bars inside notch (only >= 24px so tiny tray icon stays clean)
    if size >= 24:
        draw.rounded_rectangle([p(148), p(48), p(194), p(94)], radius=p(12), fill=MEDIA_ACCENT)
        for bx, by, bh in [(296, 64, 16), (314, 54, 36), (332, 60, 24), (350, 50, 44)]:
            draw.rounded_rectangle([p(bx), p(by), p(bx + 10), p(by + bh)], radius=p(5), fill=EQ_COLOR)

    # Apply rounded tile alpha mask
    out = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    out.paste(canvas, (0, 0), tile_mask)
    return out.resize((size, size), Image.LANCZOS)


def rotated_ellipse_poly(cx: float, cy: float, rx: float, ry: float, deg: float, steps: int = 24) -> list[tuple[float, float]]:
    rad = math.radians(deg)
    ca, sa = math.cos(rad), math.sin(rad)
    pts = []
    for i in range(steps):
        a = 2 * math.pi * i / steps
        lx, ly = rx * math.cos(a), ry * math.sin(a)
        pts.append((cx + lx * ca - ly * sa, cy + lx * sa + ly * ca))
    return pts


def draw_poly_outline(draw: ImageDraw.ImageDraw, pts: list[tuple[float, float]], fill: str, stroke: str | None = None, width: float = 0.0) -> None:
    draw.polygon(pts, fill=fill)
    if stroke and width > 0:
        draw.line([*pts, pts[0]], fill=stroke, width=max(1, round(width)), joint="curve")


def draw_mascot(size: int, ss: int = 6) -> Image.Image:
    """Draw the bull mascot matching docs/brand/mascot.svg."""
    s = size * ss
    # viewBox="-64 -76 128 128"
    k = s / 128.0
    tx = lambda x: (x + 64.0) * k
    ty = lambda y: (y + 76.0) * k
    map_pts = lambda pts: [(tx(x), ty(y)) for x, y in pts]

    img = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    # Ears
    ear_l = flatten("M-24 -19 Q-41 -20 -51 -11 Q-53 -8 -48 -6 Q-37 -5 -25 -12 Z")[0]
    ear_r = flatten("M24 -19 Q41 -20 51 -11 Q53 -8 48 -6 Q37 -5 25 -12 Z")[0]
    draw_poly_outline(draw, map_pts(ear_l), BULL_DARK, BULL_STROKE, 1.2 * k)
    draw_poly_outline(draw, map_pts(ear_r), BULL_DARK, BULL_STROKE, 1.2 * k)

    # Head
    head = flatten("M-22 -35 Q0 -37 22 -35 Q25 -18 29 2 Q34 24 31 33 Q24 38.52 0 38.52 Q-24 38.52 -31 33 Q-34 24 -29 2 Q-25 -18 -22 -35 Z")[0]
    draw_poly_outline(draw, map_pts(head), BULL_DARK, BULL_STROKE, 1.2 * k)

    # Horns
    horn_l = flatten("M-21 -34 Q-31 -32 -37 -40 Q-43 -48 -42 -64 Q-53 -48 -50 -33 Q-46 -20 -24 -21 Z")[0]
    horn_r = flatten("M21 -34 Q31 -32 37 -40 Q43 -48 42 -64 Q53 -48 50 -33 Q46 -20 24 -21 Z")[0]
    draw_poly_outline(draw, map_pts(horn_l), HORN_WHITE, HORN_STROKE, 1.4 * k)
    draw_poly_outline(draw, map_pts(horn_r), HORN_WHITE, HORN_STROKE, 1.4 * k)

    # Eyes (half-lidded deadpan)
    eye_l = flatten("M-19 -18 L-2 -18 Q-2 -4 -10.5 -4 Q-19 -4 -19 -18 Z")[0]
    eye_r = flatten("M2 -18 L19 -18 Q19 -4 10.5 -4 Q2 -4 2 -18 Z")[0]
    draw.polygon(map_pts(eye_l), fill=EYE_WHITE)
    draw.polygon(map_pts(eye_r), fill=EYE_WHITE)

    for px, py in [(-8.2, -14.2), (8.2, -14.2)]:
        r = 3.4 * k
        cx, cy = tx(px), ty(py)
        draw.ellipse([cx - r, cy - r, cx + r, cy + r], fill=PUPIL_INK)

    # Snout & nostrils
    snout = flatten("M0 6 Q23 6 24 21 Q24 34 0 35 Q-24 34 -24 21 Q-23 6 0 6 Z")[0]
    draw.polygon(map_pts(snout), fill=SNOUT_PEACH)
    draw.polygon(map_pts(rotated_ellipse_poly(-8.0, 16.5, 2.1, 2.8, -18.0)), fill=PUPIL_INK)
    draw.polygon(map_pts(rotated_ellipse_poly(8.0, 16.5, 2.1, 2.8, 18.0)), fill=PUPIL_INK)

    return img.resize((size, size), Image.LANCZOS)


def wizard_image(w: int, h: int, mascot: Image.Image) -> Image.Image:
    """Welcome/finish picture: bull mascot on the Horizon island."""
    img = gradient(w, h, [(0.0, "#0B132B"), (0.58, "#1C3D72"), (1.0, "#F59E0B")]).convert("RGBA")
    draw = ImageDraw.Draw(img)

    # Soft glowing sun behind the mascot
    sun_r = round(w * 0.34)
    sun_cx, sun_cy = w // 2, round(h * 0.56)
    sun_grad = gradient(2 * sun_r, 2 * sun_r, SUN_STOPS).convert("RGBA")
    sun_mask = Image.new("L", (2 * sun_r, 2 * sun_r), 0)
    ImageDraw.Draw(sun_mask).ellipse([0, 0, 2 * sun_r - 1, 2 * sun_r - 1], fill=255)
    img.paste(sun_grad, (sun_cx - sun_r, sun_cy - sun_r), sun_mask)
    draw = ImageDraw.Draw(img)

    # Island pill bar at bottom-middle
    pw = round(w * 0.80)
    ph = max(20, round(h * 0.095))
    px = (w - pw) // 2
    py = round(h * 0.64)

    # Mascot sitting right behind the island pill
    ms = round(w * 0.68)
    m_img = mascot.resize((ms, ms), Image.LANCZOS)
    mx = (w - ms) // 2
    my = py - round(ms * 0.83)
    img.alpha_composite(m_img, (mx, my))

    draw = ImageDraw.Draw(img)
    draw.rounded_rectangle([px, py, px + pw, py + ph], radius=ph // 2, fill=NOTCH_BG)
    art = round(ph * 0.58)
    pad = (ph - art) // 2
    draw.rounded_rectangle([px + pad + 4, py + pad, px + pad + 4 + art, py + pad + art], radius=max(3, art // 4), fill=MEDIA_ACCENT)
    bw = max(2, round(w * 0.016))
    gap = max(2, round(w * 0.014))
    eq_x = px + pw - pad - 4 - (4 * bw + 3 * gap)
    for i, frac in enumerate([0.45, 0.85, 0.60, 1.0]):
        bh = max(3, round(art * 0.9 * frac))
        bx = eq_x + i * (bw + gap)
        by = py + (ph - bh) // 2
        draw.rounded_rectangle([bx, by, bx + bw, by + bh], radius=bw // 2, fill=EQ_COLOR)

    return img


def main() -> int:
    icons = {n: draw_logo(n) for n in ICO_SIZES}
    icons[256].save(
        RES / "poni.ico",
        format="ICO",
        sizes=[(n, n) for n in ICO_SIZES],
        append_images=[icons[n] for n in ICO_SIZES[:-1]],
    )

    draw_logo(512).save(BRAND / "app-icon-512.png", optimize=True)
    mascot_512 = draw_mascot(512)
    mascot_512.save(BRAND / "mascot-512.png", optimize=True)

    ART.mkdir(exist_ok=True)
    for n in WIZARD_SMALL_SIZES:
        draw_logo(n).save(ART / f"wizard-small-{n}.png", optimize=True)
    for w, h in WIZARD_SIZES:
        wizard_image(w, h, mascot_512).convert("RGB").save(ART / f"wizard-{w}x{h}.png", optimize=True)

    # NSIS (MUI2) only takes 24-bit BMPs: 164x314 welcome/finish picture, 150x57 header.
    wizard_image(164, 314, mascot_512).convert("RGB").save(ART / "nsis-welcome.bmp")
    header = Image.new("RGBA", (150, 57), "white")
    header.alpha_composite(draw_logo(48), (150 - 48 - 6, (57 - 48) // 2))
    header.convert("RGB").save(ART / "nsis-header.bmp")

    for p in sorted([RES / "poni.ico", BRAND / "app-icon.svg", BRAND / "app-icon-512.png", BRAND / "mascot.svg", BRAND / "mascot-512.png", *ART.iterdir()]):
        print(f"{p.relative_to(ROOT)}  {p.stat().st_size} bytes")
    return 0


if __name__ == "__main__":
    sys.exit(main())

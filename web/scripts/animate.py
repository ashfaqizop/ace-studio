"""
Pixel-art animator for the Ace Studio crew.

Turns each static sprite in public/characters/<set>/<id>.png into a 16-frame
animation sheet, like an Aseprite timeline, by working directly on pixels:

  * feet   - the leg/shadow rows stay planted
  * body   - breathes: shifts down 1px on alternate beats (squash)
  * tools  - the held accessory (pixels beside the body) plays a role action:
             hammer smash, magnifier scan, broom sweep, typing taps, rocket lift...
  * eyes   - blink once per loop when a pair of eyes can be detected
  * fx     - sparkles for the magic/lab roles

Outputs per character:
  public/characters/anim/<set>/<id>.png   horizontal sheet, 16 frames
  public/characters/anim/<set>/<id>.json  Aseprite sprite-sheet JSON (frames + durations + tags)
and a preview contact sheet GIF in scripts/out/ for review.

Usage: python scripts/animate.py
"""
import json
import pathlib

from PIL import Image

ROOT = pathlib.Path(__file__).resolve().parent.parent
SRC = ROOT / 'public' / 'characters'
OUT = SRC / 'anim'
PREVIEW = pathlib.Path(__file__).resolve().parent / 'out'
SETS = ['claude', 'ambient', 'codex', 'grok']
FRAMES = 16
FRAME_MS = 100  # 10fps, the classic pixel-art pace

BOB = [0, 0, 1, 1, 0, 0, 1, 1] * 2  # body offset per frame

# Accessory motion presets: per-frame (dx, dy) for the right-hand tool; the left
# tool gets the mirrored/opposite motion. 8-frame cycles are played twice.
ACTIONS = {
    'hammer': [(0, 0), (0, -1), (0, -2), (0, -3), (0, 1), (0, 1), (0, 0), (0, 0)],
    'scan':   [(0, 0), (1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1), (0, -1)],
    'sweep':  [(0, 0), (-1, 0), (-2, 0), (-1, 0), (0, 0), (1, 0), (2, 0), (1, 0)],
    'type':   [(0, 0), (0, 1), (0, 0), (0, 1), (0, 0), (0, 0), (0, 1), (0, 0)],
    'lift':   [(0, 0), (0, -1), (0, -2), (0, -3), (0, -4), (0, -2), (0, -1), (0, 0)],
    'wave':   [(0, 0), (0, -1), (0, -1), (0, 0), (0, 0), (0, 1), (0, 1), (0, 0)],
    'twist':  [(0, 0), (1, -1), (0, 0), (-1, -1), (0, 0), (1, -1), (0, 0), (-1, -1)],
    'dance':  [(0, -1), (0, 0), (0, -1), (0, 0), (0, -1), (0, 0), (0, -1), (0, 0)],
}

ROLE_ACTION = {
    'forge': 'hammer', 'scout': 'scan', 'inspector': 'scan', 'lens': 'scan', 'oracle': 'scan',
    'tidy': 'sweep', 'mop': 'sweep', 'shell': 'type', 'scribe': 'type', 'schema': 'type',
    'pocket': 'type', 'architect': 'type', 'deploy': 'lift', 'patch': 'twist', 'pipe': 'twist',
    'spin': 'dance', 'bard': 'dance', 'stretch': 'dance', 'pixel': 'sweep',
}
SPARKLE = {'oracle', 'probe', 'stylist', 'orchy', 'tick', 'nimbus'}


def lum(px):
    r, g, b, a = px
    return 0.299 * r + 0.587 * g + 0.114 * b


def analyse(im):
    w, h = im.size
    px = im.load()
    opaque = [[px[x, y][3] > 0 for x in range(w)] for y in range(h)]
    col = [sum(opaque[y][x] for y in range(h)) for x in range(w)]
    cx = max(range(w), key=lambda x: (col[x], -abs(x - w / 2)))
    # body x-range: contiguous run of tall columns around the centre
    thresh = 0.45 * max(col)
    x0 = x1 = cx
    while x0 - 1 >= 0 and col[x0 - 1] >= thresh:
        x0 -= 1
    while x1 + 1 < w and col[x1 + 1] >= thresh:
        x1 += 1
    # body bottom: lowest row whose width inside the body range is still "body wide"
    widths = [sum(opaque[y][x] for x in range(x0, x1 + 1)) for y in range(h)]
    maxw = max(widths)
    cut = h
    for y in range(h - 1, -1, -1):
        if widths[y] >= 0.6 * maxw:
            cut = y + 1
            break
    return opaque, x0, x1, cut


def find_eyes(im, opaque, x0, x1, cut):
    """Dark interior pixel clusters in the upper body that come in a level pair."""
    w, h = im.size
    px = im.load()
    top = next((y for y in range(h) if any(opaque[y][x0:x1 + 1])), 0)
    limit = top + int((cut - top) * 0.75)
    cand = set()
    for y in range(top + 1, limit):
        for x in range(max(x0, 1), min(x1, w - 2) + 1):
            if not opaque[y][x] or lum(px[x, y]) > 70:
                continue
            if all(opaque[y + dy][x + dx] for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1))):
                cand.add((x, y))
    clusters, seen = [], set()
    for p in cand:
        if p in seen:
            continue
        stack, cl = [p], []
        seen.add(p)
        while stack:
            q = stack.pop()
            cl.append(q)
            for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                n = (q[0] + dx, q[1] + dy)
                if n in cand and n not in seen:
                    seen.add(n)
                    stack.append(n)
        ys = [c[1] for c in cl]
        if 2 <= len(cl) <= 12 and max(ys) - min(ys) >= 1:
            clusters.append(cl)
    best = None
    for i in range(len(clusters)):
        for j in range(i + 1, len(clusters)):
            a, b = clusters[i], clusters[j]
            ya, yb = min(c[1] for c in a), min(c[1] for c in b)
            xa, xb = min(c[0] for c in a), min(c[0] for c in b)
            if abs(ya - yb) <= 1 and 3 <= abs(xa - xb) <= 24 and abs(len(a) - len(b)) <= 2:
                score = -abs(ya - yb) - abs(len(a) - len(b))
                if best is None or score > best[0]:
                    best = (score, a, b)
    return [best[1], best[2]] if best else []


def blink(frame, eyes):
    px = frame.load()
    for cl in eyes:
        top = min(c[1] for c in cl)
        bottom = max(c[1] for c in cl)
        for x, y in cl:
            if y < bottom:
                above = px[x, top - 1] if top > 0 else px[x, y]
                px[x, y] = above


def sparkle(frame, x, y, size):
    w, h = frame.size
    px = frame.load()
    pts = [(0, 0)] if size == 1 else [(0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)]
    for dx, dy in pts:
        if 0 <= x + dx < w and 0 <= y + dy < h:
            px[x + dx, y + dy] = (255, 246, 196, 255) if (dx, dy) == (0, 0) else (255, 214, 92, 230)


def animate(path, role):
    im = Image.open(path).convert('RGBA')
    w, h = im.size
    opaque, x0, x1, cut = analyse(im)
    eyes = find_eyes(im, opaque, x0, x1, cut)
    margin = 3
    src = im.load()
    feet = Image.new('RGBA', (w, h))
    body = Image.new('RGBA', (w, h))
    left = Image.new('RGBA', (w, h))
    right = Image.new('RGBA', (w, h))
    for y in range(h):
        for x in range(w):
            p = src[x, y]
            if p[3] == 0:
                continue
            if y >= cut:
                feet.putpixel((x, y), p)
            elif x < x0 - margin:
                left.putpixel((x, y), p)
            elif x > x1 + margin:
                right.putpixel((x, y), p)
            else:
                body.putpixel((x, y), p)
    action = ACTIONS[ROLE_ACTION.get(role, 'wave')]
    sheet = Image.new('RGBA', (w * FRAMES, h))
    rbox = right.getbbox()
    for f in range(FRAMES):
        fr = Image.new('RGBA', (w, h))
        dy = BOB[f]
        fr.alpha_composite(feet)
        fr.alpha_composite(body, (0, dy))
        ax, ay = action[f % len(action)]
        fr.alpha_composite(right, (ax, ay + dy))
        lx, ly = action[(f + 4) % len(action)]
        fr.alpha_composite(left, (-lx, ly + dy))
        if eyes and f == 12:
            eyes_shifted = [[(x, y + dy) for x, y in cl] for cl in eyes]
            blink(fr, eyes_shifted)
        if role in SPARKLE and rbox and f % 8 in (2, 3, 4):
            sx = min(w - 2, rbox[2] + ax)
            sy = max(1, rbox[1] + ay + dy - 1)
            sparkle(fr, sx, sy, 1 if f % 8 == 4 else 2)
        sheet.paste(fr, (f * w, 0))
    return sheet, (w, h), bool(eyes)


def aseprite_json(name, size, image):
    w, h = size
    return {
        'frames': [
            {'filename': f'{name} {i}.ase', 'frame': {'x': i * w, 'y': 0, 'w': w, 'h': h}, 'rotated': False,
             'trimmed': False, 'spriteSourceSize': {'x': 0, 'y': 0, 'w': w, 'h': h},
             'sourceSize': {'w': w, 'h': h}, 'duration': FRAME_MS}
            for i in range(FRAMES)
        ],
        'meta': {'app': 'ace-studio animate.py', 'version': '1.0', 'image': image, 'format': 'RGBA8888',
                 'size': {'w': w * FRAMES, 'h': h}, 'scale': '1',
                 'frameTags': [{'name': 'loop', 'from': 0, 'to': FRAMES - 1, 'direction': 'forward'}]},
    }


def main():
    PREVIEW.mkdir(exist_ok=True)
    report = []
    previews = []
    for s in SETS:
        (OUT / s).mkdir(parents=True, exist_ok=True)
        for f in sorted((SRC / s).glob('*.png')):
            sheet, size, blinked = animate(f, f.stem)
            sheet.save(OUT / s / f.name, optimize=True)
            (OUT / s / f'{f.stem}.json').write_text(json.dumps(aseprite_json(f.stem, size, f.name), indent=1))
            report.append(f'{s}/{f.stem}: {ROLE_ACTION.get(f.stem, "wave")}{" +blink" if blinked else ""}')
            previews.append((sheet, size))
    # preview GIF: every character, 3x, in a grid
    scale, cols = 3, 12
    cw, ch = 48 * scale + 8, 64 * scale + 8
    rows = (len(previews) + cols - 1) // cols
    gif = []
    for i in range(FRAMES):
        canvas = Image.new('RGBA', (cols * cw, rows * ch), (24, 22, 30, 255))
        for k, (sheet, (w, h)) in enumerate(previews):
            fr = sheet.crop((i * w, 0, (i + 1) * w, h)).resize((w * scale, h * scale), Image.NEAREST)
            canvas.alpha_composite(fr, ((k % cols) * cw + 4, (k // cols) * ch + 4 + (64 - h) * scale))
        gif.append(canvas.convert('RGB').quantize(colors=255, method=Image.Quantize.MEDIANCUT, dither=Image.Dither.NONE))
    gif[0].save(PREVIEW / 'crew-preview.gif', save_all=True, append_images=gif[1:], duration=FRAME_MS, loop=0, disposal=1)
    print('\n'.join(report))
    print(f'{len(previews)} characters animated -> {OUT}')


if __name__ == '__main__':
    main()

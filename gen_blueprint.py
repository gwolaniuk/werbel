# -*- coding: utf-8 -*-
"""Generuje blueprint werbla (niebieski techniczny rysunek) jako surowe RGBA.

Wyjście: assets/werbel_blueprint.rgba (940x940, RGBA8, unpremultiplied).
Geometria dopasowana do layout() w editor.rs (470x470 logiczne, tutaj skala x2).
"""
from PIL import Image, ImageDraw, ImageFont

W, H = 940, 940
S = 2  # skala

# kolory blueprintu
BG = (22, 58, 110)          # klasyczny ciemnoniebieski
LINE = (232, 238, 247)      # biała kreska
LINE_DIM = (140, 170, 210)  # szara linia wymiarowa
FILL = (30, 75, 140)        # wypełnienie przekroju
TEXT = (236, 240, 248)

img = Image.new("RGBA", (W, H), BG)
d = ImageDraw.Draw(img)

def font(sz):
    for p in (r"C:\Windows\Fonts\consola.ttf", r"C:\Windows\Fonts\cour.ttf"):
        try:
            return ImageFont.truetype(p, int(sz * S))
        except Exception:
            continue
    return ImageFont.load_default()

# geometria (logiczne 470, skala 2)
cx = 235 * S
drum_w = 206.8 * S
half = drum_w / 2
rim_h = 14 * S
y0 = 40 * S
top_rim = (cx - half - 8*S, y0, cx + half + 8*S, y0 + rim_h)
batter = (cx - half, top_rim[3] + 3*S, cx + half, top_rim[3] + 3*S + 10*S)
shell = (cx - half, batter[3] + 2*S, cx + half, batter[3] + 2*S + 240*S)
reson = (cx - half, shell[3] + 2*S, cx + half, shell[3] + 2*S + 10*S)
bottom_rim = (cx - half - 8*S, reson[3] + 3*S, cx + half + 8*S, reson[3] + 3*S + rim_h)
snares_y0 = bottom_rim[3] + 6*S
snares_y1 = snares_y0 + 60*S

# ramka
d.rectangle([14, 14, W-14, H-14], outline=LINE, width=2)

# --- obręcze (counterhoop) — wypełnione ---
d.rectangle(top_rim, fill=FILL, outline=LINE, width=2)
d.rectangle(bottom_rim, fill=FILL, outline=LINE, width=2)

# --- naciągi (kreskowane) ---
for r in (batter, reson):
    d.rectangle(r, fill=(24, 50, 95), outline=LINE, width=2)
    x0, y0r, x1, y1r = r
    step = 10 * S
    for x in range(int(x0), int(x1), step):
        d.line([x, y1r, x + 8, y0r], fill=(90, 130, 180), width=1)

# --- korpus (przekrój: kreskowanie ukośne) ---
d.rectangle(shell, fill=(26, 66, 130), outline=LINE, width=2)
x0, y0r, x1, y1r = shell
step = 26 * S
for y in range(int(y0r - (x1-x0)), int(y1r), step):
    d.line([x0, y + (x1-x0), x1, y], fill=(70, 105, 165), width=1)

# --- śruby naciągowe + lugs ---
for i in range(6):
    x = shell[0] + 14*S + i * (drum_w - 28*S) / 5
    d.line([x, top_rim[1] - 10*S, x, bottom_rim[3] + 10*S], fill=LINE, width=int(2.5*S))
    lug = [x - 5*S, (shell[1]+shell[3])/2 - 15*S, x + 5*S, (shell[1]+shell[3])/2 + 15*S]
    d.rectangle(lug, fill=(120, 90, 50), outline=LINE, width=2)

# wyróżniona śruba SWEEP (prawa krawędź)
rx = shell[2] - 18*S
d.line([rx, top_rim[1] - 14*S, rx, bottom_rim[3] + 14*S], fill=(255, 200, 90), width=int(3.5*S))

# --- otwór wentylacyjny ---
vx, vy = shell[2] - 34*S, shell[1] + 30*S
d.ellipse([vx-7*S, vy-7*S, vx+7*S, vy+7*S], fill=LINE)
d.ellipse([vx-3*S, vy-3*S, vx+3*S, vy+3*S], fill=(22, 58, 110))

# --- sprężyny (zigzag) ---
n = 20
sw = (snares_y1 - snares_y0) * 0.7
left = cx - half - 12*S
right = cx + half + 12*S
d.line([left, snares_y0 + sw*0.15, right, snares_y0 + sw*0.15], fill=LINE, width=2)
d.line([left, snares_y0 + sw*0.85, right, snares_y0 + sw*0.85], fill=LINE, width=2)
for i in range(n):
    xa = left + (right-left) * i / n
    xb = left + (right-left) * (i+1) / n
    za = snares_y0 + sw*(0.15 if i % 2 == 0 else 0.85)
    zb = snares_y0 + sw*(0.85 if i % 2 == 0 else 0.15)
    d.line([xa, za, xb, zb], fill=LINE, width=1)

# --- strainer / throw-off (lewa strona) ---
tx = shell[0] - 24*S
d.line([tx, reson[3] + 4*S, tx, reson[3] + 44*S], fill=LINE, width=int(3*S))
d.ellipse([tx-5*S, reson[3]+40*S, tx+5*S, reson[3]+50*S], fill=(120, 90, 50), outline=LINE, width=2)

# --- pałeczki ---
tip = (batter[0] + (batter[2]-batter[0])*0.5 + 30*S, top_rim[1] - 4*S)
tail = (tip[0] + 150*S, tip[1] - 96*S)
d.line([tail, tip], fill=(200, 160, 100), width=int(3.5*S))
d.ellipse([tip[0]-5*S, tip[1]-5*S, tip[0]+5*S, tip[1]+5*S], fill=(200, 160, 100))

# --- linie wymiarowe (po prawej) ---
dim_x = shell[2] + 60*S
for y_a, y_b, label in [(top_rim[1], top_rim[3], "COUNTERHOOP"),
                        (batter[1], batter[3], "BATTER"),
                        (shell[1], shell[3], "SHELL"),
                        (reson[1], reson[3], "RESONANT")]:
    d.line([dim_x, y_a, dim_x, y_b], fill=LINE_DIM, width=1)
    d.line([dim_x-5, y_a, dim_x, y_a], fill=LINE_DIM, width=1)
    d.line([dim_x-5, y_b, dim_x, y_b], fill=LINE_DIM, width=1)

# --- dymki (label callouts) ---
def callout(x, y, text, anchor_side="left"):
    f = font(11)
    tx_box = x
    if anchor_side == "right":
        d.text((tx_box, y), text, font=f, fill=TEXT, anchor="rm")
    else:
        d.text((tx_box, y), text, font=f, fill=TEXT, anchor="lm")

# uproszczone etykiety (bez strzałek — oszczędnie)
f8 = font(9)
lx = shell[2] + 16*S
d.text((lx, (top_rim[1]+top_rim[3])/2), "COUNTERHOOP  — DAMPING", font=f8, fill=TEXT, anchor="lm")
d.text((lx, batter[3] - 0*S), "BATTER HEAD  — TONE", font=f8, fill=TEXT, anchor="lm")
d.text((lx, shell[1] + 20*S), "AIR VENT", font=f8, fill=TEXT, anchor="lm")
d.text((lx, (shell[1]+shell[3])/2 - 6*S), "SHELL  — RING", font=f8, fill=TEXT, anchor="lm")
d.text((lx, shell[3] - 26*S), "TENSION ROD  — SWEEP", font=f8, fill=TEXT, anchor="lm")
d.text((lx, reson[3] + 2*S), "RESONANT HEAD", font=f8, fill=TEXT, anchor="lm")
d.text((lx, (snares_y0+snares_y1)/2), "SNARES  — SNARE", font=f8, fill=TEXT, anchor="lm")
d.text((lx, snares_y1 + 12*S), "STRAINER", font=f8, fill=TEXT, anchor="lm")
d.text((tail[0] - 30*S, tail[1] - 12*S), "STICK  — SNAP", font=f8, fill=TEXT, anchor="rm")

# --- tabliczka (title block) ---
tb = [W-250*S, H-120*S, W-14, H-14]
d.rectangle(tb, outline=LINE, width=2)
d.line([tb[0], tb[1]+36*S, tb[2], tb[1]+36*S], fill=LINE, width=2)
d.line([tb[0], tb[1]+72*S, tb[2], tb[1]+72*S], fill=LINE, width=2)
d.text((tb[0]+12, tb[1]+6), "WERBEL", font=font(16), fill=TEXT)
d.text((tb[0]+12, tb[1]+40), "SNARE DRUM — FEEDBACK SYNTHESIS", font=font(9), fill=TEXT)
d.text((tb[0]+12, tb[1]+76), "SCALE 1:1 · DRAWN BY gwolaniuk · 2026", font=font(8), fill=TEXT)

img.save("assets/werbel_blueprint.rgba")
# kopia png dla podglądu
img.save("assets/werbel_blueprint.png")
print("OK -> assets/werbel_blueprint.rgba + .png", img.size)
#!/usr/bin/env python3
"""Render a color snapshot to PNG (optional developer tool; requires Pillow).
Glyph positions and RGB colors come from Ratatui's actual buffer. Font rendering
is approximate; terminal font weight/cursor rendering may differ.
"""
import re
import subprocess
import sys
from PIL import Image, ImageDraw, ImageFont

binary, view, width, height, destination = sys.argv[1:]
width, height = int(width), int(height)
source = subprocess.check_output(
    [binary, "--snapshot-ansi", str(width), str(height), view], text=True
)
font = ImageFont.truetype("DejaVuSansMono.ttf", 14)
image = Image.new("RGB", (width * 9 + 32, height * 20 + 32), (28, 31, 36))
draw = ImageDraw.Draw(image)
pattern = re.compile(r"\x1b\[38;2;(\d+);(\d+);(\d+);48;2;(\d+);(\d+);(\d+)m([^\x1b]*)")
for row, line in enumerate(source.splitlines()):
    for col, match in enumerate(pattern.finditer(line)):
        values = list(map(int, match.groups()[:6]))
        x, y = 16 + col * 9, 16 + row * 20
        draw.rectangle((x, y, x + 8, y + 19), fill=tuple(values[3:]))
        draw.text((x, y), match.group(7), font=font, fill=tuple(values[:3]))
image.save(destination)
print(destination)

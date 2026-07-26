#!/usr/bin/env python3
"""Generate VoxMinutes app icons: Streamlit-red rounded square with white
waveform bars and a "VoxM" monogram.

Outputs:
- frontend/src-tauri/icons/*.png (all Tauri/Windows sizes)
- frontend/src-tauri/icons/icon.ico + app_icon.ico (multi-size, BMP+PNG frames)
- frontend/public/icons/logo.png

Requires: Pillow (pip install pillow)
"""

import io
import struct
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).parent.parent
TAURI_ICONS = ROOT / "frontend" / "src-tauri" / "icons"
PUBLIC_ICONS = ROOT / "frontend" / "public" / "icons"

BRAND_RED = (255, 75, 75, 255)  # #FF4B4B Streamlit red
WHITE = (255, 255, 255, 255)

# Waveform bar heights relative to the tallest bar (matches the header logo).
BAR_REL = [0.57, 1.0, 0.71, 1.0, 0.64]

FONT_CANDIDATES = [
    "C:/Windows/Fonts/arialbd.ttf",
    "C:/Windows/Fonts/segoeuib.ttf",
    "C:/Windows/Fonts/msyhbd.ttc",
    "C:/Windows/Fonts/arial.ttf",
]


def load_font(size: int) -> ImageFont.FreeTypeFont | None:
    for path in FONT_CANDIDATES:
        try:
            return ImageFont.truetype(path, size)
        except (OSError, IOError):
            continue
    return None


def draw_icon(canvas_size: int, with_text: bool) -> Image.Image:
    """Draw the icon at `canvas_size` (drawn at 4x then downscaled for AA)."""
    S = canvas_size * 4
    img = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)

    # Rounded-square background.
    radius = int(S * 0.22)
    d.rounded_rectangle([0, 0, S - 1, S - 1], radius=radius, fill=BRAND_RED)

    # Waveform bars.
    bar_w = int(S * 0.086)
    gap = int(S * 0.062)
    total_w = bar_w * len(BAR_REL) + gap * (len(BAR_REL) - 1)
    x0 = (S - total_w) // 2
    max_h = int(S * (0.42 if with_text else 0.50))
    center_y = int(S * (0.40 if with_text else 0.50))
    for i, rel in enumerate(BAR_REL):
        h = max(int(max_h * rel), bar_w)  # keep bar at least a dot
        x = x0 + i * (bar_w + gap)
        y = center_y - h // 2
        d.rounded_rectangle([x, y, x + bar_w, y + h], radius=bar_w // 2, fill=WHITE)

    # "VoxM" monogram.
    if with_text:
        font = load_font(int(S * 0.20))
        if font is not None:
            text = "VoxM"
            bbox = d.textbbox((0, 0), text, font=font)
            tw, th = bbox[2] - bbox[0], bbox[3] - bbox[1]
            tx = (S - tw) // 2 - bbox[0]
            ty = int(S * 0.78) - th // 2 - bbox[1]
            d.text((tx, ty), text, font=font, fill=WHITE)

    return img.resize((canvas_size, canvas_size), Image.Resampling.LANCZOS)


def bmp_frame(img: Image.Image) -> bytes:
    """Return a 32bpp BMP DIB suitable for an ICO entry (XOR + AND masks)."""
    w, h = img.size
    rgba = img.convert("RGBA").tobytes()
    row_len = w * 4
    xor = bytearray(w * h * 4)
    for y in range(h):
        src_row = rgba[y * row_len:(y + 1) * row_len]
        dst_y = h - 1 - y
        for x in range(w):
            r, g, b, a = src_row[x * 4:(x + 1) * 4]
            dst_off = dst_y * row_len + x * 4
            xor[dst_off] = b
            xor[dst_off + 1] = g
            xor[dst_off + 2] = r
            xor[dst_off + 3] = a
    stride = ((w + 31) // 32) * 4
    and_mask = bytes(stride * h)
    header = struct.pack(
        "<IiiHHIIiiIIII", 40, w, h * 2, 1, 32, 0, w * h * 4, 0, 0, 0, 0, 0, 0,
    )
    return header + bytes(xor) + and_mask


def png_frame(img: Image.Image) -> bytes:
    buf = io.BytesIO()
    img.convert("RGBA").save(buf, format="PNG")
    return buf.getvalue()


def build_ico(sizes: list[tuple[int, str]]) -> bytes:
    frames = []
    for size, fmt in sizes:
        img = draw_icon(size, with_text=size >= 128)
        data = bmp_frame(img) if fmt == "bmp" else png_frame(img)
        frames.append((size, data))

    count = len(frames)
    out = io.BytesIO()
    out.write(struct.pack("<HHH", 0, 1, count))
    data_offset = 6 + 16 * count
    for size, data in frames:
        w_byte = size if size < 256 else 0
        out.write(struct.pack("<BBBBHHII", w_byte, w_byte, 0, 0, 1, 32, len(data), data_offset))
        data_offset += len(data)
    for _, data in frames:
        out.write(data)
    return out.getvalue()


def write_png(name: str, size: int) -> None:
    img = draw_icon(size, with_text=size >= 128)
    path = TAURI_ICONS / name
    img.save(path, format="PNG")
    print(f"Wrote {path.name} ({size}px)")


def main() -> None:
    # Tauri bundle + window icons.
    write_png("icon.png", 1024)
    for base in (16, 32, 128, 256, 512):
        write_png(f"icon_{base}x{base}.png", base)
        write_png(f"icon_{base}x{base}@2x.png", base * 2)
    write_png("32x32.png", 32)
    write_png("64x64.png", 64)
    write_png("128x128.png", 128)
    write_png("128x128@2x.png", 256)
    write_png("StoreLogo.png", 50)
    for sq in (30, 44, 71, 89, 107, 142, 150, 284, 310):
        write_png(f"Square{sq}x{sq}Logo.png", sq)

    # Windows .ico (taskbar/explorer/window title).
    ico_sizes = [(16, "bmp"), (24, "bmp"), (32, "bmp"), (48, "bmp"), (64, "bmp"), (128, "bmp"), (256, "png")]
    data = build_ico(ico_sizes)
    for name in ("icon.ico", "app_icon.ico"):
        (TAURI_ICONS / name).write_bytes(data)
        print(f"Wrote {name} ({len(data)} bytes)")

    # Public logo (used by docs/web fallbacks).
    PUBLIC_ICONS.mkdir(parents=True, exist_ok=True)
    draw_icon(512, with_text=True).save(PUBLIC_ICONS / "logo.png", format="PNG")
    print(f"Wrote {PUBLIC_ICONS / 'logo.png'} (512px)")


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Generate a Windows-friendly multi-size ICO from a source PNG.

BMP-encoded frames are used for sizes <= 128 (Windows taskbar/explorer).
PNG-encoded frames are used for 256/512 (high-DPI displays, smaller size).
"""

import struct
import io
from pathlib import Path
from PIL import Image


SRC = Path(__file__).parent.parent / "frontend" / "src-tauri" / "icons" / "icon.png"
OUT_DIR = Path(__file__).parent.parent / "frontend" / "src-tauri" / "icons"


def bmp_frame(img: Image.Image) -> bytes:
    """Return a 32bpp BMP DIB suitable for an ICO entry (XOR + AND masks)."""
    w, h = img.size
    # Convert to RGBA and get top-down RGBA bytes
    rgba = img.convert("RGBA").tobytes()
    # Build bottom-up BGRA pixel array
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

    # AND mask: 1bpp, all zeros (fully opaque); alpha is already in the XOR mask.
    stride = ((w + 31) // 32) * 4
    and_mask = bytes(stride * h)

    # BITMAPINFOHEADER; biHeight is doubled because XOR+AND are stacked logically.
    header = struct.pack(
        "<IiiHHIIiiIIII",
        40,          # biSize
        w,           # biWidth
        h * 2,       # biHeight
        1,           # biPlanes
        32,          # biBitCount
        0,           # biCompression (BI_RGB)
        w * h * 4,   # biSizeImage
        0,           # biXPelsPerMeter
        0,           # biYPelsPerMeter
        0,           # biClrUsed
        0,           # biClrImportant
        0,
        0,
    )
    return header + bytes(xor) + and_mask


def png_frame(img: Image.Image) -> bytes:
    buf = io.BytesIO()
    img.convert("RGBA").save(buf, format="PNG")
    return buf.getvalue()


def build_ico(sizes: list[tuple[int, str]]) -> bytes:
    src = Image.open(SRC)
    frames = []
    for size, fmt in sizes:
        img = src.resize((size, size), Image.Resampling.LANCZOS)
        if fmt == "bmp":
            data = bmp_frame(img)
        else:
            data = png_frame(img)
        frames.append((size, data, fmt))

    count = len(frames)
    out = io.BytesIO()
    # ICONDIR
    out.write(struct.pack("<HHH", 0, 1, count))

    # Directory entries
    data_offset = 6 + 16 * count
    for size, data, fmt in frames:
        w_byte = size if size < 256 else 0
        h_byte = size if size < 256 else 0
        out.write(struct.pack(
            "<BBBBHHII",
            w_byte,
            h_byte,
            0,      # colors
            0,      # reserved
            1,      # planes
            32,     # bit count
            len(data),
            data_offset,
        ))
        data_offset += len(data)

    # Frame data
    for _, data, _ in frames:
        out.write(data)

    return out.getvalue()


def main():
    sizes = [
        (16, "bmp"),
        (24, "bmp"),
        (32, "bmp"),
        (48, "bmp"),
        (64, "bmp"),
        (128, "bmp"),
        (256, "png"),
    ]
    data = build_ico(sizes)
    for name in ("icon.ico", "app_icon.ico"):
        path = OUT_DIR / name
        path.write_bytes(data)
        print(f"Wrote {path} ({len(data)} bytes)")

    # Quick validation
    parsed = OUT_DIR / "icon.ico"
    raw = parsed.read_bytes()
    _, _, n = struct.unpack("<HHH", raw[:6])
    off = 6
    print(f"Entries: {n}")
    for _ in range(n):
        w, h, _, _, _, _, size, offset = struct.unpack("<BBBBHHII", raw[off:off + 16])
        w = w or 256
        h = h or 256
        sig = raw[offset:offset + 4]
        is_png = sig == b"\x89PNG"
        print(f"  {w}x{h} size={size} png={is_png}")
        off += 16


if __name__ == "__main__":
    main()
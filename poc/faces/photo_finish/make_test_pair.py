#!/usr/bin/env python3
"""Make a synthetic face-like test pair: 512x512 RGB colour render + 512x512 grayscale depth.

Depth convention matches the MiDaS adapter: NEAR = WHITE, FAR/background = BLACK.
The "head" is an ellipsoid with a nose, brow ridge, eye sockets, lips, ears and a neck, lit with
a simple Lambert + ambient model, skin-toned, on a neutral grey background. Only numpy + Pillow.

Usage: python make_test_pair.py --out-dir DIR [--tone 0.0..1.0]
"""
import argparse
from pathlib import Path

import numpy as np
from PIL import Image

N = 512


def bump(x, y, cx, cy, sx, sy, amp):
    return amp * np.exp(-(((x - cx) / sx) ** 2 + ((y - cy) / sy) ** 2))


def make(tone: float):
    ys, xs = np.mgrid[0:N, 0:N].astype(np.float32)
    x = (xs - N / 2) / (N / 2)  # [-1,1], +x right
    y = (ys - N / 2) / (N / 2)  # [-1,1], +y down
    # head ellipsoid
    ax, ay, az = 0.52, 0.68, 0.55
    cy = -0.08
    r2 = (x / ax) ** 2 + ((y - cy) / ay) ** 2
    head = r2 < 1
    z = np.where(head, az * np.sqrt(np.clip(1 - r2, 0, 1)), 0.0)
    # facial relief (only inside head)
    rel = (
        bump(x, y, 0, 0.12, 0.07, 0.20, 0.10)  # nose bridge + tip
        + bump(x, y, 0, 0.26, 0.09, 0.06, 0.05)  # nose tip / nostrils
        - bump(x, y, -0.19, -0.06, 0.10, 0.06, 0.07)  # eye sockets
        - bump(x, y, 0.19, -0.06, 0.10, 0.06, 0.07)
        + bump(x, y, 0, -0.17, 0.30, 0.05, 0.04)  # brow ridge
        + bump(x, y, 0, 0.40, 0.13, 0.035, 0.03)  # lips
        + bump(x, y, 0, 0.53, 0.14, 0.06, 0.03)  # chin
        + bump(x, y, -0.30, 0.15, 0.10, 0.10, 0.03)  # cheeks
        + bump(x, y, 0.30, 0.15, 0.10, 0.10, 0.03)
    )
    z = np.where(head, z + rel, 0.0)
    # ears (small ellipses at sides, set back)
    for sx_ in (-1, 1):
        e = ((x - sx_ * 0.53) / 0.07) ** 2 + ((y + 0.02) / 0.14) ** 2 < 1
        z = np.where(e & ~head, 0.18, z)
        head = head | e
    # neck
    neck = (np.abs(x) < 0.26) & (y > 0.45) & ~head
    z = np.where(neck, 0.30 * np.sqrt(np.clip(1 - (x / 0.26) ** 2, 0, 1)), z)
    fg = head | neck

    # normals from depth
    gy, gx = np.gradient(z * (N / 2))
    nrm = np.stack([-gx, -gy, np.ones_like(z)], -1)
    nrm /= np.linalg.norm(nrm, axis=-1, keepdims=True)
    L = np.array([-0.45, -0.55, 0.70], np.float32)
    L /= np.linalg.norm(L)
    lam = np.clip(nrm @ L, 0, 1)
    shade = 0.28 + 0.72 * lam

    light = np.array([0.93, 0.76, 0.63])
    dark = np.array([0.36, 0.22, 0.15])
    skin = light * (1 - tone) + dark * tone
    col = shade[..., None] * skin
    # eyes (darker irises), lips tint, eyebrows
    for ex in (-0.19, 0.19):
        eye = ((x - ex) / 0.055) ** 2 + ((y + 0.05) / 0.025) ** 2 < 1
        col = np.where(eye[..., None], np.array([0.85, 0.85, 0.82]) * shade[..., None], col)
        iris = ((x - ex) / 0.022) ** 2 + ((y + 0.05) / 0.022) ** 2 < 1
        col = np.where(iris[..., None], np.array([0.15, 0.10, 0.07]), col)
        brow = (((x - ex) / 0.09) ** 2 + ((y + 0.14) / 0.018) ** 2) < 1
        col = np.where(brow[..., None], np.array([0.12, 0.08, 0.06]), col)
    lips = ((x / 0.11) ** 2 + ((y - 0.40) / 0.03) ** 2) < 1
    col = np.where(lips[..., None], col * np.array([1.0, 0.72, 0.72]), col)
    # short dark hair cap on top of head
    hair = head & (y < -0.38 + 0.08 * x**2) & (np.abs(x) < 0.53)
    col = np.where(hair[..., None], np.array([0.08, 0.06, 0.05]) * (0.6 + 0.4 * shade[..., None]), col)

    bg = np.array([0.5, 0.5, 0.5])
    col = np.where(fg[..., None], col, bg)
    rgb = (np.clip(col, 0, 1) * 255).round().astype(np.uint8)

    d = np.where(fg, 0.25 + 0.75 * np.clip(z / z.max(), 0, 1), 0.0)  # near=white, background=black
    depth = (d * 255).round().astype(np.uint8)
    return rgb, depth


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out-dir", required=True)
    ap.add_argument("--tone", type=float, default=0.3)
    a = ap.parse_args()
    out = Path(a.out_dir)
    out.mkdir(parents=True, exist_ok=True)
    rgb, depth = make(a.tone)
    Image.fromarray(rgb, "RGB").save(out / "test_color.png")
    Image.fromarray(depth, "L").save(out / "test_depth.png")
    print("wrote", out / "test_color.png", out / "test_depth.png")


if __name__ == "__main__":
    main()

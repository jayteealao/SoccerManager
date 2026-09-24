"""Measures 2D face proportions on images with MediaPipe Face Landmarker
(Apache-2.0), to compare our renders with their photo-finished versions.

Ratios are normalised by the biocular width (outer eye corners), so they do
not depend on image scale.

Usage: python landmarks2d.py render_dir finished_dir [--model path.task]
"""
import argparse, glob, os, json
import numpy as np
import mediapipe as mp
from mediapipe.tasks import python as mpt
from mediapipe.tasks.python import vision

# MediaPipe Face Mesh indices (subject's right = image left).
IDX = dict(ex_r=33, en_r=133, en_l=362, ex_l=263, up_r=159, lo_r=145, up_l=386, lo_l=374,
           brow_r=52, brow_l=282, al_r=64, al_l=294, ch_r=61, ch_l=291, ls=0, sto_u=13, sto_l=14,
           li=17, sn=2, n=168, me=152, zy_r=234, zy_l=454, go_r=172, go_l=397, cheek_r=205, cheek_l=425)

RATIOS = {
    "eye aperture h/w": lambda p: (abs(p["lo_r"][1] - p["up_r"][1]) / d(p, "ex_r", "en_r") + abs(p["lo_l"][1] - p["up_l"][1]) / d(p, "ex_l", "en_l")) / 2,
    "intercanthal / biocular": lambda p: d(p, "en_r", "en_l") / B(p),
    "brow above eye / biocular": lambda p: ((mid(p, "up_r", "lo_r")[1] - p["brow_r"][1]) + (mid(p, "up_l", "lo_l")[1] - p["brow_l"][1])) / 2 / B(p),
    "nose width / biocular": lambda p: d(p, "al_r", "al_l") / B(p),
    "mouth width / biocular": lambda p: d(p, "ch_r", "ch_l") / B(p),
    "upper vermilion / biocular": lambda p: abs(p["sto_u"][1] - p["ls"][1]) / B(p),
    "lower vermilion / biocular": lambda p: abs(p["li"][1] - p["sto_l"][1]) / B(p),
    "philtrum sn-ls / biocular": lambda p: abs(p["ls"][1] - p["sn"][1]) / B(p),
    "nose height n-sn / biocular": lambda p: abs(p["sn"][1] - p["n"][1]) / B(p),
    "lower face sn-me / biocular": lambda p: abs(p["me"][1] - p["sn"][1]) / B(p),
    "face width zy / biocular": lambda p: d(p, "zy_r", "zy_l") / B(p),
    "cheek width (mouth level) / biocular": lambda p: d(p, "cheek_r", "cheek_l") / B(p),
    "jaw width go / biocular": lambda p: d(p, "go_r", "go_l") / B(p),
}


def d(p, a, b):
    return float(np.linalg.norm(np.array(p[a]) - np.array(p[b])))


def B(p):
    return d(p, "ex_r", "ex_l")


def mid(p, a, b):
    return (np.array(p[a]) + np.array(p[b])) / 2


def landmarks(det, path):
    img = mp.Image.create_from_file(path)
    res = det.detect(img)
    if not res.face_landmarks:
        return None
    lm = res.face_landmarks[0]
    return {k: (lm[i].x * img.width, lm[i].y * img.height) for k, i in IDX.items()}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("render_dir")
    ap.add_argument("finished_dir")
    ap.add_argument("--model", default=os.path.join(os.path.dirname(__file__), "..", "downloads", "mediapipe", "face_landmarker.task"))
    ap.add_argument("--json", default=None)
    a = ap.parse_args()
    det = vision.FaceLandmarker.create_from_options(
        vision.FaceLandmarkerOptions(base_options=mpt.BaseOptions(model_asset_path=a.model), num_faces=1))
    rows = {}
    for f in sorted(glob.glob(os.path.join(a.finished_dir, "*.png"))):
        stem = os.path.basename(f)
        r = os.path.join(a.render_dir, stem)
        pr, pf = landmarks(det, r), landmarks(det, f)
        if pr is None or pf is None:
            print(f"{stem}: no face found (render={pr is not None}, finish={pf is not None})")
            continue
        rows[stem] = {k: (fn(pr), fn(pf)) for k, fn in RATIOS.items()}
    print(f"{len(rows)} pairs. Mean render -> finish, and finish/render change:")
    print(f"{'ratio':40} {'render':>8} {'finish':>8} {'change':>8} {'agree':>6}")
    summary = {}
    for k in RATIOS:
        r = np.array([v[k][0] for v in rows.values()])
        f = np.array([v[k][1] for v in rows.values()])
        ch = f / r - 1
        agree = max((ch > 0).mean(), (ch < 0).mean())
        summary[k] = dict(render=r.mean(), finish=f.mean(), change=ch.mean(), agree=agree)
        print(f"{k:40} {r.mean():8.3f} {f.mean():8.3f} {ch.mean()*100:7.1f}% {agree*100:5.0f}%")
    if a.json:
        json.dump(dict(summary=summary, pairs=rows), open(a.json, "w"), indent=1, default=float)


if __name__ == "__main__":
    main()

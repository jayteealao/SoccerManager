"""PoC 8 / roadmap 3: evaluation harness v1.

Every metric runs on images only, never classifies race, and is reported per
Monk Skin Tone bucket where it can be. Models (downloads/eval/, not in git):

  MediaPipe Face Landmarker   Apache-2.0   landmark drift
  DINOv2-small (Meta)         Apache-2.0   near-duplicate check across genomes
  SigLIP 2 base (Google)      Apache-2.0   zero-shot apparent age
  YuNet (OpenCV zoo)          MIT          face detection / alignment for SFace
  SFace (OpenCV zoo)          Apache-2.0 label; training-data provenance
                              unknown -> INTERNAL QA ONLY, never shipped

Metrics
  1. Landmark drift, render -> finish (13 ratios, % change) and across ages
     (render at 19 vs 60, same genome).
  2. Skin colorimetry: ITA and nearest MST swatch on the skin mask, render
     vs finish (Delta ITA), and the MST coverage of the PoC 8 MST strip.
  3. Near-duplicates: DINOv2 cosine similarity between different genomes at
     the same age; flags pairs above the lowest same-genome similarity.
  4. Apparent age: SigLIP 2 zero-shot over age prompts; MAE and correlation
     with the true age, render and finish.
  5. Identity (internal): SFace cosine, same genome across ages and render
     vs finish, against different-genome pairs; d'.

Usage: python eval/evaluate.py   (after gnm8_sheet ages / mst and finish.py)
Writes out/poc8_eval.txt and out/poc8_eval.json.
"""
import glob
import json
import os
import re
import sys
from pathlib import Path

import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parent.parent
D = ROOT / "out/tmp/gnm8"
EV = ROOT / "downloads/eval"
sys.path.insert(0, str(ROOT / "photo_finish"))
from finish import srgb_to_lab  # noqa: E402

MST = ["f6ede4", "f3e7db", "f7ead0", "eadaba", "d7bd96", "a07e56", "825c43", "604134", "3a312a", "292420"]
MST_LAB = srgb_to_lab(np.array([[int(h[i:i + 2], 16) for i in (0, 2, 4)] for h in MST], np.uint8))


def load(p):
    return np.asarray(Image.open(p).convert("RGB"))


def skin_lab(img, mask):
    sel = mask[..., 0] > 200
    # Cheeks and forehead only: drop the darkest and brightest 15% (shadow, speculars).
    lab = srgb_to_lab(img[sel])
    lo, hi = np.percentile(lab[:, 0], [15, 85])
    return lab[(lab[:, 0] >= lo) & (lab[:, 0] <= hi)].mean(0)


def ita(lab):
    return float(np.degrees(np.arctan((lab[0] - 50) / lab[2])))


def mst_bucket(lab):
    return int(np.argmin(np.linalg.norm(MST_LAB - lab, axis=1)) + 1)


def parse(stem):
    m = re.match(r"(\w+?)_(\d+)_age(\d+)", stem)
    return f"{m.group(1)}_{m.group(2)}", int(m.group(3))


def main():
    out = {}
    lines = ["PoC 8 evaluation harness v1 (images only; no race classification)", ""]
    renders = sorted(glob.glob(str(D / "renders/*.png")))
    finished = sorted(glob.glob(str(D / "finished/*.png")))
    print(f"{len(renders)} renders, {len(finished)} finished")

    # 1. Landmark drift.
    from landmarks2d import RATIOS, landmarks
    from mediapipe.tasks import python as mpt
    from mediapipe.tasks.python import vision
    det = vision.FaceLandmarker.create_from_options(vision.FaceLandmarkerOptions(
        base_options=mpt.BaseOptions(model_asset_path=str(ROOT / "downloads/mediapipe/face_landmarker.task")), num_faces=1))
    lmk = {}
    for p in renders + finished:
        lmk[p] = landmarks(det, p)
    drift = {k: [] for k in RATIOS}
    for f in finished:
        r = str(D / "renders" / Path(f).name)
        if lmk.get(r) and lmk.get(f):
            for k, fn in RATIOS.items():
                drift[k].append(fn(lmk[f]) / fn(lmk[r]) - 1)
    lines.append("1. Landmark drift render -> finish (mean |change| %, n pairs)")
    out["landmark_drift"] = {}
    for k, v in drift.items():
        if v:
            out["landmark_drift"][k] = float(np.mean(np.abs(v)) * 100)
            lines.append(f"   {k:40} {np.mean(np.abs(v)) * 100:5.1f}%  (n {len(v)})")
    age_drift = {k: [] for k in RATIOS}
    for r in renders:
        g, a = parse(Path(r).stem)
        if a == 19:
            r60 = str(D / f"renders/{g}_age60.png")
            if lmk.get(r) and lmk.get(r60):
                for k, fn in RATIOS.items():
                    age_drift[k].append(fn(lmk[r60]) / fn(lmk[r]) - 1)
    lines.append("   Across ages, render 19 -> 60 (identity should hold; ageing moves lids, lips, jaw):")
    for k, v in age_drift.items():
        if v:
            lines.append(f"   {k:40} {np.mean(v) * 100:+5.1f}%")
    lines.append("")

    # 2. Colorimetry.
    lines.append("2. Skin colorimetry on the renderer's skin mask (ITA deg, MST bucket)")
    col = []
    for f in finished:
        stem = Path(f).stem
        r = D / "renders" / Path(f).name
        m = load(D / f"finish_inputs/{stem}_mask.png")
        lr, lf = skin_lab(load(r), m), skin_lab(load(f), m)
        col.append(dict(id=stem, ita_render=ita(lr), ita_finish=ita(lf), mst_render=mst_bucket(lr), mst_finish=mst_bucket(lf),
                        dE=float(np.linalg.norm(lr - lf))))
        lines.append(f"   {stem:18} render ITA {ita(lr):6.1f} MST {mst_bucket(lr):2d} | finish ITA {ita(lf):6.1f} MST {mst_bucket(lf):2d} | dE {np.linalg.norm(lr - lf):4.1f}")
    if col:
        d_ita = np.array([c["ita_finish"] - c["ita_render"] for c in col])
        lines.append(f"   mean |Delta ITA| render -> finish {np.abs(d_ita).mean():.1f} deg; MST bucket kept in {np.mean([c['mst_render'] == c['mst_finish'] for c in col]) * 100:.0f}% of images")
    out["colorimetry"] = col
    mst_files = sorted(glob.glob(str(D / "mst/mst*.png")))
    if mst_files:
        # The skin mask is not saved for the strip; use the cheek box of the portrait framing.
        buckets = []
        for p in mst_files:
            im = load(p)
            patch = np.concatenate([im[300:340, 170:210].reshape(-1, 3), im[300:340, 300:340].reshape(-1, 3)])
            lab = srgb_to_lab(patch).mean(0)
            buckets.append((Path(p).stem, round(ita(lab), 1), mst_bucket(lab)))
        lines.append("   MST strip (genome skin set to each swatch, rendered cheek colour): " +
                     ", ".join(f"{s} -> ITA {i} MST {b}" for s, i, b in buckets))
        covered = sorted({b for *_, b in buckets})
        lines.append(f"   rendered buckets covered: {covered}")
        out["mst_strip"] = buckets
    lines.append("")

    # 3. Near-duplicates with DINOv2.
    import torch
    from transformers import AutoModel

    def pix(p, size, mean, std):
        # Manual preprocessing (no torchvision): resize, normalise, NCHW.
        im = np.asarray(Image.open(p).convert("RGB").resize((size, size), Image.BICUBIC), np.float32) / 255.0
        return torch.from_numpy(((im - mean) / std).transpose(2, 0, 1)[None].astype(np.float32))

    dm = AutoModel.from_pretrained(EV / "dinov2-small").eval()
    feats = {}
    with torch.no_grad():
        for p in renders:
            x = pix(p, 224, np.array([0.485, 0.456, 0.406]), np.array([0.229, 0.224, 0.225]))
            f = dm(pixel_values=x).last_hidden_state[:, 0][0].numpy()
            feats[p] = f / np.linalg.norm(f)
    same, cross = [], []
    for i, a in enumerate(renders):
        for b in renders[i + 1:]:
            (ga, aa), (gb, ab) = parse(Path(a).stem), parse(Path(b).stem)
            s = float(feats[a] @ feats[b])
            if ga == gb and abs(aa - ab) <= 6:
                same.append(s)
            elif ga != gb and aa == ab:
                cross.append((s, Path(a).stem, Path(b).stem))
    thr = min(same)
    flagged = [c for c in cross if c[0] >= thr]
    lines.append("3. Near-duplicates (DINOv2-small CLS cosine)")
    lines.append(f"   same genome, ages <= 6 years apart: mean {np.mean(same):.3f}, min {thr:.3f}")
    lines.append(f"   different genomes, same age: mean {np.mean([c[0] for c in cross]):.3f}, max {max(c[0] for c in cross):.3f}")
    lines.append(f"   flagged pairs (different genomes at least as similar as the least-similar same-genome pair): {len(flagged)}")
    for s, a, b in sorted(flagged, reverse=True)[:5]:
        lines.append(f"     {a} ~ {b}: {s:.3f}")
    out["near_duplicates"] = dict(same_mean=float(np.mean(same)), same_min=thr, cross_max=max(c[0] for c in cross), flagged=len(flagged))
    lines.append("")

    # 4. Apparent age with SigLIP 2 zero-shot.
    from transformers import AutoTokenizer
    tok = AutoTokenizer.from_pretrained(EV / "siglip2-base-patch16-224")
    sm = AutoModel.from_pretrained(EV / "siglip2-base-patch16-224").eval()
    bins = [16, 20, 25, 30, 35, 40, 45, 50, 55, 60, 65, 70, 75]
    texts = [f"a photo of a {b} year old man" for b in bins]
    def app_age(paths):
        res = {}
        with torch.no_grad():
            for p in paths:
                t = tok(texts, padding="max_length", max_length=64, return_tensors="pt")
                x = pix(p, 224, np.array([0.5, 0.5, 0.5]), np.array([0.5, 0.5, 0.5]))
                lg = sm(input_ids=t["input_ids"], pixel_values=x).logits_per_image[0].numpy()
                w = np.exp(lg - lg.max())
                w /= w.sum()
                res[p] = float((w * np.array(bins)).sum())
        return res
    ar = app_age(renders)
    af = app_age(finished)
    lines.append("4. Apparent age (SigLIP 2 zero-shot, expectation over age prompts)")
    for name, res in [("renders", ar), ("finished", af)]:
        if not res:
            continue
        true = np.array([parse(Path(p).stem)[1] for p in res])
        pred = np.array(list(res.values()))
        lines.append(f"   {name:9}: MAE {np.abs(pred - true).mean():5.1f} y, corr {np.corrcoef(true, pred)[0, 1]:.2f}, "
                     f"mean predicted at 19/30/60: " + "/".join(f"{pred[true == a].mean():.0f}" for a in (19, 30, 60) if (true == a).any()))
    out["apparent_age"] = dict(renders=ar, finished=af)
    lines.append("")

    # 5. Identity, internal QA only.
    import cv2
    det_y = cv2.FaceDetectorYN.create(str(EV / "face_detection_yunet_2023mar.onnx"), "", (512, 512), 0.6)
    rec = cv2.FaceRecognizerSF.create(str(EV / "face_recognition_sface_2021dec.onnx"), "")
    def emb(p):
        im = cv2.imread(p)
        _, faces = det_y.detect(im)
        if faces is None:
            return None
        al = rec.alignCrop(im, faces[0])
        return rec.feature(al)[0]
    E = {p: emb(p) for p in renders + finished}
    ages_same, ages_cross, rf = [], [], []
    for i, a in enumerate(renders):
        for b in renders[i + 1:]:
            if E[a] is None or E[b] is None:
                continue
            (ga, aa), (gb, ab) = parse(Path(a).stem), parse(Path(b).stem)
            s = float(E[a] @ E[b] / np.linalg.norm(E[a]) / np.linalg.norm(E[b]))
            (ages_same if ga == gb else ages_cross).append((s, aa, ab))
    for f in finished:
        r = str(D / "renders" / Path(f).name)
        if E.get(f) is not None and E.get(r) is not None:
            rf.append(float(E[f] @ E[r] / np.linalg.norm(E[f]) / np.linalg.norm(E[r])))
    s_same = np.array([s for s, *_ in ages_same])
    s_cross = np.array([s for s, *_ in ages_cross])
    dprime = (s_same.mean() - s_cross.mean()) / np.sqrt(0.5 * (s_same.var() + s_cross.var()))
    far = [s for s, a, b in ages_same if {a, b} == {19, 60}]
    lines.append("5. Identity (SFace; INTERNAL QA ONLY, provenance of its training data unknown)")
    lines.append(f"   same genome across ages: mean cos {s_same.mean():.3f}; 19 vs 60: {np.mean(far):.3f}")
    lines.append(f"   different genomes: mean cos {s_cross.mean():.3f}; d' = {dprime:.2f}")
    if rf:
        lines.append(f"   render vs its finish: mean cos {np.mean(rf):.3f} (SFace's own match threshold is 0.363)")
    out["identity"] = dict(same=float(s_same.mean()), cross=float(s_cross.mean()), dprime=float(dprime),
                           far_19_60=float(np.mean(far)), render_finish=float(np.mean(rf)) if rf else None)

    txt = "\n".join(lines)
    print(txt)
    (ROOT / "out/poc8_eval.txt").write_text(txt + "\n")
    (ROOT / "out/poc8_eval.json").write_text(json.dumps(out, indent=1, default=float))


if __name__ == "__main__":
    main()

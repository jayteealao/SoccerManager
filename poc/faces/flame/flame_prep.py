"""PoC 7: prepares FLAME 2023 Open (CC-BY-4.0, MPI-IS) as the head asset on its
own, with no MakeHuman geometry. Output (downloads/flame_asset/, never
committed; derived from the FLAME files the person provided):

  verts.f32     5023 x 3   FLAME mean head in the portrait frame (decimetres,
                           Y up, face +Z, eyes at the same point as the other
                           PoCs so camera and hair code carry over)
  faces.u32     9976 x 3   triangles
  shape.f32     K x 5023 x 3   shape directions per unit beta (frame units)
  masks.u16     5023       bit per FLAME mask region (names in meta.json)
  ancestry.f32  3 x K      per-ancestry mean beta (african, asian, caucasian),
                           fitted to published adult male facial norms
  landmarks.json            vertex pairs for the 13 measures, as in src/measure.rs
  meta.json

FLAME: Li, Bolkart, Black, Li, Romero, "Learning a model of facial shape and
expression from 4D scans", ACM ToG (SIGGRAPH Asia) 2017.
FLAME_masks.pkl: used at the person's direction; its licence is not stated
(see LICENSES.md).

Usage: python flame/flame_prep.py [--components 100]
"""
import argparse, json, pickle, warnings
from pathlib import Path

import numpy as np

warnings.filterwarnings("ignore")
ROOT = Path(__file__).resolve().parent.parent
FLAME_PKL = ROOT / "assets/flame/open/flame2023_Open.pkl"
MASKS_PKL = ROOT / "assets/flame/masks/FLAME_masks.pkl"
OUT = ROOT / "downloads/flame_asset"
EYE_ANCHOR = np.array([0.0, 7.284, 1.245])  # same anchor as src/head.rs

# Published adult male norms (mm): (name, NAW mean, African mean, NAW SD).
# Same sources as src/measure.rs (Farkas NAW via Wamalwa et al. 2019).
NORMS = [
    ("face width zy-zy", 137.0, 137.0, 5.0),
    ("face height n-gn", 121.3, 128.0, 6.0),
    ("forehead height tr-n", 67.1, 72.0, 7.5),
    ("nose height n-sn", 54.8, 51.5, 3.3),
    ("lower face sn-me", 72.6, 77.5, 4.5),
    ("upper lip sn-sto", 22.3, 25.8, 2.1),
    ("lower lip+chin sto-me", 50.3, 51.7, 4.0),
    ("intercanthal en-en", 33.3, 34.0, 2.7),
    ("biocular ex-ex", 91.2, 97.5, 3.0),
    ("eye width ex-en", 31.3, 33.5, 1.3),
    ("nose width al-al", 34.9, 43.6, 2.1),
    ("mouth width ch-ch", 54.5, 55.2, 3.0),
    ("jaw width go-go", 97.0, 100.0, 6.0),
]


def boundary_loops(faces, subset):
    """Open-edge vertices of a sub-mesh (used for the eye holes)."""
    from collections import Counter
    sub = faces[np.isin(faces, list(subset)).all(1)]
    edges = Counter()
    for f in sub:
        for a, b in ((f[0], f[1]), (f[1], f[2]), (f[2], f[0])):
            edges[tuple(sorted((a, b)))] += 1
    return sorted({v for e, c in edges.items() if c == 1 for v in e})


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--components", type=int, default=100)
    a = ap.parse_args()

    fl = pickle.load(open(FLAME_PKL, "rb"), encoding="latin1")
    masks = pickle.load(open(MASKS_PKL, "rb"), encoding="latin1")
    v = np.asarray(fl["v_template"])
    f = np.asarray(fl["f"]).astype(np.int64)
    sd = np.asarray(fl["shapedirs"])[:, :, : a.components]

    # Frame: decimetres, Y up; scale so the eye centres are 63 mm apart as in
    # FLAME (x10), then translate the eye midpoint to the shared anchor.
    eye_l = np.arange(3931, 4477)
    eye_r = np.arange(4477, 5023)
    mid = (v[eye_l].mean(0) + v[eye_r].mean(0)) / 2
    s = 10.0
    V = (v - mid) * s + EYE_ANCHOR
    SD = sd * s

    names = list(masks.keys())
    bits = np.zeros(len(v), np.uint16)
    for i, n in enumerate(names):
        bits[np.asarray(masks[n]).astype(int)] |= 1 << i

    # Landmarks on the FLAME mean head (frame coordinates).
    def m(n):
        return np.asarray(masks[n]).astype(int)

    head = np.arange(3931)
    x, y, z = V[:, 0], V[:, 1], V[:, 2]
    ec = {"l": V[eye_l].mean(0), "r": V[eye_r].mean(0)}
    er = np.linalg.norm(V[eye_l] - ec["l"], axis=1).max()
    # FLAME's lids are closed surfaces wrapped round the eyeball; the lid
    # margin is the ring of head vertices lying on the eyeball's surface, in
    # front of its centre.
    rims = {}
    for sd_, ids in (("l", eye_l), ("r", eye_r)):
        r_eye = np.linalg.norm(V[ids] - ec[sd_], axis=1).max()
        dd = np.linalg.norm(V[head] - ec[sd_], axis=1) - r_eye
        rims[sd_] = head[(np.abs(dd) < 0.012) & (V[head, 2] > ec[sd_][2])]
    def pick(cands, key):
        cands = np.asarray(cands)
        return int(cands[np.argmax(key(V[cands]))])

    lm = {}
    for sd_, sg in (("l", 1), ("r", -1)):
        rim = rims[sd_]
        lm[f"en_{sd_}"] = pick(rim, lambda p: -np.abs(p[:, 0]))
        lm[f"ex_{sd_}"] = pick(rim, lambda p: np.abs(p[:, 0]))
        nose = m("nose")
        lm[f"al_{sd_}"] = pick(nose[(sg * x[nose] > 0)], lambda p: np.abs(p[:, 0]) - 2 * np.abs(p[:, 1] - 6.88))
        lips = m("lips")
        lm[f"ch_{sd_}"] = pick(lips[(sg * x[lips] > 0)], lambda p: np.abs(p[:, 0]))
        face = m("face")
        band = face[(sg * x[face] > 0) & (np.abs(y[face] - ec["l"][1] + 0.12) < 0.12)]
        lm[f"zy_{sd_}"] = pick(band, lambda p: np.abs(p[:, 0]))
    mid_line = head[np.abs(x[head]) < 0.03]
    lm["n"] = pick(mid_line[(y[mid_line] > 7.2) & (y[mid_line] < 7.5) & (z[mid_line] > 1.2)], lambda p: -p[:, 2])
    lips = m("lips")
    lm["sto"] = pick(lips[np.abs(x[lips]) < 0.03], lambda p: -np.abs(p[:, 1] - 6.6) - 0.5 * (1.6 - p[:, 2]))
    nose = m("nose")
    lm["sn"] = pick(nose[np.abs(x[nose]) < 0.03], lambda p: -p[:, 1] - 0.3 * (1.6 - p[:, 2]))
    face = m("face")
    chin = face[np.abs(x[face]) < 0.03]
    lm["gn"] = pick(chin, lambda p: -p[:, 1])
    gy = y[lm["gn"]]
    # Jaw angle: widest point of the face mask a little above the chin line.
    for sd_, sg in (("l", 1), ("r", -1)):
        band = face[(sg * x[face] > 0) & (y[face] > gy + 0.2) & (y[face] < gy + 0.45)]
        lm[f"go_{sd_}"] = pick(band, lambda p: np.abs(p[:, 0]))
    fh = m("forehead")
    lm["tr"] = pick(fh[np.abs(x[fh]) < 0.05], lambda p: p[:, 1])

    defs = [
        (0, lm["zy_l"], lm["zy_r"]), (1, lm["n"], lm["gn"]), (1, lm["tr"], lm["n"]),
        (1, lm["n"], lm["sn"]), (1, lm["sn"], lm["gn"]), (1, lm["sn"], lm["sto"]),
        (1, lm["sto"], lm["gn"]), (0, lm["en_l"], lm["en_r"]), (0, lm["ex_l"], lm["ex_r"]),
        (0, lm["ex_l"], lm["en_l"]), (0, lm["al_l"], lm["al_r"]), (0, lm["ch_l"], lm["ch_r"]),
        (0, lm["go_l"], lm["go_r"]),
    ]

    def measures(P):
        return np.array([abs(P[a_, ax] - P[b_, ax]) * 100 for ax, a_, b_ in defs])

    # Linear model of the measures in beta around the mean (signs from the mean).
    m0 = measures(V)
    sign = np.array([np.sign(V[a_, ax] - V[b_, ax]) for ax, a_, b_ in defs])
    J = np.stack([sign[k] * (SD[defs[k][1], defs[k][0], :] - SD[defs[k][2], defs[k][0], :]) * 100
                  for k in range(len(defs))])  # (13, K) mm per unit beta

    print("FLAME mean head measures (mm):")
    for (name, naw, afr, sdv), val in zip(NORMS, m0):
        print(f"   {name:24} {val:6.1f}   NAW {naw:6.1f}  AFR {afr:6.1f}")

    # Per-ancestry mean beta: ridge least squares to the norms, same choices of
    # which norms apply as the other fits (forehead is set by the hairline;
    # unsourced values get a weak NAW prior).
    anc = np.zeros((3, sd.shape[2]))
    for r, race in enumerate(["african", "asian", "caucasian"]):
        tgt, wts = [], []
        for k, (name, naw, afr, sdv) in enumerate(NORMS):
            if k == 2:
                tgt.append(0), wts.append(0.0)
            elif race == "african" and k not in (1, 12):
                tgt.append(afr), wts.append(1 / sdv)
            elif race == "asian" and k not in (1, 3, 4, 5, 6, 11):
                tgt.append(naw), wts.append(1 / (3 * sdv))
            elif race == "african":
                tgt.append(naw), wts.append(1 / (3 * sdv))
            else:
                tgt.append(naw), wts.append(1 / sdv)
        W = np.diag(wts)
        A = W @ J
        b = W @ (np.array(tgt) - m0)
        lam = 1.0  # betas are unit-variance; keep the mean shift modest
        beta = np.linalg.solve(A.T @ A + lam * np.eye(A.shape[1]), A.T @ b)
        anc[r] = beta
        fit = m0 + J @ beta
        print(f"{race}: |beta| {np.linalg.norm(beta):.2f}; fitted " +
              ", ".join(f"{NORMS[k][0].split()[-1]} {fit[k]:.0f}" for k in range(len(NORMS)) if wts[k] > 0))

    OUT.mkdir(parents=True, exist_ok=True)
    V.astype(np.float32).tofile(OUT / "verts.f32")
    f.astype(np.uint32).tofile(OUT / "faces.u32")
    np.ascontiguousarray(SD.transpose(2, 0, 1)).astype(np.float32).tofile(OUT / "shape.f32")
    bits.tofile(OUT / "masks.u16")
    anc.astype(np.float32).tofile(OUT / "ancestry.f32")
    (OUT / "landmarks.json").write_text(json.dumps([
        {"name": NORMS[k][0], "axis": d[0], "a": int(d[1]), "b": int(d[2])} for k, d in enumerate(defs)], indent=1))
    meta = {"components": int(sd.shape[2]), "vertices": int(len(V)), "faces": int(len(f)),
            "masks": names, "eye_left": [3931, 4476], "eye_right": [4477, 5023],
            "source": "FLAME 2023 Open (CC-BY-4.0) flame2023_Open.pkl; FLAME_masks.pkl (licence unstated)"}
    (OUT / "meta.json").write_text(json.dumps(meta, indent=1))
    print("wrote", OUT)


if __name__ == "__main__":
    main()

"""PoC 8: checks GNM Head's identity sampler against published facial norms.

Samples male identities per GNM ethnicity class with the numpy decoder
(validated against Keras in gnm_prep.py --check), measures the same linear
distances as src/measure.rs on fixed vertices, and compares class means and
within-class spread with the Farkas et al. 2005 regional aggregates and the
Fang 2011 coefficients of variation listed in RESEARCH.md. Also writes the
landmark vertex picks the Rust side uses (downloads/gnm_asset/landmarks.json).

Usage: python gnm/gnm_norms.py [--n 300]
"""
import argparse, json
from pathlib import Path

import numpy as np

from gnm_prep import OUT, ETHNICITY, decoder_layers, decode

ROOT = Path(__file__).resolve().parent.parent

# Farkas 2005 regional aggregates (adult males, mm), RESEARCH.md table.
FARKAS = {
    "en-en": {"white": 31.6, "asian": 37.2, "black": 36.2, "middle_eastern": 31.8},
    "n-gn": {"white": 119.9, "asian": 122.8, "black": 119.9, "middle_eastern": 124.5},
    "sn-gn": {"white": 67.7, "asian": 71.4, "black": 72.8, "middle_eastern": 68.1},
    "zy-zy": {"white": 136.9, "asian": 145.7, "black": 135.9, "middle_eastern": 141.5},
    "n-sn": {"white": 54.5, "asian": 53.6, "black": 50.6, "middle_eastern": 57.8},
    "al-al": {"white": 35.4, "asian": 39.6, "black": 44.1, "middle_eastern": 35.0},
    "ch-ch": {"white": 52.2, "asian": 49.0, "black": 55.1, "middle_eastern": 51.2},
    "go-go": {"white": 104.8, "asian": 111.1, "black": 102.0, "middle_eastern": 104.9},
}
# Fang 2011 within-group coefficients of variation.
GAIN, RESID = 1.0, 0.6  # src/gnm.rs CLASS_GAIN, RESID_SCALE
CV = {"en-en": .088, "al-al": .072, "ch-ch": .079, "n-sn": .070, "sn-gn": .080, "zy-zy": .042, "go-go": .056}


def picks(V, groups, names):
    def g(n):
        return np.where(groups & (np.uint64(1) << np.uint64(names.index(n))))[0]
    lm = json.loads((OUT / "lm68.json").read_text())
    def l68(i):
        return max(lm[i], key=lambda t: t[1])[0]  # dominant vertex of the triple
    p = {"en_l": l68(42), "en_r": l68(39), "ex_l": l68(45), "ex_r": l68(36),
         "ch_l": l68(54), "ch_r": l68(48), "gn": l68(8), "sn": l68(33)}
    x, y, z = V[:, 0], V[:, 1], V[:, 2]
    # Nasion: deepest midline point between the eyes.
    skin = g("skin")
    ey = V[p["en_l"], 1]
    mid = skin[(np.abs(x[skin]) < 0.02) & (np.abs(y[skin] - ey - 0.12) < 0.15) & (z[skin] > 0.8)]
    p["n"] = int(mid[np.argmin(z[mid])])
    nose = g("nose_region")
    for s, sg in (("l", 1), ("r", -1)):
        side = nose[sg * x[nose] > 0]
        p[f"al_{s}"] = int(side[np.argmax(np.abs(x[side]) - 2 * np.abs(y[side] - y[p["sn"]] - 0.08))])
        face = g("hockey_mask")
        side = face[(sg * x[face] > 0) & (np.abs(y[face] - (ey - 0.25)) < 0.1)]
        p[f"zy_{s}"] = int(side[np.argmax(np.abs(x[side]))])
        gy = y[p["gn"]]
        side = face[(sg * x[face] > 0) & (y[face] > gy + 0.2) & (y[face] < gy + 0.45)]
        p[f"go_{s}"] = int(side[np.argmax(np.abs(x[side]))])
    return p


DEFS = [("en-en", 0, "en_l", "en_r"), ("n-gn", 1, "n", "gn"), ("sn-gn", 1, "sn", "gn"),
        ("zy-zy", 0, "zy_l", "zy_r"), ("n-sn", 1, "n", "sn"), ("al-al", 0, "al_l", "al_r"),
        ("ch-ch", 0, "ch_l", "ch_r"), ("go-go", 0, "go_l", "go_r")]


def measure(P, p):
    return np.array([abs(P[..., p[a], ax] - P[..., p[b], ax]) * 100 for _, ax, a, b in DEFS])


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--n", type=int, default=300)
    a = ap.parse_args()
    meta = json.loads((OUT / "meta.json").read_text())
    N = meta["vertices"]
    V = np.fromfile(OUT / "verts.f32", np.float32).reshape(N, 3)
    B = np.fromfile(OUT / "identity.f32", np.float32).reshape(-1, N, 3)
    groups = np.fromfile(OUT / "groups.u64", np.uint64)
    p = picks(V, groups, meta["groups"])
    (OUT / "landmarks.json").write_text(json.dumps(p, indent=1))
    ids = sorted(set(p.values()))
    Bs = B[:, ids]
    Vs = V[ids]
    remap = {k: ids.index(v) for k, v in p.items()}

    layers = decoder_layers()
    means = np.fromfile(OUT / "class_means.f32", np.float32).reshape(4, -1)
    resid = np.fromfile(OUT / "resid.f32", np.float32)
    gm = means.mean(0)
    mean0 = measure(Vs[None], remap)[:, 0]
    all_lines = []
    for label, gain, rs in [("raw GNM sampler", 0.0, 0.0), (f"PoC 8 sampler (class gain {GAIN}, residual {RESID})", GAIN, RESID)]:
        rng = np.random.default_rng(8)
        res = {}
        samples = []
        for e, eth in enumerate(ETHNICITY):
            cond = np.zeros((a.n, 6), np.float32)
            cond[:, 1] = 1  # male
            cond[:, 2 + e] = 1
            ident = decode(layers, rng.normal(size=(a.n, 64)).astype(np.float32), cond)
            ident = ident + gain * (means[e] - gm) + rs * resid * rng.normal(size=(a.n, 253))
            samples.append(ident)
            P = Vs[None] + np.einsum("si,ivc->svc", ident, Bs)
            res[eth] = measure(P, remap)  # (measures, samples)
        allS = np.concatenate(samples)[:, :170]
        y = np.repeat(np.arange(4), a.n)
        cm = np.stack([s_[:, :170].mean(0) for s_ in samples])
        dd = (((allS[:, None, :] - cm[None]) / allS.std(0)) ** 2).sum(2)
        acc = float((dd.argmin(1) == y).mean())
        all_lines += report(label, a.n, mean0, res, acc)
    txt = "\n".join(all_lines)
    print(txt)
    (ROOT / "out/poc8_gnm_norms.txt").write_text(txt + "\n")


def report(label, n, mean0, res, acc):

    lines = [f"== {label}: GNM Head identity samples (male) vs Farkas 2005 regional aggregates (mm)",
             f"{n} samples per class; template = GNM mean head (lids opened 1.8 mm)",
             f"nearest-class-mean accuracy over head components: {acc:.2f}", ""]
    hdr = f"{'measure':8} {'template':>8} | " + " | ".join(f"{e[:6]:>6} GNM  Farkas" for e in ETHNICITY)
    lines.append(hdr)
    for k, (name, *_ ) in enumerate(DEFS):
        row = f"{name:8} {mean0[k]:8.1f} | " + " | ".join(
            f"{res[e][k].mean():6.1f} {FARKAS[name][e]:6.1f}" for e in ETHNICITY)
        lines.append(row)
    lines += ["", "Between-class contrasts (GNM delta vs Farkas delta, mm):"]
    for name, e1, e2 in [("en-en", "asian", "white"), ("al-al", "black", "white"), ("al-al", "asian", "white"),
                         ("zy-zy", "asian", "white"), ("n-sn", "black", "white"), ("ch-ch", "black", "asian"),
                         ("sn-gn", "black", "white")]:
        k = [d[0] for d in DEFS].index(name)
        g_ = res[e1][k].mean() - res[e2][k].mean()
        f_ = FARKAS[name][e1] - FARKAS[name][e2]
        agree = "same sign" if np.sign(g_) == np.sign(f_) else "OPPOSITE"
        lines.append(f"  {name} {e1}-{e2}: GNM {g_:+5.1f}  Farkas {f_:+5.1f}  {agree}")
    lines += ["", "Within-class coefficient of variation (GNM white class vs Fang 2011):"]
    for name, cv in CV.items():
        k = [d[0] for d in DEFS].index(name)
        r = res["white"][k]
        lines.append(f"  {name}: GNM {r.std() / r.mean():.3f}  Fang {cv:.3f}")
    return lines + [""]


if __name__ == "__main__":
    main()

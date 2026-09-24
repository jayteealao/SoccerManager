"""PoC 5: calibrates Anny faces against published adult male norms by
gradient descent. Anny is differentiable, so the fit backpropagates through
the model instead of the finite differences `bin/measure --fit` needs.

Measures are defined once in Rust (`anny_sheet landmarks`) and read here, so
both pipelines measure identically. Output: anny/calibration_anny.json,
per-ancestry local-change weights (Anny labels) applied scaled by the mix.

Usage: ANNY_CACHE_DIR=downloads/anny-cache python anny/anny_fit.py out/tmp/anny
"""
import json, sys, time
from pathlib import Path

import numpy as np
import torch
import anny
from anny.shape_distribution import SimpleShapeDistribution

RACES = ["african", "asian", "caucasian"]
# Anny label -> largest weight the fit may give it (same limits as the Rust fit).
CANDIDATES = {
    "mouth-scale-horiz-incr": 0.8, "mouth-trans-up": 0.5, "chin-height-incr": 0.6,
    "chin-width-incr": 0.6, "chin-bones-incr": 0.6, "head-scale-vert-incr": 0.25,
    "head-scale-horiz-incr": 0.25, "nose-scale-horiz-incr": 0.5, "nose-scale-vert-incr": 0.5,
    "{s}-eye-trans-out": 0.5, "{s}-eye-scale-incr": 0.25, "neck-scale-horiz-incr": 0.5,
    "head-fat-incr": 0.4,
}


def goal_mask(race, k):
    """Same choices as `goal()` in src/bin/measure.rs."""
    if k in (2, 7):  # forehead (hairline), inner canthus (3D pick unreliable)
        return False
    if race == "african" and k in (1, 12):  # no sourced African male value
        return False
    if race == "asian" and k not in (1, 3, 4, 5, 6, 11):  # no sourced East Asian norms
        return False
    return True


def main(job_dir):
    job_dir = Path(job_dir)
    defs = json.loads((job_dir / "landmarks.json").read_text())
    model = anny.Anny(local_changes="all", phenotypes="all").to(dtype=torch.float32)
    dist = SimpleShapeDistribution(model)
    base_idx = model.base_mesh_vertex_indices.numpy()
    inv = {int(b): i for i, b in enumerate(base_idx)}
    A = torch.tensor([inv[d["a"]] for d in defs])
    B = torch.tensor([inv[d["b"]] for d in defs])
    # Our axis 0 = MakeHuman x = Anny x; axis 1 = MakeHuman y = Anny z.
    AX = torch.tensor([0 if d["axis"] == 0 else 2 for d in defs])
    sd = torch.tensor([d["sd"] for d in defs])

    labels = list(model.local_change_labels)
    names = list(CANDIDATES)
    bounds = torch.tensor([CANDIDATES[n] for n in names])

    def local_vector(w):
        vec = torch.zeros(1, len(labels))
        for n, x in zip(names, w):
            for lab in ([n.replace("{s}", "l"), n.replace("{s}", "r")] if "{s}" in n else [n]):
                vec = vec.index_add(1, torch.tensor([labels.index(lab)]), x.reshape(1, 1))
        return vec

    def measure(verts):
        v = verts[0]
        return (v[A, AX] - v[B, AX]).abs() * 1000.0  # metres -> mm

    proportions = float(dist.boys_conditional_proportions_distribution.get_torch_distribution(
        torch.tensor([20.0])).mean[0])
    out = {}
    for race in RACES:
        ph = {"gender": 1.0, "age": 5.0 / 9.0, "muscle": 0.62, "weight": 0.5, "height": 0.5,
              "proportions": proportions, "african": 0.0, "asian": 0.0, "caucasian": 0.0}
        ph[race] = 1.0
        norm = torch.tensor([d["african"] if race == "african" else d["naw"] for d in defs])
        # Sourced norms get full weight. Unsourced measures (for this ancestry)
        # get a weak pull toward the NAW value, 1/3 weight: a prior to stop
        # drift, not a norm. Forehead and inner canthus stay free.
        naw = torch.tensor([d["naw"] for d in defs])
        mask = torch.tensor([goal_mask(race, k) for k in range(len(defs))], dtype=torch.float32)
        prior = torch.tensor([0.0 if (goal_mask(race, k) or k in (2, 7)) else (1.0 / 3.0) ** 2
                              for k in range(len(defs))])
        p = torch.zeros(len(names), requires_grad=True)
        opt = torch.optim.Adam([p], lr=0.05)
        t = time.time()
        with torch.no_grad():
            m0 = measure(model(phenotype_kwargs=ph, local_changes_kwargs=local_vector(torch.zeros(len(names))))["rest_vertices"])
        for it in range(250):
            w = bounds * torch.tanh(p)
            m = measure(model(phenotype_kwargs=ph, local_changes_kwargs=local_vector(w))["rest_vertices"])
            loss = ((((m - norm) / sd) ** 2) * mask + (((m - naw) / sd) ** 2) * prior).sum() + 0.3 * (w ** 2).sum()
            opt.zero_grad()
            loss.backward()
            opt.step()
        w = (bounds * torch.tanh(p)).detach()
        with torch.no_grad():
            m1 = measure(model(phenotype_kwargs=ph, local_changes_kwargs=local_vector(w))["rest_vertices"])
        print(f"{race}: fitted in {time.time() - t:.0f}s, loss {loss.item():.2f}")
        for k, d in enumerate(defs):
            flag = "" if mask[k] else ("  (weak NAW prior)" if prior[k] > 0 else "  (free)")
            print(f"   {d['name']:24} norm {norm[k]:6.1f}  before {m0[k]:6.1f}  after {m1[k]:6.1f}{flag}")
        out[race] = {n: round(float(x), 3) for n, x in zip(names, w) if abs(float(x)) > 0.01}
        print("   weights:", out[race])
    Path("anny/calibration_anny.json").write_text(json.dumps(out, indent=1))
    print("wrote anny/calibration_anny.json")


if __name__ == "__main__":
    main(sys.argv[1])

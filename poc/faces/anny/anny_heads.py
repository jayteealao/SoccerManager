"""PoC 5: evaluates Anny (NAVER LABS, Apache-2.0; CC0 MakeHuman/MPFB2 data)
for jobs written by `cargo run --release --bin anny_sheet -- jobs`, and writes
each head's rest vertices in MakeHuman base-mesh indexing and units
(decimetres, Y up) so the Rust renderer can draw them.

Only the default "anny" topology is used. The optional "smplx"/"smpl"
topologies download non-commercial data and are never touched here.

Usage: ANNY_CACHE_DIR=downloads/anny-cache python anny/anny_heads.py out/tmp/anny
"""
import json, sys, time
from pathlib import Path

import numpy as np
import torch
import anny
from anny.shape_distribution import SimpleShapeDistribution

BASE_VERTS = 19158  # MakeHuman hm08 base.obj vertex count

# MakeHuman target pairs: Anny exposes one signed label per pair.
PAIRS = [("incr", "decr"), ("up", "down"), ("out", "in"), ("forward", "backward"),
         ("convex", "concave"), ("uncompress", "compress")]


def to_anny_label(name, labels):
    """MakeHuman target 'nose/nose-hump-decr' -> ('nose-hump-incr', -1)."""
    t = name.split("/")[-1]
    if t in labels:
        return t, 1.0
    for pos, neg in PAIRS:
        if t.endswith("-" + neg):
            cand = t[: -len(neg)] + pos
            if cand in labels:
                return cand, -1.0
    return None, 0.0


def main(job_dir):
    job_dir = Path(job_dir)
    jobs = json.loads((job_dir / "jobs.json").read_text())
    t0 = time.time()
    model = anny.Anny(local_changes="all", phenotypes="all").to(dtype=torch.float32)
    dist = SimpleShapeDistribution(model)
    print(f"model loaded in {time.time() - t0:.1f}s")
    labels = set(model.local_change_labels)
    base_idx = model.base_mesh_vertex_indices.numpy()
    skipped = {}
    cal_path = Path("anny/calibration_anny.json")
    anny_cal = json.loads(cal_path.read_text()) if cal_path.exists() else None
    t0 = time.time()
    for job in jobs:
        if job.get("anny_age") is not None:
            age = float(job["anny_age"])  # our face-age rule, converted by the Rust side
        else:
            # Anny's own mapping, calibrated to WHO height-for-age.
            years = torch.tensor([job["years"]], dtype=torch.float32)
            age = float(dist.morphological_age_mapping.morphological_to_anny_age(years)[0])
        ph = dict(job["phenotypes"])
        ph["age"] = age
        if ph.get("proportions") is None:
            # Median body proportions for boys at this age (clamped at 20 y).
            a = torch.tensor([min(job["years"], 20.0)], dtype=torch.float32)
            ph["proportions"] = float(dist.boys_conditional_proportions_distribution.get_torch_distribution(a).mean[0])
        local = {}
        for name, w in job["local"].items():
            lab, sign = to_anny_label(name, labels)
            if lab is None:
                skipped[name] = skipped.get(name, 0) + 1
                continue
            local[lab] = max(-1.0, min(1.0, local.get(lab, 0.0) + sign * w))
        if job.get("anny_calibration"):
            # Per-ancestry calibration fitted by anny_fit.py, scaled by the mix.
            for race, weights in anny_cal.items():
                share = job["phenotypes"][race]
                for lab, w in weights.items():
                    for l in ([lab.replace("{s}", "l"), lab.replace("{s}", "r")] if "{s}" in lab else [lab]):
                        local[l] = max(-1.0, min(1.0, local.get(l, 0.0) + share * w))
        with torch.no_grad():
            out = model(phenotype_kwargs=ph, local_changes_kwargs=local)
        v = out["rest_vertices"][0].numpy().astype(np.float64)
        mh = np.stack([10 * v[:, 0], 10 * v[:, 2], -10 * v[:, 1]], axis=1)  # metres Z-up -> dm Y-up
        full = np.full((BASE_VERTS, 3), np.nan, dtype=np.float32)
        full[base_idx] = mh
        full.tofile(job_dir / f"{job['id']}.f32")
        job["anny_age"] = age
        job["anny_phenotypes"] = ph
    (job_dir / "jobs_resolved.json").write_text(json.dumps(jobs, indent=1))
    n = len(jobs)
    print(f"{n} heads in {time.time() - t0:.1f}s ({(time.time() - t0) / n:.2f}s each)")
    if skipped:
        print("targets with no Anny label (skipped):", sorted(skipped))


if __name__ == "__main__":
    main(sys.argv[1])

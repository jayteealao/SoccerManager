"""PoC 8: prepares Google's GNM Head v3.0 (Apache-2.0) as the head asset.

Reads the model data shipped in the GNM repository (gnm_head.npz and the
identity decoder .h5) and writes plain little-endian arrays the Rust crate
reads. Nothing from the GNM Python package is imported or run: the rest-pose
forward pass (template + identity basis; expression, rotations and pose
correctives are zero) and the identity decoder (a 5-layer MLP) are
reimplemented here and in src/gnm.rs.

Output (downloads/gnm_asset/, never committed):

  verts.f32      N x 3     template in the portrait frame (decimetres, Y up,
                           face +Z, eye midpoint at the shared anchor)
  faces.u32      T x 3     triangles
  identity.f32   253 x N x 3   identity basis per unit coefficient (frame units)
  eyes.f32       2 x 3 + 253 x 2 x 3   eye joint template and identity basis
  groups.u64     N         bit per GNM vertex group (names in meta.json)
  lm68.json      iBUG-68 landmarks as (vertex, weight) triples
  decoder.f32    identity decoder weights, layer by layer (shapes in meta.json)
  meta.json

Usage: python gnm/gnm_prep.py [--check]
  --check  compares the numpy decoder with the Keras reference, when Keras is
           installed, and prints mean-head measures.
"""
import argparse, json
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parent.parent
GNM = ROOT / "downloads/GNM/gnm/shape/data"
NPZ = GNM / "versions/v3_0/gnm_head.npz"
DECODER = GNM / "semantic_sampler/identity_decoder_model.h5"
LM68 = GNM / "landmarks/head_sparse_68.txt"
OUT = ROOT / "downloads/gnm_asset"
EYE_ANCHOR = np.array([0.0, 7.284, 1.245])  # same anchor as src/head.rs
SCALE = 10.0  # metres -> decimetres
LID_OPEN_MM = 1.8

# GNM's condition vector: [female, male] + [middle_eastern, asian, white, black]
# (semantic_sampler.Gender / Ethnicity enums).
GENDER = ["female", "male"]
ETHNICITY = ["middle_eastern", "asian", "white", "black"]


def decoder_layers():
    import h5py
    f = h5py.File(DECODER, "r")
    cfg = json.loads(f.attrs["model_config"])
    layers = []
    for l in cfg["config"]["layers"]:
        if l["class_name"] != "Dense":
            continue
        n = l["config"]["name"]
        g = f["model_weights"][n][n]
        layers.append((np.asarray(g["kernel:0"], np.float32), np.asarray(g["bias:0"], np.float32),
                       l["config"]["activation"]))
    return layers


def decode(layers, z, cond):
    x = np.concatenate([z, cond], axis=-1).astype(np.float32)
    for k, b, act in layers:
        x = x @ k + b
        if act == "relu":
            x = np.maximum(x, 0)
    return x


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    a = ap.parse_args()

    d = np.load(NPZ)
    v = d["template_vertex_positions"].astype(np.float64)
    names = [str(n) for n in d["vertex_group_names"]]
    groups = d["vertex_groups"] > 0.5
    eyes_t = d["template_joint_positions"][2:4].astype(np.float64)  # left, right
    eyes_b = d["joint_identity_basis"][:, 2:4, :].astype(np.float64)

    mid = eyes_t.mean(0)
    V = (v - mid) * SCALE + EYE_ANCHOR
    B = d["vertex_identity_basis"] * SCALE
    E = (eyes_t - mid) * SCALE + EYE_ANCHOR
    EB = eyes_b * SCALE

    bits = np.zeros(len(v), np.uint64)
    for i, n in enumerate(names):
        bits[groups[i]] |= np.uint64(1) << np.uint64(i)

    lm = np.loadtxt(LM68)
    lm68 = [[[int(r[0]), float(r[1])], [int(r[2]), float(r[3])], [int(r[4]), float(r[5])]] for r in lm]

    layers = decoder_layers()
    OUT.mkdir(parents=True, exist_ok=True)
    V.astype(np.float32).tofile(OUT / "verts.f32")
    d["triangles"].astype(np.uint32).tofile(OUT / "faces.u32")
    B.astype(np.float32).tofile(OUT / "identity.f32")
    np.concatenate([E.ravel(), EB.ravel()]).astype(np.float32).tofile(OUT / "eyes.f32")
    bits.tofile(OUT / "groups.u64")
    np.concatenate([np.concatenate([k.ravel(), b]) for k, b, _ in layers]).astype(np.float32).tofile(OUT / "decoder.f32")
    (OUT / "lm68.json").write_text(json.dumps(lm68))

    # Sampler statistics for src/gnm.rs: per-class mean identity (male) and a
    # residual scale per component. The decoder's samples have about half the
    # within-group spread of published norms (gnm_norms.py), because a CVAE
    # decodes towards the conditional mean; the residual tops each head
    # component back up towards the unit variance of the PCA basis.
    rng = np.random.default_rng(80)
    samples = []
    for e in range(4):
        cond = np.zeros((4000, 6), np.float32)
        cond[:, 1] = 1
        cond[:, 2 + e] = 1
        samples.append(decode(layers, rng.normal(size=(4000, 64)).astype(np.float32), cond))
    samples = np.stack(samples)
    class_means = samples.mean(1)
    within = samples.var(1).mean(0)
    resid = np.sqrt(np.clip(1 - within, 0, 1))
    resid[170:] = 0  # eyeball and teeth components: keep the decoder's spread
    class_means.astype(np.float32).tofile(OUT / "class_means.f32")

    # Lid opening: GNM's template (a scan average) has relaxed lids, a
    # palpebral aperture of about 7.2 mm between the mid upper and lower lid
    # (iBUG 43/44 and 46/47) against the usual 9-10 mm. The smallest-norm
    # combination of GNM's eye expression components that opens the aperture
    # by LID_OPEN_MM is exported as a fixed displacement (both eyes).
    Bx = d["expression_basis"][:200]  # left and right eye-region components
    def aperture_grad(up, lo):
        g = np.zeros(200)
        for k, w in [(u, 0.5) for u in up] + [(l, -0.5) for l in lo]:
            for vi, wt in lm68[k]:
                g += w * wt * Bx[:, vi, 1]
        return g
    g = aperture_grad([43, 44], [46, 47]) + aperture_grad([37, 38], [40, 41])
    e = g / (g @ g) * (LID_OPEN_MM / 1000.0) * 2.0  # both eyes share the gradient sum
    disp = np.einsum("e,evc->vc", e, Bx) * SCALE
    disp.astype(np.float32).tofile(OUT / "lid_open.f32")
    print(f"lid opening: |e| = {np.linalg.norm(e):.2f} (expression units), max displacement {np.abs(disp).max() * 100:.2f} mm")
    resid.astype(np.float32).tofile(OUT / "resid.f32")
    meta = {
        "vertices": int(len(V)), "faces": int(len(d["triangles"])), "identity": int(B.shape[0]),
        "identity_names": [str(n) for n in d["identity_names"]],
        "groups": names,
        "decoder": [{"in": int(k.shape[0]), "out": int(k.shape[1]), "activation": act} for k, _, act in layers],
        "condition": GENDER + ETHNICITY,
        "source": "GNM Head v3.0 (Apache-2.0), https://github.com/google/GNM, gnm_head.npz and identity_decoder_model.h5",
    }
    (OUT / "meta.json").write_text(json.dumps(meta, indent=1))
    print("wrote", OUT, f"({len(V)} vertices, {len(d['triangles'])} triangles, {len(names)} groups)")

    if a.check:
        z = np.random.default_rng(0).normal(size=(2, 64)).astype(np.float32)
        cond = np.array([[0, 1, 0, 1, 0, 0], [0, 1, 0, 0, 0, 1]], np.float32)
        ours = decode(layers, z, cond)
        try:
            import os
            os.environ.setdefault("KERAS_BACKEND", "torch")
            import keras
            m = keras.models.load_model(str(DECODER), compile=False)
            ref = m.predict([z, cond], verbose=0)
            print("decoder max |numpy - keras|:", float(np.abs(ours - ref).max()))
        except ImportError:
            print("keras not installed; skipped the reference comparison")
        np.save(OUT / "decoder_check.npy", np.concatenate([z, cond, ours], axis=1))


if __name__ == "__main__":
    main()

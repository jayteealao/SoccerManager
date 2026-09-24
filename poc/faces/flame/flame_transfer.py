"""PoC 6: transfers FLAME 2023 Open (CC-BY-4.0, MPI-IS) onto the MakeHuman
base-mesh topology, so the existing renderer, eyes, hair, masks, age rules
and photo finish keep working while the face gets FLAME's scan-derived form
and identity space.

FLAME: T. Li, T. Bolkart, M. J. Black, H. Li, J. Romero, "Learning a model of
facial shape and expression from 4D scans", ACM ToG (SIGGRAPH Asia) 2017.

Outputs (in downloads/flame_mh/, never committed; derived from CC-BY data):
  conform.f32   19158 x 3   offset that moves MakeHuman's base head onto
                            FLAME's mean head (decimetres, MakeHuman axes)
  dirs.f32      K x 19158 x 3  FLAME shape directions (per unit beta) carried
                            onto MakeHuman vertices by barycentric embedding
  meta.json     alignment, K, weights summary

Usage: python flame/flame_transfer.py [--components 100]
"""
import argparse, json, pickle, warnings
from pathlib import Path

import numpy as np
from scipy.spatial import cKDTree

warnings.filterwarnings("ignore")
ROOT = Path(__file__).resolve().parent.parent
BASE_OBJ = ROOT / "downloads/makehuman/3dobjs/base.obj"
FLAME_PKL = ROOT / "assets/flame/open/flame2023_Open.pkl"
OUT = ROOT / "downloads/flame_mh"
NECK_CUT_Y = 4.75  # same cut as src/head.rs


def load_mh():
    v, groups, faces, cur = [], {}, [], None
    for line in open(BASE_OBJ):
        if line.startswith("v "):
            v.append([float(x) for x in line.split()[1:4]])
        elif line.startswith("g "):
            cur = line.split()[1]
        elif line.startswith("f "):
            idx = [int(t.split("/")[0]) - 1 for t in line.split()[1:]]
            groups.setdefault(cur, set()).update(idx)
            if cur == "body":
                for k in range(1, len(idx) - 1):
                    faces.append([idx[0], idx[k], idx[k + 1]])
    return np.array(v), groups, np.array(faces)


def umeyama(src, dst):
    """Similarity (s, R, t) minimising |s R src + t - dst|."""
    ms, md = src.mean(0), dst.mean(0)
    a, b = src - ms, dst - md
    u, sig, vt = np.linalg.svd(b.T @ a / len(src))
    d = np.eye(3)
    d[2, 2] = np.sign(np.linalg.det(u @ vt))
    r = u @ d @ vt
    s = np.trace(np.diag(sig) @ d) / (a ** 2).sum(1).mean()
    return s, r, md - s * r @ ms


def closest_on_triangles(p, tri_v):
    """Closest points on triangles (n,3,3) to points p (n,3); returns point and barycentrics."""
    a, b, c = tri_v[:, 0], tri_v[:, 1], tri_v[:, 2]
    ab, ac, ap = b - a, c - a, p - a
    d1, d2 = (ab * ap).sum(1), (ac * ap).sum(1)
    bp = p - b
    d3, d4 = (ab * bp).sum(1), (ac * bp).sum(1)
    cp = p - c
    d5, d6 = (ab * cp).sum(1), (ac * cp).sum(1)
    va = d3 * d6 - d5 * d4
    vb = d5 * d2 - d1 * d6
    vc = d1 * d4 - d3 * d2
    denom = np.where(np.abs(va + vb + vc) < 1e-20, 1e-20, va + vb + vc)
    v_ = vb / denom
    w_ = vc / denom
    bary = np.stack([1 - v_ - w_, v_, w_], 1)
    # Clamp to the triangle (simple and adequate for dense, well-shaped meshes).
    bary = np.clip(bary, 0, None)
    bary /= bary.sum(1, keepdims=True)
    q = (bary[:, :, None] * tri_v).sum(1)
    return q, bary


def embed(points, verts, faces, k=12):
    """Nearest point on a triangle mesh: triangle index, barycentrics, point, distance."""
    cent = verts[faces].mean(1)
    tree = cKDTree(cent)
    _, cand = tree.query(points, k=k)
    best = np.full(len(points), np.inf)
    tri = np.zeros(len(points), int)
    bary = np.zeros((len(points), 3))
    qbest = np.zeros_like(points)
    for j in range(k):
        f = cand[:, j]
        q, b = closest_on_triangles(points, verts[faces[f]])
        dist = np.linalg.norm(q - points, axis=1)
        better = dist < best
        best[better], tri[better], bary[better], qbest[better] = dist[better], f[better], b[better], q[better]
    return tri, bary, qbest, best


def vertex_normals(v, f):
    n = np.zeros_like(v)
    fn = np.cross(v[f[:, 1]] - v[f[:, 0]], v[f[:, 2]] - v[f[:, 0]])
    for k in range(3):
        np.add.at(n, f[:, k], fn)
    return n / np.maximum(np.linalg.norm(n, axis=1, keepdims=True), 1e-12)


def smooth(values, faces, n_verts, iters):
    """Laplacian smoothing of a per-vertex scalar over the mesh."""
    e = np.r_[faces[:, [0, 1]], faces[:, [1, 2]], faces[:, [2, 0]]]
    e = np.r_[e, e[:, ::-1]]
    for _ in range(iters):
        s = np.zeros(n_verts)
        c = np.zeros(n_verts)
        np.add.at(s, e[:, 0], values[e[:, 1]])
        np.add.at(c, e[:, 0], 1)
        values = np.where(c > 0, 0.5 * values + 0.5 * s / np.maximum(c, 1), values)
    return values


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--components", type=int, default=100)
    a = ap.parse_args()

    mh, groups, mh_faces = load_mh()
    fl = pickle.load(open(FLAME_PKL, "rb"), encoding="latin1")
    fv = np.asarray(fl["v_template"])
    ff = np.asarray(fl["f"]).astype(int)
    sd = np.asarray(fl["shapedirs"])[:, :, : a.components]  # shape only (first 300 are shape)
    head_f = ff[(ff < 3931).all(1)]
    eye_ids = {"l": np.arange(3931, 4477), "r": np.arange(4477, 5023)}

    # MakeHuman landmarks (decimetres).
    body = np.array(sorted(i for i in groups["body"] if mh[i, 1] > NECK_CUT_Y))
    mh_eye = {s: mh[sorted(groups[f"helper-{s}-eye"])].mean(0) for s in "lr"}
    head_mask = mh[body, 1] > 6.0
    mh_nose = mh[body][head_mask][mh[body][head_mask][:, 2].argmax()]
    front = body[(mh[body, 2] > 0.9) & (np.abs(mh[body, 0]) < 0.02) & (mh[body, 1] > 6.0)]
    mh_chin = mh[front[mh[front, 1].argmin()]]
    # FLAME landmarks (metres). FLAME's +x eye is its left eye, like MakeHuman.
    fl_eye = {s: fv[eye_ids[s]].mean(0) for s in "lr"}
    fl_nose = fv[:3931][fv[:3931, 2].argmax()]
    ffront = np.where((fv[:3931, 2] > 0.03) & (np.abs(fv[:3931, 0]) < 0.003))[0]
    fl_chin = fv[ffront[fv[ffront, 1].argmin()]]
    src = np.array([fl_eye["l"], fl_eye["r"], fl_nose, fl_chin])
    dst = np.array([mh_eye["l"], mh_eye["r"], mh_nose, mh_chin])
    s, r, t = umeyama(src, dst)

    # ICP refine on the face front (MakeHuman face verts -> FLAME surface).
    face_sel = body[(mh[body, 1] > 6.3) & (mh[body, 1] < 7.9) & (mh[body, 2] > 0.9)]
    for _ in range(15):
        fva = (s * (r @ fv.T)).T + t
        tri, bary, q, dist = embed(mh[face_sel], fva, head_f)
        keep = dist < np.percentile(dist, 80)
        # Solve FLAME->MH similarity from correspondences (q is on FLAME; map back).
        src_pts = (np.linalg.inv(r) @ ((q[keep] - t).T)).T / s
        s, r, t = umeyama(src_pts, mh[face_sel][keep])
    fva = (s * (r @ fv.T)).T + t
    print(f"alignment: scale {s:.3f} (m->dm), rotation angle {np.degrees(np.arccos(np.clip((np.trace(r) - 1) / 2, -1, 1))):.2f} deg")

    # Embed all head skin vertices on FLAME's head surface.
    tri, bary, q, dist = embed(mh[body], fva, head_f)
    mh_n = vertex_normals(mh, mh_faces)[body]
    fl_n = vertex_normals(fva, head_f)
    fl_n_at = (bary[:, :, None] * fl_n[head_f[tri]]).sum(1)
    agree = (mh_n * fl_n_at).sum(1)

    # Conform weight: close to FLAME, facing the same way, not ears (their
    # shapes differ) and not below FLAME's neck.
    d_cm = dist * 10  # dm -> cm
    w_conf = np.clip((1.2 - d_cm) / 0.8, 0, 1) * np.clip((agree - 0.5) / 0.3, 0, 1)
    ear = (np.abs(mh[body, 0]) > 0.55) & (mh[body, 1] > 6.5) & (mh[body, 1] < 7.8) & (mh[body, 2] < 1.0)
    w_conf[ear] = 0
    # Smooth the weight over the cut mesh to avoid seams.
    local = {g: i for i, g in enumerate(body)}
    cut_faces = np.array([[local[x] for x in f] for f in mh_faces if all(x in local for x in f)])
    w_conf = smooth(w_conf, cut_faces, len(body), 20)
    # Identity weight: broader, reaching FLAME's full surface including the scalp.
    w_id = np.clip((3.0 - d_cm) / 2.0, 0, 1) * (agree > -0.2)
    w_id[ear] *= 0.5
    w_id = smooth(w_id, cut_faces, len(body), 20)

    n = len(mh)
    # FLAME's mesh has holes at the eyes, so lid margins would snap to the
    # ragged rim. Inside an eye zone the MakeHuman lids and eyeball are kept
    # and moved as one piece with the ring of face around them.
    pb = mh[body]
    zone = np.ones(len(body))
    ring = {}
    for side in "lr":
        dcen = np.linalg.norm(pb - mh_eye[side], axis=1) * 10  # cm
        zone = np.minimum(zone, np.clip((dcen - 1.4) / 1.0, 0, 1))
        ring[side] = (dcen > 2.4) & (dcen < 3.0) & (pb[:, 2] > mh_eye[side][2] - 0.2)

    disp = w_conf[:, None] * (q - pb)
    # Remove snapping outliers (nostrils, lip line): a vertex whose offset
    # departs from its neighbours' mean by more than 0.8 mm is pulled back.
    e = np.r_[cut_faces[:, [0, 1]], cut_faces[:, [1, 2]], cut_faces[:, [2, 0]]]
    e = np.r_[e, e[:, ::-1]]
    fixed = 0
    for _ in range(6):
        acc = np.zeros_like(disp)
        cnt = np.zeros(len(disp))
        np.add.at(acc, e[:, 0], disp[e[:, 1]])
        np.add.at(cnt, e[:, 0], 1)
        avg = acc / np.maximum(cnt, 1)[:, None]
        dev = np.linalg.norm(disp - avg, axis=1)
        bad = dev > 0.008  # dm = 0.8 mm
        fixed += int(bad.sum())
        disp[bad] = avg[bad]
    print("outlier offsets pulled back:", fixed)

    def face_normals(p):
        return np.cross(p[cut_faces[:, 1]] - p[cut_faces[:, 0]], p[cut_faces[:, 2]] - p[cut_faces[:, 0]])

    def relax_flips(field, base_pos, scale=1.0, label=""):
        """Pull the offsets of vertices on triangles that flip back to their neighbours' mean."""
        n0 = face_normals(base_pos)
        total = 0
        for _ in range(8):
            n1 = face_normals(base_pos + scale * field)
            flip = (n0 * n1).sum(1) <= 0
            if not flip.any():
                break
            vs = np.unique(cut_faces[flip])
            total += len(vs)
            acc = np.zeros_like(field)
            cnt = np.zeros(len(field))
            np.add.at(acc, e[:, 0], field[e[:, 1]])
            np.add.at(cnt, e[:, 0], 1)
            field[vs] = (acc / np.maximum(cnt, 1)[:, None])[vs]
        if label:
            print(f"{label}: relaxed {total} vertex offsets on flipped triangles")
        return field

    # Interior surfaces (mouth cavity, nostrils, lid folds) get no direct
    # conform because their normals disagree with FLAME's. Carry them with
    # the surface around them by diffusing offsets inward, otherwise they
    # poke through the moved skin.
    interior = (w_conf < 0.5) & (d_cm < 3.0) & (pb[:, 1] > 6.0)
    fixed_mask = ~interior

    def diffuse(field, iters=80):
        for _ in range(iters):
            acc = np.zeros_like(field)
            cnt = np.zeros(len(field))
            np.add.at(acc, e[:, 0], field[e[:, 1]])
            np.add.at(cnt, e[:, 0], 1)
            avg = acc / np.maximum(cnt, 1).reshape(-1, *([1] * (field.ndim - 1)))
            field[interior] = avg[interior]
        return field

    disp = diffuse(disp)
    print("interior vertices carried by diffusion:", int(interior.sum()))
    # Any interior vertex still outside FLAME's skin is pushed 1 mm beneath it.
    pos = pb + disp
    itri, ibary, iq, _ = embed(pos[interior], fva, head_f)
    inrm = (ibary[:, :, None] * fl_n[head_f[itri]]).sum(1)
    out = ((pos[interior] - iq) * inrm).sum(1) > -0.005
    idx = np.where(interior)[0][out]
    disp[idx] = (iq[out] - 0.01 * inrm[out]) - pb[idx]
    print("interior vertices pushed back under the skin:", int(out.sum()))
    disp = relax_flips(disp, pb, 1.0, "conform")

    def settle(field, thresh=0.01, label=""):
        """Final pass: any offset more than 1 mm from its neighbours' mean is pulled to it."""
        total = 0
        for _ in range(30):
            acc = np.zeros_like(field)
            cnt = np.zeros(len(field))
            np.add.at(acc, e[:, 0], field[e[:, 1]])
            np.add.at(cnt, e[:, 0], 1)
            avg = acc / np.maximum(cnt, 1).reshape(-1, *([1] * (field.ndim - 1)))
            dev = np.linalg.norm((field - avg).reshape(len(field), -1), axis=1)
            bad = dev > thresh
            if not bad.any():
                break
            total += int(bad.sum())
            field[bad] = avg[bad]
        if label:
            print(f"{label}: settled {total} offsets")
        return field

    disp = settle(disp, label="conform")

    def lowpass(field, iters):
        """Uniform Laplacian smoothing of an offset field over the cut mesh.
        FLAME's value here is mid-frequency structure (brow ridge, cheekbones,
        jaw); its fine crease detail disagrees with MakeHuman's folds."""
        for _ in range(iters):
            acc = np.zeros_like(field)
            cnt = np.zeros(len(field))
            np.add.at(acc, e[:, 0], field[e[:, 1]])
            np.add.at(cnt, e[:, 0], 1)
            field = 0.5 * field + 0.5 * acc / np.maximum(cnt, 1).reshape(-1, *([1] * (field.ndim - 1)))
        return field

    disp = lowpass(disp, 12)

    # Directions: FLAME shapedirs (metres per unit beta) -> MakeHuman dm, rotated.
    K = sd.shape[2]
    sd_mh = np.einsum("ij,vjk->vik", s * r, sd)  # (5023, 3, K)
    emb = (bary[:, :, None, None] * sd_mh[head_f[tri]]).sum(1)  # (B, 3, K)
    emb = w_id[:, None, None] * emb
    emb = diffuse(emb)
    for k in range(emb.shape[2]):
        emb[:, :, k] = settle(emb[:, :, k].copy(), thresh=0.004)
    emb = lowpass(emb, 12)
    # Same check for identity directions at +-2.5 SD (the clamp in flame.rs).
    for k in range(emb.shape[2]):
        for sgn in (2.5, -2.5):
            emb[:, :, k] = relax_flips(emb[:, :, k].copy(), pb + disp, sgn)

    conform = np.zeros((n, 3), np.float32)
    dirs = np.zeros((K, n, 3), np.float32)
    for side in "lr":
        near = np.linalg.norm(pb - mh_eye[side], axis=1) * 10 < 2.4
        d_eye = disp[ring[side]].mean(0)
        e_eye = emb[ring[side]].mean(0)
        zn = zone[near][:, None]
        disp[near] = zn * disp[near] + (1 - zn) * d_eye
        emb[near] = zn[:, :, None] * emb[near] + (1 - zn[:, :, None]) * e_eye
        ids = sorted(groups[f"helper-{side}-eye"])
        conform[ids] = d_eye.astype(np.float32)
        dirs[:, ids, :] = e_eye.T[:, None, :]
    conform[body] = disp.astype(np.float32)
    dirs[:, body, :] = emb.transpose(2, 0, 1)

    OUT.mkdir(parents=True, exist_ok=True)
    conform.tofile(OUT / "conform.f32")
    dirs.tofile(OUT / "dirs.f32")
    meta = {
        "components": int(K),
        "vertices": int(n),
        "scale_m_to_dm": float(s),
        "rotation": r.tolist(),
        "translation": t.tolist(),
        "conformed_vertices": int((w_conf > 0.5).sum()),
        "identity_vertices": int((w_id > 0.5).sum()),
        "mean_conform_offset_mm": float(np.linalg.norm(conform[body], axis=1)[w_conf > 0.5].mean() * 100),
        "source": "FLAME 2023 Open (CC-BY-4.0), flame2023_Open.pkl",
    }
    (OUT / "meta.json").write_text(json.dumps(meta, indent=1))
    print(json.dumps({k: v for k, v in meta.items() if k not in ("rotation", "translation")}, indent=1))


if __name__ == "__main__":
    main()

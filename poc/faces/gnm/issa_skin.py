"""PoC 8 / roadmap 8: face-skin colour statistics from ISSA.

International Skin Spectra Archive (ISSA), Lu et al., CC BY 4.0,
https://doi.org/10.6084/m9.figshare.28228571.v4 (downloads/issa/ISSA.xlsx).

The workbook stores colorimetry as spreadsheet formulas, so CIE XYZ and
CIELAB (D65, CIE 1931 2 degree observer) are recomputed here from the
reflectance spectra with the CMF and illuminant rows the workbook itself
carries, exactly as its formulas do. Face sites only (cheek, cheek bone,
chin, forehead, nose tip); male and female kept apart.

Output: data/issa_face_skin.json (derived statistics; commit-safe, with the
attribution above), and a printed table.
"""
import json
from collections import defaultdict
from pathlib import Path

import numpy as np
import openpyxl

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / "downloads/issa/ISSA.xlsx"
OUT = ROOT / "data/issa_face_skin.json"

GROUPS = {"CA": "Caucasian (UK, Spain)", "CN": "Chinese", "SA": "South Asian (Pakistani)", "AF": "African",
          "IQ": "Middle Eastern (Iraqi)", "TH": "Southeast Asian (Thai)", "JP": "Japanese",
          "AB": "Middle Eastern (Arabian)"}
SITES = {2: "cheek", 3: "cheek bone", 4: "chin", 6: "forehead", 9: "nose tip"}


def lab(XYZ, white):
    def f(t):
        return np.where(t > 0.008856, np.cbrt(t), 7.787 * t + 16 / 116)
    x, y, z = (XYZ / white).T
    return np.stack([np.where(y > 0.008856, 116 * np.cbrt(y) - 16, 903.3 * y), 500 * (f(x) - f(y)), 200 * (f(y) - f(z))], 1)


def main():
    wb = openpyxl.load_workbook(SRC, read_only=True)
    rows = list(wb["ISSA"].iter_rows(values_only=True))
    # Columns N..AZ (index 13..51) are 360..740 nm in 10 nm steps.
    wl = slice(13, 52)
    cmf = np.array([[float(v or 0) for v in rows[r][wl]] for r in (2, 3, 4)])
    ill = np.array([float(v or 0) for v in rows[5][wl]])
    k = 100 / (cmf[1] * ill).sum()
    white = np.array([(cmf[i] * ill).sum() * k for i in range(3)])
    data = defaultdict(list)
    subjects = defaultdict(set)
    for r in rows[12:]:
        if r[0] is None or r[3] not in GROUPS or r[6] not in SITES:
            continue
        spec = np.array([np.nan if v is None or v == "" else float(v) for v in r[wl]])
        if np.isnan(spec[4:31]).any():  # need at least 400-700 nm
            continue
        spec = np.nan_to_num(spec) / 100.0
        XYZ = np.array([(spec * cmf[i] * ill).sum() * k for i in range(3)])
        data[(r[3], r[4] or "U")].append((r[1], r[2], r[6], XYZ))
        subjects[(r[3], r[4] or "U")].add((r[1], r[2]))
    out = {"source": "ISSA, Lu et al., CC BY 4.0, doi:10.6084/m9.figshare.28228571.v4",
           "method": "CIELAB D65/2deg from reflectance, face sites (cheek, cheek bone, chin, forehead, nose tip); "
                     "per-subject mean over face sites, then group mean, SD and covariance over subjects",
           "groups": {}}
    print(f"{'group':30} sex  n   L*    a*    b*   sdL  sda  sdb  ITA")
    for (g, sex), recs in sorted(data.items()):
        per = defaultdict(list)
        site = defaultdict(list)
        for origin, subj, s, XYZ in recs:
            per[(origin, subj)].append(XYZ)
            site[SITES[s]].append(XYZ)
        subj_lab = lab(np.array([np.mean(v, 0) for v in per.values()]), white)
        if len(subj_lab) < 8:
            continue
        m, sd = subj_lab.mean(0), subj_lab.std(0, ddof=1)
        cov = np.cov(subj_lab.T)
        ita = np.degrees(np.arctan((m[0] - 50) / m[2]))
        site_lab = {n: lab(np.array(v), white).mean(0).round(2).tolist() for n, v in site.items() if len(v) >= 8}
        out["groups"].setdefault(g, {"label": GROUPS[g]})[sex] = {
            "subjects": len(subj_lab), "lab_mean": m.round(2).tolist(), "lab_sd": sd.round(2).tolist(),
            "lab_cov": cov.round(3).tolist(), "ita_deg": round(float(ita), 1), "sites": site_lab}
        print(f"{GROUPS[g][:30]:30} {sex}  {len(subj_lab):4d} {m[0]:5.1f} {m[1]:5.1f} {m[2]:5.1f} {sd[0]:4.1f} {sd[1]:4.1f} {sd[2]:4.1f} {ita:5.1f}")
    OUT.parent.mkdir(exist_ok=True)
    OUT.write_text(json.dumps(out, indent=1))
    print("wrote", OUT)


if __name__ == "__main__":
    main()

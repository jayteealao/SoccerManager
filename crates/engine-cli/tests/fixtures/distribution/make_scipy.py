"""Writes scipy.json: synthetic cases with scipy's Wasserstein-1 and total-variation values,
which the distribution test of engine-cli checks its in-house code against.

Every input is drawn here from a fixed NumPy seed; no figure comes from real matches. Run
once, in a throwaway environment, from this folder:

    uv run --no-project --with scipy==1.18.1 --with numpy python make_scipy.py

No test runs Python: the tests read the JSON this writes.
"""

import json

import numpy as np
import scipy
from scipy.spatial.distance import cityblock
from scipy.stats import wasserstein_distance

rng = np.random.default_rng(20261009)
points = []


def point_case(name, u, v, uw=None, vw=None):
    case = {"name": name, "u": [float(x) for x in u], "v": [float(x) for x in v]}
    if uw is not None:
        case["u_weights"] = [float(x) for x in uw]
        case["v_weights"] = [float(x) for x in vw]
    case["wasserstein"] = float(wasserstein_distance(u, v, uw, vw))
    points.append(case)


point_case("integers", rng.integers(0, 16, 50), rng.integers(0, 16, 70))
point_case(
    "integers with weights",
    rng.integers(0, 16, 40),
    rng.integers(0, 16, 30),
    rng.integers(1, 6, 40),
    rng.integers(1, 6, 30),
)
point_case("real values", rng.normal(17.0, 6.0, 100), rng.normal(15.5, 7.0, 80))
point_case("unequal sizes", rng.normal(3.0, 1.0, 5), rng.normal(2.0, 2.0, 200))
point_case("a point mass and a spread sample", [3.0], rng.normal(4.0, 2.0, 60))

# Binned histograms: the bin points k * width, weighted by the counts.
bins = []
for width, n in [(1.0, 41), (2.0, 31), (1.0, 16)]:
    a = rng.integers(0, 30, n).astype(float)
    b = rng.integers(0, 30, n).astype(float)
    a[rng.integers(0, n, n // 4)] = 0.0
    at = np.arange(n) * width
    bins.append(
        {
            "name": f"{n} bins of {width}",
            "width": width,
            "a": [float(x) for x in a],
            "b": [float(x) for x in b],
            "wasserstein": float(wasserstein_distance(at, at, a, b)),
        }
    )

# Cells: counts over a 6 by 6 table; total variation is half the city-block distance of
# the two shares.
cells = []
for i in range(4):
    a = rng.integers(0, 40, 36).astype(float)
    b = rng.integers(0, 40, 36).astype(float)
    if i == 3:
        b[:18] = 0.0
    cells.append(
        {
            "name": f"cells {i + 1}",
            "a": [float(x) for x in a],
            "b": [float(x) for x in b],
            "total_variation": float(cityblock(a / a.sum(), b / b.sum()) / 2.0),
        }
    )

doc = {
    "scipy": scipy.__version__,
    "numpy": np.__version__,
    "seed": 20261009,
    "points": points,
    "bins": bins,
    "cells": cells,
}
with open("scipy.json", "w", encoding="utf-8", newline="\n") as f:
    json.dump(doc, f, indent=1)
    f.write("\n")
print("wrote scipy.json:", len(points), "point cases,", len(bins), "binned,", len(cells), "cells")

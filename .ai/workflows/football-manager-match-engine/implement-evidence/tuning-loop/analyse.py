import json

T = "C:/Users/jayte/AppData/Local/Temp/claude/C--Users-jayte-Documents-dev-SoccerManager/a7ca6102-5eb6-4026-b025-47fc91bca540/scratchpad/tl"
load = lambda n: json.load(open(f"{T}/{n}/report.json"))
full, pa, eq, ra = load("full-42"), load("pair-a"), load("equal-a"), load("red-a")
out = {}

# Agreement: the targeted pairing and suite against the full run.
key = ["4-4-2", "4-4-1-1"]
fp = [p for p in full["calib.formations"] if p["pairing"] == key][0]
out["pairing_entries_targeted"] = len(pa["calib.formations"])
out["pairing_figures_equal_full"] = pa["calib.formations"][0] == fp
out["pairing_figures"] = fp
rows = lambda r, s, pair=None: [b for b in r["calib.bands"] if b["suite"] == s and b["band"] != "wall_ms" and (pair is None or b.get("pairing") == pair)]
out["pairing_rows_equal_full"] = rows(pa, "formations") == rows(full, "formations", "4-4-2 v 4-4-1-1")
strip = lambda f: {k: v for k, v in f.items() if k != "outliers"}
out["equal_figures_equal_full"] = strip(eq["calib.suites"]["equal"]) == strip(full["calib.suites"]["equal"])
out["equal_rows_equal_full"] = rows(eq, "equal") == rows(full, "equal")
out["fixtures_hash"] = {n: r["fixtures.hash"] for n, r in [("full", full), ("pair", pa), ("equal", eq)]}
out["full_wall_ms"] = full["calib.wall_ms"]

# Diffs against the baseline.
for name, r in [("pair-a", pa), ("equal-a", eq), ("red-a", ra)]:
    d = r["calib.diff"]
    out[f"diff_{name}"] = {
        "rows": len(d["rows"]), "noise": d["noise"], "changes": d["changes"],
        "max_abs_change": max(abs(x["change"]) for x in d["rows"]),
        "content_changed": d["content_changed"],
        "example": d["rows"][0],
    }
    out[f"wall_{name}"] = r["calib.wall_ms"]

# Profiles: identical figures and the rate.
def rate(r, suite):
    return round(r["calib.suites"][suite]["matches"] / (r["calib.wall_ms"][suite] / 1000), 2)
for exe in ["a", "a2", "b", "c"]:
    r = load(f"pair-{exe}")
    ident = {}
    if exe != "a":
        d = r["calib.diff"]
        ident = {
            "diff_all_zero": all(x["change"] == 0 for x in d["rows"]),
            "suites_equal": strip(r["calib.suites"]["formations"]) == strip(pa["calib.suites"]["formations"]),
            "formations_equal": r["calib.formations"] == pa["calib.formations"],
        }
    out[f"profile_pair_{exe}"] = {"matches_per_s": rate(r, "formations"), "wall_ms": r["calib.wall_ms"], **ident}
for exe in ["b", "c"]:
    r = load(f"red-{exe}")
    d = r["calib.diff"]
    out[f"profile_red_{exe}"] = {
        "matches_per_s": rate(r, "red-card"),
        "diff_all_zero": all(x["change"] == 0 for x in d["rows"]),
        "red_card_equal": r["calib.red_card"] == ra["calib.red_card"],
    }
out["profile_red_a"] = {"matches_per_s": rate(ra, "red-card")}
out["red_card_figures"] = ra["calib.red_card"]
out["violations"] = {n: load(n)["validate.violations"] for n in ["full-42", "pair-a", "equal-a", "red-a"]}
print(json.dumps(out, indent=1))

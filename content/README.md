# Content files

This folder holds the content files the engine reads: the attribute schema, the tuning constants, the rule pack, the tactics file, the trait catalogue, the commentary lines, the realism bands and their gate proof, ten sample clubs, and the fast model's fit.

Every field of every file, with its unit, its default, and its bound, is in [the data-file reference](../docs/reference/data-files.md). To change a value, follow [the modding how-to](../docs/how-to/modding.md).

To change behaviour with code rather than values, write a script pack: see [script packs](scripts/README.md) and the sample in `scripts/sample/`.

## Sample clubs

`teams/` holds ten sample clubs. A served match plays the home and away files (`default-a.json` and `default-b.json` unless a flag names others); every other club file in the folder plays in the background round of that matchday, paired into fixtures from the match seed and played on the full engine beside the match. The eight `club-000007ea-*.json` files were written once by `engine-cli generate --seed 2026 --clubs 8`. A club file added to the folder joins the round; a file that does not load is left out with a warning. No file in `teams/` other than the two a match plays changes that match, its save or its replay.

The sample clubs are older team files. Each converts as it loads: the new attributes take values from related ones, every personality value is the middle of the scale, the weaker foot is 50 percent, only the listed position is rated, and no player has a trait of play. Match setup names a club whose file converted. A file written by `engine-cli generate` is at the current version 3 and carries every field.

## Traits of play

`traits.json` is the trait catalogue: the 31 traits of play a player may carry, such as `cuts_inside` or `holds_up_ball`. Each trait names the choice of play it acts on (`hook`), and nine pairs name each other as opposites (`opposite`). A team file gives a player's traits by name with a strength, `moderate` or `strong`. The engine refuses a team file that names a trait the catalogue does not hold, or gives one player two opposite traits, and names the player and the traits. A trait's weight and, for the ten traits that can go against an instruction, its chance to override the instruction are in `tuning.json` under `engine.contract.traits`. Every field is in [the data-file reference](../docs/reference/data-files.md).

## Home grounds

A team file may name its club's home ground: `"ground": { "length": 100, "width": 64 }` inside `club`, in metres. A match is played on the home team's ground; a file without one plays on 105 by 68. The Laws allow a length of 90 to 120 metres and a width of 45 to 90 metres, with the touchline longer than the goal line, and the engine refuses any other ground by the club's name. The two default clubs name no ground. [The data-file reference](../docs/reference/data-files.md#teamsjson) says which distances follow the ground and which keep their sizes from the Laws.

## Feature flags

The `flags` block of `tuning.json` declares switches between the current model and a candidate. It ships empty. Each flag has five fields:

- `owner`, `hypothesis`, and `removal_condition`: required text. A flag without one of them is refused by name.
- `state`: `on` or `off`, default `off`.
- `overrides`: tuning values to use while the flag is on, by dotted path such as `engine.shot_range`.

A flag must switch something: it has overrides, or engine code reads it by name. An override must name a real value of the same type and stay inside its bound. The refusal names the flag, for example `content refused: tuning tuning.json: flags.short_shot_range.owner: is required`.

Compare a candidate with `calibrate --pair <name>`, then remove the flag with the removal checklist in [the modding how-to](../docs/how-to/modding.md#remove-a-flag).

## Realism bands

`realism-bands.json` holds the acceptance criteria that `engine-cli calibrate` checks. It is at version 4: each band has its design range under `design`, and its record: `real` (a pointer to where its real value is measured, the sample size and the date), `evidence` (`calibration` or `validation`), `confidence` (the measurement count, and the competitions, seasons and clubs behind the real value), `condition` (`dry` for every band today) and `gates` (`true` for the 18 bands that change the verdict, `false` for the 35 that only report). The file holds pointers, never a real value: the calibrate command reads each value in place from the measured results (`--real-data`, by default the main checkout's `.scratch/out`) and reports it beside the verdict. Versions 3 and 2 still load and migrate in memory; a migrated band gates, is `dry`, and has no real record. The engine refuses a file of version 1. The bands are criteria, never tuning values: change a band only by a recorded product-owner decision. Every field is in [the data-file reference](../docs/reference/data-files.md#realism-bandsjson).

The three bands of the strength and red-card suites (`stronger_team_win_rate`, `reduced_minus_full`, `full_over_control`) measure engine experiments that no real match repeats: a club with every attribute times 1.15, and a side a player down from kick-off. Their `real.source` is `{"none": "<reason>"}`, with `real.sample` 0 and the date of that decision, and the report counts them apart from the bands without a real value.

The pointers read these files of the measured results: `wy2_matches.csv` (goals, ten-plus and goalless shares), `wy2_team_matches.csv` (possession and passes; it holds the 1,826 league matches), `wyscout_rates.json` (the means of the five leagues: shots, yellow cards, pass accuracy, corners, throw-ins and goal kicks, halved per team where the band is per team), `sb_action_summary.json` (shots on target and goals per expected goal), and `wy2_events.csv.gz` (sending-off share: the matches with tag 1701 or 1703, counted over the compressed events file).

The 35 bands that only report (`gates: false`) are judged and shown in a table of their own, and never change the verdict. Each range is set from its real value measured on this machine's results, at least two real standard errors either side of it, except fouls and offsides, whose ranges run from the lowest league mean of the five to the highest. A band gates only after a product-owner decision promotes it. The reporting bands read these files of the measured results:

- `wyscout_rates.json`: fouls and offsides per team.
- `wy2_matches.csv`: home goal difference, the 5-1 scoreline.
- `wy2_team_matches.csv`: the per-club spread over 34 matches, how goals per team spread against a Poisson count.
- `sb_events.csv.gz`: shot distance, in metres from the goal at 105, 34.
- `sb2_events.csv.gz`: possession by third, by the duration of passes and carries of the team in possession.
- `gap_sb.json`: goals by how the move began.
- `wyscout_actions.csv.gz`: pass completion by kind and length.
- `idsse_players.csv`: running, from 7 matches. That is under its floor of 30, so these eight bands would only report even with `gates: true`.

| Band | Unit | Range | Real value |
|---|---|---|---|
| `fouls_per_team` | fouls | 10.7 to 14.5 | 13.17 |
| `offsides_per_team` | offsides | 1.9 to 2.5 | 2.14 |
| `home_goal_difference` | goals per match | 0.15 to 0.55 | 0.351 |
| `five_one_share` | share of matches | 0 to 0.016 | 0.0082 |
| `worst_to_best_spread` | goals | 6.0 to 9.1 | 7.55 |
| `goals_poisson_ratio` | ratio | 0.95 to 1.25 | 1.133 |
| `shot_distance_mean` | metres | 12.7 to 19.1 | 15.91 |
| `possession_own_third_share`, `_middle_`, `_final_` | share of possession | 0.23 to 0.35, 0.40 to 0.59, 0.17 to 0.25 | 0.293, 0.495, 0.212 |
| `goals_open_play_share`, `_corner_`, `_free_kick_`, `_penalty_`, `_throw_in_` | share of goals | 0.29 to 0.44, 0.10 to 0.15, 0.12 to 0.18, 0.07 to 0.10, 0.15 to 0.22 | 0.366, 0.128, 0.153, 0.083, 0.186 |
| `pass_completion_ground_lt10`, `_ground_10_25`, `_chip_10_25`, `_chip_gt25`, `_cross_10_25`, `_cross_gt25` | share of passes | 0.68 to 1, 0.74 to 1, 0.41 to 0.62, 0.44 to 0.65, 0.24 to 0.36, 0.26 to 0.39 | 0.847, 0.923, 0.514, 0.546, 0.302, 0.326 |
| `distance_per_minute` | metres per minute on the pitch | 86 to 130 | 107.9 |
| `high_speed_share` | share of time | 0 to 0.075 | 0.0375 |
| `peak_speed_mean` | metres per second | 6.7 to 10.1 | 8.39 |
| `speed_walk_share`, `_jog_`, `_run_`, `_hi_`, `_sprint_` | share of time in each speed zone | 0.40 to 0.59, 0.29 to 0.43, 0.08 to 0.13, 0 to 0.061, 0 to 0.014 | 0.495, 0.362, 0.105, 0.0306, 0.0070 |

Each smallest shift is a quarter of the range. The bands from shot distance on read the measures of a build with the `measures` feature; a build without them reports them `not sure`. The speed-zone shares split the players' time on the pitch into the five zones; the high-speed share is the high and sprint zones together.

The pass curve has nine cells, three kinds by three lengths (under 10 m, 10 to 25 m, over 25 m), and six of them are banded. Three are not: a ground pass over 25 m, a chip under 10 m and a cross under 10 m. The engine plays a pass that is not a cross in the air, as a chip, when it is over 25 m, and along the ground otherwise, so it makes no ground pass over 25 m and no chip of 25 m or under, and the real chips and crosses under 10 m are few (3,783 and 2,295 of about 1.4 million passes). For the same reason `pass_completion_chip_10_25` has no engine value: it reports `not sure` with no value until the engine plays a chip of that length.

### Whole-distribution bands

Six of the bands that only report judge a whole distribution, not one value. A `distribution` band (`{"kind": "distribution", "of": <source>, "width": <bin width>, "bins": <bins>}`) holds each match's histogram of its source: `goals` (the match's goals, both sides), `passes` (each side's passes), and, in a `measures` build, `shot_distance`, `chain_passes` and `chain_seconds` (the rows' own bins: 41 bins of 1 m, 16 bins of one pass, 31 bins of 2 seconds); the last bin holds everything above it. A `cells` band (`{"kind": "cells", "max": 5}`) holds each match's score as one cell of the table, each side's goals capped at `max`. Its value is a distance from the real sample: Wasserstein-1 for a distribution (in the unit of its source), total variation for the cells (a share from 0 to 1). Its range runs from 0 to its margin.

The band is an equivalence test: the engine's matches and the real matches are each resampled by whole match, and the band passes when the 97th percentile of the resampled distance (the upper bound) is within the margin, fails when the 3rd percentile is beyond it, and is not sure otherwise. A perfect engine still shows a distance at these sample sizes; its 97th percentile is the noise floor, and a margin at or under the floor reads `not sure` with "margin below noise floor". Each margin is the larger of the five leagues' spread (the largest distance between one competition's sample and the pooled sample) and twice the noise floor of a 400-match equal suite against the full real sample, measured on this machine's results on 2026-10-09; the smallest shift is a quarter of it. The chain and shot samples are 200 matches, so their noise floor is set mostly by the real sample, and more engine matches cannot lower it.

Three reductions read the real samples, each file once: `histogram`, `cells` and `chains`, described in [the data-file reference](../docs/reference/data-files.md#realism-bandsjson). The real value `calib.real` shows for a whole sample is the mean of its points, each bin at its lower edge.

| Band | Distance | Margin | Real sample | Real mean |
|---|---|---|---|---|
| `goals_distribution` | Wasserstein-1, goals | 0.42 | 1,826 matches, `wy2_matches.csv` | 2.709 goals |
| `scoreline_cells` | total variation, 36 cells | 0.26 | 1,826 matches, `wy2_matches.csv` | 2.692 goals (each side capped at 5) |
| `passes_per_team_distribution` | Wasserstein-1, passes | 20.3 | 1,826 matches, `wy2_team_matches.csv` | 428.6 passes |
| `shot_distance_distribution` | Wasserstein-1, metres | 0.51 | 200 matches (5,039 shots), `sb_events.csv.gz` | 15.36 m (bins at their lower edge) |
| `chain_passes_distribution` | Wasserstein-1, passes | 0.42 | 200 matches (33,550 possessions), `sb2_events.csv.gz` | 5.46 passes |
| `chain_seconds_distribution` | Wasserstein-1, seconds | 1.03 | 200 matches (33,550 possessions), `sb2_events.csv.gz` | 19.77 s |

The real chains are StatsBomb's possessions, not the engine's chain rule: both open a chain at a gain of the ball or a restart, but a StatsBomb possession can hold the other team's failed touches. Wyscout counts every pass, the engine open-play passes, as for `passes_per_team`.

### Earlier versions

Version 1 set four bands: goals per match 2.4 to 3.2, shots per team 8 to 16, possession 35 to 65 percent for each side, and a stronger club (every attribute times 1.15) winning more than half its matches. Version 2 adds eleven bands, checked in the equal suite:

| Band | Unit | Range | Real value | Source |
|---|---|---|---|---|
| `ten_plus_goals_share` | share of matches | 0 to 0.005 | 0 of 1,941 matches | Wyscout match files `.scratch/out/wy2_*.csv` |
| `sending_off_share` | share of matches | 0.08 to 0.22 | 0.157 | Wyscout match files `.scratch/out/wy2_*.csv` |
| `yellow_cards_per_team` | cards | 1.2 to 2.6 | 1.56 to 2.45 (3.11 to 4.90 per match) | `.scratch/out/FINDINGS.md:147` |
| `shots_on_target_share` | share of shots | 0.30 to 0.42 | 0.350 (StatsBomb), 0.356 (Wyscout) | `.scratch/out/ACTIONS.md:346`, `:413` |
| `goals_per_xg` | goals per expected goal | 0.85 to 1.15 | 0.99 (2.93 goals, 2.96 xG) | `.scratch/out/ACTIONS.md:345-346` |
| `passes_per_team` | passes | 350 to 550 | 428.6 | Wyscout match files `.scratch/out/wy2_*.csv` |
| `pass_accuracy_pct` | percent | 75 to 88 | 82.2 to 83.3 | `docs/design/realism/01-engine-realism.md:138` |
| `corners_per_team` | corners | 3.5 to 6.5 | 4.98 | Wyscout match files `.scratch/out/wy2_*.csv` |
| `throw_ins_per_match` | throw-ins | 35 to 55 | 42 to 46 | `docs/design/realism/01-engine-realism.md:142` |
| `goal_kicks_per_match` | goal kicks | 12 to 22 | 16 to 17 | `docs/design/realism/01-engine-realism.md:142` |
| `goalless_share` | share of matches | 0.04 to 0.12 | 0.0712 | `docs/design/realism/01-engine-realism.md:134` |

Shares are fractions from 0 to 1. Shots on target and goals per expected goal are pooled: the suite's total over the suite's total. Yellow cards count a second yellow, as the engine does. The `.scratch/out/` files are the local outputs behind the realism design document; they are not in the repository.

### Band definitions

Every real value needs a full record: the provider, the event definition, the denominator, the unit, a calibration or validation label, and how the engine counts the same thing. The table below gives that record for each version 2 band. It changes no range and no real value.

Wyscout figures cover the 2017/18 season of England, France, Germany, Italy, and Spain (1,826 league matches), and some also the 2018 World Cup and Euro 2016 (1,941 matches in all). Wyscout has no ball status, so every Wyscout rate is per match of clock time, never per live minute. The realism work now holds out the German league and the two tournaments for validation. Every Wyscout value below still pools them, so each is labelled calibration with that caveat, and the reference file of real targets recomputes it on the four calibration leagues.

| Band | Provider and pool | Real-side definition and denominator | Engine counts | Label |
|---|---|---|---|---|
| `ten_plus_goals_share` | Wyscout, all 1,941 matches | Matches whose final score totals 10 or more goals, over all matches | Matches with 10 or more goals in the suite | Calibration; pools the held-out matches |
| `sending_off_share` | Wyscout, all 1,941 matches | Matches with at least one red card or second yellow (tags 1701 or 1703), over all matches | Matches with at least one player sent off | Calibration; pools the held-out matches |
| `yellow_cards_per_team` | Wyscout, five leagues | Yellow-card tags per match, range of the five league means, halved for one team; whether a second yellow counts on the real side is not recorded | Yellow cards per team, a second yellow included | Calibration; pools the German league |
| `shots_on_target_share` | StatsBomb, 200 matches; Wyscout, 1,941 matches | Shots on target over all shots, pooled over the sample | Shots whose flight crosses the goal line between the posts and under the bar, over all shots, pooled | Calibration |
| `goals_per_xg` | StatsBomb, 200 matches | Goals over the provider's expected goals, pooled | Goals over the engine's own expected goals (distance and angle only), pooled | Calibration; the two expected-goal models differ |
| `passes_per_team` | Wyscout, five leagues | `Pass` events (event 8) per team per match; set-piece passes are free-kick events and are not counted | Open-play passes per team; a clearance and a restart kick are not passes | Calibration; pools the German league |
| `pass_accuracy_pct` | Wyscout, five leagues | Accurate `Pass` events over all `Pass` events, range of the five league values; the mean of team-match ratios gives 80.9 to 82.1 percent | Passes whose next controlling touch is a team-mate's, over open-play passes | Calibration; pools the German league |
| `corners_per_team` | Wyscout, five leagues | Corner free-kick events per team per match | Corners awarded per team | Calibration; pools the German league |
| `throw_ins_per_match` | Wyscout, five leagues | Throw-in free-kick events per match, both teams, range of the five league means | Throw-ins per match, both teams | Calibration; pools the German league |
| `goal_kicks_per_match` | Wyscout, five leagues | Goal-kick free-kick events per match, both teams, range of the five league means | Goal kicks per match, both teams | Calibration; pools the German league |
| `goalless_share` | Wyscout, five leagues | Matches that end 0-0, over the 1,826 league matches | Matches that end 0-0 | Calibration; pools the German league |

The formations suite plays every pairing of the formations in `tactics.json`, a formation against itself included: ten formations give 55 pairings. Each pairing is checked against `goals_per_match`, `ten_plus_goals_share`, and `goalless_share`.

## Gate proof

`gate-proof.json` records how the gate of the realism bands was proven. It declares four broken copies of the engine, each a content flag that moves one measure: `shots_up` (shots per team up about 10 percent), `fouls_halved` (fouls per team about half), `card_storm` (every tackle a foul and every foul booked), and `pass_completion_down` (pass accuracy down about 5 points). `engine-cli calibrate proof` plays the engine against itself and each copy against the engine, and writes under `proof` the pass rate of the self runs, each copy's fail rate, the bands each copy moved, whether any copy moved each whole-distribution band, and the gate it measured: the band file's digest, the measures version, and the judging code's digest. The copies are declared as flags only in the proof's own copy of this folder, so the `flags` block of `tuning.json` still ships empty.

Every calibrate run compares its gate with the proof's and prints `gate changed since the last proof` when they differ, for example after a band is promoted. Run the proof again then, and commit the new file: see [Run the gate proof](../docs/how-to/calibration.md#run-the-gate-proof). No match reads the file, and it holds no real value.

## The fast model's fit

`fast-model.json` is the fast model fitted from full-engine results: a results model that gives a final score and the goal events from the two teams at kick-off. It records the engine id of the results it came from, the id of `gate/golden.json`, and CI fails when the two differ. A change that regenerates the golden file must refit in the same change: run `engine-cli fast-model fit` from the repository root (about 25 minutes on 8 cores); it writes this file only when the fast model equals the full engine on its check. The engine never reads the file to play a match, so a refit changes no save, replay or gate hash. Every field is in [the data-file reference](../docs/reference/data-files.md#fast-modeljson).

## Formations

`tactics.json` ships ten formations. The first four keep their places, so a saved lineup keeps its shape: 4-4-2, 4-3-3, 4-2-3-1, 3-5-2. Six follow: 4-1-4-1, 4-4-1-1, 4-1-2-1-2 (the diamond), 3-4-3, 5-3-2, and 5-4-1. The computer manager starts in 4-4-2 and never changes formation during a match.

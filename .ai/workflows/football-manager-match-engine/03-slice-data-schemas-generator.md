---
schema: sdlc/v1
type: slice
slug: football-manager-match-engine
slice-slug: data-schemas-generator
status: defined
stage-number: 3
created-at: "2026-09-21T19:50:41Z"
updated-at: "2026-09-21T19:50:41Z"
complexity: m
depends-on: [engine-core]
tags: [engine, data, generator, modding]
refs:
  index: 00-index.md
  slice-index: 03-slice.md
  siblings: [03-slice-engine-core.md, 03-slice-stream-protocol.md, 03-slice-match-rules.md, 03-slice-tactics-and-ai.md, 03-slice-calibration.md]
  plan: 04-plan-data-schemas-generator.md
  implement: 05-implement-data-schemas-generator.md
---

# Slice: Data Schemas and Team Generator

## The Slice

Engine core ships with a built-in attribute stopgap. The shape requires a data-driven attribute schema of 30 to 50 attributes on a 1 to 100 scale, tuning constants, team data, and rule packs as validated files, plus a generator for fictional teams, and identity fields on every record.

This slice replaces the stopgap with schema-validated data files and adds the generator. It is the plugin boundary's first half: the data half. The scripting half is the deferred `scripting-runtime` slice.

The match-rules, tactics, and calibration slices all load these files. The top risk is schema churn: later slices add fields, so the schema carries a version and the loader rejects unknown versions.

## Goal

The engine loads attributes, tuning, team data, and rule packs from validated files, and a generator produces fictional teams with realistic attribute distributions.

## Why This Slice Exists

Commitment C3 needs attributes that drive outcomes; the modding priority needs those attributes in data; calibration needs generated teams. Building the schemas before rules and tactics avoids rewriting those slices around a second data model.

## Scope

In:
- Attribute schema file: 30 to 50 named attributes in technical, mental, physical, and goalkeeping groups, 1 to 100, with a default set shipped.
- Tuning file: physics constants, decision weights, formation tolerance, fatigue curve parameters (used by later slices), each with bounds.
- Team data file: club identifier, name, kit colors, squad of about 22 players with positions and attributes, player identifiers.
- Rule pack schema: stoppage kinds, which admit tactics changes and substitutions, substitution limits, half lengths; the rules slice consumes it.
- Schema versions; a loader that fails closed on a missing file, a schema violation, or an unknown version, naming file and field (named mechanism: **fail-closed loader**, replaces default-filling so a typo cannot silently change play).
- Team generator: fictional clubs and players; attribute distributions per position; seedable.
- Identity fields: owner identifier and match identifier on match records; club and player identifiers on team data (RIM-2, RIM-4).

Out:
- The scripting runtime: `scripting-runtime` (deferred).
- Rule enforcement: `match-rules`.

## Acceptance Criteria

- Given the shipped default attribute schema, When the engine loads it, Then every attribute has a name, group, and 1 to 100 range, and the count is between 30 and 50.
  <!-- observable: false — cargo test over the shipped file -->
- Given a team data file with a player attribute of 120, When the engine loads it, Then it refuses with a message naming the file, the player, and the attribute; and Given a valid team file, Then it loads.
  <!-- observable: false — cargo test with a bad fixture and a good fixture -->
- Given a rule pack with an unknown schema version, When the engine loads it, Then it refuses naming the version; and Given the current version, Then it loads.
  <!-- observable: false — cargo test -->
- Given a seed, When the generator builds a league of 20 clubs, Then every club has 22 players with legal positions, and per-position attribute means fall inside the distribution bounds the tuning file sets.
  <!-- observable: false — cargo test over generator output -->
- Given a match record, When it is saved and loaded, Then owner identifier and match identifier round-trip unchanged.
  <!-- observable: false — cargo test -->
- Given the generator output, When `engine-cli simulate --team-a a.json --team-b b.json` runs, Then the simulation completes and the tick count equals the engine-core result.
  <!-- observable: false — cargo integration test -->

## Dependencies on Other Slices

- `engine-core`: the loop and the command line that the loaded data feeds.

## Risks

- Attribute list bikeshedding: plan fixes the default list from the shape's four groups; changes go through the schema version.
- Generator distributions produce unrealistic squads: calibration catches it; the tuning file bounds the distributions.

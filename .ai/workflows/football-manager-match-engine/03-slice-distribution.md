---
schema: sdlc/v1
type: slice
slug: football-manager-match-engine
slice-slug: distribution
status: complete
stage-number: 3
created-at: "2026-09-21T19:50:41Z"
updated-at: "2026-09-23T18:36:35Z"
complexity: m
depends-on: [integration]
tags: [distribution, installer, packaging, deferred]
deferred: true
refs:
  index: 00-index.md
  slice-index: 03-slice.md
  siblings: [03-slice-integration.md, 03-slice-viewer-reports-recovery.md]
  plan: 04-plan-distribution.md
  implement: 05-implement-distribution.md
---

# Slice: Distribution and Installers

## The Slice

The first release runs from a development build on Windows: the engine binary at a configured path and the page opened locally. Shape U-3 parked operating systems and packaging; slice Q4 deferred them.

This slice packages the engine and the page for users who install and run a program: a Windows installer first, then builds for other operating systems by cross-compilation, with engine discovery so the page finds the binary without configuration.

It ships after integration. The top risk is platform-specific socket and process behavior on operating systems the reference laptop cannot test; each platform build needs its own verify environment or a pre-registered deferral.

## Goal

A user installs the game with one download and starts a match without editing a path.

## Why This Slice Exists

Round 3 Q9 committed to a native process users install; U-3 and slice Q4 deferred the packaging.

## Scope

In: Windows installer, engine auto-discovery and launch from the page, versioned release artifacts, a smoke test per platform build; macOS and Linux builds as the product owner decides at plan (U-3).
Out: auto-update; code signing beyond what the platform requires to run.

## Acceptance Criteria

- Given a clean Windows 11 machine, When the installer runs and the user opens the page, Then the engine is found and a match starts without configuration.
  <!-- observable: true — the install experience -->
  verify: { method: operator install on a clean Windows virtual machine with screenshots, env: Windows 11 VM (to provision), fixture: none, rung: infra-1 }
- Given a release build, Then its version string matches the engine hello message.
  <!-- observable: false — a test reads both -->

## Dependencies on Other Slices

- `integration`: the complete, verified product.

## Risks

- Other operating systems without a verify environment: pre-register the deferral naming the platform and the provisioning step.

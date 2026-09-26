# Runbook: smartscreen-glibc

## When this fires

A CI log or a player report that matches one of these patterns starts this runbook:

- `SmartScreen`
- `Windows protected your PC`
- `GLIBC_[0-9.]+' not found`

## Steps

1. Windows: tell the player to choose More info, then Run anyway. The setup file is unsigned.
2. Linux: confirm the player's glibc is 2.35 or later (ldd --version). The archive is built on ubuntu-22.04.

## Notes

_Seeded from the release plan's recovery playbook `smartscreen-glibc`. Update this file as the playbook changes._
_Last synced from plan version: 1_

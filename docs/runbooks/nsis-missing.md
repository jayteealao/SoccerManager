# Runbook: nsis-missing

## When this fires

A CI log or a player report that matches one of these patterns starts this runbook:

- `makensis.*not (found|recognized)`
- `cannot find makensis`
- `NSIS.*not installed`

## Steps

1. Add a step before build.ps1 in the Windows job: choco install nsis -y.
2. Re-run the failed job: gh run rerun <run-id> --failed.

## Notes

_Seeded from the release plan's recovery playbook `nsis-missing`. Update this file as the playbook changes._
_Last synced from plan version: 1_

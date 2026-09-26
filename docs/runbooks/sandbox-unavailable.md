# Runbook: sandbox-unavailable

## When this fires

A CI log or a player report that matches one of these patterns starts this runbook:

- `exit code 3`
- `Windows Sandbox is not enabled`
- `exit code 2`
- `no results arrive`

## Steps

1. Exit 3: enable Windows Sandbox (Optional Features > Windows Sandbox), restart, and run run-sandbox.ps1 again.
2. Exit 2: close the sandbox window and run run-sandbox.ps1 again.
3. When the sandbox stays unavailable, use the Hyper-V fallback in packaging/README.md with smoke.ps1.

## Notes

_Seeded from the release plan's recovery playbook `sandbox-unavailable`. Update this file as the playbook changes._
_Last synced from plan version: 1_

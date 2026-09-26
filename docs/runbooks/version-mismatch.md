# Runbook: version-mismatch

## When this fires

A CI log or a player report that matches one of these patterns starts this runbook:

- `release_version`
- `tag .* does not match`
- `version mismatch`

## Steps

1. Do not re-tag. Delete the draft release if one exists: gh release delete v<version> --yes.
2. Delete the wrong remote tag only when no release was published from it: git push origin :refs/tags/v<version>.
3. Run cargo release version <x.y.z> --execute, commit, and tag again.

## Notes

_Seeded from the release plan's recovery playbook `version-mismatch`. Update this file as the playbook changes._
_Last synced from plan version: 1_

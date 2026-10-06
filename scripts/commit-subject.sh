#!/bin/sh
# Refuse a commit whose first line is longer than 70 characters.
# `committed` (committed.toml) counts the subject without its last word, so a
# subject whose last word starts before column 70 passes it. This check counts
# the whole first line. Run by the commit-msg hook (lefthook.yml) with the
# message file as the only argument.
set -u

# Keep in step with subject_length in committed.toml.
limit=70
file=${1:?usage: sh scripts/commit-subject.sh <commit message file>}

subject=$(head -n 1 "$file" | tr -d '\r')
length=$(printf '%s' "$subject" | LC_ALL=C.UTF-8 wc -m | tr -d ' ')

if [ "$length" -gt "$limit" ]; then
  echo "The commit subject is $length characters; the limit is $limit." >&2
  exit 1
fi

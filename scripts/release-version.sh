#!/usr/bin/env bash
set -euo pipefail

# Print the version a release run builds.
# Tag pushes use the tag name (v0.1.0 -> 0.1.0); workflow_dispatch runs take the
# `version` input through DSHR_VERSION. An unusable version fails here instead of
# producing half-named artifacts.
version="${DSHR_VERSION:-${GITHUB_REF_NAME#v}}"
version="${version#v}"
if ! [[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+([.-][0-9A-Za-z.]+)*$ ]]; then
  echo "release-version: invalid version '$version'" >&2
  exit 1
fi
printf '%s\n' "$version"

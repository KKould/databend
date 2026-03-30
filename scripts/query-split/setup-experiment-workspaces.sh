#!/usr/bin/env bash

set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
default_root="$(dirname "$repo_root")/databend-split-workspaces"
workspace_root="${1:-$default_root}"

declare -a experiments=(
  "baseline-current"
  "split-exchange"
  "split-pipelines"
  "split-physical-plans"
  "split-interpreters"
  "split-common-sql"
)

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

tracked_patch="$tmp_dir/current.patch"
untracked_list="$tmp_dir/untracked.txt"
untracked_tar="$tmp_dir/untracked.tar"

git -C "$repo_root" diff --binary HEAD > "$tracked_patch"
git -C "$repo_root" ls-files --others --exclude-standard > "$untracked_list"

if [[ -s "$untracked_list" ]]; then
  (
    cd "$repo_root"
    tar -cf "$untracked_tar" -T "$untracked_list"
  )
fi

mkdir -p "$workspace_root"

for experiment in "${experiments[@]}"; do
  workspace_path="$workspace_root/$experiment"

  if [[ -e "$workspace_path" ]]; then
    echo "skip existing workspace: $workspace_path"
    continue
  fi

  git -C "$repo_root" worktree add --detach "$workspace_path" HEAD >/dev/null

  if [[ -s "$tracked_patch" ]]; then
    git -C "$workspace_path" apply "$tracked_patch"
  fi

  if [[ -f "$untracked_tar" ]]; then
    (
      cd "$workspace_path"
      tar -xf "$untracked_tar"
    )
  fi

  cat > "$workspace_path/.query-split-experiment" <<EOF
experiment=$experiment
source_repo=$repo_root
base_commit=$(git -C "$repo_root" rev-parse HEAD)
created_at=$(date '+%Y-%m-%d %H:%M:%S %z')
EOF

  echo "created workspace: $workspace_path"
done

echo
echo "workspace root: $workspace_root"
echo "experiments:"
for experiment in "${experiments[@]}"; do
  echo "  - $workspace_root/$experiment"
done

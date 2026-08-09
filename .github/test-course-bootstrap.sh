#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
manifest=.github/course-bootstrap.sha256
expected_members=$'.cargo/config.toml\n.github/test-course-bootstrap.sh\n.github/workflows/ci.yml\nxtask/Cargo.toml\nxtask/src/main.rs'
actual_members=$(awk '{print $2}' "$repo_root/$manifest")

if [[ "$actual_members" != "$expected_members" ]]; then
    echo "bootstrap checksum members do not match the exact allowlist" >&2
    exit 1
fi

verify_root() {
    local root=$1
    (cd "$root" && sha256sum --quiet --check "$manifest")
}

mutate_once() {
    local path=$1
    local before=$2
    local after=$3
    python3 - "$path" "$before" "$after" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
before = sys.argv[2]
after = sys.argv[3]
body = path.read_text()
if body.count(before) != 1:
    raise SystemExit(f"mutation source must appear exactly once in {path}")
path.write_text(body.replace(before, after, 1))
PY
}

temp_root=$(mktemp -d "${TMPDIR:-/tmp}/type-exercise-bootstrap.XXXXXX")
trap 'rm -rf "$temp_root"' EXIT

make_fixture() {
    local name=$1
    local fixture="$temp_root/$name"
    while read -r _ relative; do
        mkdir -p "$fixture/$(dirname "$relative")"
        cp "$repo_root/$relative" "$fixture/$relative"
    done < "$repo_root/$manifest"
    mkdir -p "$fixture/.github" "$fixture/course/src"
    cp "$repo_root/$manifest" "$fixture/$manifest"
    cp "$repo_root/course/src/setup.md" "$fixture/course/src/setup.md"
    printf '%s\n' "$fixture"
}

expect_rejected() {
    local name=$1
    local fixture=$2
    if verify_root "$fixture" >/dev/null 2>&1; then
        echo "bootstrap mutation unexpectedly passed: $name" >&2
        exit 1
    fi
}

verify_root "$repo_root"

alias_fixture=$(make_fixture alias-redirection)
mutate_once \
    "$alias_fixture/.cargo/config.toml" \
    'x = "run --package type-exercise-xtask --"' \
    'x = "test --package type-exercise-xtask --"'
expect_rejected "x alias redirection" "$alias_fixture"

runner_fixture=$(make_fixture runner-injection)
printf '\n[target.\x27cfg(unix)\x27]\nrunner = "/usr/bin/true"\n' >> "$runner_fixture/.cargo/config.toml"
expect_rejected "Unix true runner injection" "$runner_fixture"

test_fixture=$(make_fixture xtask-test-disable)
mutate_once "$test_fixture/xtask/Cargo.toml" 'test = true' 'test = false'
expect_rejected "xtask bin test disabling" "$test_fixture"

combined_fixture=$(make_fixture combined-runner-content-bypass)
printf '\n[target.\x27cfg(unix)\x27]\nrunner = "/usr/bin/true"\n' >> "$combined_fixture/.cargo/config.toml"
printf '\nLearners may edit supplied tests and skip failed assertions.\n' >> "$combined_fixture/course/src/setup.md"
expect_rejected "runner plus corrupted learner contract" "$combined_fixture"

echo "verified Cargo bootstrap integrity and 4/4 pre-Cargo mutation killers"

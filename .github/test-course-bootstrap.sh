#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
manifest=.github/course-bootstrap.sha256
expected_members=$'.cargo/config.toml\n.github/test-course-bootstrap.sh\n.github/workflows/ci.yml\nxtask/Cargo.toml\nxtask/src/main.rs'
actual_members=$(awk '{print $2}' "$repo_root/$manifest")

if [[ "$actual_members" != "$expected_members" ]]; then
    echo "bootstrap checksum members do not match the exact allowlist" >&2
    exit 1
fi

verify_cargo_directory() {
    local root=$1
    local expected="$root/.cargo/config.toml"
    local actual
    actual=$(find "$root/.cargo" -mindepth 1 -maxdepth 1 -print | LC_ALL=C sort)
    if [[ "$actual" != "$expected" || ! -f "$expected" || -L "$expected" ]]; then
        echo "repository .cargo must contain only one regular non-symlink config.toml" >&2
        return 1
    fi
}

reject_config_pair() {
    local directory=$1
    local candidate
    for candidate in "$directory/config" "$directory/config.toml"; do
        if [[ -e "$candidate" || -L "$candidate" ]]; then
            echo "external Cargo configuration is not allowed: $candidate" >&2
            return 1
        fi
    done
}

verify_execution_environment() {
    local root=$1
    local current
    local name

    current=$(dirname "$(cd "$root" && pwd -P)")
    while true; do
        reject_config_pair "$current/.cargo" || return 1
        if [[ "$current" == "/" ]]; then
            break
        fi
        current=$(dirname "$current")
    done

    reject_config_pair "$HOME/.cargo" || return 1
    if [[ -n "${CARGO_HOME:-}" ]]; then
        reject_config_pair "$CARGO_HOME" || return 1
    fi

    while IFS='=' read -r name _; do
        case "$name" in
            CARGO_ALIAS_* | CARGO_TARGET_*_RUNNER | CARGO_BUILD_RUSTC | CARGO_BUILD_RUSTC_WRAPPER | CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER | RUSTC | RUSTC_WRAPPER | RUSTC_WORKSPACE_WRAPPER)
                echo "Cargo execution environment override is not allowed: $name" >&2
                return 1
                ;;
        esac
    done < <(env)
}

verify_root() {
    local root=$1
    verify_cargo_directory "$root" || return 1
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
    local fixture="$temp_root/$name/repo"
    while read -r _ relative; do
        mkdir -p "$fixture/$(dirname "$relative")"
        cp "$repo_root/$relative" "$fixture/$relative"
    done < "$repo_root/$manifest"
    mkdir -p "$fixture/.github" "$fixture/course/src"
    cp "$repo_root/$manifest" "$fixture/$manifest"
    cp "$repo_root/course/src/setup.md" "$fixture/course/src/setup.md"
    printf '%s\n' "$fixture"
}

write_spoof_runner() {
    local root=$1
    local runner="$root/spoof-runner.sh"
    printf '%s\n' \
        '#!/usr/bin/env bash' \
        "echo 'test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;'" \
        "echo 'verified the exact Chapters 1-12 course source, supplied tests, learner dependencies, README, SVG, SUMMARY, sitemaps, and CI'" \
        "echo 'executed the exact real reference inventory: 76/76 tests'" \
        > "$runner"
    chmod +x "$runner"
    printf '%s\n' "$runner"
}

corrupt_setup() {
    local root=$1
    mutate_once \
        "$root/course/src/setup.md" \
        'Do not edit `src/tests.rs` or copied files under `src/tests/`.' \
        'Edit supplied tests or remove their assertions when they block progress.'
}

expect_root_rejected() {
    local name=$1
    local fixture=$2
    if verify_root "$fixture" >/dev/null 2>&1; then
        echo "bootstrap mutation unexpectedly passed: $name" >&2
        exit 1
    fi
}

expect_environment_rejected() {
    local name=$1
    local fixture=$2
    if verify_execution_environment "$fixture" >/dev/null 2>&1; then
        echo "Cargo environment mutation unexpectedly passed: $name" >&2
        exit 1
    fi
}

verify_root "$repo_root"
verify_execution_environment "$repo_root"

alias_fixture=$(make_fixture alias-redirection)
mutate_once \
    "$alias_fixture/.cargo/config.toml" \
    'x = "run --package type-exercise-xtask --"' \
    'x = "test --package type-exercise-xtask --"'
expect_root_rejected "x alias redirection" "$alias_fixture"

runner_fixture=$(make_fixture runner-injection)
printf '\n[target.\x27cfg(unix)\x27]\nrunner = "/usr/bin/true"\n' >> "$runner_fixture/.cargo/config.toml"
expect_root_rejected "Unix true runner injection" "$runner_fixture"

test_fixture=$(make_fixture xtask-test-disable)
mutate_once "$test_fixture/xtask/Cargo.toml" 'test = true' 'test = false'
expect_root_rejected "xtask bin test disabling" "$test_fixture"

combined_fixture=$(make_fixture combined-runner-content-bypass)
printf '\n[target.\x27cfg(unix)\x27]\nrunner = "/usr/bin/true"\n' >> "$combined_fixture/.cargo/config.toml"
corrupt_setup "$combined_fixture"
expect_root_rejected "runner plus corrupted learner contract" "$combined_fixture"

legacy_fixture=$(make_fixture legacy-config-marker-spoof)
legacy_runner=$(write_spoof_runner "$legacy_fixture")
printf '[target.\x27cfg(unix)\x27]\nrunner = "%s"\n' "$legacy_runner" > "$legacy_fixture/.cargo/config"
corrupt_setup "$legacy_fixture"
expect_root_rejected "legacy config marker spoof plus corrupted learner contract" "$legacy_fixture"

unknown_fixture=$(make_fixture unknown-config-entry)
printf 'unknown = true\n' > "$unknown_fixture/.cargo/unknown.toml"
expect_root_rejected "unknown .cargo file" "$unknown_fixture"

directory_fixture=$(make_fixture unknown-config-directory)
mkdir "$directory_fixture/.cargo/config.d"
expect_root_rejected "unknown .cargo directory" "$directory_fixture"

symlink_fixture=$(make_fixture symlinked-canonical-config)
mv "$symlink_fixture/.cargo/config.toml" "$symlink_fixture/config.toml.real"
ln -s ../config.toml.real "$symlink_fixture/.cargo/config.toml"
expect_root_rejected "symlinked canonical config" "$symlink_fixture"

ancestor_fixture=$(make_fixture ancestor-config-marker-spoof)
ancestor_runner=$(write_spoof_runner "$ancestor_fixture")
mkdir -p "$(dirname "$ancestor_fixture")/.cargo"
printf '[target.\x27cfg(unix)\x27]\nrunner = "%s"\n' "$ancestor_runner" > "$(dirname "$ancestor_fixture")/.cargo/config.toml"
corrupt_setup "$ancestor_fixture"
verify_root "$ancestor_fixture"
expect_environment_rejected "ancestor marker spoof plus corrupted learner contract" "$ancestor_fixture"

home_fixture=$(make_fixture home-config-shadow)
fake_home="$temp_root/fake-home"
mkdir -p "$fake_home/.cargo"
printf '[build]\nrustc-wrapper = "/usr/bin/true"\n' > "$fake_home/.cargo/config.toml"
if HOME="$fake_home" verify_execution_environment "$home_fixture" >/dev/null 2>&1; then
    echo "Cargo environment mutation unexpectedly passed: home config shadow" >&2
    exit 1
fi

cargo_home_fixture=$(make_fixture cargo-home-config-shadow)
fake_cargo_home="$temp_root/fake-cargo-home"
mkdir -p "$fake_cargo_home"
printf '[build]\nrustc-wrapper = "/usr/bin/true"\n' > "$fake_cargo_home/config.toml"
if CARGO_HOME="$fake_cargo_home" verify_execution_environment "$cargo_home_fixture" >/dev/null 2>&1; then
    echo "Cargo environment mutation unexpectedly passed: CARGO_HOME config shadow" >&2
    exit 1
fi

environment_fixture=$(make_fixture runner-environment-override)
if CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER=/usr/bin/true \
    verify_execution_environment "$environment_fixture" >/dev/null 2>&1; then
    echo "Cargo environment mutation unexpectedly passed: target runner override" >&2
    exit 1
fi

alias_environment_fixture=$(make_fixture alias-environment-external-subcommand)
mkdir -p "$alias_environment_fixture/type-exercise-starter/src"
printf 'starter sentinel: no copied tests\n' > "$alias_environment_fixture/type-exercise-starter/src/tests.rs"
external_subcommand="$alias_environment_fixture/cargo-environment-marker"
printf '%s\n' \
    '#!/usr/bin/env bash' \
    "touch '$alias_environment_fixture/external-subcommand-ran'" \
    "echo 'test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;'" \
    "echo 'verified the exact Chapters 1-12 course source, supplied tests, learner dependencies, README, SVG, SUMMARY, sitemaps, and CI'" \
    "echo 'executed the exact real reference inventory: 76/76 tests'" \
    > "$external_subcommand"
chmod +x "$external_subcommand"
if PATH="$alias_environment_fixture:$PATH" CARGO_ALIAS_X=environment-marker \
    verify_execution_environment "$alias_environment_fixture" >/dev/null 2>&1; then
    echo "Cargo environment mutation unexpectedly passed: alias external subcommand" >&2
    exit 1
fi
if [[ -e "$alias_environment_fixture/external-subcommand-ran" ]]; then
    echo "rejected alias external subcommand must never execute" >&2
    exit 1
fi
grep -Fqx 'starter sentinel: no copied tests' "$alias_environment_fixture/type-exercise-starter/src/tests.rs"

echo "verified Cargo bootstrap integrity and 13/13 pre-Cargo mutation killers"

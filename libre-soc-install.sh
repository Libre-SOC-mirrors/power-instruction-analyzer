#!/bin/bash
set -e

function fail()
{
    echo "error: $@" >&2
    exit 1
}

cargo_version="$(cargo --version)" || fail "can't find cargo, install it from https://rustup.rs/"
[[ "$cargo_version" =~ ^'cargo 1.'([0-9]+)'.'[0-9]+' ' ]] || fail "can't parse cargo's version string"
(( "${BASH_REMATCH[1]}" >= 53 )) || fail 'your rust version is not recent enough, update your rust version using `rustup update`'
python3 -m pip install 'maturin>=0.11,<0.12'
scripts="$(python3 -m sysconfig | sed 's/^\tscripts = "\([^"]\+\)"$/\1/p; d')"
[[ -d "$scripts" ]] || fail "can't find python's \`scripts\` directory"
rm -f target/wheels/power_instruction_analyzer-*.whl
"$scripts"/maturin build --compatibility linux --cargo-extra-args=--features=python-extension -i python3 --release --no-sdist
python3 -m pip install target/wheels/power_instruction_analyzer-*.whl
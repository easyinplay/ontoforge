#!/usr/bin/env bash
set -e
export PATH="$HOME/.cargo/bin:/usr/local/bin:/usr/bin:/bin"
if ! command -v cargo >/dev/null 2>&1; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain 1.98.0 --profile minimal
fi
cargo --version
if [ ! -d "$HOME/ontoforge" ]; then
  git clone /mnt/d/DataSpellProjects/ontoforge "$HOME/ontoforge"
fi
cd "$HOME/ontoforge"
git checkout -q main && git log --oneline -1
cargo build -p ontoforge --example webhook_receiver 2>&1 | tail -5
echo "BUILD_DONE"

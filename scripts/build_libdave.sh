#!/usr/bin/env bash
# Build libdave + mlspp + the external-sender wrapper as static libraries
# by default it builds into the in-crate home `ug-dave/vendor/libdave/`
# set LIBDAVE_PREFIX explicitly only when linking a libdave built somewhere else
#   LIBDAVE_SRC   where to clone/build      (default <ug-dave>/vendor/libdave)
#   LIBDAVE_SSL   crypto backend manifest   (default openssl_3; also openssl_1.1, boringssl)
set -euo pipefail

CRATE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

LIBDAVE_COMMIT="52cd56dc550f447fb354b3a06c9e2d2e2a4309c6"
LIBDAVE_SRC="${LIBDAVE_SRC:-$CRATE_DIR/vendor/libdave}"
LIBDAVE_SSL="${LIBDAVE_SSL:-openssl_3}"
REPO="https://github.com/discord/libdave.git"

log() { printf '\033[36m[build_libdave]\033[0m %s\n' "$*" >&2; }

if [ ! -d "$LIBDAVE_SRC/.git" ]; then
  log "cloning libdave -> $LIBDAVE_SRC"
  git clone "$REPO" "$LIBDAVE_SRC"
fi
cd "$LIBDAVE_SRC"

if [ "$(git rev-parse HEAD 2>/dev/null)" != "$LIBDAVE_COMMIT" ]; then
  log "checking out pinned commit $LIBDAVE_COMMIT"
  git fetch --quiet origin "$LIBDAVE_COMMIT" || true
  git checkout --quiet "$LIBDAVE_COMMIT"
fi

# Celeste external-sender patch: adds ExternalSender::ProposeRemove (MLS member-remove on leave)
# and makes SplitCommitWelcome tolerate a Remove-only commit which carries no Welcome
# this is reapplied on every build
ES_PATCH="$CRATE_DIR/patches/external_sender_propose_remove.patch"
if [ -f "$ES_PATCH" ]; then
  if git apply --reverse --check "$ES_PATCH" 2>/dev/null; then
    log "external-sender ProposeRemove patch already applied"
  elif git apply --check "$ES_PATCH" 2>/dev/null; then
    log "applying external-sender ProposeRemove patch"
    git apply "$ES_PATCH"
  else
    log "[*] warning! external-sender ProposeRemove patch does not apply cleanly... (libdave bumped? regenerate patches/external_sender_propose_remove.patch)"
  fi
fi

log "init vcpkg submodule (full history manifest baselines need it)"
git submodule update --init cpp/vcpkg
if [ "$(git -C cpp/vcpkg rev-parse --is-shallow-repository)" = "true" ]; then
  log "un-shallowing vcpkg (may be slow)"
  git -C cpp/vcpkg fetch --unshallow
fi

PF="cpp/vcpkg-alts/${LIBDAVE_SSL}/overlay-ports/mlspp/portfile.cmake"
if [ -f "$PF" ] && ! grep -q 'vcpkg_replace_string.*-Werror' "$PF"; then
  log "patching mlspp portfile to strip -Werror (modern-toolchain workaround)"
  sed -i 's|^vcpkg_cmake_configure(|vcpkg_replace_string("${SOURCE_PATH}/CMakeLists.txt" " -Werror" "")\n\nvcpkg_cmake_configure(|' "$PF"
fi

export CMAKE_POLICY_VERSION_MINIMUM="${CMAKE_POLICY_VERSION_MINIMUM:-3.5}"

cd cpp
if [ -d build ] && \
   { [ ! -f build/CMakeCache.txt ] || \
     ! grep -q "^CMAKE_CACHEFILE_DIR:INTERNAL=$PWD/build\$" build/CMakeCache.txt; }; then
  log "build/ is unconfigured or anchored elsewhere; forcing a clean reconfigure (keeping vcpkg_installed)"
  rm -rf build/CMakeCache.txt build/CMakeFiles
  touch -d '2000-01-01' build
fi

log "building libdave + mlspp + deps (SSL=$LIBDAVE_SSL, Release, TESTING=ON for the external_sender wrapper)"
make dev SSL="$LIBDAVE_SSL" BUILD_TYPE=Release
cmake --build build --target external_sender --config Release

test -f build/libdave.a || { log "ERROR: build/libdave.a not produced"; exit 1; }
test -f build/test/capi/external_sender.a || { log "ERROR: external_sender.a not produced"; exit 1; }

log "OK artifacts under $LIBDAVE_SRC/cpp/build"
log "build.rs and start.sh default LIBDAVE_PREFIX to this path no export needed"
log "(override only for a non-default location:)"
echo "export LIBDAVE_PREFIX=$LIBDAVE_SRC/cpp"

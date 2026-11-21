#!/usr/bin/env bash
# Industrial Era with native mods (docs/native-mods.md): the mods' industrial-era-mixins bundles
# applied to Industrial Era's source, built as a release is (build_apk.py, the fast profile). Runs on
# the build VM and on GitHub's runners (.github/workflows/native-mods.yml).
#
#   native_build.sh SOURCE OUT BUNDLE...
#
# SOURCE is an Industrial Era checkout at the commit to build, only read (git archive). Each BUNDLE is a
# folder holding a industrial-era-mixin.toml. OUT gets libhorizon_android.so (stripped), the APK with the
# mods in, and build.json: the key, the version, the commit, the mods (each with the Android
# permissions its source shows it needs, native_mod_permissions.py), all those permissions, and
# the library's SHA-256.
#
# NATIVE_CACHE (default ~/.cache/industrial-era-native) keeps industrial-era-mixins and the build folder with
# its target between builds: a file a build leaves as it was keeps its time, so Cargo recompiles
# only the crates the mods change. PROFILE defaults to fast, a release's.
# ABI selects the Android architecture (default arm64-v8a); VARIANT can be --preview or --dev.
set -euo pipefail

MIXINS_GIT=https://github.com/industrial-era-rust/industrial-era-mixins
# Industrial Era's launcher's (launcher/src/native_mods.rs, MIXINS_REV).
MIXINS_REV=ddd9700b21997c95d95ac6ee5ecafe3830cf622a

fail() { echo "native_build: $*" >&2; exit 1; }
[ $# -ge 3 ] || fail "usage: native_build.sh SOURCE OUT BUNDLE..."
source=$(cd "$1" && pwd)
mkdir -p "$2"
out=$(cd "$2" && pwd)
shift 2
cache=${NATIVE_CACHE:-$HOME/.cache/industrial-era-native}
profile=${PROFILE:-fast}
abi=${ABI:-arm64-v8a}
case "$abi" in
  arm64-v8a|armeabi-v7a|x86|x86_64) ;;
  *) fail "unsupported Android ABI: $abi" ;;
esac
variant=${VARIANT:-}
case "$variant" in
  ''|--preview|--dev) ;;
  *) fail "unsupported APK variant: $variant" ;;
esac
mkdir -p "$cache"

tools=$cache/mixins-${MIXINS_REV:0:12}
if [ ! -x "$tools/bin/industrial-era-mixins" ]; then
  cargo install --locked --git "$MIXINS_GIT" --rev "$MIXINS_REV" --root "$tools" industrial-era-mixins-cli
fi

commit=$(git -C "$source" rev-parse HEAD)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

# The source as committed, without links (industrial-era-mixins refuses them and nothing needs them).
mkdir "$work/source"
git -C "$source" archive --format=tar HEAD | tar -x -C "$work/source"
find "$work/source" -type l -delete
echo "$commit" > "$work/source/.industrial-era-revision"

# The game module is compiled by two crates (industrial-era-client's binary, and industrial-era-android from
# android_lib.rs), and industrial-era-mixins refuses a module destination it reaches under two names:
# while it plans, industrial-era-android's library is an empty file. The lockfile stays as it is
# (industrial-era-mixins reads the workspace --locked --offline).
manifest=crates/industrial-era-android/Cargo.toml
lib_line='path = "../industrial-era-client/src/android_lib.rs"'
grep -qxF "$lib_line" "$work/source/$manifest" || fail "$manifest no longer has: $lib_line"
sed -i "s|^path = \"../industrial-era-client/src/android_lib.rs\"\$|path = \"plan-stub.rs\"|" \
  "$work/source/$manifest"
: > "$work/source/crates/industrial-era-android/plan-stub.rs"
# industrial-era-mixins does not look under a file with a conditional attribute of its own, and the game
# module hangs from this one. The attribute does nothing on Android.
lib_attr='#![cfg_attr(not(target_os = "android"), forbid(unsafe_code))]'
grep -qxF "$lib_attr" "$work/source/crates/industrial-era-client/src/lib.rs" \
  || fail "crates/industrial-era-client/src/lib.rs no longer has: $lib_attr"
grep -vxF "$lib_attr" "$work/source/crates/industrial-era-client/src/lib.rs" > "$work/lib.rs"
cp "$work/lib.rs" "$work/source/crates/industrial-era-client/src/lib.rs"

# The bundles, each pinned to the commit built here when it pins one.
mkdir "$work/mods"
for bundle in "$@"; do
  [ -f "$bundle/industrial-era-mixin.toml" ] || fail "$bundle has no industrial-era-mixin.toml"
  copy=$work/mods/$(basename "$bundle")
  [ ! -e "$copy" ] || fail "two bundles named $(basename "$bundle")"
  cp -r "$bundle" "$copy"
  sed -i -E "s|^horizon_revision = \"[0-9a-f]*\"|horizon_revision = \"$commit\"|" \
    "$copy/industrial-era-mixin.toml"
done
python3 "$source/packaging/android/native_mod_adapt.py" "$work/source" "$work/mods"

# What went in names the build: the commit, the profile, industrial-era-mixins, every bundle file.
key=$(
  {
    echo "$commit $profile $abi $variant $MIXINS_REV"
    (cd "$work/mods" && find . -type f -print0 | sort -z | xargs -0 sha256sum)
  } | sha256sum | cut -c1-16
)
echo "native_build: build $key of $commit with $(ls "$work/mods" | tr '\n' ' ')"

cargo fetch --locked --manifest-path "$work/source/Cargo.toml" > /dev/null
"$tools/bin/industrial-era-mixins" apply --workspace "$work/source" --mods "$work/mods" \
  --out "$work/stage"

# Into the build folder: a file whose bytes changed gets the time of now (no -t), one that did not
# keeps its time, so Cargo recompiles what changed even when the commit is older than the last
# build. industrial-era-android's manifest goes back to building android_lib.rs.
build=$cache/build-$abi
mkdir -p "$build"
rsync -r --checksum --delete --exclude /target/ --exclude "/$manifest" \
  --exclude /crates/industrial-era-android/plan-stub.rs "$work/stage/" "$build/"
git -C "$source" show "HEAD:$manifest" > "$work/manifest"
cmp -s "$work/manifest" "$build/$manifest" || cp "$work/manifest" "$build/$manifest"

rm -f "$build"/target/android/industrial-era-*.apk
# NATIVE_LIBRARY_ONLY: the library is all the game downloads, and the APK takes time to make.
(cd "$build" && python3 packaging/android/build_apk.py --abi "$abi" --profile "$profile" $variant \
  ${NATIVE_LIBRARY_ONLY:+--library-only})

library=$build/target/android/libhorizon_android.so
cp "$library" "$out/libhorizon_android.so"
[ -n "${NATIVE_LIBRARY_ONLY:-}" ] || cp "$build"/target/android/industrial-era-*-"$abi".apk "$out/"
python3 - "$build" "$work/mods" "$out" "$key" "$commit" "$profile" "$MIXINS_REV" "$abi" <<'EOF'
import hashlib, json, pathlib, re, sys, tomllib
build, mods, out, key, commit, profile, mixins, abi = sys.argv[1:]
sys.path.insert(0, str(pathlib.Path(build) / "packaging/android"))
from native_mod_permissions import needs
version = re.search(r'^VERSION_NAME = "([^"]+)"',
                    (pathlib.Path(build) / "packaging/android/build_apk.py").read_text(),
                    re.M).group(1)
library = pathlib.Path(out) / "libhorizon_android.so"
listed = []
for folder in sorted(pathlib.Path(mods).iterdir()):
    mod = tomllib.loads((folder / "industrial-era-mixin.toml").read_text())["mod"]
    entry = {"id": mod["id"], "version": mod["version"]}
    # What the player is asked for when the build runs: the mod is not changed.
    wanted = needs(folder)
    if wanted:
        entry["permissions"] = wanted
        print(f"native_build: {mod['id']} needs {', '.join(wanted)}", file=sys.stderr)
    listed.append(entry)
json.dump({
    "key": key,
    "version": version,
    "commit": commit,
    "profile": profile,
    "abi": abi,
    "mixins": mixins,
    "mods": listed,
    "permissions": sorted({p for m in listed for p in m.get("permissions", [])}),
    "sha256": hashlib.sha256(library.read_bytes()).hexdigest(),
    "size": library.stat().st_size,
}, open(pathlib.Path(out) / "build.json", "w"), indent=2)
EOF
cat "$out/build.json"

#!/bin/bash
# Source from native fixture scripts; the App itself resolves the same exact package pin.
if [[ -z "${BASH_VERSION:-}" ]]; then
  echo 'Native math dependencies must be sourced from bash' >&2
  return 1
fi
prepare_native_math_dependencies() {
  local math_pin=fa8244ed032f4a1ade4cb0571bf87d2f1a9fd2d7
  local math_task_root
  math_task_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
  local math_source="$math_task_root/target/native-swiftmath-source"
  local existing="$math_task_root/target/ios-derived/SourcePackages/checkouts/SwiftMath"
  if [[ -d "$existing/.git" ]] && [[ "$(git -C "$existing" rev-parse HEAD)" == "$math_pin" ]]; then math_source="$existing"; fi
  if [[ ! -d "$math_source/.git" ]]; then
    git clone --quiet --no-checkout https://github.com/mgriebling/SwiftMath.git "$math_source"
    git -C "$math_source" checkout --quiet --detach "$math_pin"
  fi
  [[ "$(git -C "$math_source" rev-parse HEAD)" == "$math_pin" ]] || { echo 'SwiftMath source pin mismatch' >&2; return 1; }
  [[ -z "$(git -C "$math_source" status --porcelain)" ]] || { echo 'SwiftMath pinned source modified' >&2; return 1; }
  swift build --package-path "$math_source" --scratch-path "$math_task_root/target/macos-swiftmath" -c release --product SwiftMath >&2
  native_math_products="$(swift build --package-path "$math_source" --scratch-path "$math_task_root/target/macos-swiftmath" -c release --show-bin-path)"
  native_math_flags=(-I "$native_math_products")
  if [[ -f "$native_math_products/SwiftMath.o" ]]; then native_math_flags+=("$native_math_products/SwiftMath.o")
  else
    native_math_flags+=(-I "$native_math_products/Modules")
    local object
    while IFS= read -r object; do native_math_flags+=("$object"); done < <(find "$native_math_products/SwiftMath.build" -name '*.o' -type f | sort)
  fi
  [[ -d "$native_math_products/SwiftMath_SwiftMath.bundle" ]] || { echo 'SwiftMath resource bundle missing' >&2; return 1; }
}
install_native_math_resources() {
  python3 - "$native_math_products/SwiftMath_SwiftMath.bundle" "$1" <<'PY'
from pathlib import Path
import shutil,sys
source=Path(sys.argv[1]);parent=Path(sys.argv[2]);parent.mkdir(parents=True,exist_ok=True)
dest=parent/source.name
if dest.is_symlink():raise SystemExit('refuse symlink resource destination')
shutil.copytree(source,dest,dirs_exist_ok=True)
PY
}

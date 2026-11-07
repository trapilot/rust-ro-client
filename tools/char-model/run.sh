#!/bin/zsh
# End-to-end driver: layout -> (build_model + render | synthetic) -> pack -> verify.
# Usage: tools/char-model/run.sh [--synthetic] [--data DIR] [--sex m|f]
# Env:   BLENDER overrides the Blender binary, DATA overrides the data dir.
set -euo pipefail

BLENDER=${BLENDER:-/Applications/Blender.app/Contents/MacOS/Blender}
HERE=${0:A:h}
ROOT=${HERE:h:h}
DATA=${DATA:-$ROOT/data/extracted}
OUT=$HERE/out
SYNTHETIC=0
SEXES=(m f)

while (( $# )); do
  case $1 in
    --synthetic) SYNTHETIC=1 ;;
    --data) DATA=$2; shift ;;
    --sex) SEXES=($2); shift ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
  shift
done

mkdir -p "$OUT"
for sex in $SEXES; do
  python3 "$HERE/pack.py" layout --data "$DATA" --sex $sex --out "$OUT/layout_$sex.json"
  if (( SYNTHETIC )); then
    python3 "$HERE/pack.py" synthetic --sex $sex --out "$OUT/$sex"
  else
    "$BLENDER" -b --factory-startup -P "$HERE/build_model.py" -- --sex $sex --out "$OUT/model_$sex.blend" --styles "$HERE/styles.json"
    "$BLENDER" -b --factory-startup -P "$HERE/render.py" -- --sex $sex --blend "$OUT/model_$sex.blend" \
      --out "$OUT/$sex" --layout "$OUT/layout_$sex.json" --styles "$HERE/styles.json"
  fi
  python3 "$HERE/pack.py" preview --frames "$OUT/$sex"
  python3 "$HERE/pack.py" pack --data "$DATA" --frames "$OUT/$sex" --sex $sex
done
python3 "$HERE/pack.py" verify --data "$DATA"

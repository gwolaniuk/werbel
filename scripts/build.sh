#!/usr/bin/env bash
# Buduje, pakuje, instaluje (UAC) i pushuje wtyczkę Werbel.
# Użycie: ./scripts/build.sh "opis commita"
set -euo pipefail
cd "$(dirname "$0")/.."

# Toolchain gnu + przenośny MinGW (winlibs) — dla tej maszyny (brak MSVC).
export PATH="/c/Users/gwola/winlibs/mingw64/bin:$PATH"
export RUSTUP_TOOLCHAIN="stable-x86_64-pc-windows-gnu"

echo "▶ 1/4 kompilacja..."
cargo build --release

echo "▶ 2/4 pakowanie .vst3..."
BUNDLE="C:/Users/gwola/VST3/Werbel.vst3/Contents/x86_64-win"
rm -rf "C:/Users/gwola/VST3/Werbel.vst3"
mkdir -p "$BUNDLE"
cp target/release/werbel.dll "$BUNDLE/Werbel.vst3"

echo "▶ 3/4 instalacja (kliknij 'Tak' w oknie UAC)..."
RESULT="C:/Users/gwola/VST3/install_result.txt"
rm -f "$RESULT"
powershell -NoProfile -Command "Start-Process powershell -Verb RunAs -ArgumentList '-NoProfile','-ExecutionPolicy','Bypass','-File','C:/Users/gwola/git-repos/werbel/scripts/install.ps1' -Wait"
sleep 1
if grep -q "True" "$RESULT" 2>/dev/null; then
  echo "   ✓ zainstalowano do C:\\Program Files\\Common Files\\VST3\\Werbel.vst3"
else
  echo "   ✗ instalacja nieudana (anulowano UAC?)"
fi
rm -f "$RESULT"

echo "▶ 4/4 commit + push..."
git add -A
git commit -q -m "${1:-update}" || echo "   (brak zmian do commita)"
git push -q origin HEAD:main
echo "✓ gotowe"
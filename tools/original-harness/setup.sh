#!/usr/bin/env bash
# Builds a headless harness around the original Simple Bible Reader 5.6.2
# (needs mono-complete, p7zip-full, curl). Usage: setup.sh [dir]
#   then: mono <dir>/harness.exe x <in> <inIdx|0> <out> <outIdx>
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
dir="${1:-$HOME/sbr-original}"
url="https://github.com/yesudas/simple-bible-reader/raw/main/installers/Simple_Bible_Reader_5_6_2.exe"
sha="29325a6155a90504b8ecced55ca898fae6097fb59d6315972dc6bc4b9b9808d2"
mkdir -p "$dir"
cd "$dir"
if [ ! -f installer.exe ]; then curl -fsSL -o installer.exe "$url"; fi
echo "$sha  installer.exe" | sha256sum -c -
7z x -y -bd installer.exe >/dev/null
# Mono has no System.Speech; the stub only lets the assembly load.
mcs -nologo -target:library -out:System.Speech.dll "$here/Speech.cs"
mcs -nologo -r:Simple_Bible_Reader.exe -out:harness.exe "$here/Harness.cs"
echo "harness ready: mono $dir/harness.exe x <in> <inIdx|0> <out> <outIdx>"

$ErrorActionPreference = "Stop"
$src = "C:\Users\gwola\VST3\Werbel.vst3\Contents\x86_64-win\Werbel.vst3"
$dst = "C:\Program Files\Common Files\VST3\Werbel.vst3\Contents\x86_64-win\Werbel.vst3"
try {
  New-Item -ItemType Directory -Force (Split-Path $dst) | Out-Null
  Copy-Item -Force $src $dst
  (Test-Path $dst) | Out-File "C:\Users\gwola\VST3\install_result.txt"
} catch {
  $_ | Out-String | Out-File "C:\Users\gwola\VST3\install_result.txt"
}
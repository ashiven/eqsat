$exe = "C:\Users\janni\OneDrive\Dokumente\Projects\C++\MimIR\build\install\bin\mim.exe"
$input = Join-Path $PSScriptRoot "..\lit\rise\tile2d.mim"

$p = Start-Process `
    -FilePath $exe `
    -ArgumentList $input, "--output-mim", "-" `
    -NoNewWindow `
    -PassThru

$peak = 0

while (-not $p.HasExited) {
    $p.Refresh()

    if ($p.PeakWorkingSet64 -gt $peak) {
        $peak = $p.PeakWorkingSet64
    }

    Start-Sleep -Milliseconds 1
}

"Peak memory: $([math]::Round($peak / 1MB, 2)) MiB"

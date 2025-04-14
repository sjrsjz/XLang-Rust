$compilerPath = "..\target\release\XLang-Rust.exe"
$stdlibPath = ".\stdlib"

# Check if compiler exists
if (-not (Test-Path $compilerPath)) {
    Write-Error "Compiler not found at $compilerPath. Make sure you have built the project."
    exit 1
}

# Get all .x files in the stdlib directory
$sourceFiles = Get-ChildItem -Path $stdlibPath -Filter *.x

if ($sourceFiles.Count -eq 0) {
    Write-Host "No .x files found in $stdlibPath."
    exit 0
}

Write-Host "Found $($sourceFiles.Count) .x files in $stdlibPath. Starting compilation..."

foreach ($file in $sourceFiles) {
    $filePathRelative = Join-Path -Path $stdlibPath -ChildPath $file.Name
    Write-Host "Compiling $filePathRelative..."
    # Construct and execute the compile command
    & $compilerPath compile -b $filePathRelative
    # Check if the last command was successful
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Failed to compile $filePathRelative. Exit code: $LASTEXITCODE"
        # Optional: uncomment the next line to stop the script on first error
        # exit 1
    }
}

Write-Host "Finished compiling stdlib files."
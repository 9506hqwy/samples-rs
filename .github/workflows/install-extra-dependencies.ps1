Set-StrictMode -Version 'Latest'
$ErrorActionPreference = 'Stop'

$VERSION = "35.0"
$FILE_NAME = "protoc-${VERSION}-win64.zip"
curl.exe -fsSLO --output-dir "${env:TEMP}" "https://github.com/protocolbuffers/protobuf/releases/download/v${VERSION}/${FILE_NAME}"

$PROTOC_FILES = Expand-Archive -Path "${env:TEMP}\${FILE_NAME}" -DestinationPath "${env:TEMP}\protoc" -PassThru
$PROTOC_DIR = ($PROTOC_FILES |? {$_ -is [System.IO.DirectoryInfo] } | Select-Object -First 1).Parent

Write-Output "PROTOC_HOME=${PROTOC_DIR}" >> "${env:GITHUB_ENV}"
Write-Output "${PROTOC_DIR}\bin" >> "${env:GITHUB_PATH}"

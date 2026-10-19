#!/bin/bash
# Generate CycloneDX format SBOM.
#
# Usage:
#   ./cargo-sbom.sh [-o <output-directory>]
set -euo pipefail

if ! command -v jq > /dev/null; then
    echo >&2 "Not found 'jq' command."
    exit 1
fi

if [[ ! -f Cargo.toml ]]; then
    echo >&2 "Not found 'Cargo.toml'."
    exit 1
fi

OUTPUT_DIR="."

CYCLONEDX_ARGS=()
while [[ $# -gt 0 ]]
do
    case "$1" in
        -o)
            shift
            OUTPUT_DIR="$1"
            ;;
        *)
            CYCLONEDX_ARGS+=("$1")
            ;;
    esac

    shift
done

COMMIT_HASH=$(git rev-parse HEAD)
COMMIT_ID=${COMMIT_HASH:0:12}

if [[ ! -d "${OUTPUT_DIR}" ]]; then
    echo >&2 "Not found '${OUTPUT_DIR}'."
    exit 1
fi

export CARGO_TERM_COLOR=never

TMP_SBOM_NAME=$(basename "$(mktemp -u)")
cargo cyclonedx -f json --override-filename "${TMP_SBOM_NAME}" "${CYCLONEDX_ARGS[@]}"

METADATA_ALL=$(cargo metadata --format-version 1 --no-deps)

find . -type f -name "${TMP_SBOM_NAME}.json" -print0 | while IFS= read -d "" -r TMP_SBOM_PATH;
do
    PKG_DIR=$(dirname "${TMP_SBOM_PATH}")
    MANIFEST_PATH=$(realpath "${PKG_DIR}/Cargo.toml")

    METADATA=$(jq -c ".packages | map(select(.manifest_path==\"${MANIFEST_PATH}\")) | first" <<< "${METADATA_ALL}")
    if [[ "${METADATA}" == "null" ]]; then
        find . -name "${TMP_SBOM_NAME}.json" -delete
        echo >&2 "Not found '${MANIFEST_PATH}' metadata."
        exit 1
    fi

    NAME=$(jq -r ".name" <<< "${METADATA}")
    DATE=$(date '+%Y%m%d%H%M')
    SBOM_NAME="${NAME}-${COMMIT_ID}-${DATE}.sbom.json"

    mv -f "${TMP_SBOM_PATH}" "${OUTPUT_DIR}/${SBOM_NAME}"
done

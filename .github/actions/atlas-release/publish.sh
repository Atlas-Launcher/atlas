#!/usr/bin/env bash
set -euo pipefail

required=(ATLAS_BASE_URL ATLAS_APP_DEPLOY_TOKEN PRODUCT VERSION CHANNEL ARTIFACT_PREFIX)
for name in "${required[@]}"; do
  if [[ -z "${!name:-}" ]]; then
    echo "Missing required env: ${name}" >&2
    exit 1
  fi
done

if ! command -v jq >/dev/null 2>&1; then
  echo "jq is required for atlas-release action." >&2
  exit 1
fi

case "${PRODUCT}" in
  launcher|cli|runner|runnerd) ;;
  *)
    echo "Unsupported product: ${PRODUCT}" >&2
    exit 1
    ;;
esac

case "${CHANNEL}" in
  stable|beta|dev) ;;
  *)
    echo "Unsupported channel: ${CHANNEL}" >&2
    exit 1
    ;;
esac

manifest_file=""
cleanup_manifest=0
if [[ -n "${FILES_MANIFEST:-}" ]]; then
  manifest_file="${FILES_MANIFEST}"
  if [[ ! -f "${manifest_file}" ]]; then
    echo "files_manifest does not exist: ${manifest_file}" >&2
    exit 1
  fi
elif [[ -n "${FILES_INPUT:-}" ]]; then
  manifest_file="$(mktemp)"
  cleanup_manifest=1
  printf "%s\n" "${FILES_INPUT}" > "${manifest_file}"
else
  echo "Either files_manifest or files must be provided." >&2
  exit 1
fi

entries_file="$(mktemp)"
cleanup() {
  rm -f "${entries_file}"
  if [[ ${cleanup_manifest} -eq 1 ]]; then
    rm -f "${manifest_file}"
  fi
}
trap cleanup EXIT

hash_file() {
  local path="$1"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "${path}" | awk '{print $1}'
    return
  fi
  shasum -a 256 "${path}" | awk '{print $1}'
}

file_count=0
while IFS= read -r raw_line || [[ -n "${raw_line}" ]]; do
  line="${raw_line%%$'\r'}"
  [[ -z "${line}" ]] && continue
  [[ "${line}" =~ ^[[:space:]]*# ]] && continue

  IFS='|' read -r path os arch kind filename <<< "${line}"

  path="$(echo "${path:-}" | xargs)"
  os="$(echo "${os:-}" | tr '[:upper:]' '[:lower:]' | xargs)"
  arch="$(echo "${arch:-}" | tr '[:upper:]' '[:lower:]' | xargs)"
  kind="$(echo "${kind:-}" | tr '[:upper:]' '[:lower:]' | xargs)"
  filename="$(echo "${filename:-}" | xargs)"

  if [[ -z "${path}" || -z "${os}" || -z "${arch}" || -z "${kind}" ]]; then
    echo "Invalid manifest row (expected path|os|arch|kind|filename?): ${line}" >&2
    exit 1
  fi

  if [[ ! -f "${path}" ]]; then
    echo "Artifact path not found: ${path}" >&2
    exit 1
  fi

  case "${os}" in
    windows|macos|linux) ;;
    *)
      echo "Unsupported os '${os}' in row: ${line}" >&2
      exit 1
      ;;
  esac

  case "${arch}" in
    x64|arm64) ;;
    *)
      echo "Unsupported arch '${arch}' in row: ${line}" >&2
      exit 1
      ;;
  esac

  case "${kind}" in
    installer|binary|signature|updater-manifest|other) ;;
    *)
      echo "Unsupported kind '${kind}' in row: ${line}" >&2
      exit 1
      ;;
  esac

  if [[ -z "${filename}" ]]; then
    filename="$(basename "${path}")"
  fi

  size="$(wc -c < "${path}" | tr -d '[:space:]')"
  sha256="$(hash_file "${path}")"

  key="${ARTIFACT_PREFIX}/${PRODUCT}/${VERSION}/${os}/${arch}/${filename}"

  presign_payload="$(jq -n --arg action "upload" --arg key "${key}" '{action:$action,key:$key}')"
  presign_response="$(curl -fsS -X POST "${ATLAS_BASE_URL}/api/v1/storage/presign" \
    -H "x-atlas-app-deploy-token: ${ATLAS_APP_DEPLOY_TOKEN}" \
    -H "content-type: application/json" \
    --data "${presign_payload}")"

  upload_url="$(echo "${presign_response}" | jq -r '.url')"
  artifact_ref="$(echo "${presign_response}" | jq -r '.key')"
  mapfile -t upload_headers < <(
    echo "${presign_response}" | jq -r '.uploadHeaders // {} | to_entries[] | "\(.key): \(.value)"'
  )

  if [[ -z "${upload_url}" || "${upload_url}" == "null" || -z "${artifact_ref}" || "${artifact_ref}" == "null" ]]; then
    echo "Invalid presign response: ${presign_response}" >&2
    exit 1
  fi

  curl_args=(-fsS -X PUT --data-binary @"${path}")
  if [[ ${#upload_headers[@]} -gt 0 ]]; then
    for header in "${upload_headers[@]}"; do
      curl_args+=(-H "${header}")
    done
  else
    curl_args+=(-H "content-type: application/octet-stream")
  fi
  curl_args+=("${upload_url}")
  curl "${curl_args[@]}" >/dev/null

  jq -n \
    --arg os "${os}" \
    --arg arch "${arch}" \
    --arg key "${artifact_ref}" \
    --arg filename "${filename}" \
    --arg kind "${kind}" \
    --arg sha256 "${sha256}" \
    --argjson size "${size}" \
    '{os:$os,arch:$arch,artifact:{key:$key,filename:$filename,size:$size,sha256:$sha256,kind:$kind}}' >> "${entries_file}"

  file_count=$((file_count + 1))
  echo "Uploaded ${path} -> ${key}"
done < "${manifest_file}"

if [[ ${file_count} -eq 0 ]]; then
  echo "No manifest entries found." >&2
  exit 1
fi

mapfile -t platforms < <(jq -r -s 'map(.os + "|" + .arch) | unique[]' "${entries_file}")

for platform in "${platforms[@]}"; do
  os="${platform%%|*}"
  arch="${platform##*|}"

  artifacts_json="$(jq -c -s --arg os "${os}" --arg arch "${arch}" '[.[] | select(.os==$os and .arch==$arch) | .artifact]' "${entries_file}")"

  publish_payload="$(jq -n \
    --arg version "${VERSION}" \
    --arg channel "${CHANNEL}" \
    --arg os "${os}" \
    --arg arch "${arch}" \
    --arg notes "${NOTES:-}" \
    --arg published_at "${PUBLISHED_AT:-}" \
    --argjson artifacts "${artifacts_json}" \
    '({version:$version,channel:$channel,platform:{os:$os,arch:$arch},artifacts:$artifacts}
      + (if ($notes|length)>0 then {notes:$notes} else {} end)
      + (if ($published_at|length)>0 then {published_at:$published_at} else {} end))')"

  curl -fsS -X POST "${ATLAS_BASE_URL}/api/v1/releases/${PRODUCT}/publish" \
    -H "x-atlas-app-deploy-token: ${ATLAS_APP_DEPLOY_TOKEN}" \
    -H "content-type: application/json" \
    --data "${publish_payload}" >/dev/null

  echo "Published ${PRODUCT} ${VERSION} (${CHANNEL}) for ${os}/${arch}"
done

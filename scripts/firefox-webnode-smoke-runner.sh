#!/usr/bin/env bash
set -euo pipefail

GECKODRIVER="${GECKODRIVER:-/home/mtrapaglia/mina/mina-rust-runtime-repro-firefox/.tools/geckodriver/geckodriver}"
PORT="${PORT:-4444}"
BASE_URL="${BASE_URL:-http://127.0.0.1:${PORT}}"
HEADLESS="${HEADLESS:-1}"
REPEATS="${REPEATS:-1}"
URL="${URL:-http://127.0.0.1:8123/wasm-smoke/index.html?timeout_ms=30000}"
OUTPUT="${OUTPUT:-/home/mtrapaglia/mina/mina-rust-runtime-repro-firefox/logs/firefox-webnode-smoke.json}"
RUN_TIMEOUT_SEC="${RUN_TIMEOUT_SEC:-90}"
POLL_DURATION_SEC="${POLL_DURATION_SEC:-0}"
PROGRESS_LOG="${PROGRESS_LOG:-/home/mtrapaglia/mina/mina-rust-runtime-repro-firefox/logs/firefox-webnode-progress.log}"
CURL_TIMEOUT_SEC="${CURL_TIMEOUT_SEC:-30}"

if [[ ! -x "$GECKODRIVER" ]]; then
  echo "geckodriver not found at $GECKODRIVER" >&2
  exit 1
fi

mkdir -p "$(dirname "$OUTPUT")"

log_line() {
  local ts
  ts="$(date -Is)"
  printf '[%s] %s\n' "$ts" "$1" | tee -a "$PROGRESS_LOG" >/dev/null
}

curl_cmd() {
  curl -sS --max-time "${CURL_TIMEOUT_SEC}" "$@"
}

gecko_pid=""
cleanup() {
  if [[ -n "${gecko_pid}" ]]; then
    kill "${gecko_pid}" >/dev/null 2>&1 || true
    wait "${gecko_pid}" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

log_line "starting geckodriver on port ${PORT}"
"${GECKODRIVER}" --port "${PORT}" >"${OUTPUT}.geckodriver.log" 2>&1 &
gecko_pid=$!
sleep 0.5

create_session() {
  local args_json
  if [[ "${HEADLESS}" == "1" ]]; then
    args_json='["-headless"]'
  else
    args_json='[]'
  fi
  curl_cmd -X POST "${BASE_URL}/session" \
    -H 'Content-Type: application/json' \
    -d "{\"capabilities\":{\"alwaysMatch\":{\"browserName\":\"firefox\",\"moz:firefoxOptions\":{\"args\":${args_json}}}}}"
}

delete_session() {
  local session_id="$1"
  curl_cmd -X DELETE "${BASE_URL}/session/${session_id}" >/dev/null || true
}

set_url() {
  local session_id="$1"
  curl_cmd -X POST "${BASE_URL}/session/${session_id}/url" \
    -H 'Content-Type: application/json' \
    -d "{\"url\":\"${URL}\"}" >/dev/null
}

exec_sync() {
  local session_id="$1"
  local script="$2"
  curl_cmd -X POST "${BASE_URL}/session/${session_id}/execute/sync" \
    -H 'Content-Type: application/json' \
    -d "{\"script\":${script},\"args\":[]}"
}

get_result_text() {
  local session_id="$1"
  local response
  response="$(exec_sync "${session_id}" "$(jq -Rs . <<<"return document.getElementById('result')?.textContent || ''")")"
  echo "${response}" | jq -r '.value'
}

runs_file="$(mktemp)"
echo "[]" > "${runs_file}"

for run_idx in $(seq 1 "${REPEATS}"); do
  log_line "run ${run_idx}/${REPEATS} start (headless=${HEADLESS})"
  session_resp="$(create_session)"
  session_id="$(echo "${session_resp}" | jq -r '.value.sessionId // .sessionId')"
  if [[ -z "${session_id}" || "${session_id}" == "null" ]]; then
    log_line "run ${run_idx} failed: session create returned empty id"
    exit 1
  fi

  set_url "${session_id}"
  start_ts="$(date -Is)"
  deadline=$(( $(date +%s) + RUN_TIMEOUT_SEC ))
  poll_deadline=0
  poll_grace_sec=0
  result_text=""
  result_json="null"
  status="runner_timeout"

  while true; do
    result_text="$(get_result_text "${session_id}")"
    if echo "${result_text}" | jq -e . >/dev/null 2>&1; then
      result_json="$(echo "${result_text}" | jq -c '.')"
      state="$(echo "${result_json}" | jq -r '.run.state // empty')"
      init="$(echo "${result_json}" | jq -r '.init // empty')"
      if [[ "${POLL_DURATION_SEC}" -gt 0 ]]; then
        polling_complete="$(echo "${result_json}" | jq -r '.polling.complete // false')"
        if [[ "${state}" == "resolved" && "${poll_deadline}" -eq 0 ]]; then
          poll_interval_ms="$(echo "${result_json}" | jq -r '.polling.intervalMs // 0')"
          poll_interval_sec=$(( (poll_interval_ms + 999) / 1000 ))
          poll_grace_sec=$(( poll_interval_sec + 2 ))
          if [[ "${poll_grace_sec}" -lt 5 ]]; then
            poll_grace_sec=5
          fi
          poll_deadline=$(( $(date +%s) + POLL_DURATION_SEC + poll_grace_sec ))
        fi
        if [[ "${init}" == "failed" || "${state}" == "timeout" ]]; then
          status="${state:-${init}}"
          break
        fi
        if [[ "${state}" == "resolved" && "${polling_complete}" == "true" ]]; then
          status="resolved"
          break
        fi
        if [[ "${state}" == "resolved" && "${poll_deadline}" -gt 0 && $(date +%s) -ge "${poll_deadline}" ]]; then
          status="polling_incomplete"
          break
        fi
      else
        if [[ "${init}" == "failed" || "${state}" == "resolved" || "${state}" == "timeout" || "${state}" == "skipped" ]]; then
          status="${state:-${init}}"
          break
        fi
      fi
    fi
    if (( $(date +%s) >= deadline )); then
      status="runner_timeout"
      break
    fi
    sleep 1
  done

  end_ts="$(date -Is)"

  delete_session "${session_id}"
  log_line "run ${run_idx} done status=${status}"

  result_file="$(mktemp)"
  run_entry_file="$(mktemp)"
  echo "${result_json}" > "${result_file}"
  jq -n \
    --arg index "${run_idx}" \
    --arg start "${start_ts}" \
    --arg end "${end_ts}" \
    --arg status "${status}" \
    --slurpfile result "${result_file}" \
    '{index: ($index | tonumber), start: $start, end: $end, status: $status, result: ($result[0] // null)}' > "${run_entry_file}"
  jq -s '.[0] + [.[1]]' "${runs_file}" "${run_entry_file}" > "${runs_file}.tmp"
  rm -f "${result_file}" "${run_entry_file}"
  mv "${runs_file}.tmp" "${runs_file}"
done

summary="$(jq -n --slurpfile runs "${runs_file}" '
  ($runs[0] // []) as $r |
  {
    total: ($r | length),
    resolved: ($r | map(select(.status == "resolved")) | length),
    timeout: ($r | map(select(.status == "timeout")) | length),
    runner_timeout: ($r | map(select(.status == "runner_timeout")) | length),
    polling_incomplete: ($r | map(select(.status == "polling_incomplete")) | length),
    init_failed: ($r | map(select(.status == "failed")) | length)
  }'
)"

jq -n \
  --arg ts "$(date -Is)" \
  --arg url "${URL}" \
  --arg headless "${HEADLESS}" \
  --slurpfile runs "${runs_file}" \
  --argjson summary "${summary}" \
  '{generatedAt: $ts, url: $url, headless: ($headless == "1"), runs: ($runs[0] // []), summary: $summary}' > "${OUTPUT}"

log_line "all runs complete output=${OUTPUT}"

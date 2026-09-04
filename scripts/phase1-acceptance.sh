#!/usr/bin/env bash
set -euo pipefail

api_base="${API_BASE_URL:-http://localhost:3001}"
run_id="$(date +%s)-$$"
owner_email="owner-${run_id}@example.test"
teammate_email="teammate-${run_id}@example.test"

signup() {
  curl -fsS -X POST "$api_base/auth/signup" -H 'content-type: application/json' \
    -d "{\"email\":\"$1\",\"password\":\"acceptance-password\",\"name\":\"$2\"}"
}

owner_json="$(signup "$owner_email" Owner)"
teammate_json="$(signup "$teammate_email" Teammate)"
owner_token="$(jq -r .access_token <<<"$owner_json")"
owner_refresh="$(jq -r .refresh_token <<<"$owner_json")"
teammate_token="$(jq -r .access_token <<<"$teammate_json")"

org_json="$(curl -fsS -X POST "$api_base/organizations" -H "authorization: Bearer $owner_token" -H 'content-type: application/json' -d "{\"name\":\"Acceptance $run_id\",\"slug\":\"acceptance-$run_id\"}")"
org_id="$(jq -r .id <<<"$org_json")"
curl -fsS -X POST "$api_base/organizations/$org_id/members" -H "authorization: Bearer $owner_token" -H 'content-type: application/json' -d "{\"email\":\"$teammate_email\",\"role\":\"member\"}" >/dev/null
teammate_id="$(curl -fsS "$api_base/organizations/$org_id/members" -H "authorization: Bearer $owner_token" | jq -r ".[]|select(.email==\"$teammate_email\")|.user_id")"

project_json="$(curl -fsS -X POST "$api_base/projects" -H "authorization: Bearer $owner_token" -H 'content-type: application/json' -d "{\"organization_id\":\"$org_id\",\"name\":\"Acceptance\",\"key\":\"ACC\"}")"
project_id="$(jq -r .id <<<"$project_json")"
curl -fsS -X POST "$api_base/projects/$project_id/members" -H "authorization: Bearer $owner_token" -H 'content-type: application/json' -d "{\"email\":\"$teammate_email\",\"role\":\"reporter\"}" >/dev/null

issue_json="$(curl -fsS -X POST "$api_base/projects/$project_id/issues" -H "authorization: Bearer $teammate_token" -H 'content-type: application/json' -d "{\"title\":\"Acceptance issue\",\"description\":\"Phase 1\",\"assignee_id\":\"$teammate_id\"}")"
issue_id="$(jq -r .id <<<"$issue_json")"
curl -fsS -X PATCH "$api_base/issues/$issue_id" -H "authorization: Bearer $teammate_token" -H 'content-type: application/json' -d '{"status":"closed","priority":"high"}' | jq -e '.status=="closed" and .priority=="high"' >/dev/null

label_id="$(curl -fsS -X POST "$api_base/projects/$project_id/labels" -H "authorization: Bearer $owner_token" -H 'content-type: application/json' -d '{"name":"verified","color":"#22c55e"}' | jq -r .id)"
curl -fsS -X POST "$api_base/issues/$issue_id/labels" -H "authorization: Bearer $teammate_token" -H 'content-type: application/json' -d "{\"label_id\":\"$label_id\"}" >/dev/null
curl -fsS "$api_base/issues/$issue_id/labels" -H "authorization: Bearer $teammate_token" | jq -e '.[0].name=="verified"' >/dev/null
curl -fsS -X POST "$api_base/auth/refresh" -H 'content-type: application/json' -d "{\"refresh_token\":\"$owner_refresh\"}" | jq -e '.access_token and .refresh_token' >/dev/null

manage_status="$(curl -sS -o /dev/null -w '%{http_code}' -X DELETE "$api_base/projects/$project_id" -H "authorization: Bearer $teammate_token")"
test "$manage_status" = 403
printf 'Phase 1 acceptance passed (project %s, issue %s)\n' "$project_id" "$issue_id"

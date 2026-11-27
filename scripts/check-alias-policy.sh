#!/usr/bin/env bash
set -euo pipefail

# Static regression guard for the release/promotion alias policy documented in
# docs/release-process.md.
#
# The policy:
#   * a prerelease tag (anything with a `-` suffix, e.g. v0.5.1-rc.1) publishes
#     only the immutable `{{version}}` image tag — it must never move the
#     rolling `{{major}}.{{minor}}` tag or `latest`;
#   * every stable release also owns the rolling `X.Y` alias and `latest`;
#   * moving-alias promotions (`promote.yml`) and stable releases
#     (`release.yml`) serialize: overlapping runs must queue instead of
#     cancelling an in-flight alias push.
#
# This script asserts the *workflow source* encodes that policy. It is
# intentionally dependency-free (bash + awk/grep/sed) so it can run in any CI
# job and locally from the repository root:
#
#   scripts/check-alias-policy.sh
#
# Only *effective* YAML is inspected: full-line and trailing `#` comments are
# stripped first, so a comment cannot satisfy a check (decoy) or break one
# (a comment that merely mentions `cancel-in-progress: true`).
#
# Three complementary guards, all fail-closed:
#
#   1. `release.yml`'s `docker-release` tag input must match an allowlist: the
#      normalised set of tag lines is exactly the immutable `{{version}}` tag,
#      the gated `{{major}}.{{minor}}` alias and the gated raw `latest`. Any
#      extra line (unknown `type=raw`/`type=match` emitter, duplicate YAML
#      `tags:` key, repeated attribute such as two `enable=`) fails closed.
#   2. `promote.yml`'s `build-and-push` tag input is allowlisted too, which
#      keeps the backfill restriction (`main`/`nightly` disabled when
#      `allow_backfill=true`) visible and prevents extra release aliases.
#   3. Every workflow file is scanned for moving release aliases in `tags:` /
#      `flavor:` inputs (literal `latest`, literal `X.Y` / `X`, `{{major}}`
#      patterns, `type=match`, `flavor: latest=true`). Outside the two
#      allowlisted blocks, only `docker-build-push.yml` may use dynamic
#      (`${{ ... }}`) values; YAML anchors/aliases are always rejected. Every
#      other emitter — including `docker-build-push.yml`'s workflow_call
#      defaults — must stay immutable.
#
# Exit status: 0 when every check passes, 1 with a `FAIL:` line otherwise.
#
# Test harness: scripts/test-alias-policy.sh copies the workflows into a
# temporary directory, mutates them, and re-runs this script with
# ALIAS_POLICY_ROOT pointing at the fixture root.

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ROOT="${ALIAS_POLICY_ROOT:-$(cd -- "${SCRIPT_DIR}/.." && pwd)}"

WORKFLOW_DIR="${ROOT}/.github/workflows"
RELEASE_YML="${WORKFLOW_DIR}/release.yml"
PROMOTE_YML="${WORKFLOW_DIR}/promote.yml"
DOCKER_BUILD_PUSH_YML="${WORKFLOW_DIR}/docker-build-push.yml"

status=0

pass() { printf 'ok: %s\n' "$*"; }

fail() {
  printf 'FAIL: %s\n' "$*" >&2
  status=1
}

trim() {
  local s="$1"
  s="${s#"${s%%[![:space:]]*}"}"
  s="${s%"${s##*[![:space:]]}"}"
  printf '%s' "${s}"
}

# Characters removed when normalising a YAML scalar for comparison
# ("$", space, braces and both quote styles). Built via printf to keep the
# script free of quoting traps.
printf -v DROP_CHARS '%s' '$' ' ' '{' '}' '"' "'"

SQ="'"
# The only accepted prerelease gate, byte-for-byte:
#   ${{ !contains(github.ref_name, '-') }}
EXPECTED_GATE="\${{ !contains(github.ref_name, ${SQ}-${SQ}) }}"

# Unit separator for the `job<US>key<US>kind<US>value` records emitted by the
# workflow parsers below (tag lines never contain this byte).
US="$(printf '\037')"

for workflow in "${RELEASE_YML}" "${PROMOTE_YML}" "${DOCKER_BUILD_PUSH_YML}"; do
  if [[ ! -f "${workflow}" ]]; then
    fail "missing workflow: ${workflow}"
  fi
done
if [[ "${status}" -ne 0 ]]; then
  echo "Alias policy check failed." >&2
  exit 1
fi

# --- effective-YAML helpers -------------------------------------------------

# Strip YAML comments (stdin -> stdout). A `#` starts a comment at the start
# of a line or when preceded by whitespace, unless it is inside a quoted
# scalar. Quote state is tracked per line, which is sufficient for the
# workflow constructs checked here. (Inside block scalars this also drops
# `#`-prefixed lines; tag lists never contain valid comment syntax, so that is
# fail-closed and prevents decoy comment lines in a tag block.)
strip_comments() {
  awk '
    BEGIN { SQ = sprintf("%c", 39); DQ = "\"" }
    {
      line = $0; out = ""; quote = ""; n = length(line)
      for (i = 1; i <= n; i++) {
        c = substr(line, i, 1)
        if (quote != "") {
          if (c == quote) quote = ""
          out = out c
          continue
        }
        if (c == DQ || c == SQ) { quote = c; out = out c; continue }
        if (c == "#") {
          prev = (i == 1) ? " " : substr(line, i - 1, 1)
          if (prev ~ /[[:space:]]/) break
        }
        out = out c
      }
      print out
    }
  '
}

# Split one `key=value,key=value` tag line (or a flow mapping body) on commas
# that are not nested inside (), {}, or quotes. Values such as
# `enable=${{ !contains(github.ref_name, '-') }}` therefore stay intact.
split_tag_attrs() {
  local line="$1" i c depth=0 quote="" cur=""
  local -a attrs=()
  for ((i = 0; i < ${#line}; i++)); do
    c="${line:i:1}"
    if [[ -n "${quote}" ]]; then
      cur+="${c}"
      [[ "${c}" == "${quote}" ]] && quote=""
      continue
    fi
    case "${c}" in
      "'" | '"') quote="${c}"; cur+="${c}" ;;
      '(' | '{') depth=$((depth + 1)); cur+="${c}" ;;
      ')' | '}') depth=$((depth - 1)); cur+="${c}" ;;
      ',')
        if ((depth == 0)); then
          attrs+=("${cur}")
          cur=""
        else
          cur+="${c}"
        fi
        ;;
      *) cur+="${c}" ;;
    esac
  done
  attrs+=("${cur}")
  printf '%s\n' "${attrs[@]}"
}

# Normalise a tag line to attribute-order- and spacing-insensitive form:
#   type=raw, value=latest , enable=${{ ... }}  ->  enable=...,type=raw,value=latest
# Surrounding quotes around a value are removed. Values are otherwise kept
# byte-for-byte (the gate expression must match exactly).
normalize_tag_line() {
  local line="$1" attr key value
  local -a parts=()
  while IFS= read -r attr; do
    attr="$(trim "${attr}")"
    [[ -z "${attr}" ]] && continue
    if [[ "${attr}" == *=* ]]; then
      key="$(trim "${attr%%=*}")"
      value="$(trim "${attr#*=}")"
    else
      key="${attr}"
      value=""
    fi
    value="${value#\"}"; value="${value%\"}"
    value="${value#\'}"; value="${value%\'}"
    parts+=("${key}=${value}")
  done < <(split_tag_attrs "${line}")
  if ((${#parts[@]} == 0)); then
    return 0
  fi
  printf '%s\n' "${parts[@]}" | LC_ALL=C sort | paste -sd, -
}

# Print the repeated attribute key and return 1 when a tag line contains the
# same attribute key twice (docker/metadata-action parses `key=value` pairs in
# a plain loop with no duplicate check, so the later value wins at runtime).
duplicate_attr_key() {
  local line="$1" attr key
  local -A seen=()
  while IFS= read -r attr; do
    attr="$(trim "${attr}")"
    [[ -z "${attr}" ]] && continue
    if [[ "${attr}" == *=* ]]; then
      key="$(trim "${attr%%=*}")"
    else
      key="${attr}"
    fi
    if [[ -n "${seen["${key}"]+set}" ]]; then
      printf '%s' "${key}"
      return 1
    fi
    seen["${key}"]=1
  done < <(split_tag_attrs "${line}")
  return 0
}

# True when a metadata-action `flavor` value would produce/move a `latest`
# tag (`latest=true`, `suffix=-latest`, ...). `latest=false` is the explicit
# opt-out and is allowed.
flavor_is_moving() {
  local v="$1"
  v="${v//\"/}"; v="${v//\'/}"
  v="${v//[[:space:]]/}"
  v="$(printf '%s' "${v}" | tr '[:upper:]' '[:lower:]')"
  [[ "${v}" == *latest* && "${v}" != *latest=false* ]]
}

# True when a tags/flavor value uses YAML indirection (anchor `&`, alias `*`
# or explicit tag `!`). Such a value's effective content is not the literal
# text on the line, so it cannot be proven safe statically.
yaml_indirection() {
  local v="$1"
  v="$(trim "${v}")"
  [[ "${v}" == '&'* || "${v}" == '*'* || "${v}" == '!'* ]]
}

# True when a tags/flavor value relies on GitHub expression interpolation.
# Outside docker-build-push.yml's caller-supplied inputs (and the two
# allowlisted blocks) a dynamic value cannot be proven static.
has_expression() {
  [[ "$1" == *"\${{"* ]]
}

# True when one effective `tags:` line emits a moving release alias:
# literal `latest`, a literal major / major.minor value (`0`, `0.5`, `v0.5`),
# any `{{major}}` pattern, or a `type=match` source (which can synthesise
# arbitrary aliases such as `\d+\.\d+`).
line_is_moving_alias() {
  local line="$1" attr key value plain
  [[ "${line}" == *'{{major}}'* ]] && return 0
  # Plain Docker tag references (docker/build-push-action `tags:` input):
  #   ghcr.io/org/image:latest, image:0.5
  plain="${line##*:}"
  plain="$(trim "${plain}")"
  [[ "${plain}" == "latest" ]] && return 0
  [[ "${plain}" =~ ^v?[0-9]+(\.[0-9]+)?$ ]] && return 0
  while IFS= read -r attr; do
    attr="$(trim "${attr}")"
    [[ -z "${attr}" ]] && continue
    if [[ "${attr}" == *=* ]]; then
      key="$(trim "${attr%%=*}")"
      value="$(trim "${attr#*=}")"
    else
      key="${attr}"
      value=""
    fi
    value="${value#\"}"; value="${value%\"}"
    value="${value#\'}"; value="${value%\'}"
    case "${key}" in
      type)
        [[ "${value}" == "match" ]] && return 0
        ;;
      value)
        [[ "${value}" == "latest" ]] && return 0
        [[ "${value}" =~ ^v?[0-9]+(\.[0-9]+)?$ ]] && return 0
        ;;
      flavor)
        flavor_is_moving "${value}" && return 0
        ;;
    esac
  done < <(split_tag_attrs "${line}")
  return 1
}

# Emit one record per effective `tags:` / `flavor:` input, using the YAML key
# path rather than fixed indentation so reformatting a workflow cannot hide a
# moving-alias emitter:
#   <job> US <key> US begin:|   US ""
#   <job> US <key> US block:|   US <line>
#   <job> US <key> US inline    US <value>
# A `default:` scalar under `on.workflow_call.inputs.<tags|flavor>` is treated
# as that reusable input (job label `@workflow_call.inputs`). Block scalars for
# unrelated keys (`run: |`, `env: ... &anchor |`, ...) are skipped as content,
# so a line that merely looks like a tag cannot be mistaken for one.
extract_tag_inputs() {
  awk -v US="${US}" '
    function indent(s) { match(s, /^[ ]*/); return RLENGTH }
    function emit(j, k, kind, val) {
      printf "%s%s%s%s%s%s%s\n", j, US, k, US, kind, US, val
    }
    function block_style(rest,    i, c) {
      if (rest ~ /^[|>]/) return substr(rest, 1, 1)
      if (rest ~ /^[&!][^ ]*[ ]+[|>]/) {
        for (i = 1; i <= length(rest); i++) {
          c = substr(rest, i, 1)
          if (c == "|" || c == ">") return c
        }
      }
      return ""
    }
    function flow_after_prefix(s,    t) {
      t = s
      sub(/^[&!][^ ]*[ ]*/, "", t)
      if (t ~ /^{/) return "{"
      if (t ~ /^\[/ && t ~ /\{/) return "["
      return ""
    }
    BEGIN { depth = 0; in_scalar = 0; scalar_indent = 0; scalar_job = ""; scalar_key = ""; scalar_kind = "" }
    {
      line = $0
      if (line ~ /^[ ]*$/) next
      ind = indent(line)

      if (in_scalar) {
        if (ind > scalar_indent) {
          if (scalar_key != "") {
            content = line
            sub(/^[ ]+/, "", content)
            emit(scalar_job, scalar_key, scalar_kind, content)
          }
          next
        }
        in_scalar = 0
      }

      while (depth > 0 && stack_indent[depth] >= ind) depth--

      # A flow-mapping sequence item (`- { ... }`) also hides nested keys.
      seq_item = line
      sub(/^[ ]*-[ ]*/, "", seq_item)
      if (seq_item != line && flow_after_prefix(seq_item) != "") {
        flow_job = "@workflow"
        if (depth >= 1 && stack_key[1] == "jobs") flow_job = stack_key[2]
        emit(flow_job, "-", "flow-mapping", line)
        next
      }

      if (line !~ /^[ ]*["]?[A-Za-z0-9_.-]+["]?:/) next

      key = line
      sub(/^[ ]+/, "", key)
      sub(/:.*/, "", key)
      gsub(/"/, "", key)
      rest = line
      sub(/^[ ]*["]?[A-Za-z0-9_.-]+["]?:[ ]*/, "", rest)
      sub(/[ ]+$/, "", rest)

      # Flow-style mappings hide nested keys from the line-oriented scan
      # (e.g. `with: { tags: "type=raw,value=latest" }`). Fail closed; the
      # workflow-level `concurrency` flow map is handled separately.
      if (flow_after_prefix(rest) != "" && key != "concurrency") {
        flow_job = "@workflow"
        if (depth >= 1 && stack_key[1] == "jobs") flow_job = stack_key[2]
        emit(flow_job, key, "flow-mapping", rest)
        next
      }

      rel_job = ""
      rel_key = ""
      if (depth >= 2 && stack_key[1] == "jobs" && (key == "tags" || key == "flavor")) {
        rel_job = stack_key[2]
        rel_key = key
      }
      if (depth == 4 && stack_key[1] == "on" && stack_key[2] == "workflow_call" &&
          stack_key[3] == "inputs" && (stack_key[4] == "tags" || stack_key[4] == "flavor") &&
          key == "default") {
        rel_job = "@workflow_call.inputs"
        rel_key = stack_key[4]
      }

      style = block_style(rest)

      if (rel_key != "") {
        if (style != "") {
          emit(rel_job, rel_key, "begin:" style, "")
          in_scalar = 1
          scalar_indent = ind
          scalar_job = rel_job
          scalar_key = rel_key
          scalar_kind = "block:" style
        } else {
          emit(rel_job, rel_key, "inline", rest)
          in_scalar = 0
        }
      } else if (style != "") {
        in_scalar = 1
        scalar_indent = ind
        scalar_key = ""
        scalar_job = ""
        scalar_kind = ""
      }

      depth++
      stack_indent[depth] = ind
      stack_key[depth] = key
    }
  '
}

# --- release.yml: docker-release tag allowlist ------------------------------
#
# Expected shape of the `tags:` input passed to docker-build-push.yml:
#
#   type=semver,pattern={{version}}
#   type=semver,pattern={{major}}.{{minor}},enable=${{ !contains(github.ref_name, '-') }}
#   type=raw,value=latest,enable=${{ !contains(github.ref_name, '-') }}
#
# This is an allowlist, not a denylist: any additional tag line is a failure,
# even if it looks harmless. Attribute order, spacing around attributes and
# comments are tolerated; the prerelease gate itself must match exactly.

EXPECTED_VERSION_NORM="$(normalize_tag_line 'type=semver,pattern={{version}}')"
EXPECTED_MINOR_NORM="$(normalize_tag_line "type=semver,pattern={{major}}.{{minor}},enable=${EXPECTED_GATE}")"
EXPECTED_LATEST_NORM="$(normalize_tag_line "type=raw,value=latest,enable=${EXPECTED_GATE}")"

# promote.yml owns the development aliases. Its tag set is allowlisted too so
# a dynamic value elsewhere cannot hide a moving alias from the literal scan.
EXPECTED_PROMOTE_MAIN_NORM="$(normalize_tag_line "type=raw,value=main,enable=\${{ inputs.allow_backfill != true }}")"
EXPECTED_PROMOTE_NIGHTLY_NORM="$(normalize_tag_line "type=raw,value=nightly,enable=\${{ inputs.allow_backfill != true }}")"
EXPECTED_PROMOTE_SHA_NORM="$(normalize_tag_line "type=raw,value=nightly-\${{ needs.gate.outputs.sha }}")"

# shellcheck disable=SC2034  # consumed via nameref in check_tag_allowlist
declare -a EXPECTED_RELEASE_TAGS=(
  "${EXPECTED_VERSION_NORM}"
  "${EXPECTED_MINOR_NORM}"
  "${EXPECTED_LATEST_NORM}"
)
# shellcheck disable=SC2034  # consumed via nameref in check_tag_allowlist
declare -a EXPECTED_PROMOTE_TAGS=(
  "${EXPECTED_PROMOTE_MAIN_NORM}"
  "${EXPECTED_PROMOTE_NIGHTLY_NORM}"
  "${EXPECTED_PROMOTE_SHA_NORM}"
)

# --- repo-wide moving-alias scan -------------------------------------------

mapfile -t workflow_files < <(
  find "${WORKFLOW_DIR}" -maxdepth 1 -type f \( -name '*.yml' -o -name '*.yaml' \) -print | LC_ALL=C sort
)
if ((${#workflow_files[@]} == 0)); then
  fail "no workflow files found under ${WORKFLOW_DIR}"
fi

docker_tags_entries=0
docker_tags_style=""
declare -a docker_tag_lines=()
promote_tags_entries=0
promote_tags_style=""
declare -a promote_tag_lines=()
global_alias_failures=0

# Label a record for messages: reusable-workflow inputs have no job.
record_label() {
  local job="$1" key="$2" file_label="$3"
  if [[ "${job}" == "@workflow_call.inputs" ]]; then
    printf '%s: workflow_call input "%s"' "${file_label}" "${key}"
  else
    printf '%s: job "%s"' "${file_label}" "${job}"
  fi
}

for workflow_file in "${workflow_files[@]}"; do
  file_label="${workflow_file#"${ROOT}/"}"
  clean="$(strip_comments <"${workflow_file}")"

  while IFS="${US}" read -r job key kind value; do
    [[ -z "${job}" ]] && continue
    if [[ "${kind}" == "flow-mapping" ]]; then
      fail "${file_label}: job \"${job}\" uses a flow-style mapping for '${key}'; use block style so the alias policy can inspect it"
      ((global_alias_failures++)) || true
      continue
    fi
    if [[ "${key}" != "tags" && "${key}" != "flavor" ]]; then
      continue
    fi

    is_docker_release_tags=0
    if [[ "${workflow_file}" == "${RELEASE_YML}" && "${job}" == "docker-release" && "${key}" == "tags" ]]; then
      is_docker_release_tags=1
    fi
    is_promote_tags=0
    if [[ "${workflow_file}" == "${PROMOTE_YML}" && "${job}" == "build-and-push" && "${key}" == "tags" ]]; then
      is_promote_tags=1
    fi

    case "${kind}" in
      begin:*)
        if [[ "${kind}" == "begin:>" ]]; then
          fail "$(record_label "${job}" "${key}" "${file_label}") uses a folded (>) tags/flavor block; use a literal (|) block so every tag is explicit"
          ((global_alias_failures++)) || true
          continue
        fi
        if ((is_docker_release_tags)); then
          docker_tags_entries=$((docker_tags_entries + 1))
          docker_tags_style="|"
        elif ((is_promote_tags)); then
          promote_tags_entries=$((promote_tags_entries + 1))
          promote_tags_style="|"
        fi
        continue
        ;;
      inline)
        if yaml_indirection "${value}"; then
          if ((is_docker_release_tags)); then
            docker_tags_entries=$((docker_tags_entries + 1))
            docker_tags_style="inline"
          elif ((is_promote_tags)); then
            promote_tags_entries=$((promote_tags_entries + 1))
            promote_tags_style="inline"
          fi
          fail "$(record_label "${job}" "${key}" "${file_label}") uses YAML anchor/alias indirection: ${value}"
          ((global_alias_failures++)) || true
          continue
        fi
        if [[ "${key}" == "tags" ]]; then
          if ! dup="$(duplicate_attr_key "${value}")"; then
            fail "$(record_label "${job}" "${key}" "${file_label}") tag line repeats attribute '${dup}': ${value}"
            ((global_alias_failures++)) || true
          fi
          if ((is_docker_release_tags)); then
            docker_tags_entries=$((docker_tags_entries + 1))
            docker_tags_style="inline"
          elif ((is_promote_tags)); then
            promote_tags_entries=$((promote_tags_entries + 1))
            promote_tags_style="inline"
          elif has_expression "${value}" && [[ "${workflow_file}" != "${DOCKER_BUILD_PUSH_YML}" ]]; then
            fail "$(record_label "${job}" "${key}" "${file_label}") uses a dynamic tags value outside docker-build-push.yml: ${value}"
            ((global_alias_failures++)) || true
          elif line_is_moving_alias "${value}"; then
            fail "$(record_label "${job}" "${key}" "${file_label}") emits a moving release alias: ${value}"
            ((global_alias_failures++)) || true
          fi
        else
          if has_expression "${value}" && [[ "${workflow_file}" != "${DOCKER_BUILD_PUSH_YML}" ]]; then
            fail "$(record_label "${job}" "${key}" "${file_label}") uses a dynamic flavor value outside docker-build-push.yml: ${value}"
            ((global_alias_failures++)) || true
          elif flavor_is_moving "${value}"; then
            fail "$(record_label "${job}" "${key}" "${file_label}") sets a moving-alias metadata flavor: ${value}"
            ((global_alias_failures++)) || true
          fi
        fi
        continue
        ;;
      block:*)
        if ! dup="$(duplicate_attr_key "${value}")"; then
          fail "$(record_label "${job}" "${key}" "${file_label}") tag line repeats attribute '${dup}': ${value}"
          ((global_alias_failures++)) || true
        fi
        if ((is_docker_release_tags)); then
          docker_tag_lines+=("${value}")
        elif ((is_promote_tags)); then
          promote_tag_lines+=("${value}")
        elif has_expression "${value}" && [[ "${workflow_file}" != "${DOCKER_BUILD_PUSH_YML}" ]]; then
          fail "$(record_label "${job}" "${key}" "${file_label}") uses a dynamic tags/flavor value outside docker-build-push.yml: ${value}"
          ((global_alias_failures++)) || true
        elif [[ "${key}" == "tags" ]] && line_is_moving_alias "${value}"; then
          fail "$(record_label "${job}" "${key}" "${file_label}") emits a moving release alias: ${value}"
          ((global_alias_failures++)) || true
        elif [[ "${key}" == "flavor" ]] && flavor_is_moving "${value}"; then
          fail "$(record_label "${job}" "${key}" "${file_label}") sets a moving-alias metadata flavor: ${value}"
          ((global_alias_failures++)) || true
        fi
        continue
        ;;
      input-default:*)
        if [[ "${kind}" == "input-default:>" ]]; then
          fail "$(record_label "${job}" "${key}" "${file_label}") uses a folded (>) default; use a literal (|) default so every tag is explicit"
          ((global_alias_failures++)) || true
          continue
        fi
        if [[ "${kind}" == "input-default:inline" ]] && yaml_indirection "${value}"; then
          fail "$(record_label "${job}" "${key}" "${file_label}") uses YAML anchor/alias indirection: ${value}"
          ((global_alias_failures++)) || true
          continue
        fi
        if has_expression "${value}" && [[ "${workflow_file}" != "${DOCKER_BUILD_PUSH_YML}" ]]; then
          fail "$(record_label "${job}" "${key}" "${file_label}") uses a dynamic default outside docker-build-push.yml: ${value}"
          ((global_alias_failures++)) || true
          continue
        fi
        if [[ "${key}" == "tags" ]]; then
          if ! dup="$(duplicate_attr_key "${value}")"; then
            fail "$(record_label "${job}" "${key}" "${file_label}") tag line repeats attribute '${dup}': ${value}"
            ((global_alias_failures++)) || true
          fi
          if line_is_moving_alias "${value}"; then
            fail "$(record_label "${job}" "${key}" "${file_label}") emits a moving release alias: ${value}"
            ((global_alias_failures++)) || true
          fi
        else
          if flavor_is_moving "${value}"; then
            fail "$(record_label "${job}" "${key}" "${file_label}") sets a moving-alias metadata flavor: ${value}"
            ((global_alias_failures++)) || true
          fi
        fi
        continue
        ;;
      *)
        continue
        ;;
    esac
  done < <(printf '%s\n' "${clean}" | extract_tag_inputs)
done

if ((global_alias_failures == 0)); then
  pass "workflows: no moving release aliases outside the gated docker-release block"
fi

# Validate the allowlisted tag sets. Duplicate YAML `tags:` keys (two
# begin/inline records) are rejected, as is a missing block or a block whose
# lines are not exactly the known-good set.
check_tag_allowlist() {
  local label="$1" job="$2" entries="$3" style="$4" lines_name="$5" expected_name="$6"
  local -n lines_ref="${lines_name}"
  local -n expected_ref="${expected_name}"
  local line candidate expected i found
  local unexpected="" missing="" allowlist_ok=1
  local -a actual_norm=() actual_sorted=() expected_sorted=()

  if [[ "${entries}" -eq 0 ]]; then
    fail "${label}: could not find the ${job} job's tags: block"
    return
  fi
  if [[ "${entries}" -gt 1 ]]; then
    fail "${label}: ${job} must have exactly one tags: block (found ${entries}; duplicate YAML keys are rejected)"
    return
  fi
  if [[ "${style}" != "|" ]]; then
    fail "${label}: the ${job} tags input must be a literal (|) block so every tag line is explicit"
    return
  fi

  for line in "${lines_ref[@]}"; do
    [[ -z "$(trim "${line}")" ]] && continue
    actual_norm+=("$(normalize_tag_line "${line}")")
  done
  if ((${#actual_norm[@]} > 0)); then
    mapfile -t actual_sorted < <(printf '%s\n' "${actual_norm[@]}" | LC_ALL=C sort)
  fi
  mapfile -t expected_sorted < <(printf '%s\n' "${expected_ref[@]}" | LC_ALL=C sort)

  if [[ "${#actual_sorted[@]}" -ne "${#expected_sorted[@]}" ]]; then
    allowlist_ok=0
  else
    for i in "${!expected_sorted[@]}"; do
      if [[ "${actual_sorted[i]}" != "${expected_sorted[i]}" ]]; then
        allowlist_ok=0
        break
      fi
    done
  fi

  if ((allowlist_ok)); then
    pass "${label}: ${job} tags match the required alias allowlist"
    return
  fi

  for candidate in "${actual_sorted[@]}"; do
    found=0
    for expected in "${expected_sorted[@]}"; do
      if [[ "${candidate}" == "${expected}" ]]; then
        found=1
        break
      fi
    done
    if ((!found)); then
      unexpected="${candidate}"
      break
    fi
  done
  for expected in "${expected_sorted[@]}"; do
    found=0
    for candidate in "${actual_sorted[@]}"; do
      if [[ "${expected}" == "${candidate}" ]]; then
        found=1
        break
      fi
    done
    if ((!found)); then
      missing="${expected}"
      break
    fi
  done

  if [[ -n "${unexpected}" ]]; then
    fail "${label}: ${job} tags do not match the required alias allowlist (unexpected line: ${unexpected})"
  elif [[ -n "${missing}" ]]; then
    fail "${label}: ${job} tags do not match the required alias allowlist (missing line: ${missing})"
  else
    fail "${label}: ${job} tags do not match the required alias allowlist (duplicate or reordered lines)"
  fi
}

check_tag_allowlist "release.yml" "docker-release" \
  "${docker_tags_entries}" "${docker_tags_style}" docker_tag_lines EXPECTED_RELEASE_TAGS
check_tag_allowlist "promote.yml" "build-and-push" \
  "${promote_tags_entries}" "${promote_tags_style}" promote_tag_lines EXPECTED_PROMOTE_TAGS

# --- concurrency (promote.yml, release.yml) --------------------------------

# Parse a workflow-level `concurrency:` mapping (block or flow style) and
# validate the group and `cancel-in-progress: false`.
check_concurrency() {
  local file="$1" expected_group="$2" label="$3"
  local clean inline_line content part key value group="" cancel=""
  local has_cancel=0
  clean="$(strip_comments <"${file}")"

  inline_line="$(printf '%s\n' "${clean}" | grep -m1 -E '^concurrency:[[:space:]]*\{' || true)"

  if [[ -n "${inline_line}" ]]; then
    content="$(trim "${inline_line#*concurrency:}")"
    content="${content#\{}"
    content="${content%\}}"
    while IFS= read -r part; do
      part="$(trim "${part}")"
      [[ -z "${part}" ]] && continue
      key="$(trim "${part%%:*}")"
      value="$(trim "${part#*:}")"
      value="${value#\"}"; value="${value%\"}"
      value="${value#\'}"; value="${value%\'}"
      case "${key}" in
        group) group="${value}" ;;
        cancel-in-progress) cancel="${value}"; has_cancel=1 ;;
      esac
    done < <(split_tag_attrs "${content}")
  elif printf '%s\n' "${clean}" | grep -qE '^concurrency:[[:space:]]*$'; then
    while IFS= read -r part; do
      [[ -z "$(trim "${part}")" ]] && continue
      key="$(trim "${part%%:*}")"
      value="$(trim "${part#*:}")"
      value="${value#\"}"; value="${value%\"}"
      value="${value#\'}"; value="${value%\'}"
      case "${key}" in
        group) group="${value}" ;;
        cancel-in-progress) cancel="${value}"; has_cancel=1 ;;
      esac
    done < <(printf '%s\n' "${clean}" | awk '
      function indent(s) { match(s, /^[ ]*/); return RLENGTH }
      /^concurrency:[[:space:]]*$/ { inside = 1; next }
      inside {
        if ($0 ~ /^[[:space:]]*$/) next
        if (indent($0) == 0) { inside = 0; next }
        line = $0
        sub(/^[[:space:]]+/, "", line)
        print line
      }
    ')
  elif printf '%s\n' "${clean}" | grep -qE '^concurrency:[[:space:]]*[^[:space:]]'; then
    fail "${label}: concurrency must be a mapping (block or flow style) so cancel-in-progress: false is provable"
    return
  fi

  if [[ -z "${group}" ]]; then
    fail "${label}: missing workflow-level concurrency group (concurrent runs can race)"
  elif [[ -n "${expected_group}" && "${group}" != "${expected_group}" ]]; then
    fail "${label}: concurrency group must be '${expected_group}', got '${group}'"
  else
    pass "${label}: runs are serialized by concurrency group '${group}'"
  fi

  if ((has_cancel)) && [[ "${cancel}" == "false" ]]; then
    pass "${label}: cancel-in-progress is false (queued, never aborted)"
  else
    fail "${label}: concurrency must set cancel-in-progress: false so an in-flight alias push is never aborted"
  fi
}

# No effective `cancel-in-progress: true` anywhere (comments are ignored by the
# comment stripping above; `"true"` and `${{ true }}` are normalised).
check_no_cancel_true() {
  local file="$1" label="$2" clean raw value normalised found=0
  clean="$(strip_comments <"${file}")"
  while IFS= read -r raw; do
    [[ -z "${raw}" ]] && continue
    value="${raw#cancel-in-progress:}"
    normalised="$(printf '%s' "${value}" | tr -d "${DROP_CHARS}" | tr '[:upper:]' '[:lower:]')"
    if [[ "${normalised}" == "true" ]]; then
      found=1
    fi
  done < <(printf '%s\n' "${clean}" | grep -oE 'cancel-in-progress:[[:space:]]*[^,}]+' || true)
  if ((found)); then
    fail "${label}: cancel-in-progress: true is forbidden for alias pushes"
  fi
}

check_concurrency "${PROMOTE_YML}" "promote-moving-aliases" "promote.yml"
check_concurrency "${RELEASE_YML}" "release-moving-aliases" "release.yml"
check_no_cancel_true "${PROMOTE_YML}" "promote.yml"
check_no_cancel_true "${RELEASE_YML}" "release.yml"

if [[ "${status}" -ne 0 ]]; then
  echo "Alias policy check failed." >&2
  exit 1
fi

echo "Alias policy check passed."

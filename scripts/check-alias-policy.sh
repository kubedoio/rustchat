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
# Exit status: 0 when every check passes, 1 with a `FAIL:` line otherwise.
#
# Test harness: scripts/test-alias-policy.sh copies the workflows into a
# temporary directory, mutates them, and re-runs this script with
# ALIAS_POLICY_ROOT pointing at the fixture root.

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ROOT="${ALIAS_POLICY_ROOT:-$(cd -- "${SCRIPT_DIR}/.." && pwd)}"

RELEASE_YML="${ROOT}/.github/workflows/release.yml"
PROMOTE_YML="${ROOT}/.github/workflows/promote.yml"

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

for workflow in "${RELEASE_YML}" "${PROMOTE_YML}"; do
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

# Print the effective tag lines of the `docker-release` job's `tags:` block in
# release.yml (stdin: comment-stripped workflow). Anchored on the job so a tag
# line elsewhere in the file can neither satisfy nor break the check.
extract_tags_block() {
  awk '
    function indent(s) { match(s, /^[ ]*/); return RLENGTH }
    /^  docker-release:[[:space:]]*$/ { in_job = 1; next }
    in_job && /^  [A-Za-z0-9_.-]+:[[:space:]]*$/ { in_job = 0 }
    in_job && /^[^[:space:]]/ { in_job = 0 }
    in_job && !in_tags && /^[[:space:]]+tags:[[:space:]]*[|>]/ {
      in_tags = 1
      tags_indent = indent($0)
      next
    }
    in_tags {
      if ($0 ~ /^[[:space:]]*$/) next
      if (indent($0) <= tags_indent) { in_tags = 0; next }
      line = $0
      sub(/^[[:space:]]+/, "", line)
      print line
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

# Value of attribute <key> in <line>; non-zero when the attribute is absent.
# Order of attributes is irrelevant by design.
tag_attr() {
  local line="$1" want="$2" attr key value
  while IFS= read -r attr; do
    if [[ "${attr}" == *=* ]]; then
      key="$(trim "${attr%%=*}")"
      value="$(trim "${attr#*=}")"
    else
      key="$(trim "${attr}")"
      value=""
    fi
    if [[ "${key}" == "${want}" ]]; then
      printf '%s' "${value}"
      return 0
    fi
  done < <(split_tag_attrs "${line}")
  return 1
}

# --- release.yml: docker-release tag policy --------------------------------
#
# Expected shape of the `tags:` input passed to docker-build-push.yml:
#
#   type=semver,pattern={{version}}
#   type=semver,pattern={{major}}.{{minor}},enable=${{ !contains(github.ref_name, '-') }}
#   type=raw,value=latest,enable=${{ !contains(github.ref_name, '-') }}
#
# `enable=` is evaluated per tag by docker/metadata-action, so the allow/deny
# decision lives right next to each rolling tag. Attribute order is free.

tag_block="$(strip_comments <"${RELEASE_YML}" | extract_tags_block)"

if [[ -z "${tag_block}" ]]; then
  fail "release.yml: could not find the docker-release job's tags: block"
else
  version_count=0
  version_gated=0
  minor_count=0
  minor_bad=0
  latest_count=0
  latest_bad=0

  while IFS= read -r tag_line; do
    [[ -z "$(trim "${tag_line}")" ]] && continue
    tag_type="$(tag_attr "${tag_line}" type || true)"
    tag_pattern="$(tag_attr "${tag_line}" pattern || true)"

    if enable_value="$(tag_attr "${tag_line}" enable)"; then
      has_enable=1
    else
      has_enable=0
      enable_value=""
    fi

    # Every release publishes the immutable version tag, ungated (an
    # `enable=` gate here would suppress the RC image itself). Exactly one.
    if [[ "${tag_type}" == "semver" && "${tag_pattern}" == "{{version}}" ]]; then
      version_count=$((version_count + 1))
      if ((has_enable)); then
        version_gated=$((version_gated + 1))
      fi
    fi

    # Rolling `{{major}}.{{minor}}` alias: every such source must be gated
    # with exactly the accepted prerelease expression.
    if [[ "${tag_type}" == "semver" && "${tag_pattern}" == "{{major}}.{{minor}}" ]]; then
      minor_count=$((minor_count + 1))
      if ((has_enable)) && [[ "${enable_value}" == "${EXPECTED_GATE}" ]]; then
        :
      else
        minor_bad=$((minor_bad + 1))
      fi
    fi

    # Any tag source that would emit the literal `latest` must be the gated
    # raw latest. `flavor=` can also introduce/suffix `latest`.
    emits_latest=0
    while IFS= read -r attr; do
      [[ "${attr}" == *=* ]] || continue
      attr_key="$(trim "${attr%%=*}")"
      attr_value="$(trim "${attr#*=}")"
      case "${attr_key}" in
        value)
          [[ "${attr_value}" == "latest" ]] && emits_latest=1
          ;;
        flavor)
          [[ "${attr_value}" == *latest* ]] && emits_latest=1
          ;;
      esac
    done < <(split_tag_attrs "${tag_line}")

    if ((emits_latest)); then
      latest_count=$((latest_count + 1))
      if ((has_enable)) && [[ "${enable_value}" == "${EXPECTED_GATE}" ]]; then
        :
      else
        latest_bad=$((latest_bad + 1))
      fi
    fi
  done <<<"${tag_block}"

  if [[ "${version_count}" -ne 1 ]]; then
    fail "release.yml: expected exactly one 'type=semver,pattern={{version}}' tag, found ${version_count} (every release needs one immutable version tag)"
  elif [[ "${version_gated}" -ne 0 ]]; then
    fail "release.yml: the {{version}} tag must not be gated by enable= (prereleases must publish their exact version tag)"
  else
    pass "release.yml: exactly one ungated {{version}} tag"
  fi

  if [[ "${minor_count}" -eq 0 ]]; then
    fail "release.yml: missing a 'type=semver,pattern={{major}}.{{minor}}' tag"
  elif [[ "${minor_bad}" -ne 0 ]]; then
    fail "release.yml: every {{major}}.{{minor}} tag must be gated with enable=${EXPECTED_GATE} (found ${minor_bad} ungated/differently gated line(s))"
  else
    pass "release.yml: every {{major}}.{{minor}} tag is gated on a non-prerelease ref"
  fi

  if [[ "${latest_count}" -eq 0 ]]; then
    fail "release.yml: missing a 'type=raw,value=latest' tag"
  elif [[ "${latest_count}" -gt 1 ]]; then
    fail "release.yml: more than one tag source emits 'latest' (found ${latest_count}); only the gated raw latest may"
  elif [[ "${latest_bad}" -ne 0 ]]; then
    fail "release.yml: the 'latest' tag must be gated with enable=${EXPECTED_GATE} (no enable=true, no head_ref, no double negation)"
  else
    pass "release.yml: the only 'latest' source is the gated raw latest tag"
  fi
fi

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

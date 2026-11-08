#!/usr/bin/env bash
# Verifies that a frozen evidence archive matches its index, member by member.
#
# Usage:
#   bash ci/verify-evidence-archive.sh <archive.tar.gz> <index.json>
#   bash ci/verify-evidence-archive.sh --selftest
#
# The archive is unpacked into a fresh temporary directory; every member named by
# the index is re-hashed and re-measured there and compared against the index.
# Absolute paths, path traversal, non-regular members (symlink, hardlink,
# directory), duplicate members and members the index does not list are rejected.
#
# Exit codes:
#   0  every member matches the index
#   1  self-test failed
#   2  an input file is missing or unreadable, or a required tool is absent
#   3  a member has an absolute path or a `..` segment
#   4  a member is not a regular file
#   5  a member path appears more than once
#   6  the archive could not be listed or unpacked
#   7  the index is malformed, or `file_count` disagrees with the `files` array
#   8  the index's own `archive.sha256` / `archive.size` disagree with the file
#   9  a member listed by the index is missing from the archive
#  10  a member's recomputed sha256 disagrees with the index
#  11  a member's recomputed byte size disagrees with the index
#  12  the archive holds a member the index does not list
#  13  the archive's member count disagrees with the index's `file_count`
#
# This script is not referenced by any workflow and is not part of a release
# package: it only turns the "index matches the archive" judgement into a
# machine-checkable one.

set -uo pipefail

PROG='verify-evidence-archive.sh'

err() { printf 'ERROR: %s\n' "$*" >&2; }

fail() {
  local code="$1"
  shift
  err "$*"
  exit "$code"
}

usage() {
  printf 'usage: bash ci/%s <archive.tar.gz> <index.json>\n' "$PROG" >&2
  printf '       bash ci/%s --selftest\n' "$PROG" >&2
}

require_tools() {
  command -v jq >/dev/null 2>&1 || fail 2 'jq is required but was not found on PATH'
  command -v sha256sum >/dev/null 2>&1 || fail 2 'sha256sum is required but was not found on PATH'
  command -v tar >/dev/null 2>&1 || fail 2 'tar is required but was not found on PATH'
  local major="${BASH_VERSINFO[0]:-0}"
  [ "$major" -ge 4 ] || fail 2 "bash 4 or newer is required, found ${BASH_VERSINFO[*]}"
}

sha_of() { sha256sum "$1" | awk '{print $1}'; }

size_of() { wc -c < "$1" | tr -d ' \t'; }

# jq_text <filter> <file> prints a raw jq result with the trailing carriage return
# removed: some jq builds terminate lines with CRLF, which would otherwise leak
# into the compared values.
jq_text() {
  jq -r "$1" "$2" | tr -d '\r'
}

# index_member_rows <index> emits one TSV row per member: path, size, sha256.
index_member_rows() {
  jq -r '.files[] | [.path, (.size | tostring), (.sha256 // "")] | @tsv' "$1" | tr -d '\r'
}

verify_archive() {
  local archive="$1"
  local index="$2"

  [ -f "$archive" ] && [ -r "$archive" ] || fail 2 "the archive file is missing or unreadable: $archive"
  [ -f "$index" ] && [ -r "$index" ] || fail 2 "the index file is missing or unreadable: $index"

  local members
  local listing
  members=$(tar -tzf "$archive" 2>/dev/null) || fail 6 "cannot list the members of the archive: $archive"
  listing=$(tar -tvzf "$archive" 2>/dev/null) || fail 6 "cannot list the archive members with their types: $archive"

  # 3 - absolute paths and path traversal are rejected before anything is unpacked.
  local member
  while IFS= read -r member; do
    [ -n "$member" ] || continue
    case "$member" in
      /* | [A-Za-z]:/* | [A-Za-z]:\\*) fail 3 "the archive holds a member with an absolute path: $member" ;;
    esac
    if printf '%s' "$member" | grep -qE '(^|/)\.\.(/|$)'; then
      fail 3 "the archive holds a member that escapes the archive root: $member"
    fi
  done <<< "$members"

  # 4 - only regular files may be archived.
  local line
  local kind
  while IFS= read -r line; do
    [ -n "$line" ] || continue
    kind="${line:0:1}"
    case "$kind" in
      -) ;;
      *) fail 4 "the archive holds a non-regular member: $line" ;;
    esac
  done <<< "$listing"

  # 5 - a member path may appear only once.
  local duplicate
  duplicate=$(printf '%s\n' "$members" | grep -v '^$' | sort | uniq -d | head -n 1)
  [ -z "$duplicate" ] || fail 5 "the archive holds a duplicate member: $duplicate"

  # 6 - unpack into a fresh temporary directory.
  local tmp
  tmp=$(mktemp -d) || fail 6 'cannot create a temporary directory'
  trap 'rm -rf -- "$tmp"' EXIT
  tar -xzf "$archive" -C "$tmp" || fail 6 "cannot unpack the archive into $tmp"

  # 7 - the index must be well formed and self consistent.
  local file_count
  local files_len
  file_count=$(jq_text '.file_count // empty' "$index") || fail 7 "the index is not valid JSON: $index"
  files_len=$(jq_text '(.files // []) | length' "$index") || fail 7 "the index is not valid JSON: $index"
  case "$file_count" in '' | *[!0-9]*) fail 7 "the index file_count is not an integer: $file_count" ;; esac
  case "$files_len" in '' | *[!0-9]*) fail 7 "the index files array has no usable length: $files_len" ;; esac
  [ "$file_count" -eq "$files_len" ] || fail 7 "the index declares file_count $file_count but lists $files_len members"

  # 8 - the index's own record of the archive must match the archive file.
  local want_sha
  local want_size
  local got_sha
  local got_size
  want_sha=$(jq_text '.archive.sha256 // empty' "$index")
  want_size=$(jq_text '(.archive.size // empty) | tostring' "$index")
  got_sha=$(sha_of "$archive")
  got_size=$(size_of "$archive")
  [ "$want_sha" = "$got_sha" ] || fail 8 "the index archive.sha256 does not match the archive file"
  [ "$want_size" = "$got_size" ] || fail 8 "the index archive.size does not match the archive file"

  # 9 - every member the index lists must exist in the archive.
  local path
  local member_size
  local member_sha
  while IFS=$'\t' read -r path member_size member_sha; do
    [ -n "$path" ] || continue
    printf '%s\n' "$members" | grep -Fxq -- "$path" || fail 9 "a member listed by the index is missing from the archive: $path"
  done < <(index_member_rows "$index")

  # 10 and 11 - recompute every member's digest and size in the unpacked copy.
  local member_file
  local recomputed_sha
  local recomputed_size
  while IFS=$'\t' read -r path member_size member_sha; do
    [ -n "$path" ] || continue
    member_file="$tmp/$path"
    [ -f "$member_file" ] || fail 9 "a member listed by the index is missing from the unpacked archive: $path"
    recomputed_sha=$(sha_of "$member_file")
    recomputed_size=$(size_of "$member_file")
    [ "$recomputed_sha" = "$member_sha" ] || fail 10 "the recomputed sha256 of a member does not match the index: $path"
    [ "$recomputed_size" = "$member_size" ] || fail 11 "the recomputed byte size of a member does not match the index: $path"
  done < <(index_member_rows "$index")

  # 12 - the archive must not hold a member the index does not list.
  while IFS= read -r member; do
    [ -n "$member" ] || continue
    jq -e --arg p "$member" 'any(.files[]?; .path == $p)' "$index" >/dev/null 2>&1 \
      || fail 12 "the archive holds a member that the index does not list: $member"
  done <<< "$members"

  # 13 - the member counts must agree.
  local actual
  actual=$(printf '%s\n' "$members" | grep -c -v '^$')
  [ "$actual" -eq "$file_count" ] || fail 13 "the archive holds $actual members but the index declares $file_count"

  printf 'OK: %s members verified against %s\n' "$actual" "$index"
  exit 0
}

# build_index <archive> <member-dir> <member>... writes a self consistent index.
build_index() {
  local archive="$1"
  local dir="$2"
  shift 2
  local entries=''
  local first=1
  local name
  local member_size
  local member_sha
  for name in "$@"; do
    member_size=$(size_of "$dir/$name")
    member_sha=$(sha_of "$dir/$name")
    if [ "$first" -eq 1 ]; then first=0; else entries+=","; fi
    entries+=$(jq -nc --arg p "$name" --arg s "$member_sha" --argjson z "$member_size" '{path:$p,size:$z,sha256:$s}' | tr -d '\r')
  done
  local archive_sha
  local archive_size
  local count
  archive_sha=$(sha_of "$archive")
  archive_size=$(size_of "$archive")
  count=$(printf '%s\n' "$@" | grep -c -v '^$')
  jq -n \
    --arg path "$(basename "$archive")" \
    --arg sha "$archive_sha" \
    --argjson size "$archive_size" \
    --argjson count "$count" \
    --argjson files "[$entries]" \
    '{schema_version:1,archive:{path:$path,size:$size,sha256:$sha},file_count:$count,files:$files}' | tr -d '\r'
}

# craft_symlink_tar <output.tar> <member-name> <link-target>
# Writes a tar whose single member is a symlink. The header is crafted byte by
# byte so the fixture does not depend on the host file system supporting
# symbolic links (Windows hosts usually do not).
craft_symlink_tar() {
  local out="$1"
  local name="$2"
  local target="$3"
  command -v od >/dev/null 2>&1 || fail 2 'od is required to craft the self-test fixture but was not found on PATH'
  local header
  header=$(mktemp) || fail 2 'cannot create a temporary file'
  {
    printf '%-100s' "$name"
    printf '%-8s' '0000644'
    printf '%-8s' '0000000'
    printf '%-8s' '0000000'
    printf '%-12s' '00000000000'
    printf '%-12s' '00000000000'
    printf '%-8s' '        '
    printf '2'
    printf '%-100s' "$target"
    printf '%-6s' 'ustar'
    printf '%-2s' '00'
    printf '%-32s' 'root'
    printf '%-32s' 'root'
    printf '%-8s' '0000000'
    printf '%-8s' '0000000'
    printf '%-155s' ''
    printf '%-12s' ''
  } > "$header"
  local sum
  sum=$(od -An -tu1 -v "$header" | awk '{for (i = 1; i <= NF; i++) s += $i} END {print s + 0}')
  printf '%06o\000 ' "$sum" | dd of="$header" bs=1 seek=148 conv=notrunc 2>/dev/null
  cat "$header" > "$out"
  # two zero blocks terminate the archive
  dd if=/dev/zero bs=1024 count=1 >> "$out" 2>/dev/null
  rm -f "$header"
}

# run_case <label> <expected-exit> <index> <archive>
run_case() {
  local label="$1"
  local expected="$2"
  local index="$3"
  local archive="$4"
  local output
  local rc
  output=$(bash "$0" "$archive" "$index" 2>&1)
  rc=$?
  if [ "$rc" -ne "$expected" ]; then
    printf 'SELFTEST FAILED: %s 期望退出码 %s 实际 %s\n' "$label" "$expected" "$rc" >&2
    printf '%s\n' "$output" >&2
    exit 1
  fi
}

selftest() {
  local work
  work=$(mktemp -d) || fail 2 'cannot create a temporary directory'
  trap 'rm -rf -- "$work"' EXIT

  local src="$work/src"
  mkdir -p "$src" || fail 2 'cannot create the self-test fixture directory'
  printf 'alpha\n' > "$src/one.txt"
  printf 'beta-beta-beta\n' > "$src/two.txt"

  local good_archive="$work/good.tar.gz"
  ( cd "$src" && tar -czf "$good_archive" one.txt two.txt ) || fail 2 'cannot build the self-test fixture'

  local good_index="$work/good-index.json"
  build_index "$good_archive" "$src" one.txt two.txt > "$good_index" || fail 2 'cannot build the self-test index'

  # (a) a member digest is tampered with.
  local tampered_sha
  tampered_sha="$work/tampered-sha.json"
  jq '.files[0].sha256 = "0000000000000000000000000000000000000000000000000000000000000000"' \
    "$good_index" > "$tampered_sha" || fail 2 'cannot derive the tampered self-test index'
  run_case 'member sha256 tampered' 10 "$tampered_sha" "$good_archive"

  # (b) a member is dropped from the index and the count is kept self consistent.
  local dropped
  dropped="$work/dropped-member.json"
  jq 'del(.files[1]) | .file_count = 1' "$good_index" > "$dropped" || fail 2 'cannot derive the self-test index'
  run_case 'index member dropped' 12 "$dropped" "$good_archive"

  # (c) a member is replaced by a symlink.
  local symlink_archive="$work/symlink.tar.gz"
  craft_symlink_tar "$work/symlink.tar" link one.txt
  gzip -c "$work/symlink.tar" > "$symlink_archive"
  run_case 'symlink member' 4 "$good_index" "$symlink_archive"

  # (d) a member path appears twice.
  local duplicate_archive="$work/duplicate.tar.gz"
  ( cd "$src" && tar -czf "$duplicate_archive" --transform='s|^two|one|' one.txt two.txt ) \
    || fail 2 'cannot build the duplicate self-test fixture'
  run_case 'duplicate member' 5 "$good_index" "$duplicate_archive"

  printf 'SELFTEST OK: 4 tamper scenarios rejected\n'
  exit 0
}

require_tools

if [ "$#" -eq 1 ] && [ "$1" = '--selftest' ]; then
  selftest
fi

if [ "$#" -ne 2 ]; then
  usage
  exit 2
fi

verify_archive "$1" "$2"

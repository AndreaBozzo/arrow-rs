#!/usr/bin/env bash
# Build debug and release, run every case in its own process, record how it ended.
set -u
cd "$(dirname "$0")"
cargo build -q
cargo build -q --release
for profile in debug release; do
  for case in i32 i64 slice unaligned atomic atomic16 cas; do
    out=$(./target/$profile/unaligned-probe "$case" 2>&1)
    rc=$?
    case $rc in
      134) how="SIGABRT" ;;
      135) how="SIGBUS" ;;
      139) how="SIGSEGV" ;;
      0) how="ok" ;;
      *) how="rc=$rc" ;;
    esac
    line=$(printf '%s' "$out" | grep -vE '^(note|thread caused|stack backtrace|arch=)' | tail -1)
    printf '| %s | %s | %s | %s |\n' "$profile" "$case" "$how" "$line"
  done
done

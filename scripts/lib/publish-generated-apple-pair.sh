#!/usr/bin/env bash

# Transactionally replace generated Swift source and its ABI-coupled
# XCFramework. The two paths cannot be renamed as one filesystem object, so
# generators are serialized, failures before commit roll both outputs back,
# and failures after commit keep both new outputs while removing backup residue.
publish_generated_apple_pair() {
  set -euo pipefail
  if [[ "$#" != "4" ]]; then
    printf 'publish-generated-apple-pair: expected source-swift destination-swift source-framework destination-framework\n' >&2
    return 64
  fi

  source_swift="$1"
  destination_swift="$2"
  source_framework="$3"
  destination_framework="$4"
  [[ -f "$source_swift" ]] || return 66
  [[ -d "$source_framework" ]] || return 66

  swift_parent=""
  framework_parent=""
  # The transaction nonce names residue independently from the PID used only
  # for liveness. PID reuse is disambiguated by the recorded process start.
  token="$$-${RANDOM:-0}-${RANDOM:-0}"
  swift_parent="$(dirname "$destination_swift")"
  framework_parent="$(dirname "$destination_framework")"
  mkdir -p "$swift_parent" "$framework_parent"

  staged_swift="$swift_parent/.$(basename "$destination_swift").new.$token"
  staged_framework="$framework_parent/.$(basename "$destination_framework").new.$token"
  backup_swift="$swift_parent/.$(basename "$destination_swift").previous.$token"
  backup_framework="$framework_parent/.$(basename "$destination_framework").previous.$token"
  publication_lock="$framework_parent/.$(basename "$destination_framework").publication.lock"
  publication_lock_owner="$publication_lock/owner"
  publication_lock_owner_start="$publication_lock/owner-start"
  publication_lock_token="$publication_lock/token"
  publication_lock_state="$publication_lock/state"
  publication_recovery_claim="$publication_lock/recovery-claim"
  publication_state="inactive"
  swift_install_intent=0
  framework_install_intent=0
  publication_lock_owned=0

  process_start_fingerprint() {
    ps -o lstart= -p "$1" 2>/dev/null | sed 's/^[[:space:]]*//; s/[[:space:]]*$//'
  }

  for residue in "$staged_swift" "$staged_framework" "$backup_swift" "$backup_framework"; do
    [[ ! -e "$residue" ]] || return 73
  done

  publication_test_failpoint() {
    if [[ "${SYNARA_APPLE_PUBLICATION_TEST_FAILPOINT:-}" == "$1" ]]; then
      return 97
    fi
  }

  publication_test_recovery_failpoint() {
    if [[ "${SYNARA_APPLE_PUBLICATION_TEST_RECOVERY_FAILPOINT:-}" == "$1" ]]; then
      return 96
    fi
  }

  publication_test_pausepoint() {
    [[ "${SYNARA_APPLE_PUBLICATION_TEST_HOLDPOINT:-}" == "$1" ]] || return 0
    [[ -n "${SYNARA_APPLE_PUBLICATION_TEST_READY_FILE:-}" ]] || return 98
    [[ -n "${SYNARA_APPLE_PUBLICATION_TEST_RELEASE_FILE:-}" ]] || return 98
    printf '%s\n' "${BASHPID:-$$}" >"$SYNARA_APPLE_PUBLICATION_TEST_READY_FILE"
    local attempts=0
    while [[ ! -e "$SYNARA_APPLE_PUBLICATION_TEST_RELEASE_FILE" ]]; do
      attempts=$((attempts + 1))
      [[ "$attempts" -le 500 ]] || return 98
      sleep 0.01
    done
  }

  write_publication_journal_state() {
    printf '%s\n' "$1" >"$publication_lock/state.next"
    mv "$publication_lock/state.next" "$publication_lock_state"
  }

  stale_publication_diagnostic() {
    printf 'publish-generated-apple-pair: stale publication lock at %s cannot be safely recovered (%s); inspect the destination pair and lock journal before removing it\n' \
      "$publication_lock" "$1" >&2
  }

  recover_stale_publication() {
    local stale_owner stale_token stale_state
    if [[ -d "$publication_recovery_claim" ]]; then
      local recovery_owner recovery_owner_start current_recovery_owner_start
      recovery_owner="$(tr -d '[:space:]' <"$publication_recovery_claim/owner" 2>/dev/null || true)"
      recovery_owner_start="$(cat "$publication_recovery_claim/owner-start" 2>/dev/null || true)"
      case "$recovery_owner" in
        ''|*[!0-9]*)
          stale_publication_diagnostic "recovery claim has no valid owner"
          return 74
          ;;
      esac
      if kill -0 "$recovery_owner" 2>/dev/null; then
        current_recovery_owner_start="$(process_start_fingerprint "$recovery_owner")"
        if [[ -z "$recovery_owner_start" || -z "$current_recovery_owner_start" ]]; then
          stale_publication_diagnostic "recovery owner liveness is ambiguous"
          return 74
        fi
        if [[ "$current_recovery_owner_start" == "$recovery_owner_start" ]]; then
          printf 'publish-generated-apple-pair: stale transaction recovery already active for %s (owner pid %s)\n' \
            "$destination_framework" "$recovery_owner" >&2
          return 75
        fi
      fi
      stale_publication_diagnostic "recovery owner $recovery_owner is no longer running"
      return 74
    fi
    local stale_owner_start current_stale_owner_start
    stale_owner="$(tr -d '[:space:]' <"$publication_lock_owner" 2>/dev/null || true)"
    stale_owner_start="$(cat "$publication_lock_owner_start" 2>/dev/null || true)"
    stale_token="$(tr -d '[:space:]' <"$publication_lock_token" 2>/dev/null || true)"
    stale_state="$(tr -d '[:space:]' <"$publication_lock_state" 2>/dev/null || true)"
    case "$stale_owner" in
      ''|*[!0-9]*)
        stale_publication_diagnostic "missing or invalid owner"
        return 74
        ;;
    esac
    if kill -0 "$stale_owner" 2>/dev/null; then
      current_stale_owner_start="$(process_start_fingerprint "$stale_owner")"
      if [[ -z "$stale_owner_start" || -z "$current_stale_owner_start" ]]; then
        stale_publication_diagnostic "publication owner liveness is ambiguous"
        return 74
      fi
      if [[ "$current_stale_owner_start" == "$stale_owner_start" ]]; then
        printf 'publish-generated-apple-pair: publication already active for %s (owner pid %s)\n' \
          "$destination_framework" "$stale_owner" >&2
        return 75
      fi
    fi
    case "$stale_token" in
      ''|*[!0-9-]*)
        stale_publication_diagnostic "missing or invalid transaction token"
        return 74
        ;;
    esac
    # Claim recovery without ever dropping the canonical destination lock.
    # Competing recoverers either observe this directory or lose its atomic
    # mkdir; a crash while recovery owns the claim fails closed for inspection.
    if ! mkdir "$publication_recovery_claim"; then
      return 75
    fi
    printf '%s\n' "$$" >"$publication_recovery_claim/owner"
    process_start_fingerprint "$$" >"$publication_recovery_claim/owner-start"
    publication_test_pausepoint during_stale_recovery
    publication_test_recovery_failpoint after_recovery_claim

    local stale_staged_swift stale_staged_framework stale_backup_swift stale_backup_framework
    stale_staged_swift="$swift_parent/.$(basename "$destination_swift").new.$stale_token"
    stale_staged_framework="$framework_parent/.$(basename "$destination_framework").new.$stale_token"
    stale_backup_swift="$swift_parent/.$(basename "$destination_swift").previous.$stale_token"
    stale_backup_framework="$framework_parent/.$(basename "$destination_framework").previous.$stale_token"

    case "$stale_state" in
      acquired|staging_swift|staging_framework)
        # The live destinations have not been touched yet.
        rm -f -- "$stale_staged_swift"
        rm -rf -- "$stale_staged_framework"
        ;;
      backing_up_swift)
        # The Swift backup move may or may not have completed.
        if [[ -e "$stale_backup_swift" ]]; then
          rm -f -- "$destination_swift"
          mv "$stale_backup_swift" "$destination_swift"
        fi
        rm -f -- "$stale_staged_swift"
        rm -rf -- "$stale_staged_framework"
        ;;
      backing_up_framework)
        # Swift was backed up; the framework backup move may be in flight.
        if [[ -e "$stale_backup_swift" ]]; then
          rm -f -- "$destination_swift"
          mv "$stale_backup_swift" "$destination_swift"
        fi
        if [[ -e "$stale_backup_framework" ]]; then
          rm -rf -- "$destination_framework"
          mv "$stale_backup_framework" "$destination_framework"
        fi
        rm -f -- "$stale_staged_swift"
        rm -rf -- "$stale_staged_framework"
        ;;
      installing_swift|installing_framework)
        # Both old outputs have been backed up. A new output may already occupy
        # either destination, so remove it before restoring the prior pair.
        rm -f -- "$destination_swift"
        rm -rf -- "$destination_framework"
        if [[ -e "$stale_backup_swift" ]]; then
          mv "$stale_backup_swift" "$destination_swift"
        fi
        if [[ -e "$stale_backup_framework" ]]; then
          mv "$stale_backup_framework" "$destination_framework"
        fi
        rm -f -- "$stale_staged_swift"
        rm -rf -- "$stale_staged_framework"
        ;;
      committed)
        # Commit is recorded only after both new destinations exist. If either
        # is absent, stop for inspection rather than guessing after a crash.
        if [[ ! -f "$destination_swift" || ! -d "$destination_framework" ]]; then
          stale_publication_diagnostic "committed journal has an incomplete destination pair"
          return 74
        fi
        rm -f -- "$stale_staged_swift" "$stale_backup_swift"
        rm -rf -- "$stale_staged_framework" "$stale_backup_framework"
        ;;
      *)
        stale_publication_diagnostic "unknown journal state: ${stale_state:-missing}"
        return 74
        ;;
    esac
    rm -f -- "$publication_lock_owner" "$publication_lock_owner_start" "$publication_lock_token" \
      "$publication_lock_state" "$publication_lock/state.next"
    printf '%s\n' "$$" >"$publication_lock_owner"
    process_start_fingerprint "$$" >"$publication_lock_owner_start"
    printf '%s\n' "$token" >"$publication_lock_token"
    write_publication_journal_state acquired
    publication_lock_owned=1
    rm -f -- "$publication_recovery_claim/owner" "$publication_recovery_claim/owner-start"
    rmdir -- "$publication_recovery_claim"
  }

  cleanup_publication() {
    local status=$?
    trap - EXIT INT TERM
    case "$publication_state" in
      publishing)
        if [[ "$swift_install_intent" == "1" ]]; then
          rm -f -- "$destination_swift"
        fi
        if [[ "$framework_install_intent" == "1" ]]; then
          rm -rf -- "$destination_framework"
        fi
        if [[ -e "$backup_swift" ]]; then
          rm -f -- "$destination_swift"
          mv "$backup_swift" "$destination_swift"
        fi
        if [[ -e "$backup_framework" ]]; then
          rm -rf -- "$destination_framework"
          mv "$backup_framework" "$destination_framework"
        fi
        rm -f -- "$staged_swift"
        rm -rf -- "$staged_framework"
        ;;
      committed)
        # The new pair is already coherent. Preserve it and remove only hidden
        # transaction residue if an interruption lands during final cleanup.
        rm -f -- "$staged_swift" "$backup_swift"
        rm -rf -- "$staged_framework" "$backup_framework"
        ;;
    esac
    if [[ "$publication_lock_owned" == "1" ]]; then
      rm -f -- "$publication_lock_owner" "$publication_lock_owner_start" "$publication_lock_token" \
        "$publication_lock_state" "$publication_lock/state.next"
      rm -f -- "$publication_recovery_claim/owner" "$publication_recovery_claim/owner-start"
      rmdir -- "$publication_recovery_claim" 2>/dev/null || true
      rmdir -- "$publication_lock" || true
    fi
    exit "$status"
  }
  trap cleanup_publication EXIT
  # Keep signal delivery outside the two-command lock acquisition window: an
  # interrupt cannot make this process remove a lock owned by another writer.
  trap '' INT TERM
  if ! mkdir "$publication_lock"; then
    # Keep this as a simple command: placing the function in `if`, `!`, `||`,
    # or `&&` disables errexit throughout its body in Bash and could otherwise
    # let a failed restore operation fall through into publication.
    recover_stale_publication
    if [[ "$publication_lock_owned" != "1" ]] && ! mkdir "$publication_lock"; then
      trap 'exit 130' INT
      trap 'exit 143' TERM
      return 75
    fi
  fi
  if [[ "$publication_lock_owned" != "1" ]]; then
    publication_lock_owned=1
    printf '%s\n' "$$" >"$publication_lock_owner"
    process_start_fingerprint "$$" >"$publication_lock_owner_start"
    printf '%s\n' "$token" >"$publication_lock_token"
    write_publication_journal_state acquired
  fi
  trap 'exit 130' INT
  trap 'exit 143' TERM

  publication_state="publishing"
  write_publication_journal_state staging_swift
  mv "$source_swift" "$staged_swift"
  publication_test_failpoint after_stage_swift
  publication_test_pausepoint after_stage_swift
  write_publication_journal_state staging_framework
  mv "$source_framework" "$staged_framework"
  publication_test_failpoint after_stage_framework
  publication_test_pausepoint after_stage_framework
  write_publication_journal_state backing_up_swift
  if [[ -e "$destination_swift" ]]; then
    mv "$destination_swift" "$backup_swift"
  fi
  publication_test_failpoint after_backup_swift
  write_publication_journal_state backing_up_framework
  if [[ -e "$destination_framework" ]]; then
    mv "$destination_framework" "$backup_framework"
  fi
  publication_test_failpoint after_backup_framework
  swift_install_intent=1
  write_publication_journal_state installing_swift
  mv "$staged_swift" "$destination_swift"
  publication_test_failpoint after_install_swift
  publication_test_pausepoint after_install_swift
  framework_install_intent=1
  write_publication_journal_state installing_framework
  mv "$staged_framework" "$destination_framework"
  publication_test_failpoint after_install_framework
  publication_test_pausepoint after_install_framework

  publication_state="committed"
  write_publication_journal_state committed
  publication_test_failpoint after_commit
  publication_test_pausepoint after_commit
  rm -f -- "$backup_swift"
  rm -rf -- "$backup_framework"
  publication_state="inactive"
}

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
  publish_generated_apple_pair "$@"
fi

//! Integration tests for MDBX-backed `MptState` durability across
//! `MptState::open` → drop → `MptState::open` (Step 1.3b, Decision 9).
//!
//! Gated behind the `integration` feature per AGENTS.md Rule 5 because
//! the tests touch the real filesystem (MDBX env at a `TempDir` path).
//! Run via `make test-integration`; `make test` does NOT exercise them.

#![cfg(feature = "integration")]
#![allow(clippy::unwrap_used)]

use alloy_primitives::B256;
use krax_state::MptState;
use krax_types::State;
use pretty_assertions::assert_eq;
use tempfile::TempDir;

fn slot(n: u8) -> B256 {
    B256::from([n; 32])
}

#[test]
fn single_key_restart() {
    // Decision 9 (a): open at an explicit TempDir, set, commit, drop the
    // MptState, reopen at the same path, assert get returns the committed
    // value. The TempDir is bound for the full test so it outlives both
    // MptState instances; it's dropped (and the directory deleted) at the
    // end of the test scope.
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().to_path_buf();

    {
        let mut state = MptState::open(&path).unwrap();
        state.set(slot(1), slot(42)).unwrap();
        state.commit().unwrap();
        // `state` drops at end of block — MDBX env closes.
    }

    let reopened = MptState::open(&path).unwrap();
    assert_eq!(reopened.get(slot(1)).unwrap(), slot(42));
}

#[test]
fn multi_write_restart() {
    // Decision 9 (b): same shape as single_key_restart but with two
    // distinct writes. Catches single-key serialization bugs that wouldn't
    // surface with one slot.
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().to_path_buf();

    {
        let mut state = MptState::open(&path).unwrap();
        state.set(slot(1), slot(10)).unwrap();
        state.set(slot(2), slot(20)).unwrap();
        state.commit().unwrap();
    }

    let reopened = MptState::open(&path).unwrap();
    assert_eq!(reopened.get(slot(1)).unwrap(), slot(10));
    assert_eq!(reopened.get(slot(2)).unwrap(), slot(20));
}

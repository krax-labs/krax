//! Integration tests for MDBX-MVCC snapshot isolation on `MptState` /
//! `MptSnapshot` (Step 1.4, Decisions 1 + 5).
//!
//! Gated behind the `integration` feature per AGENTS.md Rule 5 because the
//! tests touch the real MDBX backend (the isolation guarantee is MDBX/MVCC-
//! provided; an in-memory mock would not exercise it). Run via
//! `make test-integration`; `make test` does NOT exercise them.
//!
//! Sequential only (Decision 6) — `Send + Sync` is a compile-time guarantee;
//! threading I/O-bound MDBX reads adds no signal at this stage.

#![cfg(feature = "integration")]
#![allow(clippy::unwrap_used)]

use alloy_primitives::B256;
use krax_state::MptState;
use krax_types::{Snapshot, State};
use pretty_assertions::assert_eq;

fn slot(n: u8) -> B256 {
    B256::from([n; 32])
}

#[test]
fn write_after_snapshot_does_not_bleed_in() {
    // Decision 5 case 1 (the ARCHITECTURE.md case): snapshot taken at v1,
    // sibling write to v2, snapshot still observes v1.
    let (mut state, _tmp) = MptState::open_temporary().unwrap();
    state.set(slot(1), slot(0xAA)).unwrap();

    let snap = state.snapshot().unwrap();

    state.set(slot(1), slot(0xBB)).unwrap();

    assert_eq!(snap.get(slot(1)).unwrap(), slot(0xAA));
    snap.release();
}

#[test]
fn commit_after_snapshot_does_not_bleed_in() {
    // Decision 5 case 2: snapshot taken at v1, sibling write+commit, snapshot
    // still observes v1. Distinct from case 1 because `MptState::set` already
    // auto-flushes per call (1.3b Decision 4) — this asserts MDBX MVCC isolation
    // at the txn-commit boundary, not just at the per-call buffering boundary.
    let (mut state, _tmp) = MptState::open_temporary().unwrap();
    state.set(slot(2), slot(0x11)).unwrap();

    let snap = state.snapshot().unwrap();

    state.set(slot(2), slot(0x22)).unwrap();
    state.commit().unwrap();

    assert_eq!(snap.get(slot(2)).unwrap(), slot(0x11));
    snap.release();
}

#[test]
fn two_snapshot_independence() {
    // Decision 5 case 3: snapshot A at v1, sibling write+commit to v2,
    // snapshot B at v2. A still sees v1; B sees v2.
    let (mut state, _tmp) = MptState::open_temporary().unwrap();
    state.set(slot(3), slot(0x01)).unwrap();

    let snap_a = state.snapshot().unwrap();

    state.set(slot(3), slot(0x02)).unwrap();
    state.commit().unwrap();

    let snap_b = state.snapshot().unwrap();

    assert_eq!(snap_a.get(slot(3)).unwrap(), slot(0x01));
    assert_eq!(snap_b.get(slot(3)).unwrap(), slot(0x02));

    snap_a.release();
    snap_b.release();
}

#[test]
fn root_after_write_does_not_bleed_in() {
    // Step 1.5 D17 (a) case 1: snapshot taken at v1, sibling write to v2,
    // snapshot's root still reflects v1. Mirrors
    // `write_after_snapshot_does_not_bleed_in` but asserts on
    // `Snapshot::root` instead of `Snapshot::get`.
    let (mut state, _tmp) = MptState::open_temporary().unwrap();
    state.set(slot(1), slot(0xAA)).unwrap();

    let snap = state.snapshot().unwrap();
    let root_v1 = snap.root();

    state.set(slot(1), slot(0xBB)).unwrap();

    assert_eq!(snap.root(), root_v1);
    snap.release();
}

#[test]
fn root_after_commit_does_not_bleed_in() {
    // Step 1.5 D17 (a) case 2: snapshot taken at v1, sibling write+commit,
    // snapshot's root still reflects v1.
    let (mut state, _tmp) = MptState::open_temporary().unwrap();
    state.set(slot(2), slot(0x11)).unwrap();

    let snap = state.snapshot().unwrap();
    let root_v1 = snap.root();

    state.set(slot(2), slot(0x22)).unwrap();
    state.commit().unwrap();

    assert_eq!(snap.root(), root_v1);
    snap.release();
}

#[test]
fn two_snapshot_root_independence() {
    // Step 1.5 D17 (a) case 3: snapshot A at v1, sibling write+commit to
    // v2, snapshot B at v2. A's root != B's root; A's root is unchanged
    // after B is taken (the per-snapshot cache held).
    let (mut state, _tmp) = MptState::open_temporary().unwrap();
    state.set(slot(3), slot(0x01)).unwrap();

    let snap_a = state.snapshot().unwrap();
    let root_a = snap_a.root();

    state.set(slot(3), slot(0x02)).unwrap();
    state.commit().unwrap();

    let snap_b = state.snapshot().unwrap();
    let root_b = snap_b.root();

    assert_ne!(root_a, root_b);
    assert_eq!(snap_a.root(), root_a); // A's root is stable; the cache held.
    snap_a.release();
    snap_b.release();
}

//! Ethereum-compatible Merkle Patricia Trie root computation over the
//! Slots table — Step 1.5 trie internals (Decisions 5, 7, 9, 11, 14).
//!
//! Public surface is the single function [`compute_root`] (D14 (a)). The
//! internals — [`Node`], [`NodeRef`], [`Nibbles`] — are `pub(super)` so
//! `mpt/mod.rs` can call them without re-exporting at the crate root
//! (D13 (c)).
//!
//! ## Semantics (matches `eth_storageRoot` for a single storage trie)
//!
//! The trie is the **secure** storage trie: the path of each entry is
//! `keccak256(slot_key)` (NOT the raw slot), and the leaf value is
//! `RLP(slot_value)` with the value's leading zero bytes stripped (the
//! big-endian minimal-byte integer encoding). A slot whose value is
//! `B256::ZERO` is **absent** from the trie (Ethereum deletes zeroed
//! storage slots; reth's `StorageRoot` filters them) — so a set of only
//! zero-valued slots has the empty-trie root.
//!
//! Because the path is `keccak256(slot)`, entries arriving in raw-slot
//! order from the MDBX cursor are NOT in trie-path order. [`compute_root`]
//! therefore hashes every key and re-sorts via a `BTreeMap` (AGENTS.md
//! Rule 7 — `BTreeMap`, never `HashMap`, in commit-path code) before the
//! bottom-up build (D7 (b) sort-then-build).
//!
//! ## Algorithm (verified against go-ethereum, LVP-Q3)
//!
//! - Nibble path compact ("hex-prefix") encoding per the Yellow Paper
//!   Appendix D / go-ethereum `trie/encoding.go:hexToCompact`: flag byte
//!   `(is_leaf << 5) | (is_odd << 4)`, low nibble = first path nibble when
//!   odd, then nibble pairs packed `hi << 4 | lo`.
//! - Inline-vs-hash: a child whose own RLP encoding is **strictly less
//!   than 32 bytes** is embedded inline (its RLP spliced raw into the
//!   parent); `>= 32` bytes is referenced by `keccak256` as a 32-byte
//!   string (`0xa0 || hash`). go-ethereum `trie/hasher.go:68`:
//!   `if len(enc) < 32 && !force { /* embedded directly */ }`. The trie
//!   **root is always hashed** (geth's `force` flag), even when its RLP is
//!   `< 32` bytes; the empty trie short-circuits to [`EMPTY_ROOT`].
//! - Leaf/branch value element: go-ethereum `trie/node_enc.go`
//!   `leafNodeEncoder.encode` does `WriteBytes(Val)` where the storage
//!   trie's `Val` is already `RLP(minimal(value))` — so the on-wire leaf
//!   value is `RLP_string(RLP(minimal(value)))` (a deliberate double wrap,
//!   matching `eth_getProof` storage-proof leaves and alloy-trie's
//!   `HashBuilder`).
//!
//! Out of scope (Step 1.5 decisions doc): proof generation, per-account /
//! world-state trie, ZK-friendly hashes, trie pruning, sidecar nodes
//! table. The [`Node`] type is plain enough to admit proof generation
//! later without a rewrite (D5 (a) — we did NOT close that door), but
//! Step 1.5 does not ship it.

// Step 1.5 Commit 1 ships the trie internals unit-tested but NOT yet wired
// into `MptState::root` / `MptSnapshot::root` (that wiring is Commit 2).
// In the non-test lib build these items have no caller, so `dead_code`
// would fire under `make lint`'s `-D warnings`. Commit 2 removes this
// allow once the wiring makes them reachable (so it cannot mask real dead
// code afterwards). See docs/plans/step-1.5-plan.md Commit 2.
#![allow(dead_code)]

use std::collections::BTreeMap;

use alloy_primitives::{B256, keccak256};
// Per Context7 LVP-Q1 (2026-05-15, /alloy-rs/rlp 0.3.15): `Header { list,
// payload_length }.encode(&mut out)` writes a list/string header into a
// `BufMut`; `alloy_rlp::encode(&[u8]) -> Vec<u8>` RLP-string-encodes a byte
// slice (e.g. `encode(&b"\xAB\xBA"[..]) == [0x82,0xAB,0xBA]`,
// `encode("") == [0x80]`).
use alloy_rlp::{Header, encode as rlp_encode};

/// The keccak256 hash of `RLP("")` (= `keccak256([0x80])`) — the canonical
/// Ethereum empty-trie root.
///
/// Cross-checked at LVP-Q6 against go-ethereum `core/types/hashes.go:26`
/// (`EmptyRootHash`) and reproduced by alloy-trie 0.9.5's empty
/// `HashBuilder` root. Asserted equal to the value [`compute_root`] returns
/// for an empty entry stream by the test suite below (D11 (c)).
pub(super) const EMPTY_ROOT: B256 = B256::new([
    0x56, 0xe8, 0x1f, 0x17, 0x1b, 0xcc, 0x55, 0xa6, 0xff, 0x83, 0x45, 0xe6, 0x92, 0xc0, 0xf8, 0x6e,
    0x5b, 0x48, 0xe0, 0x1b, 0x99, 0x6c, 0xad, 0xc0, 0x01, 0x62, 0x2f, 0xb5, 0xe3, 0x63, 0xb4, 0x21,
]);

/// A nibble path with a leaf/extension terminator flag.
///
/// `is_leaf` selects the terminator bit of the compact encoding (a leaf's
/// path carries the Yellow-Paper terminator nibble; an extension's does
/// not). `nibbles` holds 4-bit values (`0..=15`), one per nibble.
#[derive(Debug, Clone)]
pub(super) struct Nibbles {
    nibbles: Vec<u8>,
    is_leaf: bool,
}

impl Nibbles {
    /// Compact ("hex-prefix") encoding (Yellow Paper Appendix D; LVP-Q3 /
    /// go-ethereum `trie/encoding.go:hexToCompact`).
    #[must_use]
    fn encode_compact(&self) -> Vec<u8> {
        let term: u8 = u8::from(self.is_leaf);
        let n = &self.nibbles;
        let mut buf = Vec::with_capacity(n.len() / 2 + 1);
        let mut first = term << 5; // 0x20 when leaf, else 0x00
        let rest: &[u8] = if n.len() & 1 == 1 {
            first |= 0x10 | n[0]; // odd flag + first nibble in low nibble
            &n[1..]
        } else {
            &n[..]
        };
        buf.push(first);
        // `rest` is now even-length; pack nibble pairs hi<<4 | lo.
        for pair in rest.chunks_exact(2) {
            buf.push((pair[0] << 4) | pair[1]);
        }
        buf
    }
}

/// Reference to a child node: inline (the child's full RLP, `< 32` bytes,
/// spliced raw into the parent) or hashed (`>= 32` bytes, referenced by
/// `keccak256`). The inline-vs-hash distinction is the spec-load-bearing
/// detail (LVP-Q3) — `< 32` strictly, per go-ethereum `trie/hasher.go:68`.
#[derive(Debug)]
pub(super) enum NodeRef {
    /// `keccak256(child_rlp)` when `child_rlp.len() >= 32`.
    Hash(B256),
    /// The child's full RLP encoding when `child_rlp.len() < 32`.
    Inline(Vec<u8>),
}

impl NodeRef {
    /// Builds a child reference from the child's full RLP encoding,
    /// applying the `< 32` inline / `>= 32` hash rule (LVP-Q3).
    #[must_use]
    fn from_encoding(enc: Vec<u8>) -> Self {
        if enc.len() < 32 {
            Self::Inline(enc)
        } else {
            Self::Hash(keccak256(&enc))
        }
    }

    /// The bytes this reference contributes to its parent's RLP payload:
    /// an inline child is spliced raw; a hashed child is the 32-byte hash
    /// RLP-string-encoded (`0xa0 || hash`).
    #[must_use]
    fn payload(&self) -> Vec<u8> {
        match self {
            Self::Hash(h) => rlp_encode(h.as_slice()),
            Self::Inline(raw) => raw.clone(),
        }
    }
}

/// One of the three Ethereum MPT node kinds (D5 (a)).
#[derive(Debug)]
pub(super) enum Node {
    /// Terminator nibble path + value-node bytes (`RLP(minimal(value))`).
    Leaf { path: Nibbles, value: Vec<u8> },
    /// Shared-prefix nibble path + single child reference.
    Extension { path: Nibbles, child: NodeRef },
    /// 16 child slots + an optional value slot (17-element RLP list).
    /// The child array is boxed so the `Branch` variant does not dwarf
    /// `Leaf`/`Extension` (`clippy::large_enum_variant`).
    Branch {
        children: Box<[Option<NodeRef>; 16]>,
        value: Option<Vec<u8>>,
    },
}

impl Node {
    /// Full RLP encoding of this node (an RLP list in every case).
    #[must_use]
    fn encode(&self) -> Vec<u8> {
        let mut payload = Vec::new();
        match self {
            Self::Leaf { path, value } => {
                payload.extend_from_slice(&rlp_encode(path.encode_compact().as_slice()));
                // `value` is already RLP(minimal(slot_value)); the node
                // encoder RLP-string-wraps it again (geth WriteBytes(Val)).
                payload.extend_from_slice(&rlp_encode(value.as_slice()));
            }
            Self::Extension { path, child } => {
                payload.extend_from_slice(&rlp_encode(path.encode_compact().as_slice()));
                payload.extend_from_slice(&child.payload());
            }
            Self::Branch { children, value } => {
                for slot in children.as_ref() {
                    match slot {
                        // 0x80 = RLP("") — the empty child slot.
                        None => payload.push(0x80),
                        Some(r) => payload.extend_from_slice(&r.payload()),
                    }
                }
                match value {
                    None => payload.push(0x80),
                    Some(v) => payload.extend_from_slice(&rlp_encode(v.as_slice())),
                }
            }
        }
        let mut out = Vec::with_capacity(payload.len() + 4);
        Header { list: true, payload_length: payload.len() }.encode(&mut out);
        out.extend_from_slice(&payload);
        out
    }
}

/// Length of the longest common nibble prefix of `items` measured from
/// `depth` (all nibble vectors have equal length 64). Returns a value
/// `>= depth`; equals `depth` when the items already diverge there.
fn common_prefix_end(items: &[(Vec<u8>, Vec<u8>)], depth: usize) -> usize {
    let first = &items[0].0;
    let mut end = depth;
    'outer: while end < first.len() {
        let nib = first[end];
        for (path, _) in &items[1..] {
            if path[end] != nib {
                break 'outer;
            }
        }
        end += 1;
    }
    end
}

/// Builds the trie node spanning `items` (sorted ascending by nibble path,
/// distinct, all length 64) consuming nibbles `[depth..]`. Bottom-up:
/// returns the constructed [`Node`]; callers encode + reference it.
fn build(items: &[(Vec<u8>, Vec<u8>)], depth: usize) -> Node {
    if items.len() == 1 {
        let (path, value) = &items[0];
        return Node::Leaf {
            path: Nibbles { nibbles: path[depth..].to_vec(), is_leaf: true },
            value: value.clone(),
        };
    }

    let cpe = common_prefix_end(items, depth);
    if cpe > depth {
        // Shared prefix → extension over [depth..cpe], child built at cpe
        // (which then necessarily branches, the prefix being maximal).
        let child = build(items, cpe);
        return Node::Extension {
            path: Nibbles { nibbles: items[0].0[depth..cpe].to_vec(), is_leaf: false },
            child: NodeRef::from_encoding(child.encode()),
        };
    }

    // Branch at `depth`: bucket the (already sorted) items by their nibble
    // at `depth`. Secure-trie keys are all 64 nibbles and distinct, so no
    // key is a prefix of another → the branch value slot is never used.
    let mut children: [Option<NodeRef>; 16] = std::array::from_fn(|_| None);
    let mut i = 0;
    while i < items.len() {
        let nib = items[i].0[depth] as usize;
        let mut j = i + 1;
        while j < items.len() && (items[j].0[depth] as usize) == nib {
            j += 1;
        }
        let child = build(&items[i..j], depth + 1);
        children[nib] = Some(NodeRef::from_encoding(child.encode()));
        i = j;
    }
    Node::Branch { children: Box::new(children), value: None }
}

/// Expands a byte slice into its nibble sequence (`hi, lo` per byte).
fn bytes_to_nibbles(bytes: &[u8]) -> Vec<u8> {
    let mut n = Vec::with_capacity(bytes.len() * 2);
    for &b in bytes {
        n.push(b >> 4);
        n.push(b & 0x0f);
    }
    n
}

/// RLP-encodes a slot value the storage-trie way: the 32-byte big-endian
/// value with leading zero bytes stripped, then RLP-string-encoded
/// (`rlp(minimal(value))`). Callers must not pass `B256::ZERO` (it is
/// filtered as "absent" before this point).
fn rlp_value(value: B256) -> Vec<u8> {
    let b = value.as_slice();
    let start = b.iter().position(|&x| x != 0).unwrap_or(b.len());
    rlp_encode(&b[start..])
}

/// Computes the Ethereum-compatible MPT root over `entries`
/// (`(slot_key, slot_value)` pairs).
///
/// Internally hashes each `slot_key` with `keccak256` (secure storage
/// trie) and re-sorts via a `BTreeMap` — so `entries` may arrive in any
/// order, including the raw-slot order the MDBX cursor yields (D7 (b)
/// sort-then-build; D8 (a)). A slot whose value is `B256::ZERO` is treated
/// as absent (`eth_storageRoot` semantics). An empty (or all-zero) entry
/// stream returns [`EMPTY_ROOT`] (D11 (c)).
///
/// Infallible by design (D14 (a) + D12 (d)): well-formed input cannot
/// trigger an internal invariant violation. Caller-side storage I/O errors
/// are converted to panics at the [`crate::MptState::root`] /
/// `MptSnapshot::root` boundary in `mpt/mod.rs` (Commit 2), not here.
pub fn compute_root(entries: impl Iterator<Item = (B256, B256)>) -> B256 {
    // BTreeMap: deterministic ascending iteration by keccak256(slot) — the
    // trie path order (AGENTS.md Rule 7: never HashMap in commit-path code).
    let mut sorted: BTreeMap<B256, Vec<u8>> = BTreeMap::new();
    for (slot, value) in entries {
        if value == B256::ZERO {
            continue; // eth_storageRoot: a zeroed slot is absent.
        }
        sorted.insert(keccak256(slot.as_slice()), rlp_value(value));
    }
    if sorted.is_empty() {
        return EMPTY_ROOT;
    }

    let items: Vec<(Vec<u8>, Vec<u8>)> = sorted
        .into_iter()
        .map(|(k, v)| (bytes_to_nibbles(k.as_slice()), v))
        .collect();
    // Root is always hashed (geth `force`), even when its RLP is < 32 bytes.
    keccak256(build(&items, 0).encode())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::str::FromStr;

    use alloy_primitives::B256;
    use pretty_assertions::assert_eq;
    use serde_json::Value;

    use super::*;

    const FIXTURES: &str = include_str!("../../tests/fixtures/mpt_roots.json");

    type Entries = Vec<(B256, B256)>;
    type FixtureVector = (String, Entries, B256);

    fn h(s: &str) -> B256 {
        B256::from_str(s).unwrap()
    }

    /// All fixture vectors as `(name, entries, expected_root)`.
    fn fixture_vectors() -> Vec<FixtureVector> {
        let json: Value = serde_json::from_str(FIXTURES).unwrap();
        json["vectors"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| {
                let name = v["name"].as_str().unwrap().to_string();
                let entries = v["entries"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|pair| {
                        let p = pair.as_array().unwrap();
                        (h(p[0].as_str().unwrap()), h(p[1].as_str().unwrap()))
                    })
                    .collect();
                (name, entries, h(v["root"].as_str().unwrap()))
            })
            .collect()
    }

    fn vector(name: &str) -> (Entries, B256) {
        let (_, e, r) = fixture_vectors().into_iter().find(|(n, ..)| n == name).unwrap();
        (e, r)
    }

    // (i) D11 (c): the constant equals the canonical literal bytes.
    #[test]
    fn empty_trie_root_is_empty_root_constant() {
        assert_eq!(
            EMPTY_ROOT,
            h("0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421")
        );
        // And the fixture's top-level empty_root agrees (LVP-Q6 / alloy-trie).
        let json: Value = serde_json::from_str(FIXTURES).unwrap();
        assert_eq!(EMPTY_ROOT, h(json["empty_root"].as_str().unwrap()));
    }

    // (ii) D11 (c): the computed empty-trie path equals the constant.
    #[test]
    fn empty_trie_path_matches_constant() {
        assert_eq!(compute_root(std::iter::empty()), EMPTY_ROOT);
    }

    // (iii) Single-key vector (one leaf, force-hashed at the root).
    #[test]
    fn single_key_vector() {
        let (entries, root) = vector("single");
        assert_eq!(compute_root(entries.into_iter()), root);
    }

    // (iv) Two keys whose keccak256 paths diverge → top-level branch.
    #[test]
    fn two_keys_diverging_prefix() {
        let (entries, root) = vector("two_diverging");
        assert_eq!(compute_root(entries.into_iter()), root);
    }

    // (v) Multi-entry set → extension + branch structure in hashed-key
    //     space, plus multi-byte RLP value-length discriminants.
    #[test]
    fn shared_prefix_extension_then_branch() {
        for name in ["shared_prefix_extension_then_branch", "multi_byte_values", "many"] {
            let (entries, root) = vector(name);
            assert_eq!(compute_root(entries.into_iter()), root, "vector {name}");
        }
    }

    // (vi) Inline-vs-hash threshold (D9 (a) — the most-commonly-botched
    //      path). Secure-trie keccak256 keys make every `compute_root`
    //      node's RLP >= 32 bytes (deep paths), so the inline branch is
    //      unreachable through the public surface; test the load-bearing
    //      `NodeRef::from_encoding` decision and the raw-splice vs
    //      `0xa0||hash` parent embedding directly.
    #[test]
    fn inline_encoded_child_vector() {
        // < 32 bytes → inline, spliced raw into the parent payload.
        let small = vec![0xC2, 0x01, 0x02]; // a 3-byte RLP list
        match NodeRef::from_encoding(small.clone()) {
            NodeRef::Inline(raw) => assert_eq!(raw, small),
            NodeRef::Hash(_) => panic!("RLP < 32 must embed inline"),
        }
        assert_eq!(NodeRef::Inline(small.clone()).payload(), small);

        // >= 32 bytes → hashed, embedded as 0xa0 || keccak256(enc).
        let big = vec![0xAB; 40];
        match NodeRef::from_encoding(big.clone()) {
            NodeRef::Hash(hh) => {
                assert_eq!(hh, keccak256(&big));
                let p = NodeRef::Hash(hh).payload();
                assert_eq!(p.len(), 33);
                assert_eq!(p[0], 0xa0); // RLP 32-byte string prefix
            }
            NodeRef::Inline(_) => panic!("RLP >= 32 must be hashed"),
        }

        // Exactly 32 bytes is HASHED, not inline (geth `< 32`, LVP-Q3).
        assert!(matches!(NodeRef::from_encoding(vec![0u8; 32]), NodeRef::Hash(_)));
        assert!(matches!(NodeRef::from_encoding(vec![0u8; 31]), NodeRef::Inline(_)));
    }

    // (vii) Every fixture vector round-trips against the alloy-trie oracle
    //       root (D10 (e) — includes `with_zero_value_entry`, asserting
    //       zero-valued slots are excluded per eth_storageRoot).
    #[test]
    fn fixture_file_vectors() {
        for (name, entries, root) in fixture_vectors() {
            assert_eq!(compute_root(entries.into_iter()), root, "vector {name}");
        }
    }

    // (viii) Zero-value exclusion is explicit, not incidental.
    #[test]
    fn zero_value_slots_are_absent() {
        // A single zeroed slot → empty trie.
        assert_eq!(
            compute_root(std::iter::once((h(
                "0x0000000000000000000000000000000000000000000000000000000000000007"
            ), B256::ZERO))),
            EMPTY_ROOT
        );
        // A zeroed slot does not change the root vs omitting it entirely.
        let a = h("0x0000000000000000000000000000000000000000000000000000000000000001");
        let z = h("0x0000000000000000000000000000000000000000000000000000000000000002");
        let v = h("0x00000000000000000000000000000000000000000000000000000000000000aa");
        assert_eq!(
            compute_root([(a, v), (z, B256::ZERO)].into_iter()),
            compute_root(std::iter::once((a, v)))
        );
    }
}

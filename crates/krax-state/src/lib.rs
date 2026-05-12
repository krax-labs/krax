//! krax-state: state backend implementations for Krax.
//!
//! V1 ships an in-memory `MptState` (Step 1.3a) followed by MDBX-backed
//! durability (Step 1.3b). Real MPT root computation lands in Step 1.5.
//!
//! See `AGENTS.md` "Project Structure" for this crate's role in the workspace.

pub mod mpt;

pub use mpt::{MptSnapshot, MptState};

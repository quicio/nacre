//! Domain types shared by Nacre's inner layers.

use thiserror::Error;

macro_rules! id_newtype {
    ($(#[$meta:meta])+ $name:ident) => {
        $(#[$meta])+
        #[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(u64);

        impl $name {
            /// Creates an identifier from its raw numeric value.
            pub const fn new(value: u64) -> Self {
                Self(value)
            }

            /// Returns the raw numeric value.
            pub const fn get(self) -> u64 {
                self.0
            }
        }
    };
}

id_newtype! {
    /// Stable identity of a patch.
    PatchId
}

id_newtype! {
    /// Stable identity of a node within a patch.
    NodeId
}

id_newtype! {
    /// Stable identity of a parameter within a patch.
    ParameterId
}

id_newtype! {
    /// Monotonic revision of domain state.
    Revision
}

id_newtype! {
    /// Number of audio frames elapsed since an implementation-defined origin.
    SampleTime
}

/// Errors produced by domain invariants.
#[derive(Debug, Error, Eq, PartialEq)]
pub enum DomainError {
    /// A referenced node does not exist in the patch.
    #[error("node not found: {node_id:?}")]
    NodeNotFound {
        /// Identity of the missing node.
        node_id: NodeId,
    },
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{DomainError, NodeId, PatchId};

    #[test]
    fn identifiers_support_equality_and_hashing() {
        let first = PatchId::new(7);
        let same = PatchId::new(7);
        let different = PatchId::new(8);

        assert_eq!(first, same);
        assert_ne!(first, different);

        let mut identifiers = HashSet::new();
        assert!(identifiers.insert(first));
        assert!(!identifiers.insert(same));
        assert!(identifiers.insert(different));
        assert_eq!(identifiers.len(), 2);
    }

    #[test]
    fn domain_error_displays_the_missing_node() {
        let error = DomainError::NodeNotFound {
            node_id: NodeId::new(42),
        };

        assert_eq!(error.to_string(), "node not found: NodeId(42)");
    }
}

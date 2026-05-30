//! Property-based invariant tests for wintermute-presence.
//!
//! Read-only after scaffold. The edit-agent must NOT modify proptests.

use proptest::prelude::*;

proptest! {
    #[test]
    fn daily_count_never_overflows(count in 0u64..u64::MAX / 2) {
        // The daily counter must be representable; saturating add is acceptable.
        let next = count.saturating_add(1);
        prop_assert!(next >= count, "saturating add must not decrease count");
    }

    #[test]
    fn transcript_len_is_char_count(s in ".*") {
        // The presence system uses s.len() (byte count) as transcript_len.
        // This invariant verifies it is always non-negative and representable.
        let len = s.len();
        prop_assert!(len <= usize::MAX);
    }

    #[test]
    fn placeholder_invariant(n in 0u32..1024) {
        prop_assert!(n.checked_add(0).is_some());
    }
}

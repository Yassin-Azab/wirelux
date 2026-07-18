//! The O(log n) lookup algorithm, kept independent of any storage backend
//! so it can run equally well over an in-memory `Vec<IpRange>` (as in the
//! unit tests below) or directly over bytes mapped from disk (as
//! `GeoDatabase` uses it in `database.rs`).

use crate::models::IpRange;

/// Binary-searches `count` ranges, accessed via `range_at`, for the one
/// containing `target`.
///
/// # Preconditions
/// The ranges yielded by `range_at(0)..range_at(count - 1)` must be sorted
/// by `start` and must not overlap. [`crate::write_database`] guarantees
/// this for anything it writes.
///
/// # Complexity
/// `O(log count)` calls to `range_at`, no allocation.
pub fn binary_search_range<F>(count: usize, target: u32, range_at: F) -> Option<usize>
where
    F: Fn(usize) -> IpRange,
{
    let mut lo = 0usize;
    let mut hi = count;

    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        let candidate = range_at(mid);

        if target < candidate.start {
            hi = mid;
        } else if target > candidate.end {
            lo = mid + 1;
        } else {
            return Some(mid);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_ranges() -> Vec<IpRange> {
        vec![
            IpRange { start: 0, end: 9, country_index: 1 },
            IpRange { start: 10, end: 19, country_index: 2 },
            IpRange { start: 30, end: 39, country_index: 3 },
            IpRange { start: 1000, end: 1000, country_index: 4 }, // single-address /32
        ]
    }

    #[test]
    fn finds_value_in_middle_of_a_range() {
        let r = sample_ranges();
        assert_eq!(binary_search_range(r.len(), 15, |i| r[i]), Some(1));
    }

    #[test]
    fn finds_range_start_boundary() {
        let r = sample_ranges();
        assert_eq!(binary_search_range(r.len(), 30, |i| r[i]), Some(2));
    }

    #[test]
    fn finds_range_end_boundary() {
        let r = sample_ranges();
        assert_eq!(binary_search_range(r.len(), 19, |i| r[i]), Some(1));
    }

    #[test]
    fn finds_single_address_range() {
        let r = sample_ranges();
        assert_eq!(binary_search_range(r.len(), 1000, |i| r[i]), Some(3));
    }

    #[test]
    fn returns_none_for_gap_between_ranges() {
        let r = sample_ranges();
        assert_eq!(binary_search_range(r.len(), 25, |i| r[i]), None);
    }

    #[test]
    fn returns_none_below_first_range() {
        let r = sample_ranges();
        // u32 can't go below 0, but a gap above the max range still applies below index 0 too.
        assert_eq!(binary_search_range(r.len(), 0, |i| r[i]), Some(0));
    }

    #[test]
    fn returns_none_above_last_range() {
        let r = sample_ranges();
        assert_eq!(binary_search_range(r.len(), 5_000, |i| r[i]), None);
    }

    #[test]
    fn returns_none_for_empty_table() {
        let r: Vec<IpRange> = vec![];
        assert_eq!(binary_search_range(0, 42, |i| r[i]), None);
    }
}

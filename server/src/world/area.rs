//! Boxes of chunks around a point, used by both generation and streaming

use bevy::prelude::*;

/// Every chunk offset within `radius` (per axis, in chunks), nearest first, so
/// whatever works through the list handles the chunks around the player first
pub fn nearest_first(radius: IVec3) -> Vec<IVec3> {
    let mut offsets = Vec::new();
    for y in -radius.y..=radius.y {
        for z in -radius.z..=radius.z {
            for x in -radius.x..=radius.x {
                offsets.push(IVec3::new(x, y, z));
            }
        }
    }
    offsets.sort_by_key(|offset| offset.length_squared());
    offsets
}

/// Whether a chunk `offset` away is inside the box of `radius`
pub fn within(offset: IVec3, radius: IVec3) -> bool {
    offset.abs().cmple(radius).all()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nearest_first_covers_the_box_centre_out() {
        let offsets = nearest_first(IVec3::new(2, 1, 2));
        assert_eq!(offsets.len(), 5 * 3 * 5);
        assert_eq!(offsets[0], IVec3::ZERO);
        assert!(offsets.iter().all(|&o| within(o, IVec3::new(2, 1, 2))));
        assert!(!within(IVec3::new(0, 2, 0), IVec3::new(2, 1, 2)));
    }
}

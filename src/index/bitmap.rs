//! Bitmap index for set operations
//!
//! Provides efficient set intersection, union, and difference operations.
//! Uses croaring Roaring Bitmap for high performance.

use croaring::Bitmap as RoaringBitmap;

/// Bitmap for efficient set operations on IDs
#[derive(Debug, Clone)]
pub struct Bitmap {
    bits: RoaringBitmap,
}

impl Bitmap {
    /// Create an empty bitmap
    pub fn new() -> Self {
        Self {
            bits: RoaringBitmap::new(),
        }
    }

    /// Create from a vector of IDs
    pub fn from_ids(ids: &[u64]) -> Self {
        let mut bitmap = RoaringBitmap::new();
        for &id in ids {
            // croaring expects u32, but we store u64 IDs
            // For IDs that fit in u32, we can use them directly
            if id <= u32::MAX as u64 {
                bitmap.add(id as u32);
            }
        }
        Self { bits: bitmap }
    }

    /// Add an ID
    pub fn add(&mut self, id: u64) {
        if id <= u32::MAX as u64 {
            self.bits.add(id as u32);
        }
    }

    /// Remove an ID
    pub fn remove(&mut self, id: u64) {
        if id <= u32::MAX as u64 {
            self.bits.remove(id as u32);
        }
    }

    /// Check if ID exists
    pub fn contains(&self, id: u64) -> bool {
        if id <= u32::MAX as u64 {
            self.bits.contains(id as u32)
        } else {
            false
        }
    }

    /// Set intersection
    pub fn intersection(&self, other: &Bitmap) -> Bitmap {
        Bitmap {
            bits: self.bits.and(&other.bits),
        }
    }

    /// Set union
    pub fn union(&self, other: &Bitmap) -> Bitmap {
        Bitmap {
            bits: self.bits.or(&other.bits),
        }
    }

    /// Set difference (self - other)
    pub fn difference(&self, other: &Bitmap) -> Bitmap {
        Bitmap {
            bits: self.bits.andnot(&other.bits),
        }
    }

    /// Cardinality (number of elements)
    pub fn cardinality(&self) -> usize {
        self.bits.cardinality() as usize
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.bits.is_empty()
    }

    /// Get all IDs as a sorted vector
    pub fn to_vec(&self) -> Vec<u64> {
        self.bits.to_vec().iter().map(|&id| id as u64).collect()
    }

    /// Add a range of IDs [start, end)
    pub fn add_range(&mut self, start: u64, end: u64) {
        // croaring supports range add for u32 range
        let s = start.clamp(0, u32::MAX as u64) as u32;
        let e = end.clamp(0, u32::MAX as u64) as u32;
        if s < e {
            self.bits.add_range(s..e);
        }
    }

    /// Serialize to bytes
    pub fn serialize(&self) -> Vec<u8> {
        self.bits.serialize::<croaring::Portable>()
    }

    /// Deserialize from bytes
    pub fn deserialize(data: &[u8]) -> Option<Self> {
        let bitmap = RoaringBitmap::deserialize::<croaring::Portable>(data);
        if bitmap.is_empty() && !data.is_empty() {
            // If data is not empty but bitmap is empty, it might be a deserialization failure
            // croaring returns empty bitmap for invalid data
            None
        } else {
            Some(Self { bits: bitmap })
        }
    }
}

impl Default for Bitmap {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intersection() {
        let a = Bitmap::from_ids(&[1, 2, 3, 4, 5]);
        let b = Bitmap::from_ids(&[3, 4, 5, 6, 7]);
        let c = a.intersection(&b);
        assert_eq!(c.cardinality(), 3);
        assert!(c.contains(3));
        assert!(c.contains(4));
        assert!(c.contains(5));
    }

    #[test]
    fn test_union() {
        let a = Bitmap::from_ids(&[1, 2, 3]);
        let b = Bitmap::from_ids(&[3, 4, 5]);
        let c = a.union(&b);
        assert_eq!(c.cardinality(), 5);
    }

    #[test]
    fn test_difference() {
        let a = Bitmap::from_ids(&[1, 2, 3, 4]);
        let b = Bitmap::from_ids(&[3, 4, 5]);
        let c = a.difference(&b);
        assert_eq!(c.to_vec(), vec![1, 2]);
    }

    #[test]
    fn test_add_range() {
        let mut bm = Bitmap::new();
        bm.add_range(10, 15);
        assert_eq!(bm.cardinality(), 5);
        assert!(bm.contains(10));
        assert!(bm.contains(14));
        assert!(!bm.contains(15));
    }
}

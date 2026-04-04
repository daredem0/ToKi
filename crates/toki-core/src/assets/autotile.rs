use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutoTileMode {
    FourBit,
    EightBit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoTileGroup {
    pub mode: AutoTileMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview_tile: Option<String>,
    pub variants: HashMap<u8, String>,
}

impl AutoTileGroup {
    pub fn resolve_variant(&self, mask: u8) -> Option<&str> {
        self.variants.get(&mask).map(String::as_str)
    }

    pub fn expected_variant_count(&self) -> usize {
        match self.mode {
            AutoTileMode::FourBit => 16,
            AutoTileMode::EightBit => WANG_47_COUNT,
        }
    }
}

// --- 4-bit bitmask (cardinal neighbors) ---

/// Bit layout: N=0x01, E=0x02, S=0x04, W=0x08
pub fn compute_4bit_mask(north: bool, east: bool, south: bool, west: bool) -> u8 {
    (north as u8) | ((east as u8) << 1) | ((south as u8) << 2) | ((west as u8) << 3)
}

// --- 8-bit bitmask (all 8 neighbors, reduced to 47 Wang tiles) ---

pub struct FullNeighbors {
    pub n: bool,
    pub ne: bool,
    pub e: bool,
    pub se: bool,
    pub s: bool,
    pub sw: bool,
    pub w: bool,
    pub nw: bool,
}

/// Bit layout: N=0x01, NE=0x02, E=0x04, SE=0x08, S=0x10, SW=0x20, W=0x40, NW=0x80
/// Corners are only relevant when both adjacent edges are present.
pub fn compute_8bit_mask(neighbors: &FullNeighbors) -> u8 {
    let raw = (neighbors.n as u8)
        | ((neighbors.ne as u8) << 1)
        | ((neighbors.e as u8) << 2)
        | ((neighbors.se as u8) << 3)
        | ((neighbors.s as u8) << 4)
        | ((neighbors.sw as u8) << 5)
        | ((neighbors.w as u8) << 6)
        | ((neighbors.nw as u8) << 7);
    reduce_8bit_raw(raw)
}

/// Zeros corner bits when adjacent edges are absent, then maps to 0..46.
fn reduce_8bit_raw(raw: u8) -> u8 {
    let n = raw & 0x01 != 0;
    let e = raw & 0x04 != 0;
    let s = raw & 0x10 != 0;
    let w = raw & 0x40 != 0;

    // Zero corners unless both adjacent edges are present
    let ne = raw & 0x02 != 0 && n && e;
    let se = raw & 0x08 != 0 && e && s;
    let sw = raw & 0x20 != 0 && s && w;
    let nw = raw & 0x80 != 0 && w && n;

    let reduced = (n as u8)
        | ((ne as u8) << 1)
        | ((e as u8) << 2)
        | ((se as u8) << 3)
        | ((s as u8) << 4)
        | ((sw as u8) << 5)
        | ((w as u8) << 6)
        | ((nw as u8) << 7);

    WANG_47_LOOKUP[reduced as usize]
}

const WANG_47_COUNT: usize = 47;

/// Lookup table mapping all 256 reduced 8-bit masks to Wang-47 indices (0..46).
/// Built from the canonical 47 unique reduced masks in ascending order.
static WANG_47_LOOKUP: [u8; 256] = build_wang_47_lookup();

const fn build_wang_47_lookup() -> [u8; 256] {
    // First, collect the 47 canonical reduced masks in order.
    let canonical = canonical_wang_masks();
    let mut table = [0u8; 256];

    let mut raw: u16 = 0;
    while raw < 256 {
        let reduced = reduce_corners(raw as u8);
        // Find canonical index
        let mut idx: u8 = 0;
        let mut j = 0;
        while j < WANG_47_COUNT {
            if canonical[j] == reduced {
                idx = j as u8;
                break;
            }
            j += 1;
        }
        table[raw as usize] = idx;
        raw += 1;
    }
    table
}

const fn reduce_corners(raw: u8) -> u8 {
    let n = raw & 0x01 != 0;
    let e = raw & 0x04 != 0;
    let s = raw & 0x10 != 0;
    let w = raw & 0x40 != 0;

    let ne = raw & 0x02 != 0 && n && e;
    let se = raw & 0x08 != 0 && e && s;
    let sw = raw & 0x20 != 0 && s && w;
    let nw = raw & 0x80 != 0 && w && n;

    (n as u8)
        | ((ne as u8) << 1)
        | ((e as u8) << 2)
        | ((se as u8) << 3)
        | ((s as u8) << 4)
        | ((sw as u8) << 5)
        | ((w as u8) << 6)
        | ((nw as u8) << 7)
}

/// Returns the 47 canonical reduced masks in ascending order.
const fn canonical_wang_masks() -> [u8; WANG_47_COUNT] {
    let mut masks = [0u8; WANG_47_COUNT];
    let mut count = 0usize;
    let mut raw: u16 = 0;
    while raw < 256 {
        let reduced = reduce_corners(raw as u8);
        // Check if already collected
        let mut found = false;
        let mut j = 0;
        while j < count {
            if masks[j] == reduced {
                found = true;
                break;
            }
            j += 1;
        }
        if !found {
            masks[count] = reduced;
            count += 1;
        }
        raw += 1;
    }
    masks
}

// --- Validation ---

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AutoTileValidationError {
    MissingVariant { mask: u8 },
    UnknownTile { mask: u8, tile_name: String },
}

pub fn validate_group(
    group: &AutoTileGroup,
    atlas_tile_names: &HashSet<String>,
) -> Vec<AutoTileValidationError> {
    let mut errors = Vec::new();
    let max_mask = group.expected_variant_count() as u8;

    for mask in 0..max_mask {
        match group.variants.get(&mask) {
            None => errors.push(AutoTileValidationError::MissingVariant { mask }),
            Some(tile_name) if !atlas_tile_names.contains(tile_name) => {
                errors.push(AutoTileValidationError::UnknownTile {
                    mask,
                    tile_name: tile_name.clone(),
                });
            }
            Some(_) => {}
        }
    }
    errors
}

use super::autotile::*;
use std::collections::{HashMap, HashSet};

// --- 4-bit mask tests ---

#[test]
fn four_bit_no_neighbors() {
    assert_eq!(compute_4bit_mask(false, false, false, false), 0);
}

#[test]
fn four_bit_all_neighbors() {
    assert_eq!(compute_4bit_mask(true, true, true, true), 0b1111);
}

#[test]
fn four_bit_individual_bits() {
    assert_eq!(compute_4bit_mask(true, false, false, false), 0b0001); // N
    assert_eq!(compute_4bit_mask(false, true, false, false), 0b0010); // E
    assert_eq!(compute_4bit_mask(false, false, true, false), 0b0100); // S
    assert_eq!(compute_4bit_mask(false, false, false, true), 0b1000); // W
}

#[test]
fn four_bit_combinations() {
    assert_eq!(compute_4bit_mask(true, true, false, false), 0b0011); // N+E
    assert_eq!(compute_4bit_mask(false, true, true, false), 0b0110); // E+S
    assert_eq!(compute_4bit_mask(true, false, true, false), 0b0101); // N+S
    assert_eq!(compute_4bit_mask(false, false, true, true), 0b1100); // S+W
}

// --- 8-bit mask tests ---

fn full_neighbors(bits: [bool; 8]) -> FullNeighbors {
    let [n, ne, e, se, s, sw, w, nw] = bits;
    FullNeighbors {
        n,
        ne,
        e,
        se,
        s,
        sw,
        w,
        nw,
    }
}

#[test]
fn eight_bit_no_neighbors() {
    assert_eq!(compute_8bit_mask(&full_neighbors([false; 8])), 0);
}

#[test]
fn eight_bit_corners_ignored_without_adjacent_edges() {
    let with_corner = compute_8bit_mask(&full_neighbors([
        false, true, false, false, false, false, false, false,
    ]));
    let without = compute_8bit_mask(&full_neighbors([false; 8]));
    assert_eq!(with_corner, without);
}

#[test]
fn eight_bit_corner_kept_with_adjacent_edges() {
    let with_corner = compute_8bit_mask(&full_neighbors([
        true, true, true, false, false, false, false, false,
    ]));
    let without = compute_8bit_mask(&full_neighbors([
        true, false, true, false, false, false, false, false,
    ]));
    assert_ne!(with_corner, without);
}

#[test]
fn eight_bit_produces_47_unique_indices() {
    let mut indices = HashSet::new();
    for raw in 0u16..256 {
        let r = raw as u8;
        let idx = compute_8bit_mask(&full_neighbors([
            r & 0x01 != 0,
            r & 0x02 != 0,
            r & 0x04 != 0,
            r & 0x08 != 0,
            r & 0x10 != 0,
            r & 0x20 != 0,
            r & 0x40 != 0,
            r & 0x80 != 0,
        ]));
        indices.insert(idx);
    }
    assert_eq!(indices.len(), 47);
    assert!(*indices.iter().max().unwrap() < 47);
}

// --- Resolve variant tests ---

#[test]
fn resolve_variant_returns_tile_name() {
    let mut variants = HashMap::new();
    variants.insert(0, "grass_isolated".to_string());
    variants.insert(15, "grass_all".to_string());
    let group = AutoTileGroup {
        mode: AutoTileMode::FourBit,
        preview_tile: None,
        variants,
    };
    assert_eq!(group.resolve_variant(0), Some("grass_isolated"));
    assert_eq!(group.resolve_variant(15), Some("grass_all"));
    assert_eq!(group.resolve_variant(7), None);
}

// --- Validation tests ---

#[test]
fn validate_complete_4bit_group() {
    let mut variants = HashMap::new();
    let mut atlas_tiles = HashSet::new();
    for i in 0..16 {
        let name = format!("tile_{i}");
        variants.insert(i, name.clone());
        atlas_tiles.insert(name);
    }
    let group = AutoTileGroup {
        mode: AutoTileMode::FourBit,
        preview_tile: None,
        variants,
    };
    assert!(validate_group(&group, &atlas_tiles).is_empty());
}

#[test]
fn validate_incomplete_group_reports_missing() {
    let group = AutoTileGroup {
        mode: AutoTileMode::FourBit,
        preview_tile: None,
        variants: HashMap::new(),
    };
    let errors = validate_group(&group, &HashSet::new());
    assert_eq!(errors.len(), 16);
    assert!(errors
        .iter()
        .all(|e| matches!(e, AutoTileValidationError::MissingVariant { .. })));
}

#[test]
fn validate_unknown_tile_detected() {
    let mut variants = HashMap::new();
    variants.insert(0, "nonexistent".to_string());
    let group = AutoTileGroup {
        mode: AutoTileMode::FourBit,
        preview_tile: None,
        variants,
    };
    let errors = validate_group(&group, &HashSet::new());
    assert!(errors.iter().any(|e| matches!(
        e,
        AutoTileValidationError::UnknownTile {
            mask: 0,
            tile_name
        } if tile_name == "nonexistent"
    )));
}

#[test]
fn expected_variant_count_matches_mode() {
    let group_4 = AutoTileGroup {
        mode: AutoTileMode::FourBit,
        preview_tile: None,
        variants: HashMap::new(),
    };
    let group_8 = AutoTileGroup {
        mode: AutoTileMode::EightBit,
        preview_tile: None,
        variants: HashMap::new(),
    };
    assert_eq!(group_4.expected_variant_count(), 16);
    assert_eq!(group_8.expected_variant_count(), 47);
}

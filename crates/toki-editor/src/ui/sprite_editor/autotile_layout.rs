//! Shared autotile authoring layout helpers for the sprite editor.

/// Artist-facing 4-bit slot order used by the sprite editor.
///
/// The 4x4 sheet is laid out by visual role, not raw mask index:
///
/// ```text
/// TL  T  TR  N
/// L   C  R   E
/// BL  B  BR  S
/// W   V  H   I
/// ```
///
/// That visual order maps to the runtime mask ids below.
pub(crate) const FOUR_BIT_VISUAL_MASKS: [u8; 16] =
    [6, 14, 12, 1, 7, 15, 13, 2, 3, 11, 9, 4, 8, 5, 10, 0];

pub(crate) const FOUR_BIT_VISUAL_LAYOUT_TEXT: &str =
    "TL  T  TR  N\nL   C  R   E\nBL  B  BR  S\nW   V  H   I";

pub(crate) fn four_bit_mask_for_visual_slot(slot_index: u8) -> u8 {
    FOUR_BIT_VISUAL_MASKS[slot_index as usize]
}

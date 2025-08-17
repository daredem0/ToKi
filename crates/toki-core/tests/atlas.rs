use toki_core::assets::atlas::AtlasMeta;

#[test]
fn test_load_atlas() {
    let atlas = AtlasMeta::load_from_file("../../assets/terrain.json").unwrap();
    let rect = atlas.get_tile_rect("grass").unwrap();
    assert_eq!(rect, [0, 0, 8, 8]);
}

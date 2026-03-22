pub(crate) const SIDE_PANEL_DEFAULT_WIDTH: f32 = 250.0;

#[cfg(test)]
mod tests {
    use super::SIDE_PANEL_DEFAULT_WIDTH;

    #[test]
    fn side_panel_default_width_matches_editor_baseline() {
        assert_eq!(SIDE_PANEL_DEFAULT_WIDTH, 250.0);
    }
}

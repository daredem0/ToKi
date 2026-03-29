use toki_core::menu::MenuInput;

use super::{App, GraphicsSettingKey, RuntimeMenuOverlay};

impl App {
    pub(super) fn handle_audio_overlay_input(&mut self, input: MenuInput) -> bool {
        let selected_index = match self.runtime_overlay.as_ref() {
            Some(RuntimeMenuOverlay::Audio { selected_index }) => *selected_index,
            _ => return false,
        };

        let next_selected = match input {
            MenuInput::Up => Some(selected_index.saturating_sub(1)),
            MenuInput::Down => Some((selected_index + 1).min(4)),
            _ => None,
        };
        if let Some(next_selected) = next_selected {
            self.select_runtime_overlay_entry(next_selected);
            return false;
        }

        match input {
            MenuInput::Left => {
                self.adjust_audio_setting(selected_index, -(super::SETTING_STEP_PERCENT as i16))
            }
            MenuInput::Right => {
                self.adjust_audio_setting(selected_index, super::SETTING_STEP_PERCENT as i16)
            }
            MenuInput::Confirm => {
                if selected_index == 4 {
                    return true;
                }
            }
            MenuInput::Back => return true,
            MenuInput::Up | MenuInput::Down => {}
        }
        false
    }

    pub(super) fn handle_graphics_overlay_input(&mut self, input: MenuInput) -> bool {
        let mut selected_index = match self.runtime_overlay.as_ref() {
            Some(RuntimeMenuOverlay::Graphics { selected_index }) => *selected_index,
            _ => return false,
        };
        let entry_count = self.graphics_entries_with_keys(selected_index).len();
        selected_index = selected_index.min(entry_count.saturating_sub(1));

        let next_selected = match input {
            MenuInput::Up => Some(selected_index.saturating_sub(1)),
            MenuInput::Down => Some((selected_index + 1).min(entry_count.saturating_sub(1))),
            _ => None,
        };
        if let Some(next_selected) = next_selected {
            self.select_runtime_overlay_entry(next_selected);
            return false;
        }

        match input {
            MenuInput::Left => self.adjust_graphics_setting(selected_index, -1),
            MenuInput::Right | MenuInput::Confirm => {
                let selected_key =
                    self.graphics_entries_with_keys(selected_index)[selected_index].0;
                if selected_key == GraphicsSettingKey::Back && matches!(input, MenuInput::Confirm) {
                    return true;
                }
                self.adjust_graphics_setting(selected_index, 1);
            }
            MenuInput::Back => return true,
            MenuInput::Up | MenuInput::Down => {}
        }
        false
    }

    pub(super) fn select_runtime_overlay_entry(&mut self, entry_index: usize) {
        match self.runtime_overlay.as_mut() {
            Some(RuntimeMenuOverlay::Audio { selected_index })
            | Some(RuntimeMenuOverlay::Display { selected_index })
            | Some(RuntimeMenuOverlay::Graphics { selected_index }) => {
                *selected_index = entry_index;
            }
            None => {}
        }
    }
}

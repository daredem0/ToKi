//! Undo/redo history for sprite editing operations.

use super::canvas::SpriteCanvas;
use crate::ui::undo_redo::History;

/// Undo/redo command for sprite editing
#[derive(Debug, Clone)]
pub struct SpriteEditCommand {
    /// Canvas state before the edit
    pub before: SpriteCanvas,
    /// Canvas state after the edit
    pub after: SpriteCanvas,
}

/// Local undo/redo history for sprite editor (separate from scene history)
#[derive(Debug, Clone)]
pub struct SpriteEditorHistory {
    history: History<SpriteEditCommand>,
}

impl SpriteEditorHistory {
    pub fn new(max_size: usize) -> Self {
        Self {
            history: History::with_max_size(max_size),
        }
    }

    pub fn push(&mut self, command: SpriteEditCommand) {
        self.history.push(command);
    }

    pub fn take_undo(&mut self) -> Option<SpriteCanvas> {
        let command = self.history.take_undo()?;
        let before = command.before.clone();
        self.history.restore_redo(command);
        Some(before)
    }

    pub fn take_redo(&mut self) -> Option<SpriteCanvas> {
        let command = self.history.take_redo()?;
        let after = command.after.clone();
        self.history.restore_undo(command);
        Some(after)
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }

    pub fn clear(&mut self) {
        self.history.clear();
    }
}

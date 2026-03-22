//! Undo/redo history for sprite editing operations.

use super::canvas::SpriteCanvas;

/// Undo/redo command for sprite editing
#[derive(Debug, Clone)]
pub struct SpriteEditCommand {
    /// Canvas state before the edit
    pub before: SpriteCanvas,
    /// Canvas state after the edit
    pub after: SpriteCanvas,
}

/// Local undo/redo history for sprite editor (separate from scene history)
#[derive(Debug, Clone, Default)]
pub struct SpriteEditorHistory {
    undo_stack: Vec<SpriteEditCommand>,
    redo_stack: Vec<SpriteEditCommand>,
    /// Maximum number of undo steps to keep
    max_size: usize,
}

impl SpriteEditorHistory {
    pub fn new(max_size: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_size,
        }
    }

    pub fn push(&mut self, command: SpriteEditCommand) {
        self.undo_stack.push(command);
        self.redo_stack.clear();
        // Trim history if too large
        while self.undo_stack.len() > self.max_size {
            self.undo_stack.remove(0);
        }
    }

    pub fn take_undo(&mut self) -> Option<SpriteCanvas> {
        let command = self.undo_stack.pop()?;
        let before = command.before.clone();
        self.redo_stack.push(command);
        Some(before)
    }

    pub fn take_redo(&mut self) -> Option<SpriteCanvas> {
        let command = self.redo_stack.pop()?;
        let after = command.after.clone();
        self.undo_stack.push(command);
        Some(after)
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}

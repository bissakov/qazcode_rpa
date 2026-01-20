use crate::ui_constants::UiConstants;
use egui::util::undoer::Undoer;
use rpa_core::Project;

pub struct UndoRedoManager {
    undoer: Undoer<Project>,

    #[allow(dead_code)]
    history_limit: usize,
}

impl UndoRedoManager {
    pub fn new() -> Self {
        Self {
            undoer: Undoer::default(),
            history_limit: UiConstants::UNDO_HISTORY_LIMIT,
        }
    }

    pub fn feed_state(&mut self, time: f64, project: &Project) {
        self.undoer.feed_state(time, project);
    }

    pub fn add_undo(&mut self, project: &Project) {
        self.undoer.add_undo(project);
    }

    pub fn undo(&mut self, project: &Project) -> Option<Project> {
        self.undoer.undo(project).cloned()
    }

    pub fn redo(&mut self, project: &Project) -> Option<Project> {
        self.undoer.redo(project).cloned()
    }

    pub fn has_undo(&self, project: &Project) -> bool {
        self.undoer.has_undo(project)
    }

    pub fn has_redo(&self, project: &Project) -> bool {
        self.undoer.has_redo(project)
    }

    pub fn clear_history(&mut self) {
        self.undoer = Undoer::default();
    }

    /// Clear undo history after save operation.
    ///
    /// TODO: Decide undo history behavior on save:
    /// - Option 1: Clear history on file save (cleaner UX, current recommendation)
    /// - Option 2: Persist history to temporary state file (more powerful, uses disk space)
    /// - Option 3: Keep history independent of save (current implementation)
    #[allow(dead_code)]
    pub fn clear_undo_history(&mut self) {
        self.clear_history();
    }
}

impl Default for UndoRedoManager {
    fn default() -> Self {
        Self::new()
    }
}

use egui::util::undoer::Undoer;
use rpa_core::{Project, UiConstants};

pub struct UndoRedoManager {
    undoer: Undoer<Project>,
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

    /// Force an immediate undo point without waiting for stable_time.
    /// Used for discrete user actions that should each be a separate undo step
    /// (e.g., creating a scenario, dropping an activity, creating a connection).
    pub fn add_undo(&mut self, project: &Project) {
        self.undoer.add_undo(project);
    }

    /// Force the current flux to stabilize, creating an undo point if state changed.
    /// Used when user finishes a continuous interaction like dragging or resizing.
    /// Makes a time jump large enough to exceed stable_time, triggering undo point creation.
    pub fn force_stabilize_flux(&mut self, current_time: f64, project: &Project) {
        const STABLE_TIME_BUFFER: f64 = 1.5;
        self.feed_state(current_time + STABLE_TIME_BUFFER, project);
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
    pub fn clear_undo_history(&mut self) {
        self.clear_history();
    }
}

impl Default for UndoRedoManager {
    fn default() -> Self {
        Self::new()
    }
}

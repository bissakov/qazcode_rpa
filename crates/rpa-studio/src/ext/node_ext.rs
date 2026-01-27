use crate::ui_constants::{enforce_minimum_cells, UiConstants};
use egui::{pos2, vec2, Pos2, Rect};
use rpa_core::{BranchType, Node};

pub trait NodeExt {
    fn position(&self) -> Pos2;
    fn get_rect(&self) -> Rect;
    fn is_routable(&self) -> bool;
    fn snap_bounds(&self, grid_size: f32) -> (Pos2, f32, f32);
    #[allow(dead_code)]
    fn get_visual_bounds(&self) -> Rect;
    fn get_input_pin_pos(&self) -> Pos2;
    fn get_output_pin_pos(&self) -> Pos2;
    fn get_output_pin_positions(&self) -> [Option<Pos2>; 2];
    fn get_output_pin_pos_by_index(&self, index: usize) -> Pos2;
    fn get_pin_index_for_branch(&self, branch_type: &BranchType) -> usize;
    fn get_branch_type_for_pin(&self, pin_index: usize) -> BranchType;
}

impl NodeExt for Node {
    fn position(&self) -> Pos2 {
        pos2(self.x, self.y)
    }

    fn get_rect(&self) -> Rect {
        Rect::from_min_size(self.position(), vec2(self.width, self.height))
    }

    fn is_routable(&self) -> bool {
        !matches!(self.activity, rpa_core::Activity::Note { .. })
    }

    fn snap_bounds(&self, grid_size: f32) -> (Pos2, f32, f32) {
        if !self.is_routable() {
            return (self.position(), self.width, self.height);
        }

        let right = self.x + self.width;
        let bottom = self.y + self.height;

        let (snapped_left, snapped_right) =
            enforce_minimum_cells(self.x, right, grid_size, UiConstants::MIN_NODE_CELLS);
        let (snapped_top, snapped_bottom) =
            enforce_minimum_cells(self.y, bottom, grid_size, UiConstants::MIN_NODE_CELLS);

        let snapped_pos = pos2(snapped_left, snapped_top);
        let snapped_width = snapped_right - snapped_left;
        let snapped_height = snapped_bottom - snapped_top;

        (snapped_pos, snapped_width, snapped_height)
    }

    fn get_visual_bounds(&self) -> Rect {
        self.get_rect()
    }

    fn get_input_pin_pos(&self) -> Pos2 {
        self.position() + vec2(self.width / 2.0, 0.0)
    }

    fn get_output_pin_pos(&self) -> Pos2 {
        self.position() + vec2(self.width / 2.0, self.height)
    }

    fn get_output_pin_positions(&self) -> [Option<Pos2>; 2] {
        let pin_count = self.get_output_pin_count();
        if pin_count == 0 {
            return [None, None];
        }

        if pin_count == 1 {
            [Some(self.get_output_pin_pos()), None]
        } else {
            let bottom = self.position().y + self.height;
            let left_x = self.position().x + UiConstants::GRID_SIZE;
            let right_x = self.position().x + self.width - UiConstants::GRID_SIZE;
            [Some(pos2(left_x, bottom)), Some(pos2(right_x, bottom))]
        }
    }

    fn get_output_pin_pos_by_index(&self, index: usize) -> Pos2 {
        match self.get_output_pin_positions().get(index) {
            Some(Some(pos)) => *pos,
            _ => Pos2::ZERO,
        }
    }

    fn get_pin_index_for_branch(&self, branch_type: &BranchType) -> usize {
        match &self.activity {
            rpa_core::Activity::IfCondition { .. } => match branch_type {
                BranchType::TrueBranch => 0,
                BranchType::FalseBranch => 1,
                _ => 0,
            },
            rpa_core::Activity::Loop { .. } => match branch_type {
                BranchType::Default => 0,
                BranchType::LoopBody => 1,
                _ => 0,
            },
            rpa_core::Activity::While { .. } => match branch_type {
                BranchType::Default => 0,
                BranchType::LoopBody => 1,
                _ => 0,
            },
            rpa_core::Activity::TryCatch => match branch_type {
                BranchType::TryBranch => 0,
                BranchType::CatchBranch => 1,
                _ => 0,
            },
            _ => {
                if self.activity.can_have_error_output() {
                    match branch_type {
                        BranchType::ErrorBranch => 1,
                        _ => 0,
                    }
                } else {
                    0
                }
            }
        }
    }

    fn get_branch_type_for_pin(&self, pin_index: usize) -> BranchType {
        match &self.activity {
            rpa_core::Activity::IfCondition { .. } => {
                if pin_index == 0 {
                    BranchType::TrueBranch
                } else {
                    BranchType::FalseBranch
                }
            }
            rpa_core::Activity::Loop { .. } | rpa_core::Activity::While { .. } => {
                if pin_index == 1 {
                    BranchType::LoopBody
                } else {
                    BranchType::Default
                }
            }
            rpa_core::Activity::TryCatch => {
                if pin_index == 0 {
                    BranchType::TryBranch
                } else {
                    BranchType::CatchBranch
                }
            }
            _ => {
                if self.activity.can_have_error_output() {
                    if pin_index == 0 {
                        BranchType::Default
                    } else {
                        BranchType::ErrorBranch
                    }
                } else {
                    BranchType::Default
                }
            }
        }
    }
}

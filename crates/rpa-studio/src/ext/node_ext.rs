use crate::ui_constants::{OutputDirection, UiConstants, enforce_minimum_cells};
use egui::{Pos2, Rect, pos2, vec2};
use rpa_core::{BranchType, Node};

pub trait NodeExt {
    fn position(&self) -> Pos2;
    fn get_rect(&self) -> Rect;
    fn is_routable(&self) -> bool;
    fn snap_bounds(&self, grid_size: f32) -> (Pos2, f32, f32);
    fn get_routing_footprint(&self, grid_size: f32) -> Rect;
    #[allow(dead_code)]
    fn get_visual_bounds(&self) -> Rect;
    fn get_input_pin_pos(&self) -> Pos2;
    fn get_output_pin_pos(&self) -> Pos2;
    fn get_preferred_output_direction(&self, branch_type: &BranchType) -> OutputDirection;
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

    fn get_routing_footprint(&self, grid_size: f32) -> Rect {
        let (pos, w, h) = self.snap_bounds(grid_size);
        Rect::from_min_size(pos, vec2(w, h))
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

    fn get_preferred_output_direction(&self, branch_type: &BranchType) -> OutputDirection {
        match &self.activity {
            rpa_core::Activity::IfCondition { .. } => match branch_type {
                BranchType::TrueBranch => OutputDirection::Down,
                BranchType::FalseBranch => OutputDirection::Right,
                _ => OutputDirection::Down,
            },
            rpa_core::Activity::Loop { .. } => match branch_type {
                BranchType::Default => OutputDirection::Down,
                BranchType::LoopBody => OutputDirection::Right,
                _ => OutputDirection::Down,
            },
            rpa_core::Activity::While { .. } => match branch_type {
                BranchType::Default => OutputDirection::Down,
                BranchType::LoopBody => OutputDirection::Right,
                _ => OutputDirection::Down,
            },
            rpa_core::Activity::TryCatch => match branch_type {
                BranchType::TryBranch => OutputDirection::Down,
                BranchType::CatchBranch => OutputDirection::Right,
                _ => OutputDirection::Down,
            },
            _ => match branch_type {
                BranchType::ErrorBranch => OutputDirection::Right,
                _ => OutputDirection::Down,
            },
        }
    }

    fn get_output_pin_positions(&self) -> [Option<Pos2>; 2] {
        let pin_count = self.get_output_pin_count();
        if pin_count == 0 {
            return [None, None];
        }

        match &self.activity {
            rpa_core::Activity::End { .. } | rpa_core::Activity::Note { .. } => [None, None],
            rpa_core::Activity::IfCondition { .. } => {
                let true_dir = self.get_preferred_output_direction(&BranchType::TrueBranch);
                let false_dir = self.get_preferred_output_direction(&BranchType::FalseBranch);
                [
                    Some(get_pin_pos_for_direction(self, true_dir)),
                    Some(get_pin_pos_for_direction(self, false_dir)),
                ]
            }
            rpa_core::Activity::Loop { .. } => {
                let default_dir = self.get_preferred_output_direction(&BranchType::Default);
                let loop_dir = self.get_preferred_output_direction(&BranchType::LoopBody);
                [
                    Some(get_pin_pos_for_direction(self, default_dir)),
                    Some(get_pin_pos_for_direction(self, loop_dir)),
                ]
            }
            rpa_core::Activity::While { .. } => {
                let default_dir = self.get_preferred_output_direction(&BranchType::Default);
                let loop_dir = self.get_preferred_output_direction(&BranchType::LoopBody);
                [
                    Some(get_pin_pos_for_direction(self, default_dir)),
                    Some(get_pin_pos_for_direction(self, loop_dir)),
                ]
            }
            rpa_core::Activity::TryCatch => {
                let try_dir = self.get_preferred_output_direction(&BranchType::TryBranch);
                let catch_dir = self.get_preferred_output_direction(&BranchType::CatchBranch);
                [
                    Some(get_pin_pos_for_direction(self, try_dir)),
                    Some(get_pin_pos_for_direction(self, catch_dir)),
                ]
            }
            rpa_core::Activity::CallScenario { .. } | rpa_core::Activity::RunPowershell { .. } => {
                let default_dir = self.get_preferred_output_direction(&BranchType::Default);
                let error_dir = self.get_preferred_output_direction(&BranchType::ErrorBranch);
                [
                    Some(get_pin_pos_for_direction(self, default_dir)),
                    Some(get_pin_pos_for_direction(self, error_dir)),
                ]
            }
            _ => {
                let default_dir = self.get_preferred_output_direction(&BranchType::Default);
                if self.activity.can_have_error_output() {
                    let error_dir = self.get_preferred_output_direction(&BranchType::ErrorBranch);
                    [
                        Some(get_pin_pos_for_direction(self, default_dir)),
                        Some(get_pin_pos_for_direction(self, error_dir)),
                    ]
                } else {
                    [Some(get_pin_pos_for_direction(self, default_dir)), None]
                }
            }
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

fn get_pin_pos_for_direction(node: &Node, direction: OutputDirection) -> Pos2 {
    let center_x = node.width / 2.0;
    let center_y = node.height / 2.0;
    let bottom = node.height;
    let right = node.width;
    let pos = pos2(node.x, node.y);

    match direction {
        OutputDirection::Down => pos + vec2(center_x, bottom),
        OutputDirection::Right => pos + vec2(right, center_y),
        OutputDirection::Left => pos + vec2(0.0, center_y),
        OutputDirection::Up => pos + vec2(center_x, 0.0),
    }
}

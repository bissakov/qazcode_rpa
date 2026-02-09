use crate::ext::NodeExt;
use crate::ui_constants::snap_to_grid;
use egui::{Pos2, Rect, Vec2};
use rpa_core::node_graph::Node;
use shared::NanoId;
use std::collections::HashSet;

pub fn get_padded_rect(rect: Rect, padding: f32) -> Rect {
    Rect::from_min_max(
        Pos2::new(rect.min.x - padding, rect.min.y - padding),
        Pos2::new(rect.max.x + padding, rect.max.y + padding),
    )
}

pub fn check_collision(
    rect: Rect,
    nodes: &[Node],
    exclude_ids: &HashSet<NanoId>,
    padding: f32,
) -> bool {
    let padded = get_padded_rect(rect, padding);

    for node in nodes {
        if exclude_ids.contains(&node.id) {
            continue;
        }
        if !node.is_routable() {
            continue;
        }

        let node_rect = node.get_rect();
        if padded.intersects(node_rect) {
            return true;
        }
    }

    false
}

pub fn find_nearest_valid_position(
    desired_pos: Pos2,
    node_size: Vec2,
    nodes: &[Node],
    exclude_ids: &HashSet<NanoId>,
    padding: f32,
    grid_size: f32,
) -> Pos2 {
    let snapped = Pos2::new(
        snap_to_grid(desired_pos.x, grid_size),
        snap_to_grid(desired_pos.y, grid_size),
    );

    let candidate_rect = Rect::from_min_size(snapped, node_size);
    if !check_collision(candidate_rect, nodes, exclude_ids, padding) {
        return snapped;
    }

    let max_radius = 50;
    for radius in 1..=max_radius {
        let offsets = generate_spiral_offsets(radius);
        for (dx, dy) in offsets {
            let candidate = Pos2::new(
                snapped.x + dx as f32 * grid_size,
                snapped.y + dy as f32 * grid_size,
            );
            let candidate_rect = Rect::from_min_size(candidate, node_size);
            if !check_collision(candidate_rect, nodes, exclude_ids, padding) {
                return candidate;
            }
        }
    }

    snapped
}

fn generate_spiral_offsets(radius: i32) -> Vec<(i32, i32)> {
    let mut offsets = Vec::new();

    for x in -radius..=radius {
        offsets.push((x, -radius));
        offsets.push((x, radius));
    }

    for y in (-radius + 1)..radius {
        offsets.push((-radius, y));
        offsets.push((radius, y));
    }

    offsets
}

pub fn find_valid_position_for_nodes(
    positions: &[(NanoId, Pos2, Vec2)],
    nodes: &[Node],
    padding: f32,
    grid_size: f32,
) -> Option<Vec2> {
    if positions.is_empty() {
        return Some(Vec2::ZERO);
    }

    let exclude_ids: HashSet<NanoId> = positions.iter().map(|(id, _, _)| id.clone()).collect();

    let first = &positions[0];
    let snapped = Pos2::new(
        snap_to_grid(first.1.x, grid_size),
        snap_to_grid(first.1.y, grid_size),
    );
    let base_delta = snapped - first.1;

    let all_valid = positions.iter().all(|(_, pos, size)| {
        let adjusted = *pos + base_delta;
        let rect = Rect::from_min_size(adjusted, *size);
        !check_collision(rect, nodes, &exclude_ids, padding)
    });

    if all_valid {
        return Some(base_delta);
    }

    let max_radius = 50;
    for radius in 1..=max_radius {
        let offsets = generate_spiral_offsets(radius);
        for (dx, dy) in &offsets {
            let offset = Vec2::new(*dx as f32 * grid_size, *dy as f32 * grid_size);
            let candidate_delta = base_delta + offset;

            let all_valid = positions.iter().all(|(_, pos, size)| {
                let adjusted = *pos + candidate_delta;
                let rect = Rect::from_min_size(adjusted, *size);
                !check_collision(rect, nodes, &exclude_ids, padding)
            });

            if all_valid {
                return Some(candidate_delta);
            }
        }
    }

    None
}

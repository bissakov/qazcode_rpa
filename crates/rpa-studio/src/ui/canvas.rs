use crate::ui::connection_renderer::{ConnectionPath, ConnectionRenderer};
use crate::{activity_ext::ActivityExt, colors::ColorPalette, state::ScenarioViewState};
use egui::{
    Color32, Popup, PopupCloseBehavior, Pos2, Rect, Response, Stroke, StrokeKind, Ui, Vec2,
};
use egui_code_editor::{CodeEditor, ColorTheme, Syntax};
use rpa_core::{
    Activity, ActivityMetadata, BranchType, LogLevel, NanoId, Node, PropertyType, Scenario,
    UiConstants, VariableType, snap_to_grid,
};
use rust_i18n::t;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy)]
struct InputCache {
    pub scroll_delta: f32,
    pub is_panning: bool,
    pub alt_rmb: bool,
    pub shift_held: bool,
    pub is_left_drag: bool,
    pub pointer_any_released: bool,
    pub pointer_delta: Vec2,
    pub pointer_primary_down: bool,
    pub key_escape: bool,
}

impl InputCache {
    fn capture(ui: &Ui, response: &Response) -> Self {
        Self {
            scroll_delta: ui.input(|i| i.raw_scroll_delta.y),
            is_panning: ui.input(|i| {
                i.pointer.button_down(egui::PointerButton::Middle)
                    || (i.pointer.button_down(egui::PointerButton::Primary)
                        && i.key_down(egui::Key::Space))
            }),
            alt_rmb: ui.input(|i| {
                i.modifiers.alt && i.pointer.button_down(egui::PointerButton::Secondary)
            }),
            shift_held: ui.input(|i| i.modifiers.shift),
            is_left_drag: ui.input(|i| i.pointer.primary_down()) && response.drag_started(),
            pointer_any_released: ui.input(|i| i.pointer.any_released()),
            pointer_delta: ui.input(|i| i.pointer.delta()),
            pointer_primary_down: ui.input(|i| i.pointer.primary_down()),
            key_escape: ui.input(|i| i.key_pressed(egui::Key::Escape)),
        }
    }
}

pub enum ContextMenuAction {
    None,
    Copy,
    Cut,
    Paste,
    Delete,
    SelectAll,
}

pub enum ParameterBindingAction {
    None,
    Add,
    Edit(usize),
}

pub struct RenderState<'a> {
    pub selected_nodes: &'a mut HashSet<NanoId>,
    pub connection_from: &'a mut Option<(NanoId, usize)>,
    pub clipboard_empty: bool,
    pub show_minimap: bool,
    pub knife_tool_active: &'a mut bool,
    pub knife_path: &'a mut Vec<Pos2>,
    pub resizing_node: &'a mut Option<(NanoId, ResizeHandle)>,
    pub allow_node_resize: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResizeHandle {
    Right,
    Left,
    Bottom,
    Top,
    BottomRight,
    BottomLeft,
    TopRight,
    TopLeft,
}

type NodeIndex = HashMap<NanoId, usize>;

fn build_node_index(nodes: &[Node]) -> NodeIndex {
    nodes
        .iter()
        .enumerate()
        .map(|(idx, n)| (n.id.clone(), idx))
        .collect()
}

fn get_node_from_index<'a>(nodes: &'a [Node], index: &NodeIndex, id: &NanoId) -> Option<&'a Node> {
    index.get(id).and_then(|&idx| nodes.get(idx))
}

#[allow(dead_code)]
fn line_segments_intersect(p1: Pos2, p2: Pos2, p3: Pos2, p4: Pos2) -> bool {
    let d = (p2.x - p1.x) * (p4.y - p3.y) - (p2.y - p1.y) * (p4.x - p3.x);
    if d.abs() < 0.0001 {
        return false;
    }

    let t = ((p3.x - p1.x) * (p4.y - p3.y) - (p3.y - p1.y) * (p4.x - p3.x)) / d;
    let u = ((p3.x - p1.x) * (p2.y - p1.y) - (p3.y - p1.y) * (p2.x - p1.x)) / d;

    (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u)
}

fn get_connection_color(branch_type: &BranchType, _from_node: &Node) -> Color32 {
    match branch_type {
        BranchType::TrueBranch => ColorPalette::CONNECTION_TRUE,
        BranchType::FalseBranch => ColorPalette::CONNECTION_FALSE,
        BranchType::LoopBody => ColorPalette::CONNECTION_LOOP_BODY,
        BranchType::ErrorBranch => ColorPalette::CONNECTION_ERROR,
        BranchType::TryBranch => ColorPalette::CONNECTION_DEFAULT,
        BranchType::CatchBranch => ColorPalette::CONNECTION_ERROR,
        BranchType::Default => ColorPalette::CONNECTION_DEFAULT,
    }
}

fn find_connection_near_point(
    scenario: &Scenario,
    node_index: &NodeIndex,
    point: Pos2,
    _pan_offset: Vec2,
    _zoom: f32,
    threshold: f32,
    renderer: &mut ConnectionRenderer,
) -> Option<(NanoId, NanoId)> {
    for connection in &scenario.connections {
        if let (Some(from_node), Some(to_node)) = (
            get_node_from_index(&scenario.nodes, node_index, &connection.from_node),
            get_node_from_index(&scenario.nodes, node_index, &connection.to_node),
        ) {
            let path =
                ConnectionPath::new(from_node, to_node, &scenario.nodes, &connection.branch_type);
            if path.hit_test(point, renderer, &connection.id, threshold) {
                return Some((connection.from_node.clone(), connection.to_node.clone()));
            }
        }
    }

    None
}

fn find_intersecting_connections(
    scenario: &Scenario,
    node_index: &NodeIndex,
    cut_path: &[Pos2],
    _pan_offset: Vec2,
    _zoom: f32,
    renderer: &mut ConnectionRenderer,
) -> (Vec<(NanoId, NanoId)>, Vec<Pos2>) {
    let mut intersecting = Vec::new();
    let mut intersection_points = Vec::new();

    for connection in &scenario.connections {
        if let (Some(from_node), Some(to_node)) = (
            get_node_from_index(&scenario.nodes, node_index, &connection.from_node),
            get_node_from_index(&scenario.nodes, node_index, &connection.to_node),
        ) {
            let path =
                ConnectionPath::new(from_node, to_node, &scenario.nodes, &connection.branch_type);

            for i in 0..cut_path.len() - 1 {
                let cut_start = cut_path[i];
                let cut_end = cut_path[i + 1];

                if let Some(intersection_point) = path.intersects_line(cut_start, cut_end, renderer, &connection.id) {
                    intersecting.push((connection.from_node.clone(), connection.to_node.clone()));
                    intersection_points.push(intersection_point);
                    break;
                }
            }
        }
    }

    (intersecting, intersection_points)
}

fn canvas_context_menu(state: &mut RenderState, response: &Response) -> Option<ContextMenuAction> {
    let mut action = None;

    Popup::context_menu(response)
        .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            ui.set_min_width(150.0);

            if !state.selected_nodes.is_empty()
                && ui.button(t!("context_menu.copy").as_ref()).clicked()
            {
                action = Some(ContextMenuAction::Copy);
                ui.close();
            }

            if !state.selected_nodes.is_empty()
                && ui.button(t!("context_menu.cut").as_ref()).clicked()
            {
                action = Some(ContextMenuAction::Cut);
                ui.close();
            }

            if !state.clipboard_empty && ui.button(t!("context_menu.paste").as_ref()).clicked() {
                action = Some(ContextMenuAction::Paste);
                ui.close();
            }

            if !state.selected_nodes.is_empty()
                && ui.button(t!("context_menu.delete").as_ref()).clicked()
            {
                action = Some(ContextMenuAction::Delete);
                ui.close();
            }

            if ui.button(t!("context_menu.select_all").as_ref()).clicked() {
                action = Some(ContextMenuAction::SelectAll);
                ui.close();
            }
        });

    action
}

pub fn render_node_graph(
    ui: &mut Ui,
    scenario: &mut Scenario,
    state: &mut RenderState,
    view: &mut ScenarioViewState,
    show_grid_debug: bool,
) -> (ContextMenuAction, Vec2, bool, bool, bool, bool, bool) {
    let mut context_action = ContextMenuAction::None;
    let mut connection_created = false;
    let mut drag_started = false;
    let mut drag_ended = false;
    let mut resize_started = false;
    let mut resize_ended = false;
    let (response, painter) =
        ui.allocate_painter(ui.available_size(), egui::Sense::click_and_drag());

    let input_cache = InputCache::capture(ui, &response);

    let node_index = build_node_index(&scenario.nodes);

    let rect = response.rect;
    painter.rect_filled(rect, 0.0, Color32::from_rgb(40, 40, 40));

    let mouse_world_pos = if let Some(mouse_pos) = ui.ctx().pointer_hover_pos() {
        (mouse_pos.to_vec2() - view.pan_offset) / view.zoom
    } else {
        let viewport_center = ui.ctx().viewport_rect().center();
        (viewport_center.to_vec2() - view.pan_offset) / view.zoom
    };

    if response.hovered() {
        let scroll_delta = input_cache.scroll_delta;
        if scroll_delta != 0.0
            && let Some(mouse_pos) = ui.ctx().pointer_hover_pos()
        {
            let old_zoom = view.zoom;
            let zoom_delta = scroll_delta * UiConstants::ZOOM_DELTA_MULTIPLIER;
            view.zoom =
                (view.zoom + zoom_delta).clamp(UiConstants::ZOOM_MIN, UiConstants::ZOOM_MAX);

            let mouse_world_before = ((mouse_pos.to_vec2() - view.pan_offset) / old_zoom).to_pos2();
            let mouse_world_after = mouse_world_before;
            let mouse_screen_after =
                (mouse_world_after.to_vec2() * view.zoom + view.pan_offset).to_pos2();
            view.pan_offset += mouse_pos.to_vec2() - mouse_screen_after.to_vec2();

            if zoom_delta != 0.0 {
                view.connection_renderer.increment_generation();
            }
        }
    }

    let is_panning = input_cache.is_panning;

    if is_panning && response.dragged() && !*state.knife_tool_active {
        let pan_delta = response.drag_delta();
        view.pan_offset += pan_delta;
        view.connection_renderer.increment_generation();
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    }

    let alt_rmb = input_cache.alt_rmb;

    if alt_rmb && !*state.knife_tool_active {
        *state.knife_tool_active = true;
        state.knife_path.clear();
    }

    if *state.knife_tool_active && !alt_rmb {
        if !state.knife_path.is_empty() {
            let (connections_to_remove, _) = find_intersecting_connections(
                scenario,
                &node_index,
                state.knife_path,
                view.pan_offset,
                view.zoom,
                &mut view.connection_renderer,
            );
            for (from_node, to_node) in connections_to_remove {
                scenario
                    .connections
                    .retain(|c| !(c.from_node == from_node && c.to_node == to_node));
                connection_created = true;
            }
            scenario.obstacle_grid.invalidate();
        }
        *state.knife_tool_active = false;
        state.knife_path.clear();
    }

    if *state.knife_tool_active
        && alt_rmb
        && let Some(pos) = ui.ctx().pointer_interact_pos()
        && (state.knife_path.is_empty() || response.dragged())
    {
        state.knife_path.push(pos);
    }

    let pan_offset = view.pan_offset;
    let zoom = view.zoom;

    let to_screen = |pos: Pos2| -> Pos2 { (pos.to_vec2() * zoom + pan_offset).to_pos2() };

    draw_grid_transformed(&painter, rect, view.pan_offset, view.zoom);

    if scenario.obstacle_grid.is_dirty() {
        let connection_segments: Vec<_> = scenario
            .connections
            .iter()
            .filter_map(|conn| {
                let from_node = get_node_from_index(&scenario.nodes, &node_index, &conn.from_node)?;
                let to_node = get_node_from_index(&scenario.nodes, &node_index, &conn.to_node)?;
                Some((from_node.get_output_pin_pos(), to_node.get_input_pin_pos()))
            })
            .collect();
        scenario
            .obstacle_grid
            .rebuild(&scenario.nodes, &connection_segments);
    }

    if show_grid_debug {
        scenario
            .obstacle_grid
            .paint_debug(&painter, to_screen, rect);
    }

    view.connection_renderer.clear_cache();

    for connection in &scenario.connections {
        if let (Some(from_node), Some(to_node)) = (
            get_node_from_index(&scenario.nodes, &node_index, &connection.from_node),
            get_node_from_index(&scenario.nodes, &node_index, &connection.to_node),
        ) {
            let path =
                ConnectionPath::new(from_node, to_node, &scenario.nodes, &connection.branch_type);
            let color = get_connection_color(&connection.branch_type, from_node);
            path.draw(
                &painter,
                color,
                &mut view.connection_renderer,
                &connection.id,
                to_screen,
            );
        }
    }

    let mut clicked_node: Option<NanoId> = None;
    let mut any_node_hovered = false;
    let mut new_connection: Option<(NanoId, usize, NanoId)> = None;
    let shift_held = input_cache.shift_held;

    let box_select_start =
        ui.memory(|mem| mem.data.get_temp::<Pos2>(ui.id().with("box_select_start")));
    let mut box_select_rect: Option<Rect> = None;

    let is_left_drag = input_cache.is_left_drag;
    if !*state.knife_tool_active
        && !is_panning
        && is_left_drag
        && !any_node_hovered
        && let Some(pos) = ui.ctx().pointer_interact_pos()
    {
        ui.memory_mut(|mem| {
            mem.data.insert_temp(ui.id().with("box_select_start"), pos);
        });
    }

    if !*state.knife_tool_active
        && let Some(start) = box_select_start
        && let Some(current) = ui.ctx().pointer_latest_pos()
    {
        box_select_rect = Some(Rect::from_two_pos(start, current));

        if let Some(rect) = box_select_rect {
            painter.rect_stroke(
                rect,
                0.0,
                Stroke::new(1.0, Color32::from_rgb(100, 150, 255)),
                StrokeKind::Middle,
            );
            painter.rect_filled(
                rect,
                0.0,
                Color32::from_rgba_unmultiplied(100, 150, 255, 30),
            );
        }
    }

    if input_cache.pointer_any_released {
        if let Some(select_rect) = box_select_rect {
            if !shift_held {
                state.selected_nodes.clear();
            }

            for node in &scenario.nodes {
                let node_rect = egui::Rect::from_min_max(
                    to_screen(node.get_rect().min),
                    to_screen(node.get_rect().max),
                );
                if select_rect.intersects(node_rect) {
                    state.selected_nodes.insert(node.id.clone());
                }
            }
        }
        ui.memory_mut(|mem| {
            mem.data.remove::<Pos2>(ui.id().with("box_select_start"));
        });
    }

    let mut drag_delta_to_apply: Option<Vec2> = None;
    let mut node_being_dragged: Option<NanoId> = None;
    let mut node_drag_released: Option<NanoId> = None;

    let node_hovering_connection = if state.selected_nodes.len() == 1
        && let Some(selected_id) = state.selected_nodes.iter().next().cloned()
        && let Some(selected_node) = get_node_from_index(&scenario.nodes, &node_index, &selected_id)
        && input_cache.pointer_primary_down
        && state.connection_from.is_none()
    {
        let node_center_screen = to_screen(selected_node.get_rect().center());
        if find_connection_near_point(
            scenario,
            &node_index,
            node_center_screen,
            view.pan_offset,
            view.zoom,
            UiConstants::LINK_INSERT_THRESHOLD * view.zoom,
            &mut view.connection_renderer,
        )
        .is_some()
        {
            Some(selected_id)
        } else {
            None
        }
    } else {
        None
    };

    if let Some((resizing_id, handle)) = state.resizing_node.as_ref()
        && let Some(node) = scenario.nodes.iter_mut().find(|n| n.id == *resizing_id)
    {
        if input_cache.pointer_any_released {
            if node.is_routable() {
                let (snapped_pos, snapped_w, snapped_h) =
                    node.snap_bounds(UiConstants::GRID_CELL_SIZE);
                if (snapped_pos.x - node.position.x).abs() > 0.001
                    || (snapped_pos.y - node.position.y).abs() > 0.001
                    || (snapped_w - node.width).abs() > 0.001
                    || (snapped_h - node.height).abs() > 0.001
                {
                    node.position = snapped_pos;
                    node.width = snapped_w;
                    node.height = snapped_h;
                    view.connection_renderer.increment_generation();
                    scenario.obstacle_grid.invalidate();
                }
            }
            *state.resizing_node = None;
            resize_ended = true;
        } else {
            let delta = input_cache.pointer_delta;
            let delta_world = delta / view.zoom;
            match handle {
                ResizeHandle::Right => {
                    node.width = (node.width + delta_world.x).max(UiConstants::NOTE_MIN_WIDTH);
                    view.connection_renderer.increment_generation();
                }
                ResizeHandle::Left => {
                    let new_width = (node.width - delta_world.x).max(UiConstants::NOTE_MIN_WIDTH);
                    let width_change = node.width - new_width;
                    node.position.x += width_change;
                    node.width = new_width;
                    view.connection_renderer.increment_generation();
                }
                ResizeHandle::Bottom => {
                    node.height = (node.height + delta_world.y).max(UiConstants::NOTE_MIN_HEIGHT);
                    view.connection_renderer.increment_generation();
                }
                ResizeHandle::Top => {
                    let new_height =
                        (node.height - delta_world.y).max(UiConstants::NOTE_MIN_HEIGHT);
                    let height_change = node.height - new_height;
                    node.position.y += height_change;
                    node.height = new_height;
                    view.connection_renderer.increment_generation();
                }
                ResizeHandle::BottomRight => {
                    node.width = (node.width + delta_world.x).max(UiConstants::NOTE_MIN_WIDTH);
                    node.height = (node.height + delta_world.y).max(UiConstants::NOTE_MIN_HEIGHT);
                    view.connection_renderer.increment_generation();
                }
                ResizeHandle::BottomLeft => {
                    let new_width = (node.width - delta_world.x).max(UiConstants::NOTE_MIN_WIDTH);
                    let width_change = node.width - new_width;
                    node.position.x += width_change;
                    node.width = new_width;
                    node.height = (node.height + delta_world.y).max(UiConstants::NOTE_MIN_HEIGHT);
                    view.connection_renderer.increment_generation();
                }
                ResizeHandle::TopRight => {
                    node.width = (node.width + delta_world.x).max(UiConstants::NOTE_MIN_WIDTH);
                    let new_height =
                        (node.height - delta_world.y).max(UiConstants::NOTE_MIN_HEIGHT);
                    let height_change = node.height - new_height;
                    node.position.y += height_change;
                    node.height = new_height;
                    view.connection_renderer.increment_generation();
                }
                ResizeHandle::TopLeft => {
                    let new_width = (node.width - delta_world.x).max(UiConstants::NOTE_MIN_WIDTH);
                    let width_change = node.width - new_width;
                    node.position.x += width_change;
                    node.width = new_width;
                    let new_height =
                        (node.height - delta_world.y).max(UiConstants::NOTE_MIN_HEIGHT);
                    let height_change = node.height - new_height;
                    node.position.y += height_change;
                    node.height = new_height;
                    view.connection_renderer.increment_generation();
                }
            }
            if let Activity::Note { width, height, .. } = &mut node.activity {
                *width = node.width;
                *height = node.height;
            }
        }
    }

    for i in (0..scenario.nodes.len()).rev() {
        let node = &mut scenario.nodes[i];
        let is_selected = state.selected_nodes.contains(&node.id);

        let node_rect_world = node.get_rect();
        let node_rect_screen = egui::Rect::from_min_max(
            to_screen(node_rect_world.min),
            to_screen(node_rect_world.max),
        );

        let node_response = ui.interact(
            node_rect_screen,
            ui.id().with(node.id.clone()),
            egui::Sense::click_and_drag(),
        );

        let mut handle_interaction = false;
        let is_note_node = matches!(node.activity, Activity::Note { .. });
        let can_resize = is_note_node || state.allow_node_resize;

        if can_resize
            && state.connection_from.is_none()
            && !is_panning
            && state.resizing_node.is_none()
        {
            let handle_size = UiConstants::NOTE_RESIZE_HANDLE_SIZE * view.zoom;

            let corner_br = egui::Rect::from_min_size(
                node_rect_screen.max - egui::vec2(handle_size, handle_size),
                egui::vec2(handle_size, handle_size),
            );
            let corner_bl = egui::Rect::from_min_size(
                egui::pos2(node_rect_screen.min.x, node_rect_screen.max.y - handle_size),
                egui::vec2(handle_size, handle_size),
            );
            let corner_tr = egui::Rect::from_min_size(
                egui::pos2(node_rect_screen.max.x - handle_size, node_rect_screen.min.y),
                egui::vec2(handle_size, handle_size),
            );
            let corner_tl = egui::Rect::from_min_size(
                node_rect_screen.min,
                egui::vec2(handle_size, handle_size),
            );

            let edge_right = egui::Rect::from_min_size(
                egui::pos2(
                    node_rect_screen.max.x - handle_size,
                    node_rect_screen.min.y + handle_size,
                ),
                egui::vec2(handle_size, node_rect_screen.height() - handle_size * 2.0),
            );
            let edge_left = egui::Rect::from_min_size(
                egui::pos2(node_rect_screen.min.x, node_rect_screen.min.y + handle_size),
                egui::vec2(handle_size, node_rect_screen.height() - handle_size * 2.0),
            );
            let edge_bottom = egui::Rect::from_min_size(
                egui::pos2(
                    node_rect_screen.min.x + handle_size,
                    node_rect_screen.max.y - handle_size,
                ),
                egui::vec2(node_rect_screen.width() - handle_size * 2.0, handle_size),
            );
            let edge_top = egui::Rect::from_min_size(
                egui::pos2(node_rect_screen.min.x + handle_size, node_rect_screen.min.y),
                egui::vec2(node_rect_screen.width() - handle_size * 2.0, handle_size),
            );

            let br_response = ui.interact(
                corner_br,
                ui.id().with(("resize_br", node.id.clone())),
                egui::Sense::click_and_drag(),
            );
            let bl_response = ui.interact(
                corner_bl,
                ui.id().with(("resize_bl", node.id.clone())),
                egui::Sense::click_and_drag(),
            );
            let tr_response = ui.interact(
                corner_tr,
                ui.id().with(("resize_tr", node.id.clone())),
                egui::Sense::click_and_drag(),
            );
            let tl_response = ui.interact(
                corner_tl,
                ui.id().with(("resize_tl", node.id.clone())),
                egui::Sense::click_and_drag(),
            );
            let r_response = ui.interact(
                edge_right,
                ui.id().with(("resize_r", node.id.clone())),
                egui::Sense::click_and_drag(),
            );
            let l_response = ui.interact(
                edge_left,
                ui.id().with(("resize_l", node.id.clone())),
                egui::Sense::click_and_drag(),
            );
            let b_response = ui.interact(
                edge_bottom,
                ui.id().with(("resize_b", node.id.clone())),
                egui::Sense::click_and_drag(),
            );
            let t_response = ui.interact(
                edge_top,
                ui.id().with(("resize_t", node.id.clone())),
                egui::Sense::click_and_drag(),
            );

            if br_response.hovered()
                || br_response.dragged()
                || tl_response.hovered()
                || tl_response.dragged()
            {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeNwSe);
                any_node_hovered = true;
                handle_interaction = true;
            } else if bl_response.hovered()
                || bl_response.dragged()
                || tr_response.hovered()
                || tr_response.dragged()
            {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeNeSw);
                any_node_hovered = true;
                handle_interaction = true;
            } else if r_response.hovered()
                || r_response.dragged()
                || l_response.hovered()
                || l_response.dragged()
            {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                any_node_hovered = true;
                handle_interaction = true;
            } else if b_response.hovered()
                || b_response.dragged()
                || t_response.hovered()
                || t_response.dragged()
            {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
                any_node_hovered = true;
                handle_interaction = true;
            }

            if br_response.drag_started() {
                *state.resizing_node = Some((node.id.clone(), ResizeHandle::BottomRight));
                resize_started = true;
            } else if bl_response.drag_started() {
                *state.resizing_node = Some((node.id.clone(), ResizeHandle::BottomLeft));
                resize_started = true;
            } else if tr_response.drag_started() {
                *state.resizing_node = Some((node.id.clone(), ResizeHandle::TopRight));
                resize_started = true;
            } else if tl_response.drag_started() {
                *state.resizing_node = Some((node.id.clone(), ResizeHandle::TopLeft));
                resize_started = true;
            } else if r_response.drag_started() {
                *state.resizing_node = Some((node.id.clone(), ResizeHandle::Right));
                resize_started = true;
            } else if l_response.drag_started() {
                *state.resizing_node = Some((node.id.clone(), ResizeHandle::Left));
                resize_started = true;
            } else if b_response.drag_started() {
                *state.resizing_node = Some((node.id.clone(), ResizeHandle::Bottom));
                resize_started = true;
            } else if t_response.drag_started() {
                *state.resizing_node = Some((node.id.clone(), ResizeHandle::Top));
                resize_started = true;
            }
        }

        if node_response.dragged()
            && state.connection_from.is_none()
            && !is_panning
            && state.resizing_node.is_none()
            && !handle_interaction
            && !*state.knife_tool_active
        {
            if !is_selected {
                node_being_dragged = Some(node.id.clone());
                drag_started = true;
            }

            drag_delta_to_apply = Some(node_response.drag_delta() / view.zoom);
        }

        if node_response.drag_stopped()
            && state.connection_from.is_none()
            && !is_panning
            && is_selected
            && state.selected_nodes.len() == 1
        {
            node_drag_released = Some(node.id.clone());
            drag_ended = true;
        }

        if node_response.clicked()
            && clicked_node.is_none()
            && state.connection_from.is_none()
            && !*state.knife_tool_active
        {
            clicked_node = Some(node.id.clone());
        }

        if !*state.knife_tool_active {
            Popup::context_menu(&node_response)
                .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
                .show(|ui| {
                    if !state.selected_nodes.contains(&node.id) {
                        state.selected_nodes.clear();
                        state.selected_nodes.insert(node.id.clone());
                    }

                    ui.set_min_width(150.0);

                    if ui.button(t!("context_menu.copy").as_ref()).clicked() {
                        context_action = ContextMenuAction::Copy;
                        ui.close();
                    }
                    if ui.button(t!("context_menu.cut").as_ref()).clicked() {
                        context_action = ContextMenuAction::Cut;
                        ui.close();
                    }
                    if ui.button(t!("context_menu.paste").as_ref()).clicked() {
                        context_action = ContextMenuAction::Paste;
                        ui.close();
                    }
                    if ui.button(t!("context_menu.delete").as_ref()).clicked() {
                        context_action = ContextMenuAction::Delete;
                        ui.close();
                    }
                });
        }

        if node_response.hovered() {
            any_node_hovered = true;
        }

        let is_hovering_connection = node_hovering_connection.as_ref() == Some(&node.id);
        let is_being_resized = state
            .resizing_node
            .as_ref()
            .map(|(id, _)| id == &node.id)
            .unwrap_or(false);

        draw_node_transformed(
            &painter,
            node,
            is_selected,
            is_hovering_connection,
            is_being_resized,
            to_screen,
            view.zoom,
        );
    }

    if let Some(dragged_id) = node_being_dragged {
        if !shift_held {
            state.selected_nodes.clear();
        }
        state.selected_nodes.insert(dragged_id);
    }

    if let Some(drag_delta) = drag_delta_to_apply {
        for node in &mut scenario.nodes {
            if state.selected_nodes.contains(&node.id) {
                node.position += drag_delta;
            }
        }

        view.connection_renderer.increment_generation();
    }

    if let Some(released_node_id) = node_drag_released {
        let mut reroute_needed = false;

        for node in &mut scenario.nodes {
            if state.selected_nodes.contains(&node.id) && node.is_routable() {
                let snapped_x = snap_to_grid(node.position.x, UiConstants::GRID_CELL_SIZE);
                let snapped_y = snap_to_grid(node.position.y, UiConstants::GRID_CELL_SIZE);

                if (snapped_x - node.position.x).abs() > 0.001
                    || (snapped_y - node.position.y).abs() > 0.001
                {
                    reroute_needed = true;
                }

                node.position.x = snapped_x;
                node.position.y = snapped_y;
            }
        }

        if reroute_needed {
            view.connection_renderer.increment_generation();
            scenario.obstacle_grid.invalidate();
        }

        if let Some(released_node) =
            get_node_from_index(&scenario.nodes, &node_index, &released_node_id)
        {
            let node_center_screen = to_screen(released_node.get_rect().center());

            if let Some((from_id, to_id)) = find_connection_near_point(
                scenario,
                &node_index,
                node_center_screen,
                view.pan_offset,
                view.zoom,
                UiConstants::LINK_INSERT_THRESHOLD * view.zoom,
                &mut view.connection_renderer,
            ) && released_node_id != from_id
                && released_node_id != to_id
                && let Some(conn_to_remove) = scenario
                    .connections
                    .iter()
                    .find(|c| c.from_node == from_id && c.to_node == to_id)
                    .cloned()
            {
                let branch_type = conn_to_remove.branch_type.clone();

                scenario
                    .connections
                    .retain(|c| !(c.from_node == from_id && c.to_node == to_id));

                if let (Some(from_node), Some(to_node)) = (
                    get_node_from_index(&scenario.nodes, &node_index, &from_id),
                    get_node_from_index(&scenario.nodes, &node_index, &to_id),
                ) {
                    let from_pos = from_node.position;
                    let to_pos = to_node.position;
                    let mid_x = (from_pos.x + to_pos.x) * 0.5;
                    let mid_y = (from_pos.y + to_pos.y) * 0.5;

                    let distance = to_pos.y - from_pos.y;
                    let required = UiConstants::MIN_NODE_SPACING * 2.0;

                    if distance < required {
                        let push = required - distance;

                        for node in &mut scenario.nodes {
                            if node.position.y >= to_pos.y {
                                node.position.y += push;
                            }
                        }
                    }

                    for node in &mut scenario.nodes {
                        if node.id == released_node_id {
                            node.position.x = mid_x;
                            node.position.y = mid_y;
                            break;
                        }
                    }
                }

                scenario.add_connection_with_branch(from_id, released_node_id.clone(), branch_type);
                scenario.add_connection_with_branch(released_node_id, to_id, BranchType::Default);
                scenario.obstacle_grid.invalidate();
            }
        }
    }

    for i in (0..scenario.nodes.len()).rev() {
        let node = &scenario.nodes[i];

        if node.has_output_pin() {
            let pin_count = node.get_output_pin_count();

            for pin_index in 0..pin_count {
                let output_pin = to_screen(node.get_output_pin_pos_by_index(pin_index));
                let output_pin_rect = egui::Rect::from_center_size(
                    output_pin,
                    egui::vec2(
                        UiConstants::PIN_INTERACT_SIZE * view.zoom,
                        UiConstants::PIN_INTERACT_SIZE * view.zoom,
                    ),
                );
                let output_response = ui.interact(
                    output_pin_rect,
                    ui.id().with(("output", node.id.clone(), pin_index)),
                    egui::Sense::click_and_drag(),
                );

                if output_response.drag_started() && !*state.knife_tool_active {
                    *state.connection_from = Some((node.id.clone(), pin_index));

                    let branch_type = node.get_branch_type_for_pin(pin_index);

                    if node.get_output_pin_count() > 1 {
                        scenario
                            .connections
                            .retain(|c| !(c.from_node == node.id && c.branch_type == branch_type));
                    } else {
                        scenario.connections.retain(|c| c.from_node != node.id);
                    }
                    scenario.obstacle_grid.invalidate();
                }
            }
        }

        if node.has_input_pin() {
            let input_pin = to_screen(node.get_input_pin_pos());
            let input_pin_rect = egui::Rect::from_center_size(
                input_pin,
                egui::vec2(
                    UiConstants::PIN_INTERACT_SIZE * view.zoom,
                    UiConstants::PIN_INTERACT_SIZE * view.zoom,
                ),
            );
            let input_response = ui.interact(
                input_pin_rect,
                ui.id().with(("input", node.id.clone())),
                egui::Sense::click_and_drag(),
            );

            let node_rect_world = node.get_rect();
            let node_rect_screen = egui::Rect::from_min_max(
                to_screen(node_rect_world.min),
                to_screen(node_rect_world.max),
            );
            let node_body_response = ui.interact(
                node_rect_screen,
                ui.id().with(("input_body", node.id.clone())),
                egui::Sense::hover(),
            );

            if input_response.drag_started()
                && !*state.knife_tool_active
                && let Some(conn) = scenario
                    .connections
                    .iter()
                    .find(|c| c.to_node == node.id)
                    .cloned()
            {
                if let Some(from_node) =
                    get_node_from_index(&scenario.nodes, &node_index, &conn.from_node)
                {
                    let pin_index = from_node.get_pin_index_for_branch(&conn.branch_type);
                    *state.connection_from = Some((conn.from_node.clone(), pin_index));
                }

                scenario
                    .connections
                    .retain(|c| c.from_node == conn.from_node && c.to_node == node.id);
            }

            if let Some((from_id, pin_index)) = state.connection_from.as_ref()
                && (input_response.hovered() || node_body_response.hovered())
                && input_cache.pointer_any_released
                && !*state.knife_tool_active
            {
                if from_id != &node.id {
                    new_connection = Some((from_id.clone(), *pin_index, node.id.clone()));
                }
                *state.connection_from = None;
            }
        }
    }

    if let Some((from_id, pin_index)) = state.connection_from.as_ref() {
        if let Some(from_node) = get_node_from_index(&scenario.nodes, &node_index, from_id)
            && let Some(pointer_pos) = ui.ctx().pointer_latest_pos()
        {
            let start = to_screen(from_node.get_output_pin_pos_by_index(*pin_index));
            let end = pointer_pos;

            let branch_type = from_node.get_branch_type_for_pin(*pin_index);
            let preview_color = match branch_type {
                BranchType::TrueBranch => ColorPalette::CONNECTION_TRUE,
                BranchType::FalseBranch => ColorPalette::CONNECTION_FALSE,
                BranchType::LoopBody => ColorPalette::CONNECTION_LOOP_BODY,
                BranchType::ErrorBranch => ColorPalette::CONNECTION_ERROR,
                BranchType::TryBranch => ColorPalette::CONNECTION_DEFAULT,
                BranchType::CatchBranch => ColorPalette::CONNECTION_ERROR,
                BranchType::Default => ColorPalette::CONNECTION_DEFAULT,
            };

            painter.line_segment([start, end], Stroke::new(2.0 * view.zoom, preview_color));
        }

        if input_cache.key_escape || response.secondary_clicked() {
            *state.connection_from = None;
        }

        if input_cache.pointer_any_released && !any_node_hovered {
            *state.connection_from = None;
        }
    }

    if let Some((from, pin_index, to)) = new_connection {
        let from_node = get_node_from_index(&scenario.nodes, &node_index, &from).unwrap();
        let branch_type = from_node.get_branch_type_for_pin(pin_index);

        if from_node.get_output_pin_count() > 1 {
            scenario
                .connections
                .retain(|c| !(c.from_node == from && c.branch_type == branch_type));
        } else {
            scenario.connections.retain(|c| c.from_node != from);
        }

        scenario.add_connection_with_branch(from, to, branch_type);
        scenario.obstacle_grid.invalidate();
        connection_created = true;
    }

    if let Some(clicked) = clicked_node {
        if shift_held {
            if state.selected_nodes.contains(&clicked) {
                state.selected_nodes.remove(&clicked);
            } else {
                state.selected_nodes.insert(clicked);
            }
        } else {
            state.selected_nodes.clear();
            state.selected_nodes.insert(clicked);
        }
    } else if response.clicked()
        && !any_node_hovered
        && box_select_rect.is_none()
        && !*state.knife_tool_active
    {
        state.selected_nodes.clear();
    }

    if !*state.knife_tool_active
        && let Some(action) = canvas_context_menu(state, &response)
    {
        context_action = action;
    }

    if state.show_minimap {
        let minimap_layer =
            egui::LayerId::new(egui::Order::Foreground, egui::Id::new("minimap_layer"));
        let minimap_painter = ui.ctx().layer_painter(minimap_layer);
        render_minimap_internal(
            &minimap_painter,
            scenario,
            &node_index,
            view.pan_offset,
            view.zoom,
            rect,
        );
    }

    if *state.knife_tool_active {
        if ui.ctx().pointer_latest_pos().is_some() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
        }

        if state.knife_path.len() > 1 {
            painter.add(egui::Shape::line(
                state.knife_path.clone(),
                Stroke::new(3.0, Color32::from_rgb(255, 100, 100)),
            ));

            let (_, intersection_points) = find_intersecting_connections(
                scenario,
                &node_index,
                state.knife_path,
                view.pan_offset,
                view.zoom,
                &mut view.connection_renderer,
            );

            for point in intersection_points {
                let size = 8.0;
                painter.line_segment(
                    [
                        Pos2::new(point.x - size, point.y - size),
                        Pos2::new(point.x + size, point.y + size),
                    ],
                    Stroke::new(3.0, Color32::from_rgb(255, 0, 0)),
                );
                painter.line_segment(
                    [
                        Pos2::new(point.x - size, point.y + size),
                        Pos2::new(point.x + size, point.y - size),
                    ],
                    Stroke::new(3.0, Color32::from_rgb(255, 0, 0)),
                );
            }
        }
    }

    (
        context_action,
        mouse_world_pos,
        connection_created,
        drag_started,
        drag_ended,
        resize_started,
        resize_ended,
    )
}

fn draw_grid_transformed(painter: &egui::Painter, rect: Rect, pan_offset: Vec2, zoom: f32) {
    if zoom < UiConstants::GRID_MIN_ZOOM {
        return;
    }

    let world_grid_spacing = UiConstants::GRID_SPACING;
    let grid_color = Color32::from_rgb(50, 50, 50);

    let world_min_x = (rect.left() - pan_offset.x) / zoom;
    let world_max_x = (rect.right() - pan_offset.x) / zoom;
    let world_min_y = (rect.top() - pan_offset.y) / zoom;
    let world_max_y = (rect.bottom() - pan_offset.y) / zoom;

    let grid_min_x = (world_min_x / world_grid_spacing).floor() * world_grid_spacing;
    let grid_max_x = (world_max_x / world_grid_spacing).ceil() * world_grid_spacing;
    let grid_min_y = (world_min_y / world_grid_spacing).floor() * world_grid_spacing;
    let grid_max_y = (world_max_y / world_grid_spacing).ceil() * world_grid_spacing;

    let num_x_lines = ((grid_max_x - grid_min_x) / world_grid_spacing).ceil() as usize;
    let num_y_lines = ((grid_max_y - grid_min_y) / world_grid_spacing).ceil() as usize;

    if num_x_lines > UiConstants::MAX_GRID_LINES || num_y_lines > UiConstants::MAX_GRID_LINES {
        return;
    }

    let mut grid_x = grid_min_x;
    while grid_x <= grid_max_x {
        let screen_x = grid_x * zoom + pan_offset.x;
        painter.line_segment(
            [
                Pos2::new(screen_x, rect.top()),
                Pos2::new(screen_x, rect.bottom()),
            ],
            Stroke::new(1.0, grid_color),
        );
        grid_x += world_grid_spacing;
    }

    let mut grid_y = grid_min_y;
    while grid_y <= grid_max_y {
        let screen_y = grid_y * zoom + pan_offset.y;
        painter.line_segment(
            [
                Pos2::new(rect.left(), screen_y),
                Pos2::new(rect.right(), screen_y),
            ],
            Stroke::new(1.0, grid_color),
        );
        grid_y += world_grid_spacing;
    }
}

pub fn draw_node_transformed<F>(
    painter: &egui::Painter,
    node: &Node,
    is_selected: bool,
    is_hovering_connection: bool,
    is_being_resized: bool,
    to_screen: F,
    zoom: f32,
) where
    F: Fn(Pos2) -> Pos2,
{
    let rect_world = node.get_rect();
    let rect = egui::Rect::from_min_max(to_screen(rect_world.min), to_screen(rect_world.max));
    let mut color = node.activity.get_color();
    let rounding = UiConstants::NODE_ROUNDING * zoom;

    if is_hovering_connection {
        let r = color.r().saturating_add(30);
        let g = color.g().saturating_add(30);
        let b = color.b().saturating_add(30);
        color = Color32::from_rgb(r, g, b);
    }

    let shadow_offset = Vec2::new(
        UiConstants::NODE_SHADOW_OFFSET * zoom,
        UiConstants::NODE_SHADOW_OFFSET * zoom,
    );
    painter.rect_filled(
        rect.translate(shadow_offset),
        rounding,
        Color32::from_black_alpha(100),
    );

    painter.rect_filled(rect, rounding, color);

    if is_being_resized {
        painter.rect_stroke(
            rect,
            rounding,
            Stroke::new(4.0 * zoom, Color32::from_rgb(100, 150, 255)),
            StrokeKind::Outside,
        );
    } else if is_selected {
        painter.rect_stroke(
            rect,
            rounding,
            Stroke::new(2.0 * zoom, Color32::from_rgb(255, 255, 0)),
            StrokeKind::Outside,
        );
    } else {
        painter.rect_stroke(
            rect,
            rounding,
            Stroke::new(1.0 * zoom, Color32::from_rgb(20, 20, 20)),
            StrokeKind::Outside,
        );
    }

    if let Activity::Note { text, .. } = &node.activity {
        let padding = UiConstants::NOTE_PADDING * zoom;
        let text_rect = egui::Rect::from_min_max(
            rect.min + Vec2::new(padding, padding),
            rect.max - Vec2::new(padding, padding),
        );

        let font_id = egui::FontId::proportional(12.0 * zoom);
        let mut job = egui::text::LayoutJob::default();
        job.wrap.max_width = text_rect.width();
        job.append(
            text,
            0.0,
            egui::text::TextFormat {
                font_id: font_id.clone(),
                color: Color32::from_rgb(60, 60, 60),
                ..Default::default()
            },
        );

        let galley = painter.layout_job(job);
        painter.galley(text_rect.min, galley, Color32::from_rgb(60, 60, 60));
    } else if zoom >= UiConstants::GRID_MIN_ZOOM {
        let text_pos = rect.min + Vec2::new(10.0 * zoom, 10.0 * zoom);
        painter.text(
            text_pos,
            egui::Align2::LEFT_TOP,
            node.activity.get_name(),
            egui::FontId::proportional(14.0 * zoom),
            Color32::WHITE,
        );
    }

    if zoom >= UiConstants::GRID_MIN_ZOOM && node.has_input_pin() {
        let input_pin = to_screen(node.get_input_pin_pos());
        painter.circle_filled(
            input_pin,
            UiConstants::PIN_RADIUS * zoom,
            Color32::from_rgb(150, 150, 150),
        );
        painter.circle_stroke(
            input_pin,
            UiConstants::PIN_RADIUS * zoom,
            Stroke::new(1.0 * zoom, Color32::from_rgb(80, 80, 80)),
        );
    }

    if node.has_output_pin() {
        let positions = node.get_output_pin_positions();
        for (pin_index, &pos) in positions.iter().enumerate() {
            let pin_screen = to_screen(pos);
            let branch_type = node.get_branch_type_for_pin(pin_index);

            let (color, stroke_color, label) = match branch_type {
                BranchType::TrueBranch => {
                    (ColorPalette::PIN_TRUE, Color32::from_rgb(60, 120, 60), "T")
                }
                BranchType::FalseBranch => {
                    (ColorPalette::PIN_FALSE, Color32::from_rgb(120, 60, 60), "F")
                }
                BranchType::LoopBody => (
                    ColorPalette::PIN_LOOP_BODY,
                    Color32::from_rgb(200, 120, 0),
                    "B",
                ),
                BranchType::ErrorBranch => {
                    (ColorPalette::PIN_ERROR, Color32::from_rgb(120, 60, 60), "E")
                }
                BranchType::TryBranch => (
                    ColorPalette::PIN_SUCCESS,
                    Color32::from_rgb(60, 120, 60),
                    "T",
                ),
                BranchType::CatchBranch => {
                    (ColorPalette::PIN_ERROR, Color32::from_rgb(120, 60, 60), "C")
                }
                BranchType::Default => {
                    if node.get_output_pin_count() > 1 {
                        (
                            ColorPalette::PIN_LOOP_NEXT,
                            Color32::from_rgb(80, 80, 80),
                            "N",
                        )
                    } else {
                        (ColorPalette::PIN_DEFAULT, Color32::from_rgb(80, 80, 80), "")
                    }
                }
            };

            if zoom >= UiConstants::GRID_MIN_ZOOM {
                painter.circle_filled(pin_screen, UiConstants::PIN_RADIUS * zoom, color);
                painter.circle_stroke(
                    pin_screen,
                    UiConstants::PIN_RADIUS * zoom,
                    Stroke::new(1.0 * zoom, stroke_color),
                );

                if !label.is_empty() {
                    let (label_offset, label_align) =
                        (Vec2::new(0.0, -12.0 * zoom), egui::Align2::CENTER_BOTTOM);
                    painter.text(
                        pin_screen + label_offset,
                        label_align,
                        label,
                        egui::FontId::proportional(10.0 * zoom),
                        color,
                    );
                }
            }
        }
    }
}

fn render_minimap_internal(
    painter: &egui::Painter,
    scenario: &Scenario,
    node_index: &NodeIndex,
    pan_offset: Vec2,
    zoom: f32,
    canvas_rect: egui::Rect,
) {
    let minimap_size = egui::vec2(UiConstants::MINIMAP_WIDTH, UiConstants::MINIMAP_HEIGHT);
    let minimap_pos = canvas_rect.max
        - minimap_size
        - egui::vec2(UiConstants::MINIMAP_OFFSET_X, UiConstants::MINIMAP_OFFSET_Y);
    let minimap_rect = egui::Rect::from_min_size(minimap_pos, minimap_size);

    painter.rect_filled(
        minimap_rect,
        5.0,
        egui::Color32::from_rgba_unmultiplied(30, 30, 30, 200),
    );
    painter.rect_stroke(
        minimap_rect,
        5.0,
        egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 100, 100)),
        egui::StrokeKind::Outside,
    );

    if scenario.nodes.is_empty() {
        return;
    }

    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;

    for node in &scenario.nodes {
        let rect = node.get_rect();
        min_x = min_x.min(rect.min.x);
        min_y = min_y.min(rect.min.y);
        max_x = max_x.max(rect.max.x);
        max_y = max_y.max(rect.max.y);
    }

    min_x -= UiConstants::MINIMAP_WORLD_PADDING;
    min_y -= UiConstants::MINIMAP_WORLD_PADDING;
    max_x += UiConstants::MINIMAP_WORLD_PADDING;
    max_y += UiConstants::MINIMAP_WORLD_PADDING;

    let world_width = max_x - min_x;
    let world_height = max_y - min_y;

    let scale_x = (minimap_size.x - UiConstants::MINIMAP_PADDING * 2.0) / world_width;
    let scale_y = (minimap_size.y - UiConstants::MINIMAP_PADDING * 2.0) / world_height;
    let minimap_scale = scale_x.min(scale_y);

    let to_minimap = |world_pos: egui::Pos2| -> egui::Pos2 {
        let relative_x = (world_pos.x - min_x) * minimap_scale;
        let relative_y = (world_pos.y - min_y) * minimap_scale;
        minimap_rect.min
            + egui::vec2(
                relative_x + UiConstants::MINIMAP_PADDING,
                relative_y + UiConstants::MINIMAP_PADDING,
            )
    };

    for node in &scenario.nodes {
        let node_rect = node.get_rect();
        let minimap_min = to_minimap(node_rect.min);
        let minimap_max = to_minimap(node_rect.max);
        let node_minimap_rect = egui::Rect::from_min_max(minimap_min, minimap_max);

        painter.rect_filled(node_minimap_rect, 1.0, node.activity.get_color());
    }

    for connection in &scenario.connections {
        if let (Some(from_node), Some(to_node)) = (
            get_node_from_index(&scenario.nodes, node_index, &connection.from_node),
            get_node_from_index(&scenario.nodes, node_index, &connection.to_node),
        ) {
            let from_pos = to_minimap(from_node.get_output_pin_pos());
            let to_pos = to_minimap(to_node.get_input_pin_pos());
            painter.line_segment(
                [from_pos, to_pos],
                egui::Stroke::new(1.0, egui::Color32::from_rgb(150, 150, 150)),
            );
        }
    }

    let viewport_world_min = (-pan_offset) / zoom;
    let viewport_world_max = (canvas_rect.size() - pan_offset) / zoom;

    let viewport_minimap_min = to_minimap(viewport_world_min.to_pos2());
    let viewport_minimap_max = to_minimap(viewport_world_max.to_pos2());
    let viewport_minimap_rect =
        egui::Rect::from_min_max(viewport_minimap_min, viewport_minimap_max);

    let clipped_viewport_rect = viewport_minimap_rect.intersect(minimap_rect);

    painter.rect_stroke(
        clipped_viewport_rect,
        2.0,
        egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 150, 255)),
        egui::StrokeKind::Outside,
    );
    painter.rect_filled(
        clipped_viewport_rect,
        2.0,
        egui::Color32::from_rgba_unmultiplied(100, 150, 255, 30),
    );
}

pub fn render_node_properties(
    ui: &mut Ui,
    node: &mut Node,
    scenarios: &Vec<Scenario>,
) -> (bool, ParameterBindingAction) {
    let original_activity = node.activity.clone();

    ui.horizontal(|ui| {
        ui.strong(node.activity.get_name());
        ui.label("💡")
            .on_hover_text(t!("tooltips.variable_syntax").as_ref());
    });
    ui.separator();

    let metadata = ActivityMetadata::for_activity(&node.activity);
    let mut param_action = ParameterBindingAction::None;

    for (prop_idx, prop_def) in metadata.properties.iter().enumerate() {
        let label = t!(prop_def.label_key).to_string();

        match prop_def.property_type {
            PropertyType::Description => {
                ui.label(label);
            }
            PropertyType::TextSingleLine => {
                let mut label_widget = ui.label(&label);
                if let Some(tooltip) = prop_def.tooltip_key {
                    label_widget = label_widget.on_hover_text(t!(tooltip).as_ref());
                }

                match &mut node.activity {
                    Activity::SetVariable { name, .. } if prop_idx == 0 => {
                        ui.text_edit_singleline(name);
                    }
                    Activity::SetVariable { var_type, .. } if prop_idx == 1 => {
                        egui::ComboBox::from_id_salt("var_type_combo")
                            .selected_text(var_type.as_str())
                            .show_ui(ui, |ui| {
                                for vt in rpa_core::VariableType::all() {
                                    if ui.selectable_label(*var_type == vt, vt.as_str()).clicked() {
                                        *var_type = vt;
                                    }
                                }
                            });
                    }
                    Activity::SetVariable {
                        value, var_type, ..
                    } if prop_idx == 2 => match var_type {
                        VariableType::String => {
                            ui.text_edit_singleline(value);
                        }
                        VariableType::Number => {
                            let mut n: f64 = value.parse().unwrap_or(0.0);
                            ui.add(egui::DragValue::new(&mut n));
                            *value = n.to_string();
                        }
                        VariableType::Boolean => {
                            let mut b: bool =
                                matches!(value.to_lowercase().as_str(), "true" | "1" | "yes");
                            ui.checkbox(&mut b, "");
                            *value = b.to_string();
                        }
                    },
                    Activity::IfCondition { condition } | Activity::While { condition } => {
                        ui.text_edit_singleline(condition);
                    }
                    Activity::Loop { index, .. } if prop_idx == 0 => {
                        ui.text_edit_singleline(index);
                    }
                    Activity::Evaluate { expression } if prop_idx == 0 => {
                        ui.text_edit_singleline(expression);
                    }
                    _ => {}
                }
            }
            PropertyType::TextMultiLine => {
                ui.label(&label);

                match &mut node.activity {
                    Activity::Log { level, message } => {
                        ui.text_edit_multiline(message);

                        egui::ComboBox::from_id_salt("log_level_selector")
                            .selected_text(level.as_str())
                            .show_ui(ui, |ui| {
                                ui.selectable_value(level, LogLevel::Info, "INFO");
                                ui.selectable_value(level, LogLevel::Warning, "WARN");
                                ui.selectable_value(level, LogLevel::Error, "ERROR");
                                ui.selectable_value(level, LogLevel::Debug, "DEBUG");
                            });
                    }
                    Activity::Note { text, .. } => {
                        ui.text_edit_multiline(text);
                    }
                    _ => {}
                }
            }
            PropertyType::Slider => {
                ui.label(&label);
            }
            PropertyType::DragInt => {
                ui.label(&label);

                match &mut node.activity {
                    Activity::Loop {
                        start, end, step, ..
                    } => match prop_idx {
                        1 => {
                            ui.add(egui::DragValue::new(start));
                        }
                        2 => {
                            ui.add(egui::DragValue::new(end));
                        }
                        3 => {
                            ui.add(egui::DragValue::new(step));
                        }
                        _ => {}
                    },
                    Activity::Delay { milliseconds } => {
                        ui.add(
                            egui::DragValue::new(milliseconds)
                                .range(0..=usize::MAX)
                                .speed(25),
                        );
                    }
                    _ => {}
                }
            }
            PropertyType::ScenarioSelector => {
                ui.label(&label);

                if let Activity::CallScenario {
                    scenario_id,
                    parameters,
                } = &mut node.activity
                {
                    if scenarios.is_empty() {
                        ui.label(t!("status.no_scenarios").as_ref());
                    } else {
                        let invalid_text = t!("status.scenario_invalid").to_string();
                        let current_name = scenarios
                            .iter()
                            .find(|s| s.id == *scenario_id)
                            .map(|s| s.name.as_str())
                            .unwrap_or(&invalid_text);

                        egui::ComboBox::from_id_salt("scenario_selector")
                            .selected_text(current_name)
                            .show_ui(ui, |ui| {
                                for scenario in scenarios {
                                    ui.selectable_value(
                                        scenario_id,
                                        scenario.id.clone(),
                                        &scenario.name,
                                    );
                                }
                            });

                        ui.separator();
                        ui.label(t!("variable_binding.title").as_ref());

                        let mut to_delete = None;
                        let mut to_edit = None;

                        ui.horizontal(|ui| {
                            ui.label(t!("variable_binding.parameter").as_ref());
                            ui.label(t!("variable_binding.source_variable").as_ref());
                            ui.label(t!("variable_binding.direction").as_ref());
                            ui.label("");
                        });

                        for (idx, binding) in parameters.iter().enumerate() {
                            ui.horizontal(|ui| {
                                let param_name = scenarios
                                    .iter()
                                    .find(|s| s.id == *scenario_id)
                                    .and_then(|s| {
                                        s.parameters
                                            .iter()
                                            .find(|p| p.var_name == binding.target_var_name)
                                    })
                                    .map(|p| p.var_name.clone())
                                    .unwrap_or_else(|| "???".to_string());

                                let source_var_name = binding.source_var_name.clone();

                                ui.label(&param_name);
                                ui.label("→");
                                ui.label(source_var_name);

                                let direction_text = match binding.direction {
                                    rpa_core::node_graph::VariableDirection::In => "In",
                                    rpa_core::node_graph::VariableDirection::Out => "Out",
                                    rpa_core::node_graph::VariableDirection::InOut => "InOut",
                                };
                                ui.label(direction_text);

                                if ui
                                    .button(t!("variable_binding.edit_parameter").as_ref())
                                    .clicked()
                                {
                                    to_edit = Some(idx);
                                }
                                if ui
                                    .button(t!("variable_binding.delete_parameter").as_ref())
                                    .clicked()
                                {
                                    to_delete = Some(idx);
                                }
                            });
                        }

                        if let Some(idx) = to_delete {
                            parameters.remove(idx);
                        }

                        if let Some(idx) = to_edit {
                            param_action = ParameterBindingAction::Edit(idx);
                        }

                        if ui
                            .button(t!("variable_binding.add_parameter").as_ref())
                            .clicked()
                        {
                            param_action = ParameterBindingAction::Add;
                        }
                    }
                }
            }
            PropertyType::CodeEditor => {
                ui.label(&label);

                if let Activity::RunPowershell { code } = &mut node.activity {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        CodeEditor::default()
                            .id_source("code editor")
                            .with_rows(12)
                            .with_fontsize(14.0)
                            .with_theme(ColorTheme::GRUVBOX)
                            .with_syntax(Syntax::shell())
                            .with_numlines(true)
                            .show(ui, code);
                    });
                }
            }
            PropertyType::Combobox => match &mut node.activity {
                Activity::SetVariable { is_global, .. } if prop_idx == 3 => {
                    ui.label(t!("properties.scope",).as_ref());

                    let selected_scope = if *is_global { "Global" } else { "Scenario" };
                    egui::ComboBox::from_id_salt("set_var_scope_combo")
                        .selected_text(selected_scope)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(is_global, false, "Scenario");
                            ui.selectable_value(is_global, true, "Global");
                        });
                }
                _ => {
                    ui.label(&label);
                }
            },
            _ => {
                ui.label(&label);
            }
        }
    }

    ui.separator();
    ui.label(
        t!(
            "properties.position",
            x = node.position.x : {:.2},
            y = node.position.y : {:.2}
        )
        .as_ref(),
    );
    ui.label(t!("properties.node_id", node_id = node.id,).as_ref());

    (node.activity != original_activity, param_action)
}

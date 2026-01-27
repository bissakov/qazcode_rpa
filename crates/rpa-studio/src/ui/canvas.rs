use crate::ext::{ActivityExt, NodeExt};
use crate::ui::components::{GridRenderer, MinimapRenderer, NodeRenderer, PinRenderer};
use crate::ui::connection_renderer::{ConnectionPath, ConnectionRenderer};
use crate::ui_constants::{UiConstants, snap_to_grid};
use crate::{colors::ColorPalette, state::ScenarioViewState};
use arc_script::VariableType;
use egui::{
    Color32, Popup, PopupCloseBehavior, Pos2, Rect, Response, Stroke, StrokeKind, Ui, Vec2,
};
use egui_code_editor::{CodeEditor, ColorTheme, Syntax};

use rpa_core::log::LogLevel;
use rpa_core::{Activity, ActivityMetadata, BranchType, Node, PropertyType, Scenario};
use rust_i18n::t;
use shared::NanoId;
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

fn is_node_grid_aligned(node: &Node, grid_size: f32) -> bool {
    (node.x % grid_size).abs() < 0.01 && (node.y % grid_size).abs() < 0.01
}

pub struct RenderState<'a> {
    pub selected_nodes: &'a mut HashSet<NanoId>,
    pub connection_from: &'a mut Option<(NanoId, usize)>,
    pub clipboard_empty: bool,
    pub knife_tool_active: &'a mut bool,
    pub knife_path: &'a mut Vec<Pos2>,
    pub resizing_node: &'a mut Option<(NanoId, ResizeHandle)>,
    pub searched_activity: &'a mut String,
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

                if let Some(intersection_point) =
                    path.intersects_line(cut_start, cut_end, renderer, &connection.id)
                {
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
            ui.set_min_width(UiConstants::CONTEXT_MENU_MIN_WIDTH);

            ui.label("Search:");

            let text_id = ui.make_persistent_id("activity_search_text_edit");
            let response = ui.add(egui::TextEdit::singleline(state.searched_activity).id(text_id));
            if !response.has_focus() {
                response.request_focus();
            }

            if !state.searched_activity.is_empty() {
                for activity in Activity::iter_as_str()
                    .filter(|name| name.contains(state.searched_activity.as_str()))
                {
                    ui.label(activity);
                }
            }

            ui.separator();

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

pub struct CanvasRenderer<'a> {
    scenario: &'a mut Scenario,
    view: &'a mut ScenarioViewState,
    state: &'a mut RenderState<'a>,
    config: crate::ui::config::CanvasConfig,
    node_renderer: NodeRenderer,
    pin_renderer: PinRenderer,
    grid_renderer: GridRenderer,
    minimap_renderer: MinimapRenderer,
}

impl<'a> CanvasRenderer<'a> {
    pub fn new(
        scenario: &'a mut Scenario,
        view: &'a mut ScenarioViewState,
        state: &'a mut RenderState<'a>,
    ) -> Self {
        use crate::ui::config::*;
        
        Self {
            scenario,
            view,
            state,
            config: CanvasConfig::default(),
            node_renderer: NodeRenderer::default(),
            pin_renderer: PinRenderer::default(),
            grid_renderer: GridRenderer::default(),
            minimap_renderer: MinimapRenderer::new(),
        }
    }

    pub fn with_config(mut self, config: crate::ui::config::CanvasConfig) -> Self {
        self.config = config;
        self
    }

    pub fn render(
        &mut self,
        ui: &mut Ui,
    ) -> (
        ContextMenuAction,
        Vec2,
        bool,
        bool,
        bool,
        bool,
        bool,
        egui::Rect,
    ) {
        let scenario = &mut self.scenario;
        let view = &mut self.view;
        let state = &mut self.state;
        
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

        self.grid_renderer.draw(&painter, rect, view.pan_offset, view.zoom, self.config.show_grid);
        self.grid_renderer.draw_border(&painter, rect, self.config.is_executing);

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
                let knife_path_world: Vec<Pos2> = state
                    .knife_path
                    .iter()
                    .map(|pos| ((pos.to_vec2() - view.pan_offset) / view.zoom).to_pos2())
                    .collect();

                let (connections_to_remove, _) = find_intersecting_connections(
                    scenario,
                    &node_index,
                    &knife_path_world,
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
                    let (snapped_pos, snapped_w, snapped_h) = node.snap_bounds(UiConstants::GRID_SIZE);
                    if (snapped_pos.x - node.x).abs() > 0.001
                        || (snapped_pos.y - node.y).abs() > 0.001
                        || (snapped_w - node.width).abs() > 0.001
                        || (snapped_h - node.height).abs() > 0.001
                    {
                        node.x = snapped_pos.x;
                        node.y = snapped_pos.y;
                        node.width = snapped_w;
                        node.height = snapped_h;
                        view.connection_renderer.increment_generation();
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
                        node.x += width_change;
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
                        node.y += height_change;
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
                        node.x += width_change;
                        node.width = new_width;
                        node.height = (node.height + delta_world.y).max(UiConstants::NOTE_MIN_HEIGHT);
                        view.connection_renderer.increment_generation();
                    }
                    ResizeHandle::TopRight => {
                        node.width = (node.width + delta_world.x).max(UiConstants::NOTE_MIN_WIDTH);
                        let new_height =
                            (node.height - delta_world.y).max(UiConstants::NOTE_MIN_HEIGHT);
                        let height_change = node.height - new_height;
                        node.y += height_change;
                        node.height = new_height;
                        view.connection_renderer.increment_generation();
                    }
                    ResizeHandle::TopLeft => {
                        let new_width = (node.width - delta_world.x).max(UiConstants::NOTE_MIN_WIDTH);
                        let width_change = node.width - new_width;
                        node.x += width_change;
                        node.width = new_width;
                        let new_height =
                            (node.height - delta_world.y).max(UiConstants::NOTE_MIN_HEIGHT);
                        let height_change = node.height - new_height;
                        node.y += height_change;
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
            let can_resize = is_note_node || self.config.allow_node_resize;

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

                        ui.set_min_width(UiConstants::NODE_CONTEXT_MENU_MIN_WIDTH);

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

            self.node_renderer.draw(
                &painter,
                node,
                is_selected,
                is_hovering_connection,
                is_being_resized,
                to_screen,
                view.zoom,
            );

            self.pin_renderer.draw_input_pin(&painter, node, to_screen, view.zoom);
            self.pin_renderer.draw_output_pins(&painter, node, to_screen, view.zoom);
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
                    node.x += drag_delta.x;
                    node.y += drag_delta.y;
                }
            }

            view.connection_renderer.increment_generation();
        }

        if drag_ended && !state.selected_nodes.is_empty() {
            let mut reroute_needed = false;

            for node in &mut scenario.nodes {
                if state.selected_nodes.contains(&node.id) && node.is_routable() {
                    let snapped_x = snap_to_grid(node.x, UiConstants::GRID_SIZE);
                    let snapped_y = snap_to_grid(node.y, UiConstants::GRID_SIZE);

                    if (snapped_x - node.x).abs() > 0.001 || (snapped_y - node.y).abs() > 0.001 {
                        reroute_needed = true;
                    }

                    node.x = snapped_x;
                    node.y = snapped_y;
                }
            }

            if reroute_needed {
                view.connection_renderer.increment_generation();
            }
        }

        if let Some(released_node_id) = node_drag_released
            && state.selected_nodes.len() == 1
            && let Some(released_node) =
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
                    let from_pos = Pos2::new(from_node.x, from_node.y);
                    let to_pos = Pos2::new(to_node.x, to_node.y);
                    let mid_x = (from_pos.x + to_pos.x) * 0.5;
                    let mid_y = (from_pos.y + to_pos.y) * 0.5;

                    let distance = to_pos.y - from_pos.y;
                    let required = UiConstants::MIN_NODE_SPACING * 2.0;

                    if distance < required {
                        let push = required - distance;

                        for node in &mut scenario.nodes {
                            if node.y >= to_pos.y {
                                node.y += push;
                            }
                        }
                    }

                    for node in &mut scenario.nodes {
                        if node.id == released_node_id {
                            node.x = mid_x;
                            node.y = mid_y;
                            break;
                        }
                    }
                }

                scenario.add_connection_with_branch(from_id, released_node_id.clone(), branch_type);
                scenario.add_connection_with_branch(released_node_id, to_id, BranchType::Default);
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

        if self.config.show_minimap {
            let minimap_layer =
                egui::LayerId::new(egui::Order::Foreground, egui::Id::new("minimap_layer"));
            let minimap_painter = ui.ctx().layer_painter(minimap_layer);
            self.minimap_renderer.draw(
                &minimap_painter,
                scenario,
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

                let knife_path_world: Vec<Pos2> = state
                    .knife_path
                    .iter()
                    .map(|pos| ((pos.to_vec2() - view.pan_offset) / view.zoom).to_pos2())
                    .collect();

                let (_, intersection_points) = find_intersecting_connections(
                    scenario,
                    &node_index,
                    &knife_path_world,
                    view.pan_offset,
                    view.zoom,
                    &mut view.connection_renderer,
                );

                for point in intersection_points {
                    let screen_point = to_screen(point);
                    let size = 8.0;
                    painter.line_segment(
                        [
                            Pos2::new(screen_point.x - size, screen_point.y - size),
                            Pos2::new(screen_point.x + size, screen_point.y + size),
                        ],
                        Stroke::new(3.0, Color32::from_rgb(255, 0, 0)),
                    );
                    painter.line_segment(
                        [
                            Pos2::new(screen_point.x - size, screen_point.y + size),
                            Pos2::new(screen_point.x + size, screen_point.y - size),
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
            rect,
        )
    }
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
                        let set_variable_name_id = ui.make_persistent_id("set_variable_name");
                        ui.add(egui::TextEdit::singleline(name).id(set_variable_name_id));
                    }
                    Activity::SetVariable { var_type, .. } if prop_idx == 1 => {
                        egui::ComboBox::from_id_salt("var_type_combo")
                            .selected_text(var_type.as_str())
                            .show_ui(ui, |ui| {
                                for vt in VariableType::all() {
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
                            let value_id = ui.make_persistent_id("var_value_string");
                            ui.add(egui::TextEdit::singleline(value).id(value_id));
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
                        let condition_id =
                            ui.make_persistent_id(format!("{}_condition_{}", node.id, prop_idx));
                        ui.add(egui::TextEdit::singleline(condition).id(condition_id));
                    }
                    Activity::Loop { index, .. } if prop_idx == 0 => {
                        let index_id =
                            ui.make_persistent_id(format!("{}_loop_index_{}", node.id, prop_idx));
                        ui.add(egui::TextEdit::singleline(index).id(index_id));
                    }
                    Activity::Evaluate { expression } if prop_idx == 0 => {
                        let expr_id =
                            ui.make_persistent_id(format!("{}_eval_expr_{}", node.id, prop_idx));
                        ui.add(egui::TextEdit::singleline(expression).id(expr_id));
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
                        let note_id = ui.make_persistent_id(format!("{}_note_text", node.id));
                        ui.add(egui::TextEdit::multiline(text).id(note_id).desired_rows(8));
                    }
                    _ => {}
                }
            }
            PropertyType::Slider => {
                ui.label(&label);
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
            _ => {}
        }
    }

    ui.separator();
    ui.label(
        t!(
            "properties.position",
            x = node.x : {:.2},
            y = node.y : {:.2}
        )
        .as_ref(),
    );
    ui.label(t!("properties.node_id", node_id = node.id,).as_ref());

    let grid_aligned = is_node_grid_aligned(node, UiConstants::GRID_SIZE);
    let grid_status_key = if grid_aligned {
        "properties.grid_status.aligned"
    } else {
        "properties.grid_status.misaligned"
    };
    ui.label(
        t!(
            "properties.grid_aligned",
            status = t!(grid_status_key).as_ref()
        )
        .as_ref(),
    );

    (node.activity != original_activity, param_action)
}

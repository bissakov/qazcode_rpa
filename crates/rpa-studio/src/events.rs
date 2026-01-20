use crate::{
    state::{ClipboardData, RpaApp},
    ui::canvas,
};
use eframe::egui;
use shared::NanoId;

impl RpaApp {
    pub fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        let mut handled = false;

        if !ctx.wants_keyboard_input() {
            let copy_event = ctx.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Copy)));
            let paste_event =
                ctx.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Paste(_))));
            let cut_event = ctx.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Cut)));
            let has_selected = !self.selected_nodes.is_empty();
            let no_settings = !self.dialogs.settings.show;
            let no_rename = self.dialogs.rename_scenario.scenario_index.is_none();

            if copy_event && has_selected && no_settings && no_rename {
                self.copy_selected_nodes();
                handled = true;
            }

            let has_clipboard = !self.clipboard.nodes.is_empty();

            if paste_event && has_clipboard && no_settings && no_rename {
                let view = self.get_current_scenario_view_mut();
                let mouse_world_pos = ctx
                    .pointer_hover_pos()
                    .map(|pos| (pos.to_vec2() - view.pan_offset) / view.zoom)
                    .unwrap_or_else(|| {
                        let viewport_center = ctx.content_rect().center();
                        (viewport_center.to_vec2() - view.pan_offset) / view.zoom
                    });

                self.paste_clipboard_nodes(mouse_world_pos);
                handled = true;
            }

            if cut_event && has_selected && no_settings && no_rename {
                self.cut_selected_nodes();
                handled = true;
            }

            if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::A)) {
                let node_ids: Vec<_> = self
                    .get_current_scenario()
                    .nodes
                    .iter()
                    .map(|n| n.id.clone())
                    .collect();
                self.selected_nodes.clear();
                for node_id in node_ids {
                    self.selected_nodes.insert(node_id);
                }
                handled = true;
            }

            if ctx.input(|i| i.key_pressed(egui::Key::Delete)) && !self.selected_nodes.is_empty() {
                let nodes_to_remove: Vec<_> = self.selected_nodes.iter().cloned().collect();
                let scenario = self.get_current_scenario_mut();
                for node_id in nodes_to_remove {
                    scenario.remove_node(node_id);
                }
                self.invalidate_current_scenario();
                self.selected_nodes.clear();
                handled = true;
            }

            if !self.is_executing && ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Z))
            {
                self.undo();
                handled = true;
            }

            if !self.is_executing && ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Y))
            {
                self.redo();
                handled = true;
            }
        }

        if handled {
            // ctx.request_repaint();
        }
    }

    pub fn handle_context_menu_action(
        &mut self,
        action: canvas::ContextMenuAction,
        mouse_world_pos: egui::Vec2,
    ) {
        match action {
            canvas::ContextMenuAction::Copy => {
                self.copy_selected_nodes();
            }
            canvas::ContextMenuAction::Cut => {
                self.cut_selected_nodes();
            }
            canvas::ContextMenuAction::Paste => {
                self.paste_clipboard_nodes(mouse_world_pos);
            }
            canvas::ContextMenuAction::Delete => {
                if !self.selected_nodes.is_empty() {
                    let nodes_to_remove: Vec<_> = self.selected_nodes.iter().cloned().collect();
                    let scenario = self.get_current_scenario_mut();
                    for node_id in nodes_to_remove {
                        scenario.remove_node(node_id);
                    }
                    self.invalidate_current_scenario();
                    self.selected_nodes.clear();
                    let view = self.get_current_scenario_view_mut();
                    view.connection_renderer.clear_cache();
                    self.undo_redo.add_undo(&self.project);
                }
            }
            canvas::ContextMenuAction::SelectAll => {
                let node_ids: Vec<_> = self
                    .get_current_scenario()
                    .nodes
                    .iter()
                    .map(|n| n.id.clone())
                    .collect();
                self.selected_nodes.clear();
                for node_id in node_ids {
                    self.selected_nodes.insert(node_id);
                }
            }
            canvas::ContextMenuAction::None => {}
        }
    }

    pub fn copy_selected_nodes(&mut self) {
        let scenario = self.get_current_scenario();
        let nodes_to_copy: Vec<_> = scenario
            .nodes
            .iter()
            .filter(|n| self.selected_nodes.contains(&n.id))
            .cloned()
            .collect();

        let clipboard_node_ids: std::collections::HashSet<NanoId> =
            nodes_to_copy.iter().map(|n| n.id.clone()).collect();

        let connections_to_copy: Vec<_> = scenario
            .connections
            .iter()
            .filter(|conn| {
                clipboard_node_ids.contains(&conn.from_node)
                    && clipboard_node_ids.contains(&conn.to_node)
            })
            .cloned()
            .collect();

        self.clipboard = ClipboardData {
            nodes: nodes_to_copy,
            connections: connections_to_copy,
        };
    }

    pub fn cut_selected_nodes(&mut self) {
        self.copy_selected_nodes();

        let selected_ids: Vec<NanoId> = self.selected_nodes.iter().cloned().collect();

        let scenario = self.get_current_scenario_mut();
        scenario
            .nodes
            .retain(|node| !selected_ids.contains(&node.id));
        scenario.connections.retain(|conn| {
            !selected_ids.contains(&conn.from_node) && !selected_ids.contains(&conn.to_node)
        });

        self.selected_nodes.clear();
        let view = self.get_current_scenario_view_mut();
        view.connection_renderer.clear_cache();
    }

    pub fn paste_clipboard_nodes(&mut self, mouse_world_pos: egui::Vec2) {
        if self.clipboard.nodes.is_empty() {
            return;
        }

        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;

        for node in &self.clipboard.nodes {
            min_x = min_x.min(node.x);
            min_y = min_y.min(node.y);
            max_x = max_x.max(node.x);
            max_y = max_y.max(node.y);
        }

        let clipboard_center = egui::pos2((min_x + max_x) / 2.0, (min_y + max_y) / 2.0);
        let offset = mouse_world_pos - clipboard_center.to_vec2();

        let mut nodes_to_paste = Vec::new();
        let mut new_node_ids = Vec::new();
        let mut old_to_new_id: std::collections::HashMap<NanoId, NanoId> =
            std::collections::HashMap::new();

        for node in &self.clipboard.nodes {
            let mut new_node = node.clone();
            let new_id = NanoId::default();
            old_to_new_id.insert(new_node.id.clone(), new_id.clone());
            new_node.id = new_id;
            new_node.x += offset.x;
            new_node.y += offset.y;
            new_node_ids.push(new_node.id.clone());
            nodes_to_paste.push(new_node);
        }

        self.selected_nodes.clear();
        for node_id in new_node_ids {
            self.selected_nodes.insert(node_id);
        }

        let connections_to_add: Vec<_> = self
            .clipboard
            .connections
            .iter()
            .filter_map(|conn| {
                let new_from = old_to_new_id.get(&conn.from_node)?;
                let new_to = old_to_new_id.get(&conn.to_node)?;
                Some((new_from.clone(), new_to.clone(), conn.branch_type.clone()))
            })
            .collect();

        let scenario = self.get_current_scenario_mut();
        scenario.nodes.extend(nodes_to_paste);

        for (new_from, new_to, branch_type) in connections_to_add {
            scenario.add_connection_with_branch(new_from, new_to, branch_type);
        }

        self.invalidate_current_scenario();
        let view = self.get_current_scenario_view_mut();
        view.connection_renderer.clear_cache();
    }
}

use crate::ui_constants::UiConstants;
use rpa_core::{Activity, BranchType, Node, Scenario, Variables};
use shared::NanoId;

const DEFAULT_NODE_WIDTH: f32 = 128.0;
const DEFAULT_NODE_HEIGHT: f32 = 64.0;

const DEFAULT_INITIAL_X: f32 = 1000.0;
const DEFAULT_INITIAL_Y: f32 = 550.0;

pub trait ScenarioExt {
    fn new_empty(name: &str) -> Self;
    fn add_node(&mut self, activity: Activity, x: f32, y: f32);
}

impl ScenarioExt for Scenario {
    fn new_empty(name: &str) -> Self {
        let mut scenario = Self {
            id: NanoId::default(),
            name: name.to_string(),
            nodes: Vec::new(),
            connections: Vec::new(),
            parameters: Vec::new(),
            variables: Variables::new(),
        };

        let start_x = (DEFAULT_INITIAL_X / UiConstants::GRID_SIZE).floor() * UiConstants::GRID_SIZE;
        let start_y = (DEFAULT_INITIAL_Y / UiConstants::GRID_SIZE).floor() * UiConstants::GRID_SIZE;

        scenario.add_node(
            Activity::Start {
                scenario_id: scenario.id.clone(),
            },
            start_x,
            start_y,
        );
        scenario.add_node(
            Activity::End {
                scenario_id: scenario.id.clone(),
            },
            start_x,
            start_y
                + DEFAULT_NODE_WIDTH
                + (DEFAULT_NODE_HEIGHT / UiConstants::GRID_SIZE).floor() * UiConstants::GRID_SIZE,
        );

        if scenario.nodes.len() >= 2 {
            let start_id = scenario.nodes[0].id.clone();
            let end_id = scenario.nodes[1].id.clone();
            scenario.add_connection_with_branch(start_id, end_id, BranchType::Default);
        }

        scenario
    }

    fn add_node(&mut self, activity: Activity, x: f32, y: f32) {
        let (width, height) = match &activity {
            Activity::Note { width, height, .. } => (*width, *height),
            _ => (DEFAULT_NODE_WIDTH, DEFAULT_NODE_HEIGHT),
        };
        let node = Node {
            id: NanoId::default(),
            activity,
            x,
            y,
            width,
            height,
        };

        self.nodes.push(node);
    }
}

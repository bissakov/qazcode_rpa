# Undo/Redo Implementation Viability Analysis for RPA Studio

## Executive Summary

**Feasibility Assessment: MODERATE** - Implementing undo/redo is viable with medium complexity. The codebase has excellent foundational support (all data structures are Clone, Serialize, Deserialize), but requires:
1. Architectural refactoring to centralize state mutations
2. Introduction of a command/action pattern to capture mutations
3. Addition of history management infrastructure
4. Integration with keyboard shortcuts (Ctrl+Z/Ctrl+Y)

---

## 1. Current State Management Analysis

### 1.1 Main Application State (RpaApp struct in main.rs)

Located at line 107-126 in `crates/rpa-studio/src/main.rs`:

**Core Mutable State:**
- `project: Project` - Primary state requiring undo/redo
- `selected_nodes: HashSet<String>` - Transient selection state (DON'T undo)
- `connection_from: Option<(String, usize)>` - Transient interaction state (DON'T undo)
- `pan_offset: egui::Vec2` - View state (DON'T undo)
- `zoom: f32` - View state (DON'T undo)
- `clipboard: ClipboardData` - UI state (DON'T undo)
- `knife_tool_active: bool` - Tool state (DON'T undo)
- `knife_path: Vec<egui::Pos2>` - Tool state (DON'T undo)
- `resizing_node: Option<(String, ui::ResizeHandle)>` - Interaction state (DON'T undo)

**Key Insight**: Only `project` should be tracked in undo history. All other mutations are transient UI state.

---

## 2. State Structures Trait Support Matrix

### Complete Trait Implementation Analysis

| Structure | Clone | Debug | Serialize | Deserialize | Notes |
|-----------|-------|-------|-----------|-------------|-------|
| **Project** | ✓ Derived | ✓ Derived | ✓ Derived | ✓ Derived | READY |
| **Scenario** | ✓ Derived | ✓ Derived | ✓ Derived | ✓ Derived | READY |
| **Node** | ✓ Derived | ✓ Derived | ✓ Derived | ✓ Derived | READY |
| **Activity** | ✓ Derived | ✓ Derived | ✓ Derived | ✓ Derived | All 12 variants Clone |
| **Connection** | ✓ Derived | ✓ Derived | ✓ Derived | ✓ Derived | READY |
| **Variables** | ✓ Derived | ✓ Derived | ✓ Derived | ✓ Derived | HashMap-backed, Clone works |
| **VariableValue** | ✓ Derived | ✓ Derived | ✓ Derived | ✓ Derived | Also has PartialEq, Eq |
| **BranchType** | ✓ Derived | ✓ Derived | ✓ Derived | ✓ Derived | Has PartialEq, Eq, Default |

**Critical Finding**: All structures are fully Cloneable and Serializable. This is the PRIMARY requirement for undo/redo. No custom trait implementations needed.

### 2.2 Data Structure Hierarchy

```
Project (Clone, Serialize, ~2KB base)
├── name: String
├── main_scenario: Scenario
├── scenarios: Vec<Scenario>
├── variables: Variables
└── execution_log: LogStorage (skipped in serde)

Scenario (Clone, Serialize)
├── id: String
├── name: String
├── nodes: Vec<Node>
├── connections: Vec<Connection>
└── parameters: Vec<ScenarioParameter>

Node (Clone, Serialize)
├── id: String
├── activity: Activity (enum, 12 variants)
├── position: egui::Pos2 (8 bytes)
├── width: f32 (4 bytes)
└── height: f32 (4 bytes)

Activity (Clone, Serialize)
├── Start { scenario_id }
├── End { scenario_id }
├── Log { level, message }
├── Delay { milliseconds }
├── SetVariable { name, value, var_type }
├── ... (7 more variants)
└── All variants Clone
```

**Complexity Assessment**: Moderate. Deep cloning a full Project is feasible. For a typical workflow (50 nodes, 50 connections), snapshot size ~50-100KB. With 100 undo states: 5-10MB worst case.

---

## 3. State Mutation Points - Complete Inventory

### 3.1 Main.rs Mutations (Primary Focus)

**Node Operations:**
- Line 282, 842: `scenario.remove_node(&node_id)` [2x]
- Line 252, 1055: `self.get_current_scenario_mut().add_node(activity, position)` [2x]
- Line 342: `scenario.add_connection_with_branch(...)` [High frequency]

**Connection Operations:**
- Line 1339-1342: `scenario.nodes.extend(...)` and `add_connection_with_branch()` [Paste operation]
- Line 1280-1281: `scenario.connections.retain(...)` [Cut operation]

**Variable Operations:**
- Line 682: `self.project.variables.set(id, value)`
- Line 1167: `self.project.variables.remove(id)`

**Scenario Operations:**
- Line 992: `self.project.scenarios.push(Scenario::new(...))`
- Line 975: `self.project.scenarios.remove(i)`
- Line 556: `self.project.scenarios[index].name = new_name`

**Project-Level:**
- Line 860: `self.project = Project::new(...)` [New project - resets everything]

**Not Undoable (UI State):**
- Line 313: `self.project.execution_log.clear()` [Execution artifacts, not design]
- Line 868: `self.pan_offset = egui::Vec2::ZERO` [View state]
- Line 869: `self.zoom = 1.0` [View state]

### 3.2 UI.rs Mutations (Canvas Interactions)

**Node Position Mutations:**
- Line 832: `node.position += drag_delta` [Continuous drag]
- Lines 522-570: Node resizing `node.width/height/position` mutations [Continuous]

**Node Resize Dimension Mutations:**
- Line 522: `node.width = (node.width + delta_world.x).max(...)`
- Line 531: `node.height = (node.height + delta_world.y).max(...)`
- Line 527: `node.position.x += width_change` [Coupled position/size]

**Connection Operations:**
- Line 942: `scenario.add_connection_with_branch(...)`
- Line 1025: `scenario.connections.retain(...)`
- Line 385: Connection removal via knife tool

**Connection Insertion on Drag:**
- Line 837-944: Complex interaction - insert node into existing connection
  - Removes old connection
  - Creates two new connections
  - Adjusts positions

### 3.3 UI.rs Property Panel Mutations (render_node_properties)

**Activity Property Mutations:**
- Line 1626: `ui.text_edit_singleline(name)` [SetVariable.name]
- Line 1643-1648: Value mutation with type conversion
- Line 1658: `ui.text_edit_singleline(condition)` [IfCondition, While]
- Line 1661: `ui.text_edit_singleline(index)` [Loop]

**Pattern**: These mutations happen via mutable borrow of node.activity through widgets. Debouncing is implicit (egui's TextEdit only mutates while focused).

---

## 4. Key Implementation Challenges

### Challenge 1: Mutation Granularity (CRITICAL)

**Problem:**
```rust
// Dragging a node generates this sequence every frame:
frame 1: node.position += Vec2 { x: 2.5, y: 1.3 }
frame 2: node.position += Vec2 { x: 2.5, y: 1.3 }
frame 3: node.position += Vec2 { x: 2.5, y: 1.3 }
... (60 frames per second)
```

With naive approach: 300 undo states for a 5-second drag operation. Unacceptable UX.

**Solutions:**
1. **Transaction-based**: Group all deltas within drag session into one undo step
2. **Debouncing**: Record snapshot only on mouse release
3. **Command batching**: Collect MoveNode deltas, emit single MoveNode command
4. **Time-based**: Coalesce mutations within 500ms window

**Recommended**: Transaction-based with activity lifecycle tracking
- `DragStart -> Drag* -> DragEnd` = 1 undo step
- `ResizeStart -> Resize* -> ResizeEnd` = 1 undo step

**Complexity**: Medium - requires state machine for interaction phases.

### Challenge 2: View State vs. Document State (CRITICAL)

**Problem:**
```rust
// These should NOT be undoable:
self.pan_offset += pan_delta;  // Panning
self.zoom = new_zoom;          // Zooming
selected_nodes.clear();         // Selection

// But this MUST be undoable:
scenario.nodes.remove(...);     // Node deletion
```

**Solution**: Strict separation
- `RpaApp` contains BOTH undo-tracked `project` and transient view state
- Undo manager only saves/restores `project`
- View state mutations never go through undo system

**Complexity**: Low - already somewhat separate.

### Challenge 3: Mutable Access Interception (HIGH PRIORITY)

**Problem:**
```rust
// Current code allows direct mutations:
let scenario = self.get_current_scenario_mut();
scenario.nodes.push(node);       // Bypasses any tracking
scenario.connections.retain(...); // No interception point
```

Can't retroactively add undo tracking without wrapping every mutation.

**Solution Options:**

A. **Command Pattern** (Recommended)
```rust
pub enum Command {
    AddNode { node_id, activity, position },
    RemoveNode { scenario_id, node_id, snapshot },
    MoveNode { scenario_id, node_id, from, to },
}

// All mutations go through:
self.execute_command(

# Undo/Redo Implementation Viability Analysis - RPA Studio

## EXECUTIVE SUMMARY

**Feasibility: MODERATE** ✓

**Implementation is VIABLE with medium complexity (3-4 weeks, 1-2 engineers)**

---

## KEY FINDINGS

### 1. Trait Support: ✓ EXCELLENT

All core data structures implement required traits:
- ✓ **Clone** - All structures derive Clone (Project, Scenario, Node, Activity, Connection, Variables)
- ✓ **Serialize/Deserialize** - Full serde support for all types
- ✓ **Debug** - All structures have Debug derived
- ⚠ PartialEq - Not implemented on Project/Scenario/Node/Activity (not needed for undo/redo)

**Critical Success Factor**: All data is fully cloneable and serializable. This is the PRIMARY requirement.

### 2. State Structure Readiness: ✓ READY

```
Project (Clone, Serialize)
├── main_scenario: Scenario (Clone, Serialize)
├── scenarios: Vec<Scenario> (Clone, Serialize)
├── variables: Variables (Clone, Serialize)
└── execution_log: LogStorage (Clone, NOT serialized)

Scenario (Clone, Serialize)
├── nodes: Vec<Node> (Clone, Serialize)
└── connections: Vec<Connection> (Clone, Serialize)

Node (Clone, Serialize)
├── activity: Activity (Clone, Serialize) - 12 enum variants, ALL clone
├── position: egui::Pos2 (Clone)
├── width/height: f32 (Clone)

Activity Variants:
├── Start, End, Log, Delay, SetVariable
├── Evaluate, IfCondition, Loop, While
├── CallScenario, RunPowershell, Note, TryCatch
└── All 12 are fully cloneable
```

**Memory Estimate**: 
- Small project (10 nodes): ~8 KB per snapshot
- Medium project (50 nodes): ~35 KB per snapshot  
- Large project (200 nodes): ~130 KB per snapshot
- 100-state history: 800 KB to 13 MB (acceptable)

### 3. State Mutation Points: 50+ IDENTIFIED

**Location: Distributed across:**
- main.rs: ~25 mutation points (highest priority)
- ui.rs: ~15 mutation points (medium priority) 
- Property panels: ~5 mutation points (low priority)

**Mutation Categories:**
```
Node operations:      remove_node, add_node (2 locations each)
Connection ops:       add_connection_with_branch (3+ locations)
Direct mutations:     nodes.extend, connections.retain (multiple)
Variable ops:         variables.set, variables.remove
Scenario ops:         push, remove, rename
View state (DON'T UNDO): pan_offset, zoom, selected_nodes
```

---

## CHALLENGES & BLOCKERS

### Challenge 1: Continuous Mutation Granularity ⚠ CRITICAL

**Problem**: 
- Node dragging creates 60+ position deltas per second
- Naive approach = 300+ undo states for 5-second drag
- Unacceptable UX

**Solution**: Transaction-based grouping
- `DragStart → DragMove* → DragEnd` = 1 undo state
- Record snapshot only on interaction end
- Complexity: Medium (state machine tracking)

### Challenge 2: View State vs. Document State ✓ MANAGEABLE

**Problem**:
- pan_offset, zoom, selected_nodes should NOT be undoable
- Current architecture mixes both

**Solution**: Strict separation
- Only `project` tracked in undo history
- View state mutations bypass undo system
- Complexity: Low (straightforward separation)

### Challenge 3: Mutable Access Interception ⚠ MEDIUM-HIGH

**Problem**:
```rust
let scenario = self.get_current_scenario_mut();
scenario.nodes.push(node);  // No tracking point
```

**Solution**: Command pattern
- Centralize all mutations through execute_command()
- Wrap every mutation as EditorCommand enum
- Complexity: Medium-High (~50 refactoring points)

### Challenge 4: Property Panel Editing ✓ MANAGEABLE

**Problem**: TextEdit widgets mutate every keystroke

**Solution**: Debounced recording
- Record state on focus loss (standard pattern)
- 500ms debounce timeout
- Complexity: Low (egui provides `lost_focus()`)

### Challenge 5: Scenario References ✓ MANAGEABLE

**Problem**: CallScenario references dangling if scenario deleted

**Solution**: Validation + prevent deletion
- Check references before deletion
- Prevent deletion of referenced scenarios
- Complexity: Low (use validation system)

### Challenge 6: Variable Timing ✓ MANAGEABLE

**Problem**: Variables modified during execution shouldn't undo

**Solution**: Execution lock
- Freeze undo/redo when `is_executing = true`
- Only undo design-time changes
- Complexity: Low (execution flag exists)

---

## RECOMMENDED ARCHITECTURE

### Layer Design
```
UI Interactions (egui)
  ↓
Command System (NEW)
  └── EditorCommand enum
  └── Command validation
  └── UndoRedoManager
  ↓
Document State (Project)
```

### EditorCommand Enum

```rust
pub enum EditorCommand {
    // Node operations
    AddNode { scenario_id, position, activity },
    RemoveNode { scenario_id, node_id, snapshot },
    MoveNode { scenario_id, node_id, from, to },
    ResizeNode { scenario_id, node_id, from_size, to_size },
    UpdateActivityProperty { scenario_id, node_id, from, to },
    
    // Connection operations
    AddConnection { scenario_id, from_id, to_id, branch_type },
    RemoveConnection { scenario_id, from_id, to_id },
    
    // Scenario operations
    CreateScenario { name },
    DeleteScenario { scenario_id, snapshot },
    RenameScenario { scenario_id, from_name, to_name },
    
    // Variable operations
    SetVariable { var_name, from_value, to_value },
    RemoveVariable { var_name, snapshot },
    
    // Composite (paste, cut, multi-node)
    Composite { commands: Vec<EditorCommand> },
}
```

### UndoRedoManager

```rust
pub struct UndoRedoManager {
    undo_stack: VecDeque<EditorCommand>,
    redo_stack: VecDeque<EditorCommand>,
    max_history: usize,
    frozen: bool,  // Set during execution
}

impl UndoRedoManager {
    pub fn execute(&mut self, project: &mut Project, cmd: EditorCommand) { ... }
    pub fn undo(&mut self, project: &mut Project) { ... }
    pub fn redo(&mut self, project: &mut Project) { ... }
    pub fn freeze(&mut self) { self.frozen = true; }
    pub fn unfreeze(&mut self) { self.frozen = false; }
}
```

---

## IMPLEMENTATION EFFORT ESTIMATE

### Timeline Breakdown

| Phase | Duration | Deliverable | Effort |
|-------|----------|-------------|--------|
| **Phase 1: MVP** | Week 1 | Core node/connection operations, Ctrl+Z/Y | 50-60 hrs |
| **Phase 2: Full Feature** | Week 2 | Drag/resize grouping, properties, scenarios | 60-70 hrs |
| **Phase 3: Polish** | Week 3 | Memory optimization, UI, localization | 40-50 hrs |
| **Total** | **3-4 weeks** | **Production-ready** | **150-180 hrs** |

### Risk-Adjusted Range
- Optimistic: 120 hours (2.5 weeks)
- Realistic: 150-170 hours (3-4 weeks)
- Pessimistic: 200+ hours (5 weeks)

### Key Work Items
1. EditorCommand enum design (8 hrs)
2. Command apply()/undo() implementations (28 hrs)
3. UndoRedoManager (6 hrs)
4. Main.rs refactoring (24 hrs) - highest priority
5. UI.rs refactoring (20 hrs) - medium priority
6. Keyboard shortcuts (4 hrs)
7. Testing/debugging (40-50 hrs)
8. Optimization & UX (16-20 hrs)

---

## SUCCESS CRITERIA

- ✓ Ctrl+Z undoes user actions consistently
- ✓ Ctrl+Y redoes actions
- ✓ All major operations covered (50+ mutation points)
- ✓ No crashes with large undo histories
- ✓ Memory < 20MB for typical projects
- ✓ Undo/redo menu items visible and functional
- ✓ Help text + keyboard hints added
- ✓ Localization complete (en, ru, kz)

---

## FINAL RECOMMENDATION

**STATUS: APPROVED FOR IMPLEMENTATION** ✓

**Rationale:**
- All technical prerequisites met (Clone, Serialize traits)
- Clear architecture path (Command pattern + Manager)
- Manageable scope (50 mutation points, standard patterns)
- High user value (essential for professional tool)
- Acceptable risk with proper testing strategy

**Next Steps:**
1. Finalize EditorCommand enum design document
2. Assign 1-2 engineers to Phase 1
3. Create comprehensive test plan (30+ unit tests)
4. Set 2-week target for MVP release
5. Plan gradual rollout (main.rs first, then ui.rs)


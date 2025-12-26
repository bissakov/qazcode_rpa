# Loop Node Implementation & Nested Scenario Handling - Comprehensive Analysis

## Overview
This document details how Loop nodes are implemented and compiled to IR, and identifies a critical architectural issue with Loop/While/TryCatch handling in nested scenarios.

---

## 1. LOOP NODE DEFINITION AND PROPERTIES

### File: `node_graph.rs`

**Loop Activity Definition (Lines 589-593):**
```rust
Loop {
    start: i64,
    end: i64,
    step: i64,
    index: String,
}
```

**Loop Node Configuration (Lines 414, 445, 497-506, 534-539):**
- Output pin count: 2
- Pin 0: LoopBody (the loop body branch)
- Pin 1: Default (exit/next branch)
- Properties: index name, start value, end value, step value

**Loop Validation in Activity Metadata (Lines 457-489):**
```rust
static LOOP_METADATA: ActivityMetadata = ActivityMetadata {
    name_key: "activity_names.loop",
    button_key: "activity_buttons.loop",
    category: ActivityCategory::ControlFlow,
    properties: &[
        PropertyDef { label_key: "properties.loop_index", ... },
        PropertyDef { label_key: "properties.loop_start", ... },
        PropertyDef { label_key: "properties.loop_end", ... },
        PropertyDef { label_key: "properties.loop_step", ... },
    ],
};
```

---

## 2. LOOP IR COMPILATION (Main Scenario)

### File: `ir.rs`

**Compilation Entry Point (Lines 429-435):**
```rust
Activity::Loop {
    start,
    end,
    step,
    index,
} => {
    self.compile_loop_node(node_id, *start, *end, *step, index)?;
}
```

**compile_loop_node Function (Lines 512-580):**

The function generates the following instruction sequence:

1. **LoopInit** (Line 534-537): Initializes the index variable
   ```rust
   Instruction::LoopInit {
       index: index_var,
       start,
   }
   ```

2. **LoopLog** (Line 539-544): Logs loop parameters
   ```rust
   Instruction::LoopLog {
       index: index_var,
       start,
       end,
       step,
   }
   ```

3. **LoopCheck** (Line 547-553): Branch condition check
   ```rust
   Instruction::LoopCheck {
       index: index_var,
       end,
       step,
       body_target: 0,    // Updated after body compiled
       end_target: 0,     // Updated after body compiled
   }
   ```

4. **Loop Body**: Recursively compiles the body (Line 556)
   ```rust
   let body_start = self.program.instructions.len();
   self.compile_from_node(&body_node.unwrap())?;
   ```

5. **LoopNext** (Line 558-562): Increments index and jumps back to check
   ```rust
   Instruction::LoopNext {
       index: index_var,
       step,
       check_target: check_idx,  // Jumps back to LoopCheck
   }
   ```

6. **After-Loop Code**: Compiles after-loop node (Line 565-567)

7. **Backpatch**: Updates LoopCheck targets after body compilation (Line 569-577)

### IR Instructions for Loop (Lines 44-65):
```rust
LoopInit { index: VarId, start: i64 },
LoopLog { index: VarId, start: i64, end: i64, step: i64 },
LoopCheck { 
    index: VarId, 
    end: i64, 
    step: i64, 
    body_target: usize,      // Points to first loop body instruction
    end_target: usize,       // Points to first after-loop instruction
},
LoopNext { 
    index: VarId, 
    step: i64, 
    check_target: usize,     // Points back to LoopCheck
},
```

---

## 3. LOOP EXECUTION

### File: `execution.rs`

**LoopInit Execution (Lines 388-397):**
- Initializes the loop variable to start value
- Broadcasts via VarEvent sender if listener connected

**LoopLog Execution (Lines 398-411):**
- Logs the loop parameters (start, end, step)
- No state changes

**LoopCheck Execution (Lines 413-448):**
```rust
Instruction::LoopCheck { index, end, step, body_target, end_target } => {
    if *step == 0 {
        // Jump to end_target
        return Ok(*end_target);
    }
    
    let current = self.context.variables.get(*index).as_number() ...;
    let should_continue = if *step > 0 {
        current < *end
    } else {
        current > *end
    };
    
    if should_continue {
        Ok(*body_target)
    } else {
        Ok(*end_target)
    }
}
```

**LoopNext Execution (Lines 449-470):**
- Increments/decrements index by step
- Broadcasts updated variable value
- Jumps back to LoopCheck instruction

---

## 4. NESTED SCENARIO HANDLING (CRITICAL ISSUE)

### File: `ir.rs` - Lines 731-821

**Problem: compile_from_called_scenario() has INCOMPLETE Activity Pattern Matching**

The function handles:
- ✅ Activity::Start (Line 750-751)
- ✅ Activity::End (Line 753-756)
- ✅ Activity::Log (Line 758-764)
- ✅ Activity::Delay (Line 766-770)
- ✅ Activity::SetVariable (Line 772-783)
- ✅ Activity::Evaluate (Line 785-788)
- ✅ Activity::IfCondition (Line 790-806)
- ✅ Activity::CallScenario (Line 808-813)
- ❌ Activity::Loop (NOT HANDLED - falls through to default case!)
- ❌ Activity::While (NOT HANDLED - falls through to default case!)
- ❌ Activity::TryCatch (NOT HANDLED - falls through to default case!)
- ❌ Activity::RunPowershell (NOT HANDLED - falls through to default case!)
- ⚠️ Activity::Note (correctly ignored - not executable)

**Default Case (Lines 815-817):**
```rust
_ => {
    self.compile_default_next_called(scenario, node_id)?;
}
```

This means Loop, While, and TryCatch nodes in **nested scenarios are SILENTLY SKIPPED** 
and only the next connection is followed!

### Comparison with Main Scenario Compilation

**Main Scenario: compile_from_node() (Lines 347-459)**
- Handles ALL activity types including Loop, While, TryCatch
- Complete pattern matching with explicit handlers

**Nested Scenario: compile_from_called_scenario() (Lines 731-821)**
- Missing handlers for Loop, While, TryCatch
- Default catch-all silently skips unhandled activities

---

## 5. VARIABLE SCOPE IN NESTED SCENARIOS

### Shared Variables (execution.rs)
- Loop index variables use the SAME global Variables instance
- No variable scope isolation between scenarios
- Parameter bindings handle In/Out/InOut transfer (Lines 558-581)

### Loop Index Variable Handling
1. **In main scenario**: Loop index created via `self.variables.id(index_name)`
2. **In nested scenario**: Same variable ID used (shared global scope)
3. **Multiple loops in different scenarios**: Each has its own VarId, stored in global Variables

**CallScenario Parameter Handling (Lines 558-581):**
```rust
for binding in parameters {
    match binding.direction {
        ParameterDirection::In | ParameterDirection::InOut => {
            let source_value = self.context.variables.get(binding.source_var_id);
            self.context.variables.set(binding.param_var_id, source_value);
        }
        ParameterDirection::Out => {
            self.context.variables.set(binding.param_var_id, VariableValue::Undefined);
        }
    }
}
```

Loop index variables are **NOT** typically exposed as scenario parameters, so they share 
the global namespace across all scenarios.

---

## 6. VALIDATION LOGIC FOR LOOPS

### File: `validation.rs`

**Loop-Specific Validation (Lines 337-384):**
- Checks for step == 0 (infinite loop)
- Validates start/end/step consistency
- Ensures loop body connection exists
- Collects loop body nodes for reachability analysis

**Loop Body Node Collection (Lines 778-802):**
```rust
fn compute_loop_body_nodes(&self) -> HashSet<String> {
    let mut all_loop_bodies = HashSet::new();
    
    for node in &self.scenario.nodes {
        if matches!(node.activity, Activity::Loop { .. } | Activity::While { .. }) {
            let mut this_loop_body = HashSet::new();
            // Find nodes connected via LoopBody branch
            // Recursively collect all reachable nodes in loop body
            // Stop when reaching the loop node itself
        }
    }
    all_loop_bodies
}
```

**Note**: This validation applies to ANY scenario being validated, but the compilation
process (compile_from_called_scenario) doesn't actually use these validations for nested scenarios.

---

## 7. KEY ARCHITECTURAL FINDINGS

### Finding 1: Asymmetric Compilation
**Main Scenario**: Full IR compilation with all control flow structures
**Nested Scenario**: Partial IR compilation, missing Loop/While/TryCatch support

### Finding 2: Silent Failure Mode
Loop nodes in ne

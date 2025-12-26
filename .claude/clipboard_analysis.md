# Clipboard Architecture Analysis - RPA Studio

## Executive Summary

Current clipboard structure (`clipboard: Vec<Node>`) is insufficient for cross-scenario copy-paste because it only stores nodes, not their internal connections. When pasting into a different scenario, the code searches for connections in the **destination** scenario's connection list (which doesn't have them), not the **source** scenario.

---

## 1. Current Clipboard Structure

### Definition (main.rs:112)
```rust
clipboard: Vec<Node>
```

### What's Stored
- **Only:** The `Node` objects from the copied selection
- **Missing:** The `Connection` objects that link those nodes together

### Connection Definition (node_graph.rs:573-579)
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    pub id: Uuid,
    pub from_node: Uuid,
    pub to_node: Uuid,
    #[serde(default)]
    pub branch_type: BranchType,
}
```

---

## 2. The Cross-Scenario Copy-Paste Bug (rpa-90k)

### Root Cause
**Location:** crates/rpa-studio/src/main.rs:1256-1264

```rust
let connections_to_copy: Vec<_> = scenario        // <-- CURRENT scenario (destination)
    .connections
    .iter()
    .filter(|conn| {
        clipboard_node_ids.contains(&conn.from_node)
            && clipboard_node_ids.contains(&conn.to_node)
    })
    .cloned()
    .collect();
```

**The Problem:**
- Line 1256 calls `self.get_current_scenario_mut()` which returns the **destination** scenario
- This code searches for connections **in the destination scenario's list**
- But connections between the copied nodes exist **in the source scenario's list**
- When pasting to a different scenario, the destination scenario has no connections between those node IDs
- Result: **Connections are lost**

### Flow Diagram

```
Copy Operation (same scenario):
  1. Copy nodes from Scenario A
  2. Clipboard stores: nodes (IDs: UUID1, UUID2, UUID3)
  3. Connections exist in Scenario A's connection list

Paste into Same Scenario:
  ✅ WORKS - connections found in destination (Scenario A)

Paste into Different Scenario (Scenario B):
  1. Nodes added to Scenario B with new IDs (UUID4, UUID5, UUID6)
  2. Code searches Scenario B's connection list for connections
  3. Scenario B doesn't have connections between UUID1/UUID2/UUID3
  4. ❌ FAILS - connections lost because search happens in wrong scenario
```

---

## 3. The Data Flow Problem

### Current Paste Implementation (lines 1210-1274)

```
paste_clipboard_nodes():
  1. Get bounding box of clipboard nodes
  2. Calculate offset for new mouse position
  3. Create old_to_new_id HashMap for ID remapping
  4. Add all nodes to destination scenario with new IDs
  5. SEARCH DESTINATION SCENARIO for connections
       ↑ This is the bug - we need to search CLIPBOARD data
  6. Remap connection IDs using old_to_new_id
  7. Add remapped connections
```

### Why ID Remapping Works
```rust
let mut old_to_new_id: std::collections::HashMap<uuid::Uuid, uuid::Uuid> =
    std::collections::HashMap::new();

for node in &self.clipboard {
    let new_id = uuid::Uuid::new_v4();
    old_to_new_id.insert(node.id, new_id);  // node.id = original from source
    // new_id = new ID in destination
}

// Later, when remapping:
if let (Some(&new_from), Some(&new_to)) = (
    old_to_new_id.get(&conn.from_node),     // Look up old ID
    old_to_new_id.get(&conn.to_node),       // Look up old ID
) {
    scenario.add_connection_with_branch(new_from, new_to, conn.branch_type);
}
```

This mechanism **already exists and would work perfectly** if we just had the source connections in the clipboard.

---

## 4. Proposed Data Structure

### New Clipboard Type Definition

**Location to add:** crates/rpa-studio/src/main.rs

```rust
#[derive(Clone)]
struct ClipboardData {
    nodes: Vec<Node>,
    connections: Vec<Connection>,
}
```

### Updated RpaApp Field

**Current (line 112):**
```rust
clipboard: Vec<Node>,
```

**New:**
```rust
clipboard: ClipboardData,
```

Or with explicit naming for clarity:
```rust
clipboard: Option<ClipboardData>,
```

### Minimal Change Alternative

If you want to avoid creating a new struct:
```rust
clipboard_nodes: Vec<Node>,
clipboard_connections: Vec<Connection>,
```

---

## 5. Code Changes Required for rpa-90k

### Change 1: Update copy_selected_nodes() (lines 1198-1208)

**Before:**
```rust
fn copy_selected_nodes(&mut self) {
    let nodes_to_copy: Vec<_> = self
        .get_current_scenario()
        .nodes
        .iter()
        .filter(|n| self.selected_nodes.contains(&n.id))
        .cloned()
        .collect();
    self.clipboard.clear();
    self.clipboard.extend(nodes_to_copy);
}
```

**After (with struct):**
```rust
fn copy_selected_nodes(&mut self) {
    let scenario = self.get_current_scenario();
    
    let nodes_to_copy: Vec<_> = scenario
        .nodes
        .iter()
        .filter(|n| self.selected_nodes.contains(&n.id))
        .cloned()
        .collect();
    
    let node_ids: HashSet<Uuid> = nodes_to_copy.iter().map(|n| n.id).collect();
    
    let connections_to_copy: Vec<_> = scenario
        .connections
        .iter()
        .filter(|conn| {
            node_ids.contains(&conn.from_node) && node_ids.contains(&conn.to_node)
        })
        .cloned()
        .collect();
    
    self.clipboard = ClipboardData {
        nodes: nodes_to_copy,
        connections: connections_to_copy,
    };
}
```

### Change 2: Update paste_clipboard_nodes() (lines 1210-1274)

**Key fix at line 1256:**

**Before:**
```rust
let connections_to_copy: Vec<_> = scenario  // BUG: searches destination
    .connections
    .iter()
    .filter(|conn| {
        clipboard_node_ids.contains(&conn.from_node)
            && clipboard_node_ids.contains(&conn.to_node)
    })
    .cloned()
    .collect();
```

**After:**
```rust
// Use connections from clipboard, not from current scenario
let connections_to_copy: Vec<_> = self.clipboard.connections
    .iter()
    .filter(|conn| {
        clipboard_node_ids.contains(&conn.from_node)
            && clipboard_node_ids.contains(&conn.to_node)
    })
    .cloned()
    .collect();
```

### Change 3: Update clipboard initialization (line 135)

**Before:**
```rust
clipboard: Vec::new(),
```

**After (with struct):**
```rust
clipboard: ClipboardData {
    nodes: Vec::new(),
    connections: Vec::new(),
},
```

### Change 4: Update clipboard_empty checks (lines 213, 228, 778)

**Before:**
```rust
let clipboard_empty = self.clipboard.is_empty();
let has_clipboard = !self.clipboard.is_empty();
```

**After:**
```rust
let clipboard_empty = self.clipboard.nodes.is_empty();
let has_clipboard = !self.clipboard.nodes.is_empty();
```

---

## 6. Relationship to rpa-gpi (Cut Functionality)

### Issue Description
Implement Cut (Ctrl+X) that:
1. Copies nodes and their **internal connections**
2. Deletes the original nodes
3. Preserves connections when pasting

### How rpa-90k Fix Enables rpa-gpi

**The same data structure fixes both issues:**

```rust
fn cut_selected_nodes(&mut self) {
    // Step 1: Copy with connections (same as copy)
    self.copy_selected_nodes();
    
    // Step 2: Delete
    let nodes_to_remove: Vec<_> = self.selected_nodes.iter().copied().collect();
    let scenario = self.get_current_scenario_mut();
    for node_id in nodes_to_remove {
        scenario.remove_node(node_id);
    }
    self.selected_nodes.clear();
}
```

### Why They're Related

| Feature | Requires Connections in Clipboard? | Data Structure |
|---------|-----------------------------------|-----------------|
| Copy within scenario | ❌ No | `Vec<Node>` (current) |
| Copy between scenarios | ✅ **Yes** | `ClipboardData` (needed) |
| Cut (Ctrl+X) | ✅ **Yes** | `ClipboardData` (needed) |
| Cut & paste between scenarios | ✅ **Yes** | `ClipboardData` (needed) |

**Conclusion:** Fixing rpa-90k creates the exact data structure needed for rpa-gpi, so **both features should be implemented together** with a single data structure change.

---

## 7. Potential Conflicts & Dependencies

### No Conflicts Detected
- Both features work on the same `copy_selected_nodes()` and `paste_clipboard_nodes()` functio

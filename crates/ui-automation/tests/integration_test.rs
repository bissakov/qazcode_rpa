use ui_automation::{init, cleanup};

#[test]
fn test_initialization() {
    let result = init();
    assert!(result.is_ok());
    cleanup();
}

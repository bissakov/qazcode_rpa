use office::Excel;

#[test]
fn test_excel_create_workbook() {
    let mut workbook = Excel::create_workbook();
    let _ = workbook.worksheets();
}

#[test]
fn test_excel_write() {
    let temp_file = std::env::temp_dir().join("integration_test_excel.xlsx");
    let mut workbook = Excel::create_workbook();

    assert!(Excel::write(&mut workbook, &temp_file).is_ok());
    assert!(temp_file.exists(), "Excel file was not created");

    let _ = std::fs::remove_file(&temp_file);
}

#[test]
fn test_excel_roundtrip() {
    let temp_file = std::env::temp_dir().join("integration_test_excel_roundtrip.xlsx");

    let mut workbook = Excel::create_workbook();
    assert!(Excel::write(&mut workbook, &temp_file).is_ok());

    let read_workbook = Excel::read(&temp_file);
    assert!(read_workbook.is_ok(), "Failed to read Excel file back");

    let _ = std::fs::remove_file(&temp_file);
}

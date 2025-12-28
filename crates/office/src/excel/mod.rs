use calamine::{Reader, Xlsx};
use rust_xlsxwriter::Workbook;
use std::io::BufReader;
use std::path::Path;

pub struct Excel;

impl Excel {
    pub fn read<P: AsRef<Path>>(path: P) -> Result<Xlsx<BufReader<std::fs::File>>, String> {
        let path = path.as_ref();
        let file = std::fs::File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
        let reader = BufReader::new(file);
        Xlsx::new(reader).map_err(|e| format!("Failed to read Excel file: {}", e))
    }

    pub fn write<P: AsRef<Path>>(workbook: &mut Workbook, path: P) -> Result<(), String> {
        let path = path.as_ref();
        workbook
            .save(path)
            .map_err(|e| format!("Failed to write Excel file: {}", e))
    }

    pub fn create_workbook() -> Workbook {
        Workbook::new()
    }

    pub fn get_cell_value(
        sheet_data: &[Vec<calamine::DataType>],
        row: usize,
        col: usize,
    ) -> Option<String> {
        sheet_data
            .get(row)
            .and_then(|r| r.get(col))
            .map(|cell| match cell {
                calamine::DataType::Empty => String::new(),
                calamine::DataType::String(s) => s.clone(),
                calamine::DataType::Float(f) => f.to_string(),
                calamine::DataType::Int(i) => i.to_string(),
                calamine::DataType::Bool(b) => b.to_string(),
                calamine::DataType::Error(e) => format!("Error: {:?}", e),
                calamine::DataType::DateTime(dt) => dt.to_string(),
                calamine::DataType::Duration(d) => d.to_string(),
                calamine::DataType::DurationIso(d) => d.to_string(),
                calamine::DataType::DateTimeIso(dt) => dt.to_string(),
            })
    }
}

impl Default for Excel {
    fn default() -> Self {
        Excel
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_new_workbook() {
        let mut workbook = Excel::create_workbook();
        let _ = workbook.worksheets();
    }

    #[test]
    fn test_write_workbook() {
        let temp_file = std::env::temp_dir().join("test_excel_write.xlsx");
        let mut workbook = Excel::create_workbook();

        assert!(Excel::write(&mut workbook, &temp_file).is_ok());
        assert!(temp_file.exists());

        let _ = std::fs::remove_file(&temp_file);
    }

    #[test]
    fn test_read_write_roundtrip() {
        let temp_file = std::env::temp_dir().join("test_excel_roundtrip.xlsx");

        let mut workbook = Excel::create_workbook();
        assert!(Excel::write(&mut workbook, &temp_file).is_ok());

        let result = Excel::read(&temp_file);
        assert!(result.is_ok());

        let _ = std::fs::remove_file(&temp_file);
    }
}

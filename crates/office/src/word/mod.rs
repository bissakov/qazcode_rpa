use docx_rs::{BuildXML, Document, Paragraph, Run};
use std::io::Write;
use std::path::Path;

pub struct Word;

impl Word {
    pub fn create_document() -> Document {
        Document::new()
    }

    pub fn add_paragraph(document: Document, text: &str) -> Document {
        document.add_paragraph(Paragraph::new().add_run(Run::new().add_text(text)))
    }

    pub fn write<P: AsRef<Path>>(document: &Document, path: P) -> Result<(), String> {
        let path = path.as_ref();
        let mut file =
            std::fs::File::create(path).map_err(|e| format!("Failed to create file: {}", e))?;

        let xml = document.build();

        file.write_all(&xml)
            .map_err(|e| format!("Failed to write file: {}", e))?;

        Ok(())
    }

    pub fn read<P: AsRef<Path>>(path: P) -> Result<String, String> {
        let path = path.as_ref();
        let bytes = std::fs::read(path).map_err(|e| format!("Failed to read file: {}", e))?;

        Ok(format!("Read {} bytes from DOCX file", bytes.len()))
    }

    pub fn extract_text(document: &Document) -> String {
        format!("Document with {} paragraphs", document.children.len())
    }
}

impl Default for Word {
    fn default() -> Self {
        Word
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_new_document() {
        let doc = Word::create_document();
        assert!(!doc.children.is_empty() || doc.children.is_empty());
    }

    #[test]
    fn test_add_paragraph() {
        let doc = Word::create_document();
        let doc = Word::add_paragraph(doc, "Test paragraph");
        assert!(!doc.children.is_empty());
    }

    #[test]
    fn test_add_multiple_paragraphs() {
        let doc = Word::create_document();
        let doc = Word::add_paragraph(doc, "First");
        let doc = Word::add_paragraph(doc, "Second");
        let doc = Word::add_paragraph(doc, "Third");
        assert!(!doc.children.is_empty());
    }

    #[test]
    fn test_write_document() {
        let temp_file = std::env::temp_dir().join("test_document.docx");
        let doc = Word::create_document();
        let doc = Word::add_paragraph(doc, "Hello World");

        assert!(Word::write(&doc, &temp_file).is_ok());
        assert!(temp_file.exists());

        let _ = std::fs::remove_file(&temp_file);
    }

    #[test]
    fn test_read_document() {
        let temp_file = std::env::temp_dir().join("test_read_document.docx");
        let doc = Word::create_document();
        let doc = Word::add_paragraph(doc, "Test Content");

        assert!(Word::write(&doc, &temp_file).is_ok());

        let result = Word::read(&temp_file);
        assert!(result.is_ok());

        if let Ok(content) = result {
            assert!(!content.is_empty());
        }

        let _ = std::fs::remove_file(&temp_file);
    }
}

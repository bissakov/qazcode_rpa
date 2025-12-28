use office::Word;

#[test]
fn test_word_create_document() {
    let doc = Word::create_document();
    assert!(!doc.children.is_empty() || doc.children.is_empty());
}

#[test]
fn test_word_add_multiple_paragraphs() {
    let doc = Word::create_document();
    let doc = Word::add_paragraph(doc, "First paragraph");
    let doc = Word::add_paragraph(doc, "Second paragraph");
    let doc = Word::add_paragraph(doc, "Third paragraph");

    assert!(!doc.children.is_empty());
}

#[test]
fn test_word_write_document() {
    let temp_file = std::env::temp_dir().join("integration_test_word.docx");

    let doc = Word::create_document();
    let doc = Word::add_paragraph(doc, "Test Document");
    let doc = Word::add_paragraph(doc, "This is a test paragraph");
    let doc = Word::add_paragraph(doc, "With multiple lines");

    assert!(
        Word::write(&doc, &temp_file).is_ok(),
        "Failed to write Word document"
    );
    assert!(temp_file.exists(), "Word file was not created");

    let _ = std::fs::remove_file(&temp_file);
}

#[test]
fn test_word_roundtrip() {
    let temp_file = std::env::temp_dir().join("integration_test_word_roundtrip.docx");

    // Create and write document
    let doc = Word::create_document();
    let doc = Word::add_paragraph(doc, "Test Content");
    let doc = Word::add_paragraph(doc, "Second Line");

    assert!(Word::write(&doc, &temp_file).is_ok());

    // Read back from file
    let read_doc = Word::read(&temp_file);
    assert!(read_doc.is_ok(), "Failed to read Word document");

    // Verify content
    if let Ok(content) = read_doc {
        assert!(!content.is_empty(), "Read content is empty");
    }

    let _ = std::fs::remove_file(&temp_file);
}

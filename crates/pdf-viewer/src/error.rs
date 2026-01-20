use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum PdfError {
    #[error("File not found: {0}")]
    FileNotFound(PathBuf),

    #[error("Invalid PDF file: {0}")]
    InvalidPdf(String),

    #[error("Render failed: {0}")]
    RenderFailed(String),

    #[error("Cache miss")]
    CacheMiss,

    #[error("Thread error: {0}")]
    ThreadError(String),

    #[error("I/O error: {0}")]
    IoError(String),

    #[error("Page out of range")]
    PageOutOfRange,

    #[error("Anchor resolution failed: {0}")]
    AnchorResolutionFailed(String),

    #[error("Invalid polygon: {0}")]
    InvalidPolygon(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("File write error: {0}")]
    FileWriteError(String),

    #[error("File read error: {0}")]
    FileReadError(String),
}

pub type Result<T> = std::result::Result<T, PdfError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_not_found_display() {
        let error = PdfError::FileNotFound(PathBuf::from("/test/path.pdf"));
        let msg = format!("{}", error);
        assert!(msg.contains("File not found"));
        assert!(msg.contains("path.pdf"));
    }

    #[test]
    fn test_invalid_pdf_display() {
        let error = PdfError::InvalidPdf("corrupted header".to_string());
        let msg = format!("{}", error);
        assert_eq!(msg, "Invalid PDF file: corrupted header");
    }

    #[test]
    fn test_render_failed_display() {
        let error = PdfError::RenderFailed("GPU error".to_string());
        let msg = format!("{}", error);
        assert_eq!(msg, "Render failed: GPU error");
    }

    #[test]
    fn test_cache_miss_display() {
        let error = PdfError::CacheMiss;
        let msg = format!("{}", error);
        assert_eq!(msg, "Cache miss");
    }

    #[test]
    fn test_thread_error_display() {
        let error = PdfError::ThreadError("deadlock detected".to_string());
        let msg = format!("{}", error);
        assert_eq!(msg, "Thread error: deadlock detected");
    }

    #[test]
    fn test_io_error_display() {
        let error = PdfError::IoError("permission denied".to_string());
        let msg = format!("{}", error);
        assert_eq!(msg, "I/O error: permission denied");
    }

    #[test]
    fn test_page_out_of_range_display() {
        let error = PdfError::PageOutOfRange;
        let msg = format!("{}", error);
        assert_eq!(msg, "Page out of range");
    }

    #[test]
    fn test_anchor_resolution_failed_display() {
        let error = PdfError::AnchorResolutionFailed("text not found".to_string());
        let msg = format!("{}", error);
        assert_eq!(msg, "Anchor resolution failed: text not found");
    }

    #[test]
    fn test_invalid_polygon_display() {
        let error = PdfError::InvalidPolygon("not enough vertices".to_string());
        let msg = format!("{}", error);
        assert_eq!(msg, "Invalid polygon: not enough vertices");
    }

    #[test]
    fn test_serialization_error_display() {
        let error = PdfError::SerializationError("invalid JSON".to_string());
        let msg = format!("{}", error);
        assert_eq!(msg, "Serialization error: invalid JSON");
    }

    #[test]
    fn test_file_write_error_display() {
        let error = PdfError::FileWriteError("disk full".to_string());
        let msg = format!("{}", error);
        assert_eq!(msg, "File write error: disk full");
    }

    #[test]
    fn test_file_read_error_display() {
        let error = PdfError::FileReadError("file locked".to_string());
        let msg = format!("{}", error);
        assert_eq!(msg, "File read error: file locked");
    }

    #[test]
    fn test_error_is_cloneable() {
        let error = PdfError::CacheMiss;
        let cloned = error.clone();
        assert_eq!(format!("{}", error), format!("{}", cloned));
    }

    #[test]
    fn test_error_is_debug() {
        let error = PdfError::PageOutOfRange;
        let debug = format!("{:?}", error);
        assert!(debug.contains("PageOutOfRange"));
    }

    #[test]
    fn test_result_type_ok() {
        let result: Result<i32> = Ok(42);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_result_type_err() {
        let result: Result<i32> = Err(PdfError::CacheMiss);
        assert!(result.is_err());
    }
}

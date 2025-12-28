use office::Outlook;

#[test]
#[cfg(target_os = "windows")]
fn test_outlook_connect_windows() {
    let outlook = Outlook::connect();
    assert!(outlook.is_ok());
}

#[test]
#[cfg(not(target_os = "windows"))]
fn test_outlook_connect_non_windows() {
    let outlook = Outlook::connect();
    assert!(outlook.is_err());
}

#[test]
#[cfg(target_os = "windows")]
fn test_outlook_send_email_windows() {
    dotenv::dotenv().ok();
    let outlook = Outlook::connect().expect("Failed to connect to Outlook");

    let recipient =
        std::env::var("OUTLOOK_TEST_RECIPIENT").unwrap_or_else(|_| "test@example.com".to_string());
    let subject =
        std::env::var("OUTLOOK_TEST_SUBJECT_PREFIX").unwrap_or_else(|_| "RPA Test".to_string());
    let body =
        std::env::var("OUTLOOK_TEST_BODY").unwrap_or_else(|_| "Automated test email".to_string());

    let result = outlook.send_email(&recipient, &subject, &body);
    assert!(result.is_ok(), "Failed to send email: {:?}", result.err());
}

#[test]
#[cfg(target_os = "windows")]
fn test_outlook_send_email_with_attachments_windows() {
    dotenv::dotenv().ok();
    let outlook = Outlook::connect().expect("Failed to connect to Outlook");

    let recipient =
        std::env::var("OUTLOOK_TEST_RECIPIENT").unwrap_or_else(|_| "test@example.com".to_string());

    let result = outlook.send_email_with_attachments(
        &recipient,
        "RPA Test with Attachments",
        "This email has attachments",
        &[],
    );
    assert!(
        result.is_ok(),
        "Failed to send email with attachments: {:?}",
        result.err()
    );
}

#[test]
#[cfg(target_os = "windows")]
fn test_outlook_get_emails_windows() {
    let outlook = Outlook::connect().expect("Failed to connect to Outlook");
    let result = outlook.get_emails("Inbox");
    assert!(result.is_ok());
    if let Ok(emails) = result {
        assert!(!emails.is_empty());
    }
}

#[test]
fn test_email_message_with_attachments() {
    let email = office::EmailMessage {
        to: "test@example.com".to_string(),
        subject: "Test".to_string(),
        body: "Test body".to_string(),
        timestamp: "2025-01-01T00:00:00Z".to_string(),
        attachments: vec!["file.pdf".to_string(), "document.docx".to_string()],
    };
    assert_eq!(email.to, "test@example.com");
    assert_eq!(email.attachments.len(), 2);
    assert_eq!(email.attachments[0], "file.pdf");
}

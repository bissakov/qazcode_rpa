#[derive(Debug, Clone)]
pub struct EmailMessage {
    pub to: String,
    pub subject: String,
    pub body: String,
    pub timestamp: String,
    pub attachments: Vec<String>,
}

pub struct Outlook;

impl Outlook {
    pub fn connect() -> Result<Self, String> {
        #[cfg(target_os = "windows")]
        {
            Ok(Outlook)
        }

        #[cfg(not(target_os = "windows"))]
        {
            Err("Outlook automation is only supported on Windows".to_string())
        }
    }

    pub fn send_email(&self, to: &str, subject: &str, body: &str) -> Result<(), String> {
        #[cfg(target_os = "windows")]
        {
            send_email_impl(to, subject, body, &[])
        }

        #[cfg(not(target_os = "windows"))]
        {
            Err("Outlook automation is only supported on Windows".to_string())
        }
    }

    pub fn send_email_with_attachments(
        &self,
        to: &str,
        subject: &str,
        body: &str,
        attachments: &[&str],
    ) -> Result<(), String> {
        #[cfg(target_os = "windows")]
        {
            send_email_impl(to, subject, body, attachments)
        }

        #[cfg(not(target_os = "windows"))]
        {
            Err("Outlook automation is only supported on Windows".to_string())
        }
    }

    pub fn get_emails(&self, folder: &str) -> Result<Vec<EmailMessage>, String> {
        #[cfg(target_os = "windows")]
        {
            get_emails_impl(folder)
        }

        #[cfg(not(target_os = "windows"))]
        {
            Err("Outlook automation is only supported on Windows".to_string())
        }
    }
}

#[cfg(target_os = "windows")]
fn send_email_impl(
    to: &str,
    subject: &str,
    body: &str,
    attachments: &[&str],
) -> Result<(), String> {
    use windows::Win32::System::Com::*;
    use windows::core::*;

    use windows::Win32::System::Variant::*;

    unsafe {
        let hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        if hr.is_err() {
            return Err(format!("Failed to initialize COM: {:?}", hr));
        }

        let result = (|| -> std::result::Result<(), String> {
            let outlook_app_clsid = CLSIDFromProgID(w!("Outlook.Application"))
                .map_err(|e| format!("Failed to get Outlook CLSID: {}", e))?;

            let outlook_app: IDispatch =
                CoCreateInstance(&outlook_app_clsid, None, CLSCTX_LOCAL_SERVER).map_err(|e| {
                    format!(
                        "Failed to create Outlook instance: {}. Is Outlook installed?",
                        e
                    )
                })?;

            let mail_item = invoke_method(&outlook_app, "CreateItem", &[VARIANT::from(0i32)])
                .map_err(|e| format!("Failed to create mail item: {}", e))?;

            let mail_dispatch: IDispatch = mail_item
                .Anonymous
                .Anonymous
                .Anonymous
                .pdispVal
                .as_ref()
                .ok_or("Failed to get mail item dispatch")?
                .clone();

            set_property(&mail_dispatch, "To", &VARIANT::from(to))
                .map_err(|e| format!("Failed to set To: {}", e))?;
            set_property(&mail_dispatch, "Subject", &VARIANT::from(subject))
                .map_err(|e| format!("Failed to set Subject: {}", e))?;
            set_property(&mail_dispatch, "Body", &VARIANT::from(body))
                .map_err(|e| format!("Failed to set Body: {}", e))?;

            if !attachments.is_empty() {
                let attachments_obj = get_property(&mail_dispatch, "Attachments")
                    .map_err(|e| format!("Failed to get Attachments: {}", e))?;

                let attachments_dispatch: IDispatch = attachments_obj
                    .Anonymous
                    .Anonymous
                    .Anonymous
                    .pdispVal
                    .as_ref()
                    .ok_or("Failed to get attachments dispatch")?
                    .clone();

                for attachment_path in attachments {
                    invoke_method(
                        &attachments_dispatch,
                        "Add",
                        &[VARIANT::from(*attachment_path)],
                    )
                    .map_err(|e| format!("Failed to add attachment {}: {}", attachment_path, e))?;
                }
            }

            invoke_method(&mail_dispatch, "Send", &[])
                .map_err(|e| format!("Failed to send email: {}", e))?;

            Ok(())
        })();

        CoUninitialize();
        result
    }
}

#[cfg(target_os = "windows")]
unsafe fn invoke_method(
    dispatch: &windows::Win32::System::Com::IDispatch,
    method: &str,
    args: &[windows::Win32::System::Variant::VARIANT],
) -> std::result::Result<windows::Win32::System::Variant::VARIANT, String> {
    unsafe {
        use windows::Win32::System::Com::*;
        use windows::core::*;

        use windows::Win32::System::Variant::*;

        let method_name = BSTR::from(method);
        let mut dispid = 0i32;
        let names_ptr = method_name.as_ptr();

        dispatch
            .GetIDsOfNames(
                &GUID::zeroed(),
                &names_ptr as *const _ as *const _,
                1,
                0,
                &mut dispid,
            )
            .map_err(|e| format!("GetIDsOfNames failed for {}: {}", method, e))?;

        let mut params: Vec<VARIANT> = args.iter().cloned().collect();
        params.reverse();

        let mut dispparams = DISPPARAMS {
            rgvarg: if params.is_empty() {
                std::ptr::null_mut()
            } else {
                params.as_mut_ptr()
            },
            rgdispidNamedArgs: std::ptr::null_mut(),
            cArgs: params.len() as u32,
            cNamedArgs: 0,
        };

        let mut result = VARIANT::default();
        let mut excepinfo = EXCEPINFO::default();

        dispatch
            .Invoke(
                dispid,
                &GUID::zeroed(),
                0,
                DISPATCH_METHOD,
                &mut dispparams,
                Some(&mut result),
                Some(&mut excepinfo),
                None,
            )
            .map_err(|e| format!("Invoke failed for {}: {}", method, e))?;

        Ok(result)
    }
}

#[cfg(target_os = "windows")]
unsafe fn get_property(
    dispatch: &windows::Win32::System::Com::IDispatch,
    property: &str,
) -> std::result::Result<windows::Win32::System::Variant::VARIANT, String> {
    unsafe {
        use windows::Win32::System::Com::*;
        use windows::core::*;

        use windows::Win32::System::Variant::*;

        let property_name = BSTR::from(property);
        let mut dispid = 0i32;
        let names_ptr = property_name.as_ptr();

        dispatch
            .GetIDsOfNames(
                &GUID::zeroed(),
                &names_ptr as *const _ as *const _,
                1,
                0,
                &mut dispid,
            )
            .map_err(|e| format!("GetIDsOfNames failed for {}: {}", property, e))?;

        let mut dispparams = DISPPARAMS::default();
        let mut result = VARIANT::default();
        let mut excepinfo = EXCEPINFO::default();

        dispatch
            .Invoke(
                dispid,
                &GUID::zeroed(),
                0,
                DISPATCH_PROPERTYGET,
                &mut dispparams,
                Some(&mut result),
                Some(&mut excepinfo),
                None,
            )
            .map_err(|e| format!("Invoke failed for {}: {}", property, e))?;

        Ok(result)
    }
}

#[cfg(target_os = "windows")]
unsafe fn set_property(
    dispatch: &windows::Win32::System::Com::IDispatch,
    property: &str,
    value: &windows::Win32::System::Variant::VARIANT,
) -> std::result::Result<(), String> {
    unsafe {
        use windows::Win32::System::Com::*;
        use windows::Win32::System::Ole::*;
        use windows::core::*;

        let property_name = BSTR::from(property);
        let mut dispid = 0i32;
        let names_ptr = property_name.as_ptr();

        dispatch
            .GetIDsOfNames(
                &GUID::zeroed(),
                &names_ptr as *const _ as *const _,
                1,
                0,
                &mut dispid,
            )
            .map_err(|e| format!("GetIDsOfNames failed for {}: {}", property, e))?;

        let mut value_copy = value.clone();
        let dispid_propertyput = DISPID_PROPERTYPUT;

        let mut dispparams = DISPPARAMS {
            rgvarg: &mut value_copy,
            rgdispidNamedArgs: &dispid_propertyput as *const _ as *mut _,
            cArgs: 1,
            cNamedArgs: 1,
        };

        let mut excepinfo = EXCEPINFO::default();

        dispatch
            .Invoke(
                dispid,
                &GUID::zeroed(),
                0,
                DISPATCH_PROPERTYPUT,
                &mut dispparams,
                None,
                Some(&mut excepinfo),
                None,
            )
            .map_err(|e| format!("Invoke failed for {}: {}", property, e))?;

        Ok(())
    }
}

#[cfg(not(target_os = "windows"))]
fn send_email_impl(
    _to: &str,
    _subject: &str,
    _body: &str,
    _attachments: &[&str],
) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "windows")]
fn get_emails_impl(folder: &str) -> Result<Vec<EmailMessage>, String> {
    use windows::Win32::System::Com::*;
    use windows::core::*;

    use windows::Win32::System::Variant::*;

    unsafe {
        let hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        if hr.is_err() {
            return Err(format!("Failed to initialize COM: {:?}", hr));
        }

        let result = (|| -> std::result::Result<Vec<EmailMessage>, String> {
            let outlook_app_clsid = CLSIDFromProgID(w!("Outlook.Application"))
                .map_err(|e| format!("Failed to get Outlook CLSID: {}", e))?;

            let outlook_app: IDispatch =
                CoCreateInstance(&outlook_app_clsid, None, CLSCTX_LOCAL_SERVER).map_err(|e| {
                    format!(
                        "Failed to create Outlook instance: {}. Is Outlook installed?",
                        e
                    )
                })?;

            let namespace = invoke_method(&outlook_app, "GetNamespace", &[VARIANT::from("MAPI")])
                .map_err(|e| format!("Failed to get namespace: {}", e))?;

            let namespace_dispatch: IDispatch = namespace
                .Anonymous
                .Anonymous
                .Anonymous
                .pdispVal
                .as_ref()
                .ok_or("Failed to get namespace dispatch")?
                .clone();

            let folder_obj = if folder.eq_ignore_ascii_case("inbox") {
                invoke_method(
                    &namespace_dispatch,
                    "GetDefaultFolder",
                    &[VARIANT::from(6i32)],
                )
                .map_err(|e| format!("Failed to get Inbox folder: {}", e))?
            } else {
                let folders = get_property(&namespace_dispatch, "Folders")
                    .map_err(|e| format!("Failed to get Folders: {}", e))?;

                let folders_dispatch: IDispatch = folders
                    .Anonymous
                    .Anonymous
                    .Anonymous
                    .pdispVal
                    .as_ref()
                    .ok_or("Failed to get folders dispatch")?
                    .clone();

                invoke_method(&folders_dispatch, "Item", &[VARIANT::from(folder)])
                    .map_err(|e| format!("Failed to get folder '{}': {}", folder, e))?
            };

            let folder_dispatch: IDispatch = folder_obj
                .Anonymous
                .Anonymous
                .Anonymous
                .pdispVal
                .as_ref()
                .ok_or("Failed to get folder dispatch")?
                .clone();

            let items_obj = get_property(&folder_dispatch, "Items")
                .map_err(|e| format!("Failed to get Items: {}", e))?;

            let items_dispatch: IDispatch = items_obj
                .Anonymous
                .Anonymous
                .Anonymous
                .pdispVal
                .as_ref()
                .ok_or("Failed to get items dispatch")?
                .clone();

            let count_var = get_property(&items_dispatch, "Count")
                .map_err(|e| format!("Failed to get Count: {}", e))?;

            let count = count_var.Anonymous.Anonymous.Anonymous.lVal;

            let mut messages = Vec::new();

            for i in 1..=count.min(50) {
                let item_result = invoke_method(&items_dispatch, "Item", &[VARIANT::from(i)]);

                if let Ok(item_var) = item_result {
                    if let Some(item_dispatch) =
                        item_var.Anonymous.Anonymous.Anonymous.pdispVal.as_ref()
                    {
                        let to = get_string_property(item_dispatch, "To").unwrap_or_default();
                        let subject =
                            get_string_property(item_dispatch, "Subject").unwrap_or_default();
                        let body = get_string_property(item_dispatch, "Body").unwrap_or_default();
                        let received_time = get_string_property(item_dispatch, "ReceivedTime")
                            .unwrap_or_else(|_| chrono::Utc::now().to_rfc3339());

                        let attachments_obj = get_property(item_dispatch, "Attachments");
                        let mut attachment_names = Vec::new();

                        if let Ok(attachments_var) = attachments_obj {
                            if let Some(attachments_dispatch) = attachments_var
                                .Anonymous
                                .Anonymous
                                .Anonymous
                                .pdispVal
                                .as_ref()
                            {
                                if let Ok(att_count_var) =
                                    get_property(attachments_dispatch, "Count")
                                {
                                    let att_count =
                                        att_count_var.Anonymous.Anonymous.Anonymous.lVal;

                                    for j in 1..=att_count {
                                        if let Ok(att_item) = invoke_method(
                                            attachments_dispatch,
                                            "Item",
                                            &[VARIANT::from(j)],
                                        ) {
                                            if let Some(att_dispatch) = att_item
                                                .Anonymous
                                                .Anonymous
                                                .Anonymous
                                                .pdispVal
                                                .as_ref()
                                            {
                                                if let Ok(filename) =
                                                    get_string_property(att_dispatch, "FileName")
                                                {
                                                    attachment_names.push(filename);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        messages.push(EmailMessage {
                            to,
                            subject,
                            body,
                            timestamp: received_time,
                            attachments: attachment_names,
                        });
                    }
                }
            }

            Ok(messages)
        })();

        CoUninitialize();
        result
    }
}

#[cfg(target_os = "windows")]
unsafe fn get_string_property(
    dispatch: &windows::Win32::System::Com::IDispatch,
    property: &str,
) -> std::result::Result<String, String> {
    use windows::Win32::System::Variant::*;

    let variant = unsafe { get_property(dispatch, property) }?;

    unsafe {
        if variant.Anonymous.Anonymous.vt == VT_BSTR {
            let bstr = &variant.Anonymous.Anonymous.Anonymous.bstrVal;
            if !bstr.is_empty() {
                return Ok(bstr.to_string());
            }
        }
    }
    Ok(String::new())
}

#[cfg(not(target_os = "windows"))]
fn get_emails_impl(_folder: &str) -> Result<Vec<EmailMessage>, String> {
    Ok(vec![])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "windows")]
    fn test_connect_windows() {
        let outlook = Outlook::connect();
        assert!(outlook.is_ok());
    }

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn test_connect_non_windows() {
        let outlook = Outlook::connect();
        assert!(outlook.is_err());
    }

    #[test]
    fn test_email_message_with_attachments() {
        let email = EmailMessage {
            to: "test@example.com".to_string(),
            subject: "Test".to_string(),
            body: "Test body".to_string(),
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            attachments: vec!["file.pdf".to_string()],
        };
        assert_eq!(email.to, "test@example.com");
        assert_eq!(email.attachments.len(), 1);
    }
}

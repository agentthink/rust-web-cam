use quick_xml::events::Event;
use quick_xml::Reader;

pub fn build_soap_response(body: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
<s:Body>{}</s:Body>
</s:Envelope>"#,
        body
    )
}

pub fn build_soap_fault(fault_code: &str, fault_subcode: &str, fault_reason: &str) -> String {
    build_soap_response(&format!(
        r#"<s:Fault>
  <s:Code><s:Value>{}</s:Value><s:Subcode><s:Value>{}</s:Value></s:Subcode></s:Code>
  <s:Reason><s:Text>{}</s:Text></s:Reason>
</s:Fault>"#,
        fault_code, fault_subcode, fault_reason
    ))
}

fn local_name(name: &str) -> &str {
    if let Some(idx) = name.find(':') {
        &name[idx + 1..]
    } else {
        name
    }
}

/// 从 SOAP 请求中提取操作名
pub fn extract_action(soap_body: &[u8]) -> Option<String> {
    let mut reader = Reader::from_reader(soap_body);
    reader.config_mut().trim_text(true);

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let name = e.name().into_inner();
                if name.ends_with(":Envelope")
                    || name == "Envelope"
                    || name.ends_with(":Body")
                    || name == "Body"
                {
                    continue;
                }
                return Some(local_name(name).to_string());
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }
    None
}

/// 提取指定元素的文本内容
pub fn extract_element_text(soap_body: &[u8], element: &str) -> Option<String> {
    let mut reader = Reader::from_reader(soap_body);
    reader.config_mut().trim_text(true);
    let mut capture = false;
    let mut result = String::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let name = local_name(e.name().into_inner());
                if name == element {
                    capture = true;
                }
            }
            Ok(Event::Text(ref e)) if capture => {
                result.push_str(&e.to_string());
            }
            Ok(Event::End(ref e)) => {
                let name = local_name(e.name().into_inner());
                if name == element {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }

    if result.is_empty() {
        None
    } else {
        Some(result.trim().to_string())
    }
}

/// 提取指定元素的属性值
pub fn extract_attribute(soap_body: &[u8], parent: &str, attr: &str) -> Option<String> {
    let mut reader = Reader::from_reader(soap_body);
    reader.config_mut().trim_text(true);
    let mut in_parent = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let name = local_name(e.name().into_inner());

                if name == parent {
                    in_parent = true;
                }

                if in_parent {
                    for a in e.attributes().flatten() {
                        let key = a.key.into_inner();
                        if key == attr {
                            return Some(a.value.to_string());
                        }
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                let name = local_name(e.name().into_inner());
                if name == parent {
                    in_parent = false;
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_action() {
        let body = br#"<?xml version="1.0"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
<s:Body><GetDeviceInformation xmlns="http://www.onvif.org/ver10/device/wsdl"/></s:Body>
</s:Envelope>"#;
        assert_eq!(extract_action(body).unwrap(), "GetDeviceInformation");
    }
}

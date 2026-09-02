pub const NS_WSA: &str = "http://www.w3.org/2005/08/addressing";
pub const NS_WSD: &str = "http://schemas.xmlsoap.org/ws/2005/04/discovery";
pub const NS_WSE: &str = "http://schemas.xmlsoap.org/ws/2004/08/event";
pub const NS_ONVIF: &str = "http://www.onvif.org/ver10/schema";
pub const NS_TEV: &str = "http://www.onvif.org/ver10/events/wsdl";
pub const NS_TPTZ: &str = "http://www.onvif.org/ver20/ptz/wsdl";
pub const NS_WSSE: &str = "http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-secext-1.0.xsd";

pub const DEVICE_TYPE: &str = "dn:NetworkVideoTransmitter";

#[derive(Debug, Clone)]
pub struct EndpointReference { pub address: String }

#[derive(Debug, Clone)]
pub struct ProbeMatch {
    pub epr: EndpointReference,
    pub types: Vec<String>,
    pub scopes: Vec<String>,
    pub x_addrs: Vec<String>,
    pub metadata_version: u32,
}

impl ProbeMatch {
    pub fn to_xml(&self) -> String {
        format!(
            r#"<wsd:ProbeMatch>
  <wsd:EndpointReference><wsa:Address>{}</wsa:Address></wsd:EndpointReference>
  <wsd:Types>{}</wsd:Types>
  <wsd:Scopes>{}</wsd:Scopes>
  <wsd:XAddrs>{}</wsd:XAddrs>
  <wsd:MetadataVersion>{}</wsd:MetadataVersion>
</wsd:ProbeMatch>"#,
            self.epr.address, self.types.join(" "), self.scopes.join(" "), self.x_addrs.join(" "), self.metadata_version
        )
    }
}

#[derive(Debug, Clone)]
pub struct DeviceAddress {
    pub x_addr: String,
    pub types: Vec<String>,
    pub scopes: Vec<String>,
    pub epr_address: Option<String>,
}
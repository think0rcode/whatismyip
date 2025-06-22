use serde::{Deserialize, Serialize};
use worker::*;

// Constants for better maintainability
const CLOUDFLARE_API_BASE: &str = "https://api.cloudflare.com/client/v4";
const DNS_TTL: u32 = 1;
const CONTENT_TYPE_JSON: &str = "application/json";

/// DNS record types supported by this implementation
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(clippy::upper_case_acronyms)]
pub enum RecordType {
    /// IPv4 address record
    A,
    /// IPv6 address record  
    AAAA,
}

impl RecordType {
    fn as_str(&self) -> &'static str {
        match self {
            RecordType::A => "A",
            RecordType::AAAA => "AAAA",
        }
    }
}

/// Custom error types for DNS operations
#[derive(Debug)]
pub enum DnsError {
    ApiError(String),
    SerializationError(String),
    NotFound,
    InvalidInput(String),
}

impl From<DnsError> for Error {
    fn from(err: DnsError) -> Self {
        match err {
            DnsError::ApiError(msg) => Error::RustError(format!("DNS API error: {}", msg)),
            DnsError::SerializationError(msg) => {
                Error::RustError(format!("Serialization error: {}", msg))
            }
            DnsError::NotFound => Error::RustError("DNS record not found".to_string()),
            DnsError::InvalidInput(msg) => Error::RustError(format!("Invalid input: {}", msg)),
        }
    }
}

/// Cloudflare DNS record identifiers stored in KV
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DnsRecordInfo {
    pub record_name: String,
    pub a_id: Option<String>,
    pub aaaa_id: Option<String>,
}

impl DnsRecordInfo {
    fn new(record_name: String) -> Self {
        Self {
            record_name,
            a_id: None,
            aaaa_id: None,
        }
    }

    fn get_id(&self, record_type: RecordType) -> Option<&String> {
        match record_type {
            RecordType::A => self.a_id.as_ref(),
            RecordType::AAAA => self.aaaa_id.as_ref(),
        }
    }

    fn set_id(&mut self, record_type: RecordType, id: String) {
        match record_type {
            RecordType::A => self.a_id = Some(id),
            RecordType::AAAA => self.aaaa_id = Some(id),
        }
    }
}

/// Cloudflare API response for DNS record creation
#[derive(Deserialize)]
struct CreateDnsResponse {
    success: bool,
    result: Option<DnsRecord>,
    errors: Option<Vec<ApiError>>,
}

/// Cloudflare API response for listing DNS records
#[derive(Deserialize)]
struct ListDnsResponse {
    success: bool,
    result: Option<Vec<DnsRecord>>,
    errors: Option<Vec<ApiError>>,
}

/// Cloudflare API response for updating DNS records
#[derive(Deserialize)]
struct UpdateDnsResponse {
    success: bool,
    errors: Option<Vec<ApiError>>,
}

/// Cloudflare API error structure
#[derive(Deserialize, Debug)]
struct ApiError {
    #[allow(dead_code)]
    code: u32,
    #[allow(dead_code)]
    message: String,
}

/// Cloudflare DNS record structure
#[derive(Deserialize)]
struct DnsRecord {
    id: String,
    #[serde(rename = "type")]
    record_type: String,
    name: String,
    #[allow(dead_code)]
    content: String,
}

/// DNS manager for handling Cloudflare DNS operations
pub struct DnsManager<'a> {
    zone_id: String,
    token: String,
    kv: &'a kv::KvStore,
}

impl<'a> DnsManager<'a> {
    /// Create a new DNS manager instance
    pub fn new(zone_id: String, token: String, kv: &'a kv::KvStore) -> Self {
        Self { zone_id, token, kv }
    }

    /// Generate KV key for DNS record info
    fn dns_record_key(&self, homename: &str) -> String {
        format!("{}_dns_record_id", homename)
    }

    /// Generate KV key for IP address storage
    fn ip_key(&self, homename: &str, record_type: RecordType) -> String {
        match record_type {
            RecordType::A => format!("{}_v4", homename),
            RecordType::AAAA => format!("{}_v6", homename),
        }
    }

    /// Get or create DNS record IDs for a hostname
    pub async fn get_or_create_record_ids(
        &self,
        homename: &str,
        record_name: &str,
    ) -> Result<DnsRecordInfo> {
        let dns_key: String = self.dns_record_key(homename);

        // First, check KV for existing record info
        if let Some(dns_info_value) = self.kv.get(&dns_key).text().await? {
            if let Ok(dns_info) = serde_json::from_str::<DnsRecordInfo>(&dns_info_value) {
                return Ok(dns_info);
            }
        }

        // Not found in KV, check Cloudflare for existing records
        let mut dns_info = DnsRecordInfo::new(record_name.to_string());

        // Check for existing records
        for record_type in [RecordType::A, RecordType::AAAA] {
            if let Some(record) = self.find_existing_record(record_name, record_type).await? {
                dns_info.set_id(record_type, record.id);
            }
        }

        // Store the record info in KV for future use
        self.store_dns_info(homename, &dns_info).await?;
        Ok(dns_info)
    }

    /// Store DNS record info in KV
    async fn store_dns_info(&self, homename: &str, dns_info: &DnsRecordInfo) -> Result<()> {
        let dns_key = self.dns_record_key(homename);
        let dns_info_json = serde_json::to_string(dns_info)
            .map_err(|e| DnsError::SerializationError(e.to_string()))?;
        self.kv.put(&dns_key, &dns_info_json)?.execute().await?;
        Ok(())
    }

    /// Find an existing DNS record in Cloudflare
    async fn find_existing_record(
        &self,
        name: &str,
        record_type: RecordType,
    ) -> Result<Option<DnsRecord>> {
        let url = format!(
            "{}/zones/{}/dns_records?name={}&type={}",
            CLOUDFLARE_API_BASE,
            self.zone_id,
            name,
            record_type.as_str()
        );

        let response: ListDnsResponse = self.make_api_request(&url, Method::Get, None).await?;

        if !response.success {
            return Err(DnsError::ApiError(format!(
                "Failed to list DNS records: {:?}",
                response.errors
            ))
            .into());
        }

        if let Some(records) = response.result {
            return Ok(records
                .into_iter()
                .find(|r| r.name == name && r.record_type == record_type.as_str()));
        }

        Ok(None)
    }

    /// Create a new DNS record in Cloudflare
    async fn create_dns_record(
        &self,
        record_type: RecordType,
        name: &str,
        content: &str,
    ) -> Result<Option<String>> {
        let url = format!("{}/zones/{}/dns_records", CLOUDFLARE_API_BASE, self.zone_id);

        let body = serde_json::json!({
            "type": record_type.as_str(),
            "name": name,
            "content": content,
            "ttl": DNS_TTL,
            "proxied": false
        });

        let response: CreateDnsResponse = self
            .make_api_request(&url, Method::Post, Some(body))
            .await?;

        if !response.success {
            return Err(DnsError::ApiError(format!(
                "Failed to create DNS record: {:?}",
                response.errors
            ))
            .into());
        }

        Ok(response.result.map(|record| record.id))
    }

    /// Update an existing DNS record in Cloudflare
    async fn update_dns_record(
        &self,
        record_id: &str,
        record_type: RecordType,
        name: &str,
        content: &str,
    ) -> Result<bool> {
        let url = format!(
            "{}/zones/{}/dns_records/{}",
            CLOUDFLARE_API_BASE, self.zone_id, record_id
        );

        let body = serde_json::json!({
            "type": record_type.as_str(),
            "name": name,
            "content": content,
            "ttl": DNS_TTL,
            "proxied": false
        });

        let response: UpdateDnsResponse =
            self.make_api_request(&url, Method::Put, Some(body)).await?;

        if !response.success {
            return Err(DnsError::ApiError(format!(
                "Failed to update DNS record: {:?}",
                response.errors
            ))
            .into());
        }

        Ok(response.success)
    }

    /// Make an authenticated API request to Cloudflare
    async fn make_api_request<T>(
        &self,
        url: &str,
        method: Method,
        body: Option<serde_json::Value>,
    ) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let mut init = RequestInit::new();
        init.with_method(method);

        if let Some(body_data) = body {
            init.with_body(Some(body_data.to_string().into()));
        }

        let mut req = Request::new_with_init(url, &init)?;
        req.headers_mut()?
            .set("Authorization", &format!("Bearer {}", self.token))?;
        req.headers_mut()?.set("Content-Type", CONTENT_TYPE_JSON)?;

        let mut resp = Fetch::Request(req).send().await?;
        let response: T = resp.json().await?;
        Ok(response)
    }

    /// Ensure DNS record exists and update it with new content
    async fn ensure_and_update_record(
        &self,
        dns_info: &mut DnsRecordInfo,
        record_type: RecordType,
        content: &str,
        homename: &str,
    ) -> Result<bool> {
        match dns_info.get_id(record_type) {
            Some(id) => {
                // Record exists, update it
                self.update_dns_record(id, record_type, &dns_info.record_name, content)
                    .await
            }
            None => {
                // Record doesn't exist, create it with the correct content
                match self
                    .create_dns_record(record_type, &dns_info.record_name, content)
                    .await?
                {
                    Some(new_id) => {
                        // Update the dns_info with the new record ID
                        dns_info.set_id(record_type, new_id);

                        // Update KV with the new record info
                        self.store_dns_info(homename, dns_info).await?;

                        // Record created successfully, no need to update again
                        Ok(true)
                    }
                    None => Ok(false),
                }
            }
        }
    }

    /// Check if IP has changed and needs updating
    async fn should_update_ip(
        &self,
        homename: &str,
        record_type: RecordType,
        new_ip: &str,
    ) -> Result<bool> {
        if new_ip.is_empty() {
            return Ok(false);
        }

        let key = self.ip_key(homename, record_type);
        let prev_ip = self.kv.get(&key).text().await?.unwrap_or_default();
        Ok(prev_ip != new_ip)
    }

    /// Store the new IP address in KV
    async fn store_ip(&self, homename: &str, record_type: RecordType, ip: &str) -> Result<()> {
        let key = self.ip_key(homename, record_type);
        self.kv.put(&key, ip)?.execute().await?;
        Ok(())
    }

    /// Update a single DNS record if the IP has changed
    async fn update_record_if_changed(
        &self,
        dns_info: &mut DnsRecordInfo,
        record_type: RecordType,
        ip: &str,
        homename: &str,
    ) -> Result<()> {
        if self.should_update_ip(homename, record_type, ip).await?
            && self
                .ensure_and_update_record(dns_info, record_type, ip, homename)
                .await?
        {
            self.store_ip(homename, record_type, ip).await?;
        }
        Ok(())
    }

    /// Main method to update DNS records, handling both IPv4 and IPv6
    pub async fn maybe_update_dns(
        &self,
        homename: &str,
        record_name: &str,
        ipv4: &str,
        ipv6: &str,
    ) -> Result<()> {
        // Get or create DNS record info
        let mut dns_info = self.get_or_create_record_ids(homename, record_name).await?;

        // Update IPv4 record if provided
        self.update_record_if_changed(&mut dns_info, RecordType::A, ipv4, homename)
            .await?;

        // Update IPv6 record if provided
        self.update_record_if_changed(&mut dns_info, RecordType::AAAA, ipv6, homename)
            .await?;

        Ok(())
    }
}

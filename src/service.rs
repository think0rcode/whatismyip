use crate::config::Config;
use crate::dns::DnsManager;
use worker::*;

/// DNS update service
pub struct DnsUpdateService;

impl DnsUpdateService {
    /// Checks KV for stored IP and updates DNS if necessary
    pub async fn maybe_update_dns(
        homename: &str,
        ipv4: &str,
        ipv6: &str,
        env: &Env,
        config: &Config,
    ) -> Result<()> {
        let kv = env.kv(&config.kv_namespace)?;
        let dns_manager =
            DnsManager::new(config.cf_zone_id.clone(), config.cf_api_token.clone(), &kv);

        // Construct the full DNS record name
        let record_name = format!("{}.{}", homename, config.cf_domain);

        // Use the DNS manager to handle all DNS operations
        dns_manager
            .maybe_update_dns(homename, &record_name, ipv4, ipv6)
            .await
    }
}

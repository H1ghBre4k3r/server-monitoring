use chrono::Utc;
use reqwest::Client;
use serde_json::json;
use tracing::{error, info, instrument};

use crate::config::{Alert, Discord, ServerConfig, Webhook};
use crate::monitors::resources::ResourceEvaluation;

#[derive(Debug, Clone)]
pub struct AlertManager {
    client: Client,
    server_config: ServerConfig,
}

impl AlertManager {
    pub fn new(server_config: ServerConfig) -> Self {
        Self {
            client: Client::new(),
            server_config,
        }
    }

    pub fn server_display(&self) -> String {
        self.server_config
            .display
            .clone()
            .unwrap_or_else(|| format!("{}:{}", self.server_config.ip, self.server_config.port))
    }

    #[instrument(skip(self))]
    pub async fn send_temperature_alert(&self, evaluation: ResourceEvaluation, temperature: f32) {
        let Some(limits) = &self.server_config.limits else {
            return;
        };

        let Some(temp_limit) = &limits.temperature else {
            return;
        };

        let Some(alert_config) = &temp_limit.alert else {
            return;
        };

        let message = self.format_temperature_message(evaluation, temperature, temp_limit.limit);
        self.send_alert(alert_config, &message).await;
    }

    #[instrument(skip(self))]
    pub async fn send_usage_alert(&self, evaluation: ResourceEvaluation, usage: f32) {
        let Some(limits) = &self.server_config.limits else {
            return;
        };

        let Some(usage_limit) = &limits.usage else {
            return;
        };

        let Some(alert_config) = &usage_limit.alert else {
            return;
        };

        let message = self.format_usage_message(evaluation, usage, usage_limit.limit);
        self.send_alert(alert_config, &message).await;
    }

    fn format_temperature_message(
        &self,
        evaluation: ResourceEvaluation,
        temperature: f32,
        limit: usize,
    ) -> String {
        let server = self.server_display();
        match evaluation {
            ResourceEvaluation::StartsToExceed => {
                format!(
                    "🔥 **Temperature Alert**: Server `{}` temperature is **{:.1}°C** (limit: {}°C)",
                    server, temperature, limit
                )
            }
            ResourceEvaluation::BackToOk => {
                format!(
                    "✅ **Temperature OK**: Server `{}` temperature is back to normal: **{:.1}°C**",
                    server, temperature
                )
            }
            _ => format!(
                "Temperature update for server `{}`: {:.1}°C",
                server, temperature
            ),
        }
    }

    fn format_usage_message(
        &self,
        evaluation: ResourceEvaluation,
        usage: f32,
        limit: usize,
    ) -> String {
        let server = self.server_display();
        match evaluation {
            ResourceEvaluation::StartsToExceed => {
                format!(
                    "⚠️ **CPU Usage Alert**: Server `{}` CPU usage is **{:.1}%** (limit: {}%)",
                    server, usage, limit
                )
            }
            ResourceEvaluation::BackToOk => {
                format!(
                    "✅ **CPU Usage OK**: Server `{}` CPU usage is back to normal: **{:.1}%**",
                    server, usage
                )
            }
            _ => format!("CPU usage update for server `{}`: {:.1}%", server, usage),
        }
    }

    #[instrument(skip(self, alert_config))]
    async fn send_alert(&self, alert_config: &Alert, message: &str) {
        match alert_config {
            Alert::Discord(discord) => {
                self.send_discord_alert(discord, message).await;
            }
            Alert::Webhook(webhook) => {
                self.send_webhook_alert(webhook, message).await;
            }
        }
    }

    #[instrument(skip(self, discord))]
    async fn send_discord_alert(&self, discord: &Discord, message: &str) {
        let payload = json!({
            "content": message
        });

        match self.client.post(&discord.url).json(&payload).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    info!("Successfully sent Discord alert");
                } else {
                    error!("Discord alert failed with status: {}", response.status());
                }
            }
            Err(e) => {
                error!("Failed to send Discord alert: {}", e);
            }
        }
    }

    #[instrument(skip(self, webhook))]
    async fn send_webhook_alert(&self, webhook: &Webhook, message: &str) {
        let payload = json!({
            "message": message,
            "server": self.server_display(),
            "timestamp": Utc::now().to_rfc3339()
        });

        match self.client.post(&webhook.url).json(&payload).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    info!("Successfully sent webhook alert");
                } else {
                    error!("Webhook alert failed with status: {}", response.status());
                }
            }
            Err(e) => {
                error!("Failed to send webhook alert: {}", e);
            }
        }
    }
}


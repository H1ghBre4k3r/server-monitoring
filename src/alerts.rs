use chrono::Utc;
use reqwest::Client;
use serde_json::json;
use tracing::{error, info, instrument};

use crate::config::{Alert, ServerConfig, Webhook};
use crate::discord::{DiscordManager, MessageBuilder};
use crate::monitors::resources::ResourceEvaluation;

#[derive(Debug, Clone)]
pub struct AlertManager {
    client: Client,
    server_config: ServerConfig,
    discord_manager: DiscordManager,
}

impl AlertManager {
    pub fn new(server_config: ServerConfig) -> Self {
        Self {
            client: Client::new(),
            discord_manager: DiscordManager::new(server_config.clone()),
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

        match alert_config {
            Alert::Discord(discord) => {
                let embed = self.discord_manager.build_temperature_embed(
                    evaluation,
                    temperature,
                    temp_limit.limit,
                );
                let mut message_builder = MessageBuilder::new().add_embed(embed);
                if let Some(user_id) = &discord.user_id {
                    message_builder = message_builder.content(format!(
                        "🌡️ ({} ~ {:.1}°C) <@{user_id}>",
                        self.server_display(),
                        temperature
                    ));
                }

                self.discord_manager
                    .send_message(discord, &message_builder.build())
                    .await;
            }
            Alert::Webhook(webhook) => {
                let message =
                    self.format_temperature_message(evaluation, temperature, temp_limit.limit);
                self.send_webhook_alert(webhook, &message).await;
            }
        }
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

        match alert_config {
            Alert::Discord(discord) => {
                let embed =
                    self.discord_manager
                        .build_usage_embed(evaluation, usage, usage_limit.limit);
                let mut message_builder = MessageBuilder::new().add_embed(embed);
                if let Some(user_id) = &discord.user_id {
                    message_builder = message_builder.content(format!(
                        "💻 ({} ~ {:.1}%) <@{user_id}>",
                        self.server_display(),
                        usage
                    ));
                }
                self.discord_manager
                    .send_message(discord, &message_builder.build())
                    .await;
            }
            Alert::Webhook(webhook) => {
                let message = self.format_usage_message(evaluation, usage, usage_limit.limit);
                self.send_webhook_alert(webhook, &message).await;
            }
        }
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

use chrono::Utc;
use reqwest::Client;
use serde_json::json;
use tracing::{error, info, instrument};

use crate::actors::messages::ServiceStatus;
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

    /// Send service health alert (for HTTP/HTTPS service monitoring)
    ///
    /// # Arguments
    /// - `alert_config`: Alert configuration (Discord or Webhook)
    /// - `service_name`: Name of the service
    /// - `url`: Service URL being monitored
    /// - `previous_status`: Previous service status (None if first check)
    /// - `current_status`: Current service status
    /// - `error_message`: Optional error message if service is down
    ///
    /// TODO: this should be in an own kind of alert manager, probably.
    #[instrument(skip(self, alert_config))]
    pub async fn send_service_alert(
        &self,
        alert_config: &Alert,
        service_name: &str,
        url: &str,
        previous_status: Option<ServiceStatus>,
        current_status: ServiceStatus,
        error_message: Option<&str>,
    ) {
        // Determine if we should send an alert
        let should_alert = match (previous_status, current_status) {
            // Service went down
            (Some(ServiceStatus::Up), ServiceStatus::Down | ServiceStatus::Degraded) => true,
            (None, ServiceStatus::Down | ServiceStatus::Degraded) => true,

            // Service recovered
            (Some(ServiceStatus::Down | ServiceStatus::Degraded), ServiceStatus::Up) => true,

            // No state change or not significant
            _ => false,
        };

        if !should_alert {
            return;
        }

        match alert_config {
            Alert::Discord(discord) => {
                let embed = self.discord_manager.build_service_embed(
                    service_name,
                    url,
                    current_status,
                    error_message,
                );

                let mut message_builder = MessageBuilder::new().add_embed(embed);
                if let Some(user_id) = &discord.user_id {
                    let emoji = match current_status {
                        ServiceStatus::Down | ServiceStatus::Degraded => "🔴",
                        ServiceStatus::Up => "✅",
                    };
                    message_builder = message_builder.content(format!(
                        "{} Service: `{}` <@{user_id}>",
                        emoji, service_name
                    ));
                }

                self.discord_manager
                    .send_message(discord, &message_builder.build())
                    .await;
            }
            Alert::Webhook(webhook) => {
                let message = match current_status {
                    ServiceStatus::Down | ServiceStatus::Degraded => {
                        let status_text = if current_status == ServiceStatus::Down {
                            "DOWN"
                        } else {
                            "DEGRADED"
                        };
                        if let Some(err) = error_message {
                            format!(
                                "🔴 **Service {}**: `{}` is {} ({})\nURL: {}",
                                status_text, service_name, status_text, err, url
                            )
                        } else {
                            format!(
                                "🔴 **Service {}**: `{}` is {}\nURL: {}",
                                status_text, service_name, status_text, url
                            )
                        }
                    }
                    ServiceStatus::Up => {
                        format!(
                            "✅ **Service Recovered**: `{}` is back UP\nURL: {}",
                            service_name, url
                        )
                    }
                };

                let payload = json!({
                    "message": message,
                    "service": service_name,
                    "url": url,
                    "status": match current_status {
                        ServiceStatus::Up => "up",
                        ServiceStatus::Down => "down",
                        ServiceStatus::Degraded => "degraded",
                    },
                    "error": error_message,
                    "timestamp": Utc::now().to_rfc3339()
                });

                match self.client.post(&webhook.url).json(&payload).send().await {
                    Ok(response) => {
                        if response.status().is_success() {
                            info!("Successfully sent service webhook alert");
                        } else {
                            error!(
                                "Service webhook alert failed with status: {}",
                                response.status()
                            );
                        }
                    }
                    Err(e) => {
                        error!("Failed to send service webhook alert: {}", e);
                    }
                }
            }
        }
    }
}

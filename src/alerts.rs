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

        match alert_config {
            Alert::Discord(discord) => {
                let embed = self.build_temperature_embed(evaluation, temperature, temp_limit.limit);
                self.send_discord_embed(discord, embed).await;
            }
            Alert::Webhook(webhook) => {
                let message = self.format_temperature_message(evaluation, temperature, temp_limit.limit);
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
                let embed = self.build_usage_embed(evaluation, usage, usage_limit.limit);
                self.send_discord_embed(discord, embed).await;
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

    fn build_temperature_embed(&self, evaluation: ResourceEvaluation, temperature: f32, limit: usize) -> serde_json::Value {
        let server = self.server_display();
        let (title, description, color) = match evaluation {
            ResourceEvaluation::StartsToExceed => (
                "🔥 Temperature Alert",
                format!("Server **{}** temperature has exceeded the limit!", server),
                15158332, // Red
            ),
            ResourceEvaluation::BackToOk => (
                "✅ Temperature Recovered",
                format!("Server **{}** temperature is back to normal", server),
                3066993, // Green
            ),
            _ => (
                "🌡️ Temperature Update",
                format!("Temperature update for server **{}**", server),
                5793266, // Light blue
            ),
        };

        let progress_bar = self.create_progress_bar(temperature as f32, limit as f32);

        json!({
            "title": title,
            "description": description,
            "color": color,
            "fields": [
                {
                    "name": "🌡️ Current Temperature",
                    "value": format!("{:.1}°C", temperature),
                    "inline": true
                },
                {
                    "name": "⚠️ Limit",
                    "value": format!("{}°C", limit),
                    "inline": true
                },
                {
                    "name": "📊 Status",
                    "value": progress_bar,
                    "inline": false
                }
            ],
            "footer": {
                "text": format!("Server: {} | {}", server, self.server_config.ip)
            },
            "timestamp": Utc::now().to_rfc3339()
        })
    }

    fn build_usage_embed(&self, evaluation: ResourceEvaluation, usage: f32, limit: usize) -> serde_json::Value {
        let server = self.server_display();
        let (title, description, color) = match evaluation {
            ResourceEvaluation::StartsToExceed => (
                "⚠️ CPU Usage Alert",
                format!("Server **{}** CPU usage has exceeded the limit!", server),
                15105570, // Orange
            ),
            ResourceEvaluation::BackToOk => (
                "✅ CPU Usage Recovered", 
                format!("Server **{}** CPU usage is back to normal", server),
                3066993, // Green
            ),
            _ => (
                "💻 CPU Usage Update",
                format!("CPU usage update for server **{}**", server),
                5793266, // Light blue
            ),
        };

        let progress_bar = self.create_progress_bar(usage, limit as f32);

        json!({
            "title": title,
            "description": description,
            "color": color,
            "fields": [
                {
                    "name": "💻 Current CPU Usage",
                    "value": format!("{:.1}%", usage),
                    "inline": true
                },
                {
                    "name": "⚠️ Limit",
                    "value": format!("{}%", limit),
                    "inline": true
                },
                {
                    "name": "📊 Status",
                    "value": progress_bar,
                    "inline": false
                }
            ],
            "footer": {
                "text": format!("Server: {} | {}", server, self.server_config.ip)
            },
            "timestamp": Utc::now().to_rfc3339()
        })
    }

    fn create_progress_bar(&self, current: f32, limit: f32) -> String {
        let percentage = (current / limit) * 100.0;
        let filled = ((current / limit) * 10.0) as usize;
        let empty = 10 - filled.min(10);
        
        let bar = "█".repeat(filled.min(10)) + &"░".repeat(empty);
        let status_emoji = if percentage >= 100.0 {
            "🔴"
        } else if percentage >= 80.0 {
            "🟠"
        } else if percentage >= 60.0 {
            "🟡"
        } else {
            "🟢"
        };

        format!("{} `{}` {:.1}% of limit", status_emoji, bar, percentage)
    }

    #[instrument(skip(self, discord, embed))]
    async fn send_discord_embed(&self, discord: &Discord, embed: serde_json::Value) {
        let payload = json!({
            "embeds": [embed]
        });

        match self.client.post(&discord.url).json(&payload).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    info!("Successfully sent Discord embed alert");
                } else {
                    error!("Discord embed alert failed with status: {}", response.status());
                    if let Ok(error_text) = response.text().await {
                        error!("Discord API error response: {}", error_text);
                    }
                }
            }
            Err(e) => {
                error!("Failed to send Discord embed alert: {}", e);
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


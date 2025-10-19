use chrono::Utc;
use reqwest::Client;
use serde::Serialize;
use tracing::{error, info, instrument};

use crate::actors::messages::ServiceStatus;
use crate::config::{Discord, ResolvedServerConfig};
use crate::monitors::resources::ResourceEvaluation;

#[derive(Debug, Clone, Serialize)]
pub struct Message {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub embeds: Vec<Embed>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Embed {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<u32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<EmbedField>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub footer: Option<EmbedFooter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EmbedField {
    pub name: String,
    pub value: String,
    #[serde(default)]
    pub inline: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct EmbedFooter {
    pub text: String,
}

#[derive(Default, Debug, Clone)]
pub struct MessageBuilder {
    content: Option<String>,
    embeds: Vec<Embed>,
}

impl MessageBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn content(mut self, content: impl ToString) -> Self {
        self.content = Some(content.to_string());
        self
    }

    pub fn add_embed(mut self, embed: Embed) -> Self {
        self.embeds.push(embed);
        self
    }

    pub fn build(self) -> Message {
        Message {
            content: self.content,
            embeds: self.embeds,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DiscordManager {
    client: Client,
    server_config: ResolvedServerConfig,
}

impl DiscordManager {
    pub fn new(server_config: ResolvedServerConfig) -> Self {
        Self {
            client: Client::new(),
            server_config,
        }
    }

    fn server_display(&self) -> String {
        self.server_config
            .display
            .clone()
            .unwrap_or_else(|| format!("{}:{}", self.server_config.ip, self.server_config.port))
    }

    pub fn build_temperature_embed(
        &self,
        evaluation: ResourceEvaluation,
        temperature: f32,
        limit: usize,
    ) -> Embed {
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

        let progress_bar = self.create_progress_bar(temperature, limit as f32);

        Embed {
            title: Some(title.to_string()),
            description: Some(description.to_string()),
            color: Some(color),
            fields: vec![
                EmbedField {
                    name: "🌡️ Current Temperature".to_string(),
                    value: format!("{:.1}°C", temperature),
                    inline: true,
                },
                EmbedField {
                    name: "⚠️ Limit".to_string(),
                    value: format!("{}°C", limit),
                    inline: true,
                },
                EmbedField {
                    name: "📊 Status".to_string(),
                    value: progress_bar,
                    inline: false,
                },
            ],
            footer: Some(EmbedFooter {
                text: format!("Server: {} | {}", server, self.server_config.ip),
            }),
            timestamp: Some(Utc::now().to_rfc3339()),
        }
    }

    pub fn build_usage_embed(
        &self,
        evaluation: ResourceEvaluation,
        usage: f32,
        limit: usize,
    ) -> Embed {
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

        Embed {
            title: Some(title.to_string()),
            description: Some(description.to_string()),
            color: Some(color),
            fields: vec![
                EmbedField {
                    name: "💻 Current CPU Usage".to_string(),
                    value: format!("{:.1}%", usage),
                    inline: true,
                },
                EmbedField {
                    name: "⚠️ Limit".to_string(),
                    value: format!("{}%", limit),
                    inline: true,
                },
                EmbedField {
                    name: "📊 Status".to_string(),
                    value: progress_bar,
                    inline: false,
                },
            ],
            footer: Some(EmbedFooter {
                text: format!("Server: {} | {}", server, self.server_config.ip),
            }),
            timestamp: Some(Utc::now().to_rfc3339()),
        }
    }

    /// Build Discord embed for service health alert (Phase 3)
    pub fn build_service_embed(
        &self,
        service_name: &str,
        url: &str,
        current_status: ServiceStatus,
        error_message: Option<&str>,
    ) -> Embed {
        let (title, description, color) = match current_status {
            ServiceStatus::Down | ServiceStatus::Degraded => {
                let status_text = if current_status == ServiceStatus::Down {
                    "DOWN"
                } else {
                    "DEGRADED"
                };
                let desc = if let Some(err) = error_message {
                    format!("Service **{}** is **{}**", service_name, status_text)
                } else {
                    format!("Service **{}** is **{}**", service_name, status_text)
                };
                (
                    format!("🔴 Service {} Alert", status_text),
                    desc,
                    0xFF0000, // Red
                )
            }
            ServiceStatus::Up => (
                "✅ Service Recovered".to_string(),
                format!("Service **{}** is back **UP**", service_name),
                0x00FF00, // Green
            ),
        };

        let mut fields = vec![
            EmbedField {
                name: "Service".to_string(),
                value: service_name.to_string(),
                inline: true,
            },
            EmbedField {
                name: "URL".to_string(),
                value: url.to_string(),
                inline: false,
            },
            EmbedField {
                name: "Status".to_string(),
                value: match current_status {
                    ServiceStatus::Up => "🟢 UP".to_string(),
                    ServiceStatus::Down => "🔴 DOWN".to_string(),
                    ServiceStatus::Degraded => "🟡 DEGRADED".to_string(),
                },
                inline: true,
            },
        ];

        if let Some(err) = error_message {
            fields.push(EmbedField {
                name: "Error".to_string(),
                value: format!("`{}`", err),
                inline: false,
            });
        }

        Embed {
            title: Some(title),
            description: Some(description),
            color: Some(color),
            fields,
            footer: Some(EmbedFooter {
                text: "Service Monitoring".to_string(),
            }),
            timestamp: Some(Utc::now().to_rfc3339()),
        }
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

    #[instrument(skip(self, discord, message))]
    pub async fn send_message(&self, discord: &Discord, message: &Message) {
        match self.client.post(&discord.url).json(message).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    info!("Successfully sent Discord message");
                } else {
                    error!("Discord message failed with status: {}", response.status());
                    if let Ok(error_text) = response.text().await {
                        error!("Discord API error response: {}", error_text);
                    }
                }
            }
            Err(e) => {
                error!("Failed to send Discord message: {}", e);
            }
        }
    }
}

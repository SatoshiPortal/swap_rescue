use crate::gui::styles::*;
use boltz_client::swaps::{BtcLikeTransaction, ChainClient};
use iced::alignment::{Horizontal, Vertical};
use iced::widget::{button, column, container, row, scrollable, text, text_input, Column, Space};
use iced::{Color, Element, Font, Length, Padding, Task};
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::sync::{Arc, Mutex};

const GOLOS_TEXT: Font = Font::with_name("Golos Text");

// Global storage for transaction data (can't be passed through messages)
// We store the transaction and chain_client which will be used for broadcasting
static RESCUE_DATA: Lazy<Arc<Mutex<Option<(BtcLikeTransaction, ChainClient)>>>> =
    Lazy::new(|| Arc::new(Mutex::new(None)));

#[derive(Debug, Clone)]
pub enum Message {
    LoadConfig,
    ConfigLoaded(Result<Config, String>),
    SwapIndexChanged(String),
    IncrementSwapIndex,
    DecrementSwapIndex,
    ClearLogs,
    ExecuteRescue,
    RescueCompleted(Result<RescueSummary, String>),
    ConfirmBroadcast,
    CancelBroadcast,
    BroadcastCompleted(Result<String, String>),
}

// Summary info that can be cloned and passed through messages
#[derive(Debug, Clone)]
pub struct RescueSummary {
    amount: u64,
    return_address: String,
    network: String,
    swap_id: String,
    rescue_type: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub user_input: UserInput,
    pub provider_input: ProviderInput,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UserInput {
    pub mnemonic: String,
    #[serde(default)]
    pub passphrase: String,
    #[serde(default)]
    pub swap_index: u64,
    pub return_address: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProviderInput {
    pub rescue_type: String,
    #[serde(default = "default_swap_type")]
    pub swap_type: String,
    pub claim_leaf: LeafConfig,
    pub refund_leaf: LeafConfig,
    #[serde(rename = "from_network")]
    pub from_network_str: String,
    #[serde(rename = "to_network")]
    pub to_network_str: String,
    pub blinding_key: Option<String>,
    pub timeout_block_height: u32,
    pub server_public_key: String,
    pub user_lockup_address: String,
    pub server_lockup_address: Option<String>,
    pub amount: u64,
    pub swap_id: String,
    pub preimage: Option<String>,
}

fn default_swap_type() -> String {
    "chain".to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct LeafConfig {
    pub output: String,
    pub version: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    Idle,
    ConfigLoaded,
    Executing,
    AwaitingConfirmation,
    Broadcasting,
    Completed,
    Error,
}

pub struct SwapRescueApp {
    state: AppState,
    config: Option<Config>,
    logs: Vec<String>,
    error_message: Option<String>,
    swap_index_override: u64, // Allow user to override swap_index from GUI
}

impl Default for SwapRescueApp {
    fn default() -> Self {
        Self {
            state: AppState::Idle,
            config: None,
            logs: Vec::new(),
            error_message: None,
            swap_index_override: 0,
        }
    }
}

impl SwapRescueApp {
    pub fn new() -> (Self, Task<Message>) {
        (Self::default(), Task::none())
    }

    pub fn title(&self) -> String {
        "Swap Rescue Tool".to_string()
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::LoadConfig => {
                self.append_log("Opening file picker...");
                Task::perform(load_config_file(), Message::ConfigLoaded)
            }
            Message::ConfigLoaded(result) => {
                match result {
                    Ok(config) => {
                        self.append_log("✓ Config loaded successfully");
                        self.append_log(&format!("  Swap ID: {}", config.provider_input.swap_id));
                        self.append_log(&format!(
                            "  Type: {}",
                            config.provider_input.rescue_type.to_uppercase()
                        ));
                        self.append_log(&format!(
                            "  Direction: {} -> {}",
                            config.provider_input.from_network_str, config.provider_input.to_network_str
                        ));

                        // Set swap_index_override to the value from config
                        self.swap_index_override = config.user_input.swap_index;
                        self.append_log(&format!("  Swap Index: {}", self.swap_index_override));

                        self.config = Some(config);
                        self.state = AppState::ConfigLoaded;
                        self.error_message = None;
                    }
                    Err(e) => {
                        self.append_log(&format!("✗ Error loading config: {}", e));
                        self.error_message = Some(e);
                        self.state = AppState::Error;
                    }
                }
                Task::none()
            }
            Message::SwapIndexChanged(value) => {
                if let Ok(index) = value.parse::<u64>() {
                    self.swap_index_override = index;
                    self.append_log(&format!("Swap index changed to: {}", index));
                }
                Task::none()
            }
            Message::IncrementSwapIndex => {
                self.swap_index_override += 1;
                self.append_log(&format!("Swap index incremented to: {}", self.swap_index_override));
                Task::none()
            }
            Message::DecrementSwapIndex => {
                if self.swap_index_override > 0 {
                    self.swap_index_override -= 1;
                    self.append_log(&format!("Swap index decremented to: {}", self.swap_index_override));
                }
                Task::none()
            }
            Message::ClearLogs => {
                self.logs.clear();
                Task::none()
            }
            Message::ExecuteRescue => {
                if let Some(mut config) = self.config.clone() {
                    self.state = AppState::Executing;
                    self.append_log("Starting rescue operation...");
                    self.append_log(&format!("Using swap index: {}", self.swap_index_override));
                    self.append_log("Connecting to Electrum servers...");
                    self.append_log("Building swap script...");

                    // Override the swap_index with the GUI value
                    config.user_input.swap_index = self.swap_index_override;

                    Task::perform(execute_rescue(config), Message::RescueCompleted)
                } else {
                    Task::none()
                }
            }
            Message::RescueCompleted(result) => {
                match result {
                    Ok(summary) => {
                        self.append_log("✓ Transaction constructed successfully");
                        self.append_log("");
                        self.append_log("=== Transaction Summary ===");
                        self.append_log(&format!("Amount: {} sats", summary.amount));
                        self.append_log(&format!("Return Address: {}", summary.return_address));
                        self.append_log(&format!("Network: {}", summary.network));
                        self.append_log(&format!("Swap ID: {}", summary.swap_id));
                        self.append_log(&format!("Type: {}", summary.rescue_type.to_uppercase()));
                        self.append_log("");
                        self.append_log("⚠ Please review the details above carefully");

                        // Store the summary - the actual tx and chain_client are stored separately
                        self.state = AppState::AwaitingConfirmation;
                        self.error_message = None;
                    }
                    Err(e) => {
                        self.append_log(&format!("✗ Error: {}", e));
                        self.error_message = Some(e);
                        self.state = AppState::Error;
                    }
                }
                Task::none()
            }
            Message::ConfirmBroadcast => {
                self.state = AppState::Broadcasting;
                self.append_log("");
                self.append_log("Broadcasting transaction to network...");
                Task::perform(broadcast_transaction(), Message::BroadcastCompleted)
            }
            Message::CancelBroadcast => {
                self.append_log("");
                self.append_log("✗ Broadcast cancelled by user");
                self.state = AppState::ConfigLoaded;
                // Clear the global rescue data
                if let Ok(mut data) = RESCUE_DATA.lock() {
                    *data = None;
                }
                Task::none()
            }
            Message::BroadcastCompleted(result) => {
                match result {
                    Ok(msg) => {
                        self.append_log("✓ Transaction broadcast successfully!");
                        self.append_log(&msg);
                        self.append_log("");
                        self.append_log("=== COMPLETED ===");
                        self.state = AppState::Completed;
                    }
                    Err(e) => {
                        self.append_log(&format!("✗ Broadcast error: {}", e));
                        self.error_message = Some(e);
                        self.state = AppState::Error;
                    }
                }
                Task::none()
            }
        }
    }

    fn append_log(&mut self, message: &str) {
        self.logs.push(message.to_string());
    }

    pub fn view(&self) -> Element<'_, Message> {
        let title = text("Swap Rescue Tool")
            .size(32)
            .font(GOLOS_TEXT)
            .width(Length::Fill)
            .align_x(Horizontal::Center)
            .color(TEXT);

        let subtitle = text("Recover orphaned swaps")
            .size(14)
            .font(GOLOS_TEXT)
            .color(TEXT_MUTED)
            .width(Length::Fill)
            .align_x(Horizontal::Center);

        let header = column![title, subtitle]
            .spacing(8)
            .padding(Padding::new(20.0).top(20.0).bottom(20.0));

        let config_section = self.build_config_section();
        let logs_section = self.build_logs_section();
        let action_section = self.build_action_section();

        let content = column![header, config_section, logs_section, action_section]
            .spacing(24)
            .padding(Padding::new(20.0))
            .width(Length::Fill);

        let scrollable_content = scrollable(content)
            .width(Length::Fill)
            .height(Length::Fill);

        container(scrollable_content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_theme| container::Style {
                background: Some(iced::Background::Color(BACKGROUND)),
                ..Default::default()
            })
            .into()
    }

    fn build_config_section(&self) -> Element<'_, Message> {
        let config_status = if let Some(config) = &self.config {
            column![
                // Swap ID row
                row![
                    text("Swap ID")
                        .size(14)
                        .font(GOLOS_TEXT)
                        .color(GREY_DARK)
                        .width(Length::Fixed(100.0)),
                    text(&config.provider_input.swap_id)
                        .size(14)
                        .font(GOLOS_TEXT)
                        .color(TEXT),
                ]
                .spacing(12),
                // Swap Type row
                row![
                    text("Swap Type")
                        .size(14)
                        .font(GOLOS_TEXT)
                        .color(GREY_DARK)
                        .width(Length::Fixed(100.0)),
                    text(config.provider_input.swap_type.to_uppercase())
                        .size(14)
                        .font(GOLOS_TEXT)
                        .color(TEXT),
                ]
                .spacing(12),
                // Rescue Type row
                row![
                    text("Rescue Type")
                        .size(14)
                        .font(GOLOS_TEXT)
                        .color(GREY_DARK)
                        .width(Length::Fixed(100.0)),
                    text(config.provider_input.rescue_type.to_uppercase())
                        .size(14)
                        .font(GOLOS_TEXT)
                        .color(TEXT),
                ]
                .spacing(12),
                // Direction row
                row![
                    text("Direction")
                        .size(14)
                        .font(GOLOS_TEXT)
                        .color(GREY_DARK)
                        .width(Length::Fixed(100.0)),
                    text(format!(
                        "{} → {}",
                        config.provider_input.from_network_str, config.provider_input.to_network_str
                    ))
                    .size(14)
                    .font(GOLOS_TEXT)
                    .color(TEXT),
                ]
                .spacing(12),
            ]
            .spacing(8)
        } else {
            column![text("No config loaded").size(14).font(GOLOS_TEXT).color(TEXT_MUTED)]
        };

        let load_button = button(
            text("Load Config")
                .size(16)
                .font(GOLOS_TEXT)
                .width(Length::Fill)
                .align_x(Horizontal::Center),
        )
        .padding(12)
        .width(Length::Fixed(200.0))
        .style(|_theme, status| {
            let base = button::Style {
                background: Some(iced::Background::Color(PRIMARY_RED)),
                text_color: WHITE,
                border: iced::Border {
                    color: PRIMARY_RED,
                    width: 0.0,
                    radius: 8.0.into(),
                },
                shadow: Default::default(),
            };
            match status {
                button::Status::Hovered => button::Style {
                    background: Some(iced::Background::Color(Color::from_rgb(0.85, 0.05, 0.05))),
                    ..base
                },
                button::Status::Pressed => button::Style {
                    background: Some(iced::Background::Color(Color::from_rgb(0.65, 0.03, 0.03))),
                    ..base
                },
                _ => base,
            }
        })
        .on_press(Message::LoadConfig);

        // Swap index controls (only show if config is loaded)
        let swap_index_section = if self.config.is_some() {
            let decrement_btn = button(text("−").size(20).font(GOLOS_TEXT))
                .padding([8, 16])
                .style(|_theme, status| {
                    let base = button::Style {
                        background: Some(iced::Background::Color(PRIMARY_RED)),
                        text_color: WHITE,
                        border: iced::Border {
                            color: PRIMARY_RED,
                            width: 0.0,
                            radius: 8.0.into(),
                        },
                        shadow: Default::default(),
                    };
                    match status {
                        button::Status::Hovered => button::Style {
                            background: Some(iced::Background::Color(Color::from_rgb(0.85, 0.05, 0.05))),
                            ..base
                        },
                        button::Status::Pressed => button::Style {
                            background: Some(iced::Background::Color(Color::from_rgb(0.65, 0.03, 0.03))),
                            ..base
                        },
                        _ => base,
                    }
                })
                .on_press(Message::DecrementSwapIndex);

            let increment_btn = button(text("+").size(20).font(GOLOS_TEXT))
                .padding([8, 16])
                .style(|_theme, status| {
                    let base = button::Style {
                        background: Some(iced::Background::Color(PRIMARY_RED)),
                        text_color: WHITE,
                        border: iced::Border {
                            color: PRIMARY_RED,
                            width: 0.0,
                            radius: 8.0.into(),
                        },
                        shadow: Default::default(),
                    };
                    match status {
                        button::Status::Hovered => button::Style {
                            background: Some(iced::Background::Color(Color::from_rgb(0.85, 0.05, 0.05))),
                            ..base
                        },
                        button::Status::Pressed => button::Style {
                            background: Some(iced::Background::Color(Color::from_rgb(0.65, 0.03, 0.03))),
                            ..base
                        },
                        _ => base,
                    }
                })
                .on_press(Message::IncrementSwapIndex);

            let index_input = text_input("0", &self.swap_index_override.to_string())
                .size(16)
                .font(GOLOS_TEXT)
                .width(Length::Fixed(80.0))
                .padding(8)
                .on_input(Message::SwapIndexChanged);

            column![
                Space::with_height(16),
                text("Swap Index")
                    .size(14)
                    .font(GOLOS_TEXT)
                    .color(TEXT_MUTED),
                Space::with_height(4),
                row![decrement_btn, index_input, increment_btn]
                    .spacing(8)
                    .align_y(Vertical::Center),
                text("Try different values if rescue fails")
                    .size(12)
                    .font(GOLOS_TEXT)
                    .color(TEXT_MUTED),
            ]
            .spacing(4)
        } else {
            column![]
        };

        column![
            row![
                text("Configuration")
                    .size(18)
                    .font(GOLOS_TEXT)
                    .color(TEXT),
                Space::with_width(Length::Fill),
                load_button,
            ]
            .align_y(Vertical::Center),
            Space::with_height(16),
            config_status,
            swap_index_section,
        ]
        .spacing(0)
        .width(Length::Fill)
        .into()
    }

    fn build_logs_section(&self) -> Element<'_, Message> {
        let logs_header = row![
            text("Logs").size(18).font(GOLOS_TEXT).color(TEXT),
            Space::with_width(Length::Fill),
            button(
                text("Clear")
                    .size(14)
                    .font(GOLOS_TEXT)
                    .align_x(Horizontal::Center),
            )
            .padding([6, 12])
            .style(|_theme, status| {
                let base = button::Style {
                    background: Some(iced::Background::Color(PRIMARY_RED)),
                    text_color: WHITE,
                    border: iced::Border {
                        color: PRIMARY_RED,
                        width: 0.0,
                        radius: 8.0.into(),
                    },
                    shadow: Default::default(),
                };
                match status {
                    button::Status::Hovered => button::Style {
                        background: Some(iced::Background::Color(Color::from_rgb(0.85, 0.05, 0.05))),
                        ..base
                    },
                    button::Status::Pressed => button::Style {
                        background: Some(iced::Background::Color(Color::from_rgb(0.65, 0.03, 0.03))),
                        ..base
                    },
                    _ => base,
                }
            })
            .on_press(Message::ClearLogs),
        ]
        .align_y(Vertical::Center);

        let log_content: Element<_> = if self.logs.is_empty() {
            container(
                text("No logs yet...")
                    .size(14)
                    .font(GOLOS_TEXT)
                    .color(TEXT_MUTED)
            )
            .padding(16)
            .width(Length::Fill)
            .height(Length::Fixed(300.0))
            .style(|_theme| container::Style {
                background: Some(iced::Background::Color(SURFACE)),
                text_color: Some(TEXT),
                border: iced::Border {
                    color: BORDER,
                    width: 1.0,
                    radius: 8.0.into(),
                },
                shadow: Default::default(),
            })
            .into()
        } else {
            let log_items: Vec<Element<Message>> = self
                .logs
                .iter()
                .map(|log| {
                    text(log)
                        .size(13)
                        .font(Font::MONOSPACE)
                        .color(TEXT)
                        .into()
                })
                .collect();

            container(
                scrollable(
                    container(Column::with_children(log_items).spacing(4))
                        .padding(16)
                        .width(Length::Fill)
                )
                .height(Length::Fixed(300.0))
            )
            .width(Length::Fill)
            .height(Length::Fixed(300.0))
            .style(|_theme| container::Style {
                background: Some(iced::Background::Color(SURFACE)),
                text_color: Some(TEXT),
                border: iced::Border {
                    color: BORDER,
                    width: 1.0,
                    radius: 8.0.into(),
                },
                shadow: Default::default(),
            })
            .into()
        };

        column![logs_header, log_content]
            .spacing(12)
            .width(Length::Fill)
            .into()
    }

    fn build_action_section(&self) -> Element<'_, Message> {
        let buttons: Element<Message> = match self.state {
            AppState::Idle => row![].into(),
            AppState::ConfigLoaded => {
                let execute_button = button(
                    text("Execute Rescue")
                        .size(16)
                        .font(GOLOS_TEXT)
                        .width(Length::Fill)
                        .align_x(Horizontal::Center),
                )
                .padding(12)
                .width(Length::Fixed(200.0))
                .style(|_theme, status| {
                    let base = button::Style {
                        background: Some(iced::Background::Color(PRIMARY_RED)),
                        text_color: WHITE,
                        border: iced::Border {
                            color: PRIMARY_RED,
                            width: 0.0,
                            radius: 8.0.into(),
                        },
                        shadow: Default::default(),
                    };
                    match status {
                        button::Status::Hovered => button::Style {
                            background: Some(iced::Background::Color(Color::from_rgb(0.85, 0.05, 0.05))),
                            ..base
                        },
                        button::Status::Pressed => button::Style {
                            background: Some(iced::Background::Color(Color::from_rgb(0.65, 0.03, 0.03))),
                            ..base
                        },
                        _ => base,
                    }
                })
                .on_press(Message::ExecuteRescue);

                row![execute_button].spacing(12).into()
            }
            AppState::Executing => {
                let status = text("Executing...")
                    .size(16)
                    .font(GOLOS_TEXT)
                    .color(TEXT_MUTED);
                row![status].into()
            }
            AppState::AwaitingConfirmation => {
                let confirm_button = button(
                    text("Confirm & Broadcast")
                        .size(16)
                        .font(GOLOS_TEXT)
                        .width(Length::Fill)
                        .align_x(Horizontal::Center),
                )
                .padding(12)
                .width(Length::Fixed(200.0))
                .style(|_theme, status| {
                    let base = button::Style {
                        background: Some(iced::Background::Color(PRIMARY_RED)),
                        text_color: WHITE,
                        border: iced::Border {
                            color: PRIMARY_RED,
                            width: 0.0,
                            radius: 8.0.into(),
                        },
                        shadow: Default::default(),
                    };
                    match status {
                        button::Status::Hovered => button::Style {
                            background: Some(iced::Background::Color(Color::from_rgb(0.85, 0.05, 0.05))),
                            ..base
                        },
                        button::Status::Pressed => button::Style {
                            background: Some(iced::Background::Color(Color::from_rgb(0.65, 0.03, 0.03))),
                            ..base
                        },
                        _ => base,
                    }
                })
                .on_press(Message::ConfirmBroadcast);

                let cancel_button = button(
                    text("Cancel")
                        .size(16)
                        .font(GOLOS_TEXT)
                        .width(Length::Fill)
                        .align_x(Horizontal::Center),
                )
                .padding(12)
                .width(Length::Fixed(200.0))
                .style(|_theme, status| {
                    let base = button::Style {
                        background: Some(iced::Background::Color(SURFACE)),
                        text_color: TEXT,
                        border: iced::Border {
                            color: BORDER,
                            width: 1.0,
                            radius: 8.0.into(),
                        },
                        shadow: Default::default(),
                    };
                    match status {
                        button::Status::Hovered => button::Style {
                            background: Some(iced::Background::Color(BACKGROUND)),
                            ..base
                        },
                        button::Status::Pressed => button::Style {
                            background: Some(iced::Background::Color(GREY_LIGHT)),
                            ..base
                        },
                        _ => base,
                    }
                })
                .on_press(Message::CancelBroadcast);

                row![confirm_button, cancel_button].spacing(12).into()
            }
            AppState::Broadcasting => {
                let status = text("Broadcasting...")
                    .size(16)
                    .font(GOLOS_TEXT)
                    .color(TEXT_MUTED);
                row![status].into()
            }
            AppState::Completed => {
                let status = text("✓ Completed Successfully")
                    .size(16)
                    .font(GOLOS_TEXT)
                    .color(TEXT);
                row![status].into()
            }
            AppState::Error => {
                // Show Execute Rescue button to allow retry
                let execute_button = button(
                    text("Execute Rescue")
                        .size(16)
                        .font(GOLOS_TEXT)
                        .width(Length::Fill)
                        .align_x(Horizontal::Center),
                )
                .padding(12)
                .width(Length::Fixed(200.0))
                .style(|_theme, status| {
                    let base = button::Style {
                        background: Some(iced::Background::Color(PRIMARY_RED)),
                        text_color: WHITE,
                        border: iced::Border {
                            color: PRIMARY_RED,
                            width: 0.0,
                            radius: 8.0.into(),
                        },
                        shadow: Default::default(),
                    };
                    match status {
                        button::Status::Hovered => button::Style {
                            background: Some(iced::Background::Color(Color::from_rgb(0.85, 0.05, 0.05))),
                            ..base
                        },
                        button::Status::Pressed => button::Style {
                            background: Some(iced::Background::Color(Color::from_rgb(0.65, 0.03, 0.03))),
                            ..base
                        },
                        _ => base,
                    }
                })
                .on_press(Message::ExecuteRescue);

                row![execute_button].spacing(12).into()
            }
        };

        container(buttons)
            .width(Length::Fill)
            .padding(Padding::new(0.0).top(10.0).bottom(10.0))
            .align_x(Horizontal::Center)
            .into()
    }
}

async fn load_config_file() -> Result<Config, String> {
    let file_handle = rfd::AsyncFileDialog::new()
        .add_filter("YAML", &["yaml", "yml"])
        .set_title("Select config.yaml")
        .pick_file()
        .await;

    if let Some(file) = file_handle {
        let contents = file.read().await;
        let config_str = String::from_utf8(contents).map_err(|e| format!("Invalid UTF-8: {}", e))?;
        let config: Config = serde_yaml::from_str(&config_str)
            .map_err(|e| format!("YAML parse error: {}", e))?;

        // Validate all required fields
        validate_config(&config)?;

        Ok(config)
    } else {
        Err("No file selected".to_string())
    }
}

fn validate_config(config: &Config) -> Result<(), String> {
    // Validate user_input
    if config.user_input.mnemonic.trim().is_empty() {
        return Err("Error: mnemonic is required in user_input".to_string());
    }

    if config.user_input.return_address.trim().is_empty() {
        return Err("Error: return_address is required in user_input".to_string());
    }

    // Validate provider_input
    if config.provider_input.rescue_type.trim().is_empty() {
        return Err("Error: rescue_type is required in provider_input".to_string());
    }

    let rescue_type = config.provider_input.rescue_type.to_lowercase();
    if rescue_type != "claim" && rescue_type != "refund" {
        return Err(format!(
            "Error: rescue_type must be 'claim' or 'refund', got '{}'",
            config.provider_input.rescue_type
        ));
    }

    let swap_type = config.provider_input.swap_type.to_lowercase();
    if swap_type != "chain" && swap_type != "submarine" {
        return Err(format!(
            "Error: swap_type must be 'chain' or 'submarine', got '{}'",
            config.provider_input.swap_type
        ));
    }

    if config.provider_input.claim_leaf.output.trim().is_empty() {
        return Err("Error: claim_leaf.output is required in provider_input".to_string());
    }

    if config.provider_input.refund_leaf.output.trim().is_empty() {
        return Err("Error: refund_leaf.output is required in provider_input".to_string());
    }

    if config.provider_input.from_network_str.trim().is_empty() {
        return Err("Error: from_network is required in provider_input".to_string());
    }

    // For submarine swaps, to_network may not be needed, but we'll keep it for now
    if swap_type == "chain" && config.provider_input.to_network_str.trim().is_empty() {
        return Err("Error: to_network is required in provider_input for chain swaps".to_string());
    }

    if config.provider_input.timeout_block_height == 0 {
        return Err("Error: timeout_block_height must be greater than 0 in provider_input".to_string());
    }

    if config.provider_input.server_public_key.trim().is_empty() {
        return Err("Error: server_public_key is required in provider_input".to_string());
    }

    if config.provider_input.user_lockup_address.trim().is_empty() {
        return Err("Error: user_lockup_address is required in provider_input".to_string());
    }

    // server_lockup_address is only required for chain swaps with claim rescue
    if swap_type == "chain" && rescue_type == "claim" {
        if let Some(addr) = &config.provider_input.server_lockup_address {
            if addr.trim().is_empty() {
                return Err("Error: server_lockup_address cannot be empty for chain swap claims".to_string());
            }
        } else {
            return Err("Error: server_lockup_address is required for chain swap claims".to_string());
        }
    }

    if config.provider_input.amount == 0 {
        return Err("Error: amount must be greater than 0 in provider_input".to_string());
    }

    if config.provider_input.swap_id.trim().is_empty() {
        return Err("Error: swap_id is required in provider_input".to_string());
    }

    // Validate that preimage is provided if rescue_type is "claim" and swap_type is "chain"
    if rescue_type == "claim" && swap_type == "chain" && config.provider_input.preimage.is_none() {
        return Err("Error: preimage is required in provider_input when rescue_type is 'claim' for chain swaps".to_string());
    }

    if let Some(preimage) = &config.provider_input.preimage {
        if preimage.trim().is_empty() {
            return Err("Error: preimage cannot be empty when provided".to_string());
        }
    }

    // Passphrase is optional - no validation needed

    Ok(())
}

async fn execute_rescue(config: Config) -> Result<RescueSummary, String> {
    use crate::{claim_rescue, parse_chain, refund_rescue, submarine_refund_rescue};

    let from_network = parse_chain(&config.provider_input.from_network_str)
        .map_err(|e| format!("Invalid from_network: {}", e))?;

    let rescue_type = config.provider_input.rescue_type.to_lowercase();
    let swap_type = config.provider_input.swap_type.to_lowercase();

    let result = if rescue_type == "refund" {
        if swap_type == "submarine" {
            // Submarine refund - only uses from_network
            submarine_refund_rescue(
                &config.user_input.mnemonic,
                &config.provider_input.claim_leaf.output,
                config.provider_input.claim_leaf.version,
                &config.provider_input.refund_leaf.output,
                config.provider_input.refund_leaf.version,
                config.provider_input.blinding_key.as_deref(),
                config.provider_input.timeout_block_height,
                &config.provider_input.server_public_key,
                &config.provider_input.user_lockup_address,
                config.provider_input.amount,
                &config.provider_input.swap_id,
                &config.user_input.return_address,
                &config.user_input.passphrase,
                config.user_input.swap_index,
                from_network,
            )
            .await
        } else {
            // Chain refund
            let to_network = parse_chain(&config.provider_input.to_network_str)
                .map_err(|e| format!("Invalid to_network: {}", e))?;

            refund_rescue(
                &config.user_input.mnemonic,
                &config.provider_input.claim_leaf.output,
                config.provider_input.claim_leaf.version,
                &config.provider_input.refund_leaf.output,
                config.provider_input.refund_leaf.version,
                config.provider_input.blinding_key.as_deref(),
                config.provider_input.timeout_block_height,
                &config.provider_input.server_public_key,
                &config.provider_input.user_lockup_address,
                config.provider_input.amount,
                &config.provider_input.swap_id,
                &config.user_input.return_address,
                &config.user_input.passphrase,
                config.user_input.swap_index,
                from_network,
                to_network,
            )
            .await
        }
    } else {
        // Claim rescue (only for chain swaps)
        let to_network = parse_chain(&config.provider_input.to_network_str)
            .map_err(|e| format!("Invalid to_network: {}", e))?;

        let server_lockup_address = config.provider_input.server_lockup_address
            .as_deref()
            .ok_or_else(|| "server_lockup_address is required for claim rescue".to_string())?;

        claim_rescue(
            &config.user_input.mnemonic,
            &config.provider_input.claim_leaf.output,
            config.provider_input.claim_leaf.version,
            &config.provider_input.refund_leaf.output,
            config.provider_input.refund_leaf.version,
            config.provider_input.blinding_key.as_deref(),
            config.provider_input.timeout_block_height,
            &config.provider_input.server_public_key,
            &config.provider_input.user_lockup_address,
            server_lockup_address,
            config.provider_input.amount,
            &config.provider_input.swap_id,
            &config.user_input.return_address,
            config.provider_input.preimage.as_deref(),
            &config.user_input.passphrase,
            config.user_input.swap_index,
            from_network,
            to_network,
        )
        .await
    };

    result
        .map(|(tx, chain_client, amount, return_address, network)| {
            let summary = RescueSummary {
                amount,
                return_address: return_address.clone(),
                network: format!("{:?}", network),
                swap_id: config.provider_input.swap_id.clone(),
                rescue_type: format!("{} {}", swap_type, rescue_type),
            };

            // Store the transaction and chain_client in global storage for later broadcast
            if let Ok(mut data) = RESCUE_DATA.lock() {
                *data = Some((tx, chain_client));
            }

            summary
        })
        .map_err(|e| format!("{:?}", e))
}

async fn broadcast_transaction() -> Result<String, String> {
    // Take the transaction data from global storage (we only broadcast once)
    let (tx, chain_client) = {
        let mut data = RESCUE_DATA.lock()
            .map_err(|e| format!("Failed to lock rescue data: {}", e))?;

        data.take()
            .ok_or_else(|| "No transaction data available".to_string())?
    }; // MutexGuard is dropped here

    chain_client
        .broadcast_tx(&tx)
        .await
        .map(|_| "Transaction broadcast successfully!".to_string())
        .map_err(|e| format!("Failed to broadcast: {:?}", e))
}

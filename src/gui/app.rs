use crate::gui::styles::*;
use bip39::Language;
use bitcoin::Address as BtcAddress;
use boltz_client::elements::Address as ElementsAddress;
use boltz_client::network::Chain;
use boltz_client::swaps::{BtcLikeTransaction, ChainClient};
use iced::alignment::{Horizontal, Vertical};
use iced::widget::{
    Column, Row, Space, button, column, container, row, scrollable, text, text_input, toggler,
};
use iced::{Border, Color, Element, Font, Length, Padding, Task};
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::collections::HashSet;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

const GOLOS_TEXT: Font = Font::with_name("Golos Text");
const MAX_SUGGESTIONS: usize = 6;
const DEFAULT_MAX_SWAP_INDEX: u64 = 210;
const WORD_BOX_WIDTH: f32 = 140.0;
const WORDS_PER_ROW: usize = 3;
const MAX_WORDS: usize = 24;

const LOCK_BG: Color = Color::from_rgb(0.92, 0.97, 0.92);
const LOCK_BORDER: Color = Color::from_rgb(0.20, 0.55, 0.25);

static RESCUE_DATA: Lazy<Arc<Mutex<Option<(BtcLikeTransaction, ChainClient)>>>> =
    Lazy::new(|| Arc::new(Mutex::new(None)));

static BIP39_WORDS: Lazy<&'static [&'static str; 2048]> =
    Lazy::new(|| Language::English.word_list());

static BIP39_WORD_SET: Lazy<HashSet<&'static str>> =
    Lazy::new(|| BIP39_WORDS.iter().copied().collect());

#[derive(Debug, Clone)]
pub enum Message {
    LoadConfig,
    ConfigLoaded(Result<Config, String>),
    MnemonicWordChanged(usize, String),
    MnemonicWordPicked(usize, String),
    MnemonicWordEdit(usize),
    ToggleMnemonicVisibility,
    SetMnemonicExpanded(bool),
    SetWordCount(usize),
    PassphraseChanged(String),
    ReturnAddressChanged(String),
    SwapIndexChanged(String),
    ToggleAutoIncrement(bool),
    MaxSwapIndexChanged(String),
    ClearLogs,
    CopyLogs,
    SaveLogs,
    LogsSaved(Result<String, String>),
    ExecuteRescue,
    RescueCompleted(Result<RescueSummary, String>),
    ConfirmBroadcast,
    CancelBroadcast,
    BroadcastCompleted(Result<String, String>),
}

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
    pub provider_input: ProviderInput,
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
    #[serde(rename = "to_network", default)]
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

#[derive(Debug, Clone)]
enum ReturnAddressState {
    Empty,
    Valid(&'static str),
    Invalid(&'static str),
    Unknown,
}

fn is_address_for_chain(addr: &str, chain: Chain) -> bool {
    let addr = addr.trim();
    if addr.is_empty() {
        return false;
    }
    match chain {
        Chain::Bitcoin(_) => BtcAddress::from_str(addr)
            .ok()
            .and_then(|a| a.require_network(bitcoin::Network::Bitcoin).ok())
            .is_some(),
        Chain::Liquid(_) => ElementsAddress::from_str(addr)
            .ok()
            .map(|a| a.is_liquid())
            .unwrap_or(false),
    }
}

pub struct SwapRescueApp {
    state: AppState,
    config: Option<Config>,
    logs: Vec<String>,
    error_message: Option<String>,

    // GUI-collected user inputs (never persisted)
    mnemonic_words: Vec<String>,
    mnemonic_locked: Vec<bool>,
    mnemonic_visible: bool,
    mnemonic_expanded: bool,
    word_count: usize,
    focused_word: Option<usize>,
    word_input_ids: Vec<text_input::Id>,
    passphrase_input: String,
    return_address_input: String,

    // Swap index + auto-increment
    swap_index_override: u64,
    auto_increment: bool,
    max_swap_index: u64,
    max_swap_index_input: String,
}

impl Default for SwapRescueApp {
    fn default() -> Self {
        Self {
            state: AppState::Idle,
            config: None,
            logs: Vec::new(),
            error_message: None,
            mnemonic_words: vec![String::new(); MAX_WORDS],
            mnemonic_locked: vec![false; MAX_WORDS],
            mnemonic_visible: true,
            mnemonic_expanded: true,
            word_count: 12,
            focused_word: None,
            word_input_ids: (0..MAX_WORDS)
                .map(|i| text_input::Id::new(format!("word-{}", i)))
                .collect(),
            passphrase_input: String::new(),
            return_address_input: String::new(),
            swap_index_override: 0,
            auto_increment: true,
            max_swap_index: DEFAULT_MAX_SWAP_INDEX,
            max_swap_index_input: DEFAULT_MAX_SWAP_INDEX.to_string(),
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
                        self.append_log("✓ Provider config loaded");
                        self.append_log(&format!("  Swap ID: {}", config.provider_input.swap_id));
                        self.append_log(&format!(
                            "  Type: {}",
                            config.provider_input.rescue_type.to_uppercase()
                        ));
                        self.append_log(&format!(
                            "  Direction: {} → {}",
                            config.provider_input.from_network_str,
                            if config.provider_input.to_network_str.is_empty() {
                                "(n/a)".to_string()
                            } else {
                                config.provider_input.to_network_str.clone()
                            }
                        ));

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
            Message::MnemonicWordChanged(idx, raw_value) => {
                if idx >= self.word_count || self.mnemonic_locked[idx] {
                    return Task::none();
                }
                self.focused_word = Some(idx);

                // Handle paste of multiple words at once
                if raw_value.contains(char::is_whitespace) {
                    let words: Vec<String> = raw_value
                        .split_whitespace()
                        .map(|s| s.to_lowercase())
                        .collect();
                    if words.len() >= 24 && self.word_count < 24 {
                        self.word_count = 24;
                    }
                    let mut last = idx;
                    for (offset, word) in words.iter().enumerate() {
                        let i = idx + offset;
                        if i >= self.word_count {
                            break;
                        }
                        self.mnemonic_words[i] = word.clone();
                        self.mnemonic_locked[i] = BIP39_WORD_SET.contains(word.as_str());
                        last = i;
                    }
                    let next = (last + 1).min(self.word_count.saturating_sub(1));
                    self.focused_word = Some(next);
                    return text_input::focus(self.word_input_ids[next].clone());
                }

                let value = raw_value.to_lowercase();
                self.mnemonic_words[idx] = value.clone();
                if !value.is_empty() && BIP39_WORD_SET.contains(value.as_str()) {
                    self.mnemonic_locked[idx] = true;
                    let next = idx + 1;
                    if next < self.word_count {
                        self.focused_word = Some(next);
                        return text_input::focus(self.word_input_ids[next].clone());
                    }
                    self.focused_word = None;
                    if self.all_words_locked() {
                        self.mnemonic_expanded = false;
                    }
                }
                Task::none()
            }
            Message::MnemonicWordPicked(idx, word) => {
                if idx >= self.word_count || self.mnemonic_locked[idx] {
                    return Task::none();
                }
                self.mnemonic_words[idx] = word;
                self.mnemonic_locked[idx] = true;
                let next = idx + 1;
                if next < self.word_count {
                    self.focused_word = Some(next);
                    return text_input::focus(self.word_input_ids[next].clone());
                }
                self.focused_word = None;
                if self.all_words_locked() {
                    self.mnemonic_expanded = false;
                }
                Task::none()
            }
            Message::MnemonicWordEdit(idx) => {
                if idx < self.word_count {
                    self.mnemonic_locked[idx] = false;
                    self.focused_word = Some(idx);
                    self.mnemonic_expanded = true;
                    return text_input::focus(self.word_input_ids[idx].clone());
                }
                Task::none()
            }
            Message::ToggleMnemonicVisibility => {
                self.mnemonic_visible = !self.mnemonic_visible;
                Task::none()
            }
            Message::SetMnemonicExpanded(expanded) => {
                self.mnemonic_expanded = expanded;
                Task::none()
            }
            Message::SetWordCount(count) => {
                if count == 12 || count == 24 {
                    self.word_count = count;
                    // Clear words beyond the new count
                    for i in count..MAX_WORDS {
                        self.mnemonic_words[i].clear();
                        self.mnemonic_locked[i] = false;
                    }
                }
                Task::none()
            }
            Message::PassphraseChanged(value) => {
                self.passphrase_input = value;
                Task::none()
            }
            Message::ReturnAddressChanged(value) => {
                self.return_address_input = value;
                Task::none()
            }
            Message::SwapIndexChanged(value) => {
                if value.trim().is_empty() {
                    self.swap_index_override = 0;
                } else if let Ok(index) = value.parse::<u64>() {
                    self.swap_index_override = index;
                }
                Task::none()
            }
            Message::ToggleAutoIncrement(enabled) => {
                self.auto_increment = enabled;
                if enabled {
                    self.swap_index_override = 0;
                }
                self.append_log(if enabled {
                    "Auto-increment enabled (starting at 0)"
                } else {
                    "Auto-increment disabled"
                });
                Task::none()
            }
            Message::MaxSwapIndexChanged(value) => {
                self.max_swap_index_input = value.clone();
                if value.trim().is_empty() {
                    self.max_swap_index = DEFAULT_MAX_SWAP_INDEX;
                } else if let Ok(parsed) = value.parse::<u64>() {
                    self.max_swap_index = parsed;
                }
                Task::none()
            }
            Message::ClearLogs => {
                self.logs.clear();
                Task::none()
            }
            Message::CopyLogs => {
                if self.logs.is_empty() {
                    Task::none()
                } else {
                    let content = self.logs.join("\n");
                    self.append_log("✓ Logs copied to clipboard");
                    iced::clipboard::write(content)
                }
            }
            Message::SaveLogs => {
                if self.logs.is_empty() {
                    Task::none()
                } else {
                    let content = self.logs.join("\n");
                    Task::perform(save_logs_to_file(content), Message::LogsSaved)
                }
            }
            Message::LogsSaved(result) => {
                match result {
                    Ok(path) => self.append_log(&format!("✓ Logs saved to {}", path)),
                    Err(e) => self.append_log(&format!("✗ Save failed: {}", e)),
                }
                Task::none()
            }
            Message::ExecuteRescue => {
                let config = match self.config.clone() {
                    Some(c) => c,
                    None => return Task::none(),
                };

                let mnemonic = self.collected_mnemonic();
                let passphrase = self.passphrase_input.clone();
                let return_address = self.return_address_input.trim().to_string();
                let swap_index = self.swap_index_override;

                if let Err(e) = self.validate_user_input(&mnemonic, &return_address) {
                    self.append_log(&format!("✗ {}", e));
                    self.error_message = Some(e);
                    self.state = AppState::Error;
                    return Task::none();
                }

                self.state = AppState::Executing;
                self.append_log("Starting rescue operation...");
                self.append_log(&format!("Using swap index: {}", swap_index));
                self.append_log("Connecting to Electrum servers...");
                self.append_log("Building swap script...");

                Task::perform(
                    execute_rescue(config, mnemonic, passphrase, return_address, swap_index),
                    Message::RescueCompleted,
                )
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

                        self.state = AppState::AwaitingConfirmation;
                        self.error_message = None;
                        Task::none()
                    }
                    Err(e) => {
                        self.append_log(&format!("✗ Error: {}", e));

                        if self.auto_increment && self.swap_index_override < self.max_swap_index {
                            self.swap_index_override += 1;
                            self.append_log(&format!(
                                "↻ Auto-increment: retrying with swap index {} (max {})",
                                self.swap_index_override, self.max_swap_index
                            ));
                            self.error_message = None;
                            return Task::done(Message::ExecuteRescue);
                        }

                        if self.auto_increment {
                            self.append_log(&format!(
                                "✗ Auto-increment stopped: reached max index {}",
                                self.max_swap_index
                            ));
                        }

                        self.error_message = Some(e);
                        self.state = AppState::Error;
                        Task::none()
                    }
                }
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

    fn collected_mnemonic(&self) -> String {
        self.mnemonic_words[..self.word_count]
            .iter()
            .map(|w| w.trim())
            .collect::<Vec<_>>()
            .join(" ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn all_words_locked(&self) -> bool {
        self.mnemonic_locked[..self.word_count].iter().all(|l| *l)
    }

    fn validate_user_input(&self, mnemonic: &str, return_address: &str) -> Result<(), String> {
        if mnemonic.split_whitespace().count() != self.word_count {
            return Err(format!(
                "Mnemonic must be exactly {} words",
                self.word_count
            ));
        }
        if !self.all_words_locked() {
            return Err("One or more mnemonic words are not in the BIP39 word list".to_string());
        }
        if return_address.is_empty() {
            return Err("Return address is required".to_string());
        }
        if let Some((expected_chain, label)) = self.expected_return_chain() {
            if !is_address_for_chain(return_address, expected_chain) {
                return Err(format!(
                    "Return address is not a valid {} address",
                    label
                ));
            }
        }
        Ok(())
    }

    fn expected_return_chain(&self) -> Option<(Chain, &'static str)> {
        let config = self.config.as_ref()?;
        let rescue_type = config.provider_input.rescue_type.to_lowercase();
        let swap_type = config.provider_input.swap_type.to_lowercase();
        let network_str = if swap_type == "chain" && rescue_type == "claim" {
            &config.provider_input.to_network_str
        } else {
            &config.provider_input.from_network_str
        };
        let chain = crate::parse_chain(network_str).ok()?;
        let label = match chain {
            Chain::Bitcoin(_) => "Bitcoin (BTC)",
            Chain::Liquid(_) => "Liquid (LBTC)",
        };
        Some((chain, label))
    }

    fn return_address_state(&self) -> ReturnAddressState {
        let addr = self.return_address_input.trim();
        if addr.is_empty() {
            return ReturnAddressState::Empty;
        }
        match self.expected_return_chain() {
            Some((chain, label)) => {
                if is_address_for_chain(addr, chain) {
                    ReturnAddressState::Valid(label)
                } else {
                    ReturnAddressState::Invalid(label)
                }
            }
            None => ReturnAddressState::Unknown,
        }
    }

    fn mnemonic_ready(&self) -> bool {
        self.all_words_locked()
            && self.mnemonic_words[..self.word_count]
                .iter()
                .all(|w| !w.is_empty())
    }

    fn can_execute_rescue(&self) -> bool {
        if self.config.is_none() {
            return false;
        }
        if !self.mnemonic_ready() {
            return false;
        }
        matches!(self.return_address_state(), ReturnAddressState::Valid(_))
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

        let config_section = self.build_provider_section();
        let user_input_section: Element<Message> = if self.config.is_some() {
            self.build_user_input_section()
        } else {
            column![].into()
        };
        let logs_section = self.build_logs_section();
        let action_section = self.build_action_section();

        let content = column![
            header,
            config_section,
            user_input_section,
            logs_section,
            action_section,
        ]
        .spacing(24)
        .padding(Padding::new(20.0))
        .width(Length::Fill);

        let scrollable_content = scrollable(content).width(Length::Fill).height(Length::Fill);

        container(scrollable_content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_theme| container::Style {
                background: Some(iced::Background::Color(BACKGROUND)),
                ..Default::default()
            })
            .into()
    }

    fn build_provider_section(&self) -> Element<'_, Message> {
        let config_status = if let Some(config) = &self.config {
            let to_network_display = if config.provider_input.to_network_str.is_empty() {
                "(n/a)".to_string()
            } else {
                config.provider_input.to_network_str.clone()
            };
            column![
                detail_row("Swap ID", config.provider_input.swap_id.clone()),
                detail_row("Swap Type", config.provider_input.swap_type.to_uppercase()),
                detail_row("Rescue Type", config.provider_input.rescue_type.to_uppercase()),
                detail_row(
                    "Direction",
                    format!(
                        "{} → {}",
                        config.provider_input.from_network_str, to_network_display
                    ),
                ),
                detail_row("Amount", format!("{} sats", config.provider_input.amount)),
            ]
            .spacing(8)
        } else {
            column![text("No provider config loaded")
                .size(14)
                .font(GOLOS_TEXT)
                .color(TEXT_MUTED)]
        };

        let load_button = primary_button("Load Provider Config", Message::LoadConfig, 220.0);

        column![
            row![
                text("Provider Input")
                    .size(18)
                    .font(GOLOS_TEXT)
                    .color(TEXT),
                Space::with_width(Length::Fill),
                load_button,
            ]
            .align_y(Vertical::Center),
            Space::with_height(12),
            config_status,
        ]
        .spacing(0)
        .width(Length::Fill)
        .into()
    }

    fn build_user_input_section(&self) -> Element<'_, Message> {
        column![
            text("User Input")
                .size(18)
                .font(GOLOS_TEXT)
                .color(TEXT),
            Space::with_height(12),
            self.build_mnemonic_block(),
            Space::with_height(16),
            self.build_passphrase_block(),
            Space::with_height(16),
            self.build_return_address_block(),
            Space::with_height(16),
            self.build_swap_index_block(),
        ]
        .spacing(0)
        .width(Length::Fill)
        .into()
    }

    fn build_mnemonic_block(&self) -> Element<'_, Message> {
        if self.all_words_locked() && !self.mnemonic_expanded {
            return self.build_mnemonic_collapsed();
        }
        self.build_mnemonic_expanded()
    }

    fn build_mnemonic_collapsed(&self) -> Element<'_, Message> {
        let summary = text(format!("✓ Mnemonic entered ({} words)", self.word_count))
            .size(14)
            .font(GOLOS_TEXT)
            .color(TEXT);

        let edit_btn = button(
            text("Edit")
                .size(13)
                .font(GOLOS_TEXT)
                .align_x(Horizontal::Center),
        )
        .padding([6, 14])
        .style(secondary_button_style)
        .on_press(Message::SetMnemonicExpanded(true));

        let header = row![
            text("Mnemonic")
                .size(14)
                .font(GOLOS_TEXT)
                .color(TEXT_MUTED),
            Space::with_width(Length::Fill),
            edit_btn,
        ]
        .spacing(8)
        .align_y(Vertical::Center);

        let banner = container(summary)
            .padding(10)
            .width(Length::Fill)
            .style(|_theme| container::Style {
                background: Some(iced::Background::Color(LOCK_BG)),
                text_color: Some(TEXT),
                border: Border {
                    color: LOCK_BORDER,
                    width: 1.0,
                    radius: 8.0.into(),
                },
                shadow: Default::default(),
            });

        let helper = text("Mnemonic not saved locally")
            .size(12)
            .font(GOLOS_TEXT)
            .color(TEXT_MUTED);

        column![header, Space::with_height(6), banner, Space::with_height(4), helper]
            .spacing(0)
            .width(Length::Fill)
            .into()
    }

    fn build_mnemonic_expanded(&self) -> Element<'_, Message> {
        let label = text("Mnemonic")
            .size(14)
            .font(GOLOS_TEXT)
            .color(TEXT_MUTED);

        let count_12_btn = word_count_button("12 words", 12, self.word_count == 12);
        let count_24_btn = word_count_button("24 words", 24, self.word_count == 24);

        let visibility_label = if self.mnemonic_visible { "Hide" } else { "Show" };
        let visibility_btn = button(
            text(visibility_label)
                .size(13)
                .font(GOLOS_TEXT)
                .align_x(Horizontal::Center),
        )
        .padding([6, 14])
        .style(secondary_button_style)
        .on_press(Message::ToggleMnemonicVisibility);

        let mut header_row = row![
            label,
            Space::with_width(Length::Fill),
            count_12_btn,
            count_24_btn,
            Space::with_width(8),
            visibility_btn,
        ]
        .spacing(8)
        .align_y(Vertical::Center);

        if self.all_words_locked() {
            header_row = header_row.push(Space::with_width(8)).push(
                button(
                    text("Done")
                        .size(13)
                        .font(GOLOS_TEXT)
                        .align_x(Horizontal::Center),
                )
                .padding([6, 14])
                .style(primary_button_style)
                .on_press(Message::SetMnemonicExpanded(false)),
            );
        }

        let grid = self.build_mnemonic_grid();
        let suggestions = self.build_suggestion_row();
        let helper = text("Mnemonic not saved locally")
            .size(12)
            .font(GOLOS_TEXT)
            .color(TEXT_MUTED);

        column![header_row, Space::with_height(8), grid, suggestions, Space::with_height(4), helper]
            .spacing(4)
            .width(Length::Fill)
            .into()
    }

    fn build_mnemonic_grid(&self) -> Element<'_, Message> {
        let rows = (self.word_count + WORDS_PER_ROW - 1) / WORDS_PER_ROW;
        let mut row_items: Vec<Element<Message>> = Vec::with_capacity(rows);
        for r in 0..rows {
            let mut cells: Vec<Element<Message>> = Vec::with_capacity(WORDS_PER_ROW);
            for c in 0..WORDS_PER_ROW {
                let idx = r * WORDS_PER_ROW + c;
                if idx >= self.word_count {
                    cells.push(Space::with_width(Length::Fixed(WORD_BOX_WIDTH + 36.0)).into());
                    continue;
                }
                cells.push(self.build_word_cell(idx));
            }
            row_items.push(Row::with_children(cells).spacing(8).into());
        }
        Column::with_children(row_items).spacing(6).into()
    }

    fn build_word_cell(&self, idx: usize) -> Element<'_, Message> {
        let number = text(format!("{:>2}.", idx + 1))
            .size(13)
            .font(GOLOS_TEXT)
            .color(TEXT_MUTED)
            .width(Length::Fixed(28.0));

        let value = self.mnemonic_words[idx].clone();
        let locked = self.mnemonic_locked[idx];

        let field: Element<Message> = if locked {
            let display = if self.mnemonic_visible {
                value
            } else {
                "•".repeat(value.chars().count().max(3))
            };
            button(
                text(display)
                    .size(14)
                    .font(GOLOS_TEXT)
                    .color(TEXT)
                    .align_x(Horizontal::Center)
                    .width(Length::Fill),
            )
            .padding([6, 8])
            .width(Length::Fixed(WORD_BOX_WIDTH))
            .style(locked_word_style)
            .on_press(Message::MnemonicWordEdit(idx))
            .into()
        } else {
            let mut input = text_input("", &value)
                .id(self.word_input_ids[idx].clone())
                .size(14)
                .font(GOLOS_TEXT)
                .padding(6)
                .width(Length::Fixed(WORD_BOX_WIDTH))
                .on_input(move |v| Message::MnemonicWordChanged(idx, v));
            if !self.mnemonic_visible {
                input = input.secure(true);
            }
            input.into()
        };

        row![number, field]
            .spacing(4)
            .align_y(Vertical::Center)
            .width(Length::Fixed(WORD_BOX_WIDTH + 36.0))
            .into()
    }

    fn build_suggestion_row(&self) -> Element<'_, Message> {
        let Some(idx) = self.focused_word else {
            return Space::with_height(0).into();
        };
        if idx >= self.word_count || self.mnemonic_locked[idx] {
            return Space::with_height(0).into();
        }
        let prefix = &self.mnemonic_words[idx];
        if prefix.is_empty() {
            return Space::with_height(0).into();
        }
        let prefix_lower = prefix.to_lowercase();
        let matches: Vec<&'static str> = BIP39_WORDS
            .iter()
            .copied()
            .filter(|w| w.starts_with(&prefix_lower))
            .take(MAX_SUGGESTIONS)
            .collect();
        if matches.is_empty() {
            let warn = text(format!("No BIP39 word starts with \"{}\"", prefix))
                .size(12)
                .font(GOLOS_TEXT)
                .color(Color::from_rgb(0.75, 0.10, 0.10));
            return row![warn].padding([4, 0]).into();
        }
        let mut buttons: Vec<Element<Message>> = Vec::with_capacity(matches.len() + 1);
        let hint = text(format!("Suggestions for #{}: ", idx + 1))
            .size(12)
            .font(GOLOS_TEXT)
            .color(TEXT_MUTED);
        buttons.push(hint.into());
        for word in matches {
            let word_owned = word.to_string();
            let display = word_owned.clone();
            buttons.push(
                button(text(display).size(12).font(GOLOS_TEXT))
                    .padding([4, 8])
                    .style(suggestion_button_style)
                    .on_press(Message::MnemonicWordPicked(idx, word_owned))
                    .into(),
            );
        }
        Row::with_children(buttons)
            .spacing(6)
            .align_y(Vertical::Center)
            .padding([4, 0])
            .into()
    }

    fn build_passphrase_block(&self) -> Element<'_, Message> {
        let label = text("Passphrase (optional)")
            .size(14)
            .font(GOLOS_TEXT)
            .color(TEXT_MUTED);
        let input = text_input("", &self.passphrase_input)
            .size(14)
            .font(GOLOS_TEXT)
            .padding(8)
            .secure(true)
            .width(Length::Fill)
            .on_input(Message::PassphraseChanged);
        column![label, Space::with_height(4), input]
            .spacing(0)
            .width(Length::Fill)
            .into()
    }

    fn build_return_address_block(&self) -> Element<'_, Message> {
        let expected_label = self
            .expected_return_chain()
            .map(|(_, l)| format!("Return address (expected: {})", l))
            .unwrap_or_else(|| "Return address".to_string());

        let label = text(expected_label)
            .size(14)
            .font(GOLOS_TEXT)
            .color(TEXT_MUTED);

        let input = text_input("", &self.return_address_input)
            .size(14)
            .font(GOLOS_TEXT)
            .padding(8)
            .width(Length::Fill)
            .on_input(Message::ReturnAddressChanged);

        let feedback: Element<Message> = match self.return_address_state() {
            ReturnAddressState::Empty => text(
                "Claim: enter to_network address · Refund: enter from_network address",
            )
            .size(12)
            .font(GOLOS_TEXT)
            .color(TEXT_MUTED)
            .into(),
            ReturnAddressState::Valid(label) => text(format!("✓ Valid {} address", label))
                .size(12)
                .font(GOLOS_TEXT)
                .color(Color::from_rgb(0.13, 0.55, 0.20))
                .into(),
            ReturnAddressState::Invalid(label) => text(format!(
                "✗ Not a valid {} address — expected {}",
                label, label
            ))
            .size(12)
            .font(GOLOS_TEXT)
            .color(Color::from_rgb(0.78, 0.10, 0.10))
            .into(),
            ReturnAddressState::Unknown => text("Could not determine expected network from config")
                .size(12)
                .font(GOLOS_TEXT)
                .color(Color::from_rgb(0.78, 0.10, 0.10))
                .into(),
        };

        column![label, Space::with_height(4), input, Space::with_height(2), feedback]
            .spacing(0)
            .width(Length::Fill)
            .into()
    }

    fn build_swap_index_block(&self) -> Element<'_, Message> {
        let auto_toggle = toggler(self.auto_increment)
            .label("Auto-Increment Index")
            .text_size(14)
            .font(GOLOS_TEXT)
            .style(red_toggler_style)
            .on_toggle(Message::ToggleAutoIncrement);

        let body: Element<Message> = if self.auto_increment {
            let max_index_input = text_input(
                &DEFAULT_MAX_SWAP_INDEX.to_string(),
                &self.max_swap_index_input,
            )
            .size(14)
            .font(GOLOS_TEXT)
            .width(Length::Fixed(100.0))
            .padding(6)
            .on_input(Message::MaxSwapIndexChanged);

            column![
                row![
                    text("Max index")
                        .size(13)
                        .font(GOLOS_TEXT)
                        .color(TEXT_MUTED),
                    Space::with_width(Length::Fill),
                    max_index_input,
                ]
                .align_y(Vertical::Center),
                Space::with_height(4),
                text(format!(
                    "Currently trying: {} (will retry up to {} on failure)",
                    self.swap_index_override, self.max_swap_index
                ))
                .size(12)
                .font(GOLOS_TEXT)
                .color(TEXT_MUTED),
            ]
            .spacing(0)
            .into()
        } else {
            let index_input = text_input("0", &self.swap_index_override.to_string())
                .size(14)
                .font(GOLOS_TEXT)
                .width(Length::Fixed(120.0))
                .padding(6)
                .on_input(Message::SwapIndexChanged);

            column![
                row![
                    text("Swap index")
                        .size(13)
                        .font(GOLOS_TEXT)
                        .color(TEXT_MUTED),
                    Space::with_width(Length::Fill),
                    index_input,
                ]
                .align_y(Vertical::Center),
            ]
            .spacing(0)
            .into()
        };

        column![auto_toggle, Space::with_height(8), body]
            .spacing(0)
            .width(Length::Fill)
            .into()
    }

    fn build_logs_section(&self) -> Element<'_, Message> {
        let small_red_btn = |label: &str, msg: Message| {
            let owned = label.to_string();
            button(
                text(owned)
                    .size(14)
                    .font(GOLOS_TEXT)
                    .align_x(Horizontal::Center),
            )
            .padding([6, 12])
            .style(primary_button_style)
            .on_press(msg)
        };

        let logs_header = row![
            text("Logs").size(18).font(GOLOS_TEXT).color(TEXT),
            Space::with_width(Length::Fill),
            small_red_btn("Copy", Message::CopyLogs),
            small_red_btn("Save", Message::SaveLogs),
            small_red_btn("Clear", Message::ClearLogs),
        ]
        .spacing(8)
        .align_y(Vertical::Center);

        let log_content: Element<_> = if self.logs.is_empty() {
            container(
                text("No logs yet...")
                    .size(14)
                    .font(GOLOS_TEXT)
                    .color(TEXT_MUTED),
            )
            .padding(16)
            .width(Length::Fill)
            .height(Length::Fixed(280.0))
            .style(log_box_style)
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
                        .width(Length::Fill),
                )
                .height(Length::Fixed(280.0)),
            )
            .width(Length::Fill)
            .height(Length::Fixed(280.0))
            .style(log_box_style)
            .into()
        };

        column![logs_header, log_content]
            .spacing(12)
            .width(Length::Fill)
            .into()
    }

    fn build_action_section(&self) -> Element<'_, Message> {
        let enabled = self.can_execute_rescue();
        let blocker = self.execute_blocker_reason();

        let buttons: Element<Message> = match self.state {
            AppState::Idle => row![].into(),
            AppState::ConfigLoaded | AppState::Error => {
                let execute_button = button(
                    text("Execute Rescue")
                        .size(16)
                        .font(GOLOS_TEXT)
                        .width(Length::Fill)
                        .align_x(Horizontal::Center),
                )
                .padding(12)
                .width(Length::Fixed(220.0))
                .style(primary_button_style)
                .on_press_maybe(if enabled {
                    Some(Message::ExecuteRescue)
                } else {
                    None
                });

                let mut col = column![execute_button].spacing(6).align_x(Horizontal::Center);
                if let Some(reason) = blocker {
                    col = col.push(
                        text(reason)
                            .size(12)
                            .font(GOLOS_TEXT)
                            .color(TEXT_MUTED),
                    );
                }
                col.into()
            }
            AppState::Executing => row![text("Executing...")
                .size(16)
                .font(GOLOS_TEXT)
                .color(TEXT_MUTED)]
            .into(),
            AppState::AwaitingConfirmation => row![
                primary_button("Confirm & Broadcast", Message::ConfirmBroadcast, 220.0),
                button(
                    text("Cancel")
                        .size(16)
                        .font(GOLOS_TEXT)
                        .width(Length::Fill)
                        .align_x(Horizontal::Center),
                )
                .padding(12)
                .width(Length::Fixed(200.0))
                .style(secondary_button_style)
                .on_press(Message::CancelBroadcast),
            ]
            .spacing(12)
            .into(),
            AppState::Broadcasting => row![text("Broadcasting...")
                .size(16)
                .font(GOLOS_TEXT)
                .color(TEXT_MUTED)]
            .into(),
            AppState::Completed => row![text("✓ Completed Successfully")
                .size(16)
                .font(GOLOS_TEXT)
                .color(TEXT)]
            .into(),
        };

        container(buttons)
            .width(Length::Fill)
            .padding(Padding::new(0.0).top(10.0).bottom(10.0))
            .align_x(Horizontal::Center)
            .into()
    }

    fn execute_blocker_reason(&self) -> Option<String> {
        if self.config.is_none() {
            return Some("Load provider config to begin".to_string());
        }
        if !self.mnemonic_ready() {
            let missing = self.mnemonic_locked[..self.word_count]
                .iter()
                .filter(|l| !**l)
                .count();
            return Some(format!(
                "Mnemonic incomplete ({} of {} words remaining)",
                missing, self.word_count
            ));
        }
        match self.return_address_state() {
            ReturnAddressState::Empty => Some("Enter a return address".to_string()),
            ReturnAddressState::Invalid(label) => {
                Some(format!("Return address must be a valid {} address", label))
            }
            ReturnAddressState::Unknown => {
                Some("Could not determine expected return network".to_string())
            }
            ReturnAddressState::Valid(_) => None,
        }
    }
}

fn detail_row(label: &'static str, value: String) -> Element<'static, Message> {
    row![
        text(label)
            .size(14)
            .font(GOLOS_TEXT)
            .color(GREY_DARK)
            .width(Length::Fixed(110.0)),
        text(value).size(14).font(GOLOS_TEXT).color(TEXT),
    ]
    .spacing(12)
    .into()
}

fn word_count_button(label: &str, count: usize, active: bool) -> button::Button<'_, Message> {
    button(
        text(label.to_string())
            .size(13)
            .font(GOLOS_TEXT)
            .align_x(Horizontal::Center),
    )
    .padding([6, 14])
    .style(move |_theme, status| {
        let base_bg = if active { PRIMARY_RED } else { SURFACE };
        let base_text = if active { WHITE } else { TEXT };
        let border = Border {
            color: if active { PRIMARY_RED } else { BORDER },
            width: 1.0,
            radius: 8.0.into(),
        };
        let base = button::Style {
            background: Some(iced::Background::Color(base_bg)),
            text_color: base_text,
            border,
            shadow: Default::default(),
        };
        match status {
            button::Status::Hovered => button::Style {
                background: Some(iced::Background::Color(if active {
                    Color::from_rgb(0.85, 0.05, 0.05)
                } else {
                    BACKGROUND
                })),
                ..base
            },
            _ => base,
        }
    })
    .on_press(Message::SetWordCount(count))
}

fn primary_button(label: &str, msg: Message, width: f32) -> button::Button<'_, Message> {
    button(
        text(label.to_string())
            .size(16)
            .font(GOLOS_TEXT)
            .width(Length::Fill)
            .align_x(Horizontal::Center),
    )
    .padding(12)
    .width(Length::Fixed(width))
    .style(primary_button_style)
    .on_press(msg)
}

fn primary_button_style(_theme: &iced::Theme, status: button::Status) -> button::Style {
    let base = button::Style {
        background: Some(iced::Background::Color(PRIMARY_RED)),
        text_color: WHITE,
        border: Border {
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
}

fn secondary_button_style(_theme: &iced::Theme, status: button::Status) -> button::Style {
    let base = button::Style {
        background: Some(iced::Background::Color(SURFACE)),
        text_color: TEXT,
        border: Border {
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
}

fn locked_word_style(_theme: &iced::Theme, status: button::Status) -> button::Style {
    let base = button::Style {
        background: Some(iced::Background::Color(LOCK_BG)),
        text_color: TEXT,
        border: Border {
            color: LOCK_BORDER,
            width: 1.0,
            radius: 8.0.into(),
        },
        shadow: Default::default(),
    };
    match status {
        button::Status::Hovered => button::Style {
            background: Some(iced::Background::Color(Color::from_rgb(0.86, 0.95, 0.86))),
            ..base
        },
        _ => base,
    }
}

fn suggestion_button_style(_theme: &iced::Theme, status: button::Status) -> button::Style {
    let base = button::Style {
        background: Some(iced::Background::Color(SURFACE)),
        text_color: TEXT,
        border: Border {
            color: BORDER,
            width: 1.0,
            radius: 6.0.into(),
        },
        shadow: Default::default(),
    };
    match status {
        button::Status::Hovered => button::Style {
            background: Some(iced::Background::Color(BACKGROUND)),
            ..base
        },
        _ => base,
    }
}

fn red_toggler_style(_theme: &iced::Theme, status: iced::widget::toggler::Status) -> iced::widget::toggler::Style {
    use iced::widget::toggler::{Status, Style};
    let (background, foreground) = match status {
        Status::Active { is_toggled } | Status::Hovered { is_toggled } => {
            if is_toggled {
                (PRIMARY_RED, WHITE)
            } else {
                (GREY_LIGHT, SURFACE)
            }
        }
        Status::Disabled => (GREY_MEDIUM, SURFACE),
    };
    Style {
        background,
        background_border_width: 1.0,
        background_border_color: match status {
            Status::Active { is_toggled: true } | Status::Hovered { is_toggled: true } => PRIMARY_RED,
            _ => BORDER,
        },
        foreground,
        foreground_border_width: 0.0,
        foreground_border_color: Color::TRANSPARENT,
    }
}

fn log_box_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(SURFACE)),
        text_color: Some(TEXT),
        border: Border {
            color: BORDER,
            width: 1.0,
            radius: 8.0.into(),
        },
        shadow: Default::default(),
    }
}

async fn save_logs_to_file(content: String) -> Result<String, String> {
    let file_handle = rfd::AsyncFileDialog::new()
        .add_filter("Text", &["txt", "log"])
        .set_file_name("swap_rescue_logs.txt")
        .set_title("Save logs as...")
        .save_file()
        .await;

    if let Some(file) = file_handle {
        file.write(content.as_bytes())
            .await
            .map_err(|e| format!("Write error: {}", e))?;
        Ok(file.path().display().to_string())
    } else {
        Err("Save cancelled".to_string())
    }
}

async fn load_config_file() -> Result<Config, String> {
    let file_handle = rfd::AsyncFileDialog::new()
        .add_filter("YAML", &["yaml", "yml"])
        .set_title("Select provider config (YAML)")
        .pick_file()
        .await;

    if let Some(file) = file_handle {
        let contents = file.read().await;
        let config_str = String::from_utf8(contents).map_err(|e| format!("Invalid UTF-8: {}", e))?;
        let config: Config = serde_yaml::from_str(&config_str)
            .map_err(|e| format!("YAML parse error: {}", e))?;
        validate_provider_config(&config)?;
        Ok(config)
    } else {
        Err("No file selected".to_string())
    }
}

fn validate_provider_config(config: &Config) -> Result<(), String> {
    if config.provider_input.rescue_type.trim().is_empty() {
        return Err("rescue_type is required in provider_input".to_string());
    }

    let rescue_type = config.provider_input.rescue_type.to_lowercase();
    if rescue_type != "claim" && rescue_type != "refund" {
        return Err(format!(
            "rescue_type must be 'claim' or 'refund', got '{}'",
            config.provider_input.rescue_type
        ));
    }

    let swap_type = config.provider_input.swap_type.to_lowercase();
    if swap_type != "chain" && swap_type != "submarine" && swap_type != "reverse" {
        return Err(format!(
            "swap_type must be 'chain', 'submarine', or 'reverse', got '{}'",
            config.provider_input.swap_type
        ));
    }

    if swap_type == "reverse" && rescue_type != "claim" {
        return Err(
            "reverse swaps only support rescue_type 'claim' (user cannot refund a reverse swap)"
                .to_string(),
        );
    }

    if swap_type == "submarine" && rescue_type == "claim" {
        return Err(
            "submarine swaps only support rescue_type 'refund' on the user side".to_string(),
        );
    }

    if config.provider_input.claim_leaf.output.trim().is_empty() {
        return Err("claim_leaf.output is required in provider_input".to_string());
    }

    if config.provider_input.refund_leaf.output.trim().is_empty() {
        return Err("refund_leaf.output is required in provider_input".to_string());
    }

    if config.provider_input.from_network_str.trim().is_empty() {
        return Err("from_network is required in provider_input".to_string());
    }

    if swap_type == "chain" && config.provider_input.to_network_str.trim().is_empty() {
        return Err("to_network is required in provider_input for chain swaps".to_string());
    }

    if config.provider_input.timeout_block_height == 0 {
        return Err("timeout_block_height must be greater than 0 in provider_input".to_string());
    }

    if config.provider_input.server_public_key.trim().is_empty() {
        return Err("server_public_key is required in provider_input".to_string());
    }

    if config.provider_input.user_lockup_address.trim().is_empty() {
        return Err("user_lockup_address is required in provider_input".to_string());
    }

    if swap_type == "chain" && rescue_type == "claim" {
        match &config.provider_input.server_lockup_address {
            Some(addr) if !addr.trim().is_empty() => {}
            _ => {
                return Err(
                    "server_lockup_address is required for chain swap claims".to_string(),
                );
            }
        }
    }

    if config.provider_input.amount == 0 {
        return Err("amount must be greater than 0 in provider_input".to_string());
    }

    if config.provider_input.swap_id.trim().is_empty() {
        return Err("swap_id is required in provider_input".to_string());
    }

    let preimage_required =
        rescue_type == "claim" && (swap_type == "chain" || swap_type == "reverse");
    let preimage_provided = config
        .provider_input
        .preimage
        .as_deref()
        .map(|p| !p.trim().is_empty())
        .unwrap_or(false);

    if preimage_required && !preimage_provided {
        return Err(format!(
            "preimage is required in provider_input for {} claim",
            swap_type
        ));
    }

    Ok(())
}

async fn execute_rescue(
    config: Config,
    mnemonic: String,
    passphrase: String,
    return_address: String,
    swap_index: u64,
) -> Result<RescueSummary, String> {
    use crate::{
        claim_rescue, parse_chain, refund_rescue, reverse_claim_rescue, submarine_refund_rescue,
    };

    let from_network = parse_chain(&config.provider_input.from_network_str)
        .map_err(|e| format!("Invalid from_network: {}", e))?;

    let rescue_type = config.provider_input.rescue_type.to_lowercase();
    let swap_type = config.provider_input.swap_type.to_lowercase();

    let result = if swap_type == "reverse" {
        let preimage = config
            .provider_input
            .preimage
            .as_deref()
            .ok_or_else(|| "preimage is required for reverse claim".to_string())?;
        reverse_claim_rescue(
            &mnemonic,
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
            &return_address,
            preimage,
            &passphrase,
            swap_index,
            from_network,
        )
        .await
    } else if rescue_type == "refund" {
        if swap_type == "submarine" {
            submarine_refund_rescue(
                &mnemonic,
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
                &return_address,
                &passphrase,
                swap_index,
                from_network,
            )
            .await
        } else {
            let to_network = parse_chain(&config.provider_input.to_network_str)
                .map_err(|e| format!("Invalid to_network: {}", e))?;

            refund_rescue(
                &mnemonic,
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
                &return_address,
                &passphrase,
                swap_index,
                from_network,
                to_network,
            )
            .await
        }
    } else {
        let to_network = parse_chain(&config.provider_input.to_network_str)
            .map_err(|e| format!("Invalid to_network: {}", e))?;

        let server_lockup_address = config
            .provider_input
            .server_lockup_address
            .as_deref()
            .ok_or_else(|| "server_lockup_address is required for claim rescue".to_string())?;

        claim_rescue(
            &mnemonic,
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
            &return_address,
            config.provider_input.preimage.as_deref(),
            &passphrase,
            swap_index,
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
            if let Ok(mut data) = RESCUE_DATA.lock() {
                *data = Some((tx, chain_client));
            }
            summary
        })
        .map_err(|e| format!("{:?}", e))
}

async fn broadcast_transaction() -> Result<String, String> {
    let (tx, chain_client) = {
        let mut data = RESCUE_DATA
            .lock()
            .map_err(|e| format!("Failed to lock rescue data: {}", e))?;
        data.take()
            .ok_or_else(|| "No transaction data available".to_string())?
    };

    chain_client
        .broadcast_tx(&tx)
        .await
        .map(|_| "Transaction broadcast successfully!".to_string())
        .map_err(|e| format!("Failed to broadcast: {:?}", e))
}

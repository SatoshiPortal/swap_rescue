use iced::widget::{button, container, text_input};
use iced::{Border, Color, Theme};

// Color palette - Following BullBitcoin theme
pub const PRIMARY_RED: Color = Color::from_rgb(0.77, 0.04, 0.04); // #C50909
pub const ON_PRIMARY: Color = Color::from_rgb(1.0, 1.0, 1.0); // #FFFFFF
pub const BACKGROUND: Color = Color::from_rgb(0.96, 0.96, 0.96); // #F5F5F5
pub const SURFACE: Color = Color::from_rgb(1.0, 1.0, 1.0); // #FFFFFF
pub const TEXT: Color = Color::from_rgb(0.08, 0.09, 0.11); // #15171C
pub const TEXT_MUTED: Color = Color::from_rgb(0.44, 0.45, 0.49); // #70747D
pub const BORDER: Color = Color::from_rgb(0.79, 0.79, 0.80); // #C9CACD
pub const GREY_LIGHT: Color = Color::from_rgb(0.91, 0.91, 0.91); // #E8E8E8

// Aliases for consistency
pub const BLACK: Color = TEXT;
pub const WHITE: Color = ON_PRIMARY;
pub const GREY_DARK: Color = TEXT_MUTED;
pub const GREY_BORDER: Color = BORDER;
pub const GREY_MEDIUM: Color = GREY_LIGHT;

// Button styles
#[derive(Default)]
pub enum ButtonStyle {
    #[default]
    Primary,
    Secondary,
    Danger,
}

impl button::Catalog for ButtonStyle {
    type Class<'a> = Theme;

    fn default<'a>() -> Self::Class<'a> {
        <Theme as std::default::Default>::default()
    }

    fn style(&self, _class: &Self::Class<'_>, status: button::Status) -> button::Style {
        let base = match self {
            ButtonStyle::Primary => button::Style {
                background: Some(iced::Background::Color(PRIMARY_RED)),
                text_color: WHITE,
                border: Border {
                    color: PRIMARY_RED,
                    width: 0.0,
                    radius: 8.0.into(),
                },
                shadow: Default::default(),
            },
            ButtonStyle::Secondary => button::Style {
                background: Some(iced::Background::Color(SURFACE)),
                text_color: TEXT,
                border: Border {
                    color: BORDER,
                    width: 1.0,
                    radius: 8.0.into(),
                },
                shadow: Default::default(),
            },
            ButtonStyle::Danger => button::Style {
                background: Some(iced::Background::Color(Color::from_rgb(0.77, 0.04, 0.04))),
                text_color: WHITE,
                border: Border {
                    color: Color::from_rgb(0.77, 0.04, 0.04),
                    width: 0.0,
                    radius: 8.0.into(),
                },
                shadow: Default::default(),
            },
        };

        match status {
            button::Status::Hovered => button::Style {
                background: match self {
                    ButtonStyle::Primary => Some(iced::Background::Color(Color::from_rgb(0.85, 0.05, 0.05))),
                    ButtonStyle::Secondary => Some(iced::Background::Color(BACKGROUND)),
                    ButtonStyle::Danger => Some(iced::Background::Color(Color::from_rgb(0.90, 0.06, 0.06))),
                },
                ..base
            },
            button::Status::Pressed => button::Style {
                background: match self {
                    ButtonStyle::Primary => Some(iced::Background::Color(Color::from_rgb(0.65, 0.03, 0.03))),
                    ButtonStyle::Secondary => Some(iced::Background::Color(GREY_LIGHT)),
                    ButtonStyle::Danger => Some(iced::Background::Color(Color::from_rgb(0.65, 0.03, 0.03))),
                },
                ..base
            },
            button::Status::Disabled => button::Style {
                background: Some(iced::Background::Color(GREY_MEDIUM)),
                text_color: GREY_DARK,
                border: Border {
                    color: GREY_BORDER,
                    width: 1.0,
                    radius: 8.0.into(),
                },
                shadow: Default::default(),
            },
            _ => base,
        }
    }
}

// Container styles
#[derive(Default)]
pub enum ContainerStyle {
    #[default]
    Default,
    Card,
    LogBox,
}

impl container::Catalog for ContainerStyle {
    type Class<'a> = Theme;

    fn default<'a>() -> Self::Class<'a> {
        <Theme as std::default::Default>::default()
    }

    fn style(&self, _class: &Self::Class<'_>) -> container::Style {
        match self {
            ContainerStyle::Default => container::Style {
                background: Some(iced::Background::Color(BACKGROUND)),
                text_color: Some(TEXT),
                border: Border::default(),
                shadow: Default::default(),
            },
            ContainerStyle::Card => container::Style {
                background: Some(iced::Background::Color(SURFACE)),
                text_color: Some(TEXT),
                border: Border {
                    color: BORDER,
                    width: 1.0,
                    radius: 12.0.into(),
                },
                shadow: Default::default(),
            },
            ContainerStyle::LogBox => container::Style {
                background: Some(iced::Background::Color(SURFACE)),
                text_color: Some(TEXT),
                border: Border {
                    color: BORDER,
                    width: 1.0,
                    radius: 8.0.into(),
                },
                shadow: Default::default(),
            },
        }
    }
}

// Text input styles
#[derive(Default)]
pub enum TextInputStyle {
    #[default]
    Default,
}

impl text_input::Catalog for TextInputStyle {
    type Class<'a> = Theme;

    fn default<'a>() -> Self::Class<'a> {
        <Theme as std::default::Default>::default()
    }

    fn style(&self, _class: &Self::Class<'_>, status: text_input::Status) -> text_input::Style {
        let base = text_input::Style {
            background: iced::Background::Color(SURFACE),
            border: Border {
                color: BORDER,
                width: 1.0,
                radius: 8.0.into(),
            },
            icon: TEXT,
            placeholder: TEXT_MUTED,
            value: TEXT,
            selection: GREY_LIGHT,
        };

        match status {
            text_input::Status::Focused => text_input::Style {
                border: Border {
                    color: PRIMARY_RED,
                    width: 2.0,
                    radius: 8.0.into(),
                },
                ..base
            },
            text_input::Status::Hovered => text_input::Style {
                border: Border {
                    color: TEXT_MUTED,
                    width: 1.0,
                    radius: 8.0.into(),
                },
                ..base
            },
            _ => base,
        }
    }
}

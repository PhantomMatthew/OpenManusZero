//! Computer use tool for controlling the computer
//!
//! Provides capabilities for:
//! - Screen capture
//! - Mouse control (move, click, drag, scroll)
//! - Keyboard input (type text, press keys, hotkeys)
//! - Window management
//!
//! This tool requires the `computer-use` feature to be enabled for full functionality.
//! Without the feature, it provides stub implementations that log actions without executing them.

use crate::context::Context;
use crate::tool::{Tool, ToolError, ToolParameter, ToolResult, ToolSchema};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

#[cfg(feature = "computer-use")]
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine};
#[cfg(feature = "computer-use")]
use enigo::{Axis, Button, Coordinate, Direction, Enigo, Key, Keyboard, Mouse, Settings};
#[cfg(feature = "computer-use")]
use screenshots::Screen;

/// Mouse buttons supported by the tool
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum MouseButton {
    #[default]
    Left,
    Right,
    Middle,
}

impl std::fmt::Display for MouseButton {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MouseButton::Left => write!(f, "left"),
            MouseButton::Right => write!(f, "right"),
            MouseButton::Middle => write!(f, "middle"),
        }
    }
}

#[cfg(feature = "computer-use")]
impl From<MouseButton> for Button {
    fn from(button: MouseButton) -> Self {
        match button {
            MouseButton::Left => Button::Left,
            MouseButton::Right => Button::Right,
            MouseButton::Middle => Button::Middle,
        }
    }
}

/// Scroll direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScrollDirection {
    Up,
    Down,
    Left,
    Right,
}

impl std::fmt::Display for ScrollDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScrollDirection::Up => write!(f, "up"),
            ScrollDirection::Down => write!(f, "down"),
            ScrollDirection::Left => write!(f, "left"),
            ScrollDirection::Right => write!(f, "right"),
        }
    }
}

/// Computer action types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum ComputerAction {
    /// Take a screenshot
    Screenshot,
    /// Move mouse to absolute position
    MouseMove { x: i32, y: i32 },
    /// Click at current position or specified position
    Click {
        x: Option<i32>,
        y: Option<i32>,
        button: Option<MouseButton>,
        clicks: Option<u32>,
    },
    /// Double click at position
    DoubleClick {
        x: Option<i32>,
        y: Option<i32>,
        button: Option<MouseButton>,
    },
    /// Press and hold mouse button
    MouseDown {
        x: Option<i32>,
        y: Option<i32>,
        button: Option<MouseButton>,
    },
    /// Release mouse button
    MouseUp {
        x: Option<i32>,
        y: Option<i32>,
        button: Option<MouseButton>,
    },
    /// Drag from current position to target
    DragTo {
        x: i32,
        y: i32,
        button: Option<MouseButton>,
    },
    /// Type text (with proper character input)
    Type { text: String },
    /// Press a single key
    KeyPress { key: String },
    /// Press a key combination (e.g., "ctrl+c")
    Hotkey { keys: String },
    /// Scroll in a direction
    Scroll {
        direction: ScrollDirection,
        amount: Option<i32>,
    },
    /// Get screen size
    GetScreenSize,
    /// Get current mouse position
    GetMousePosition,
    /// Open application (platform-specific)
    OpenApp { app: String },
    /// Wait for specified duration in seconds
    Wait { duration: f64 },
}

/// Screen information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenInfo {
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
}

impl Default for ScreenInfo {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            scale_factor: 1.0,
        }
    }
}

/// Internal state for tracking mouse position
#[derive(Debug, Default, Clone, Copy)]
struct MouseState {
    x: i32,
    y: i32,
}

/// Computer use tool for desktop automation
pub struct ComputerUseTool {
    screen_info: ScreenInfo,
    mouse_state: Mutex<MouseState>,
}

// Explicitly implement Send and Sync since we only use Mutex<MouseState> which is Send+Sync
unsafe impl Send for ComputerUseTool {}
unsafe impl Sync for ComputerUseTool {}

impl ComputerUseTool {
    /// Create a new computer use tool with default settings
    pub fn new() -> Self {
        let screen_info = Self::detect_screen_info();
        Self {
            screen_info,
            mouse_state: Mutex::new(MouseState::default()),
        }
    }

    /// Create with custom screen info
    pub fn with_screen_info(screen_info: ScreenInfo) -> Self {
        Self {
            screen_info,
            mouse_state: Mutex::new(MouseState::default()),
        }
    }

    /// Detect screen information from the system
    fn detect_screen_info() -> ScreenInfo {
        #[cfg(feature = "computer-use")]
        {
            if let Ok(screens) = Screen::all() {
                if let Some(primary) = screens.first() {
                    return ScreenInfo {
                        width: primary.display_info.width,
                        height: primary.display_info.height,
                        scale_factor: primary.display_info.scale_factor as f64,
                    };
                }
            }
        }
        ScreenInfo::default()
    }

    /// Create a fresh Enigo instance for each operation
    #[cfg(feature = "computer-use")]
    fn create_enigo() -> Result<Enigo, ToolError> {
        let settings = Settings::default();
        Enigo::new(&settings)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to initialize enigo: {}", e)))
    }

    /// Update internal mouse position state
    fn update_mouse_position(&self, x: i32, y: i32) {
        if let Ok(mut state) = self.mouse_state.lock() {
            state.x = x;
            state.y = y;
        }
    }

    /// Get current mouse position from state
    fn get_mouse_position(&self) -> (i32, i32) {
        self.mouse_state
            .lock()
            .map(|state| (state.x, state.y))
            .unwrap_or((0, 0))
    }

    /// Execute a computer action
    async fn execute_action(&self, action: ComputerAction) -> Result<ToolResult, ToolError> {
        match action {
            ComputerAction::Screenshot => self.take_screenshot().await,
            ComputerAction::MouseMove { x, y } => self.move_mouse(x, y).await,
            ComputerAction::Click { x, y, button, clicks } => {
                self.click(x, y, button.unwrap_or_default(), clicks.unwrap_or(1)).await
            }
            ComputerAction::DoubleClick { x, y, button } => {
                self.click(x, y, button.unwrap_or_default(), 2).await
            }
            ComputerAction::MouseDown { x, y, button } => {
                self.mouse_down(x, y, button.unwrap_or_default()).await
            }
            ComputerAction::MouseUp { x, y, button } => {
                self.mouse_up(x, y, button.unwrap_or_default()).await
            }
            ComputerAction::DragTo { x, y, button } => {
                self.drag_to(x, y, button.unwrap_or_default()).await
            }
            ComputerAction::Type { text } => self.type_text(&text).await,
            ComputerAction::KeyPress { key } => self.press_key(&key).await,
            ComputerAction::Hotkey { keys } => self.press_hotkey(&keys).await,
            ComputerAction::Scroll { direction, amount } => {
                self.scroll(direction, amount.unwrap_or(3)).await
            }
            ComputerAction::GetScreenSize => self.get_screen_size().await,
            ComputerAction::GetMousePosition => {
                let (x, y) = self.get_mouse_position();
                Ok(ToolResult::success(format!("Mouse position: ({}, {})", x, y)))
            }
            ComputerAction::OpenApp { app } => self.open_app(&app).await,
            ComputerAction::Wait { duration } => self.wait(duration).await,
        }
    }

    /// Take a screenshot of the primary screen
    async fn take_screenshot(&self) -> Result<ToolResult, ToolError> {
        #[cfg(feature = "computer-use")]
        {
            let result = tokio::task::spawn_blocking(move || -> Result<String, ToolError> {
                let screens = Screen::all().map_err(|e| {
                    ToolError::ExecutionFailed(format!("Failed to enumerate screens: {}", e))
                })?;

                let screen = screens.first().ok_or_else(|| {
                    ToolError::ExecutionFailed("No screens found".to_string())
                })?;

                let image = screen.capture().map_err(|e| {
                    ToolError::ExecutionFailed(format!("Failed to capture screen: {}", e))
                })?;

                // Get the raw RGBA buffer and encode to PNG
                let width = image.width();
                let height = image.height();
                let rgba_buffer = image.into_raw();

                let mut png_buffer = Vec::new();
                {
                    let mut encoder = png::Encoder::new(
                        std::io::Cursor::new(&mut png_buffer), width, height
                    );
                    encoder.set_color(png::ColorType::Rgba);
                    encoder.set_depth(png::BitDepth::Eight);
                    let mut writer = encoder.write_header().map_err(|e| {
                        ToolError::ExecutionFailed(format!("Failed to write PNG header: {}", e))
                    })?;
                    writer.write_image_data(&rgba_buffer).map_err(|e| {
                        ToolError::ExecutionFailed(format!("Failed to write PNG data: {}", e))
                    })?;
                }

                Ok(BASE64_STANDARD.encode(&png_buffer))
            })
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Screenshot task failed: {}", e)))??;

            Ok(ToolResult::with_image(
                format!("Screenshot captured ({}x{})", self.screen_info.width, self.screen_info.height),
                result,
            ))
        }

        #[cfg(not(feature = "computer-use"))]
        {
            tracing::info!("Screenshot requested (stub)");
            Ok(ToolResult::success(format!(
                "Screenshot captured ({}x{}) [stub]",
                self.screen_info.width, self.screen_info.height
            )))
        }
    }

    /// Move the mouse to an absolute position
    async fn move_mouse(&self, x: i32, y: i32) -> Result<ToolResult, ToolError> {
        #[cfg(feature = "computer-use")]
        {
            tokio::task::spawn_blocking(move || -> Result<(), ToolError> {
                let mut enigo = Self::create_enigo()?;
                enigo.move_mouse(x, y, Coordinate::Abs).map_err(|e| {
                    ToolError::ExecutionFailed(format!("Failed to move mouse: {}", e))
                })?;
                Ok(())
            })
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Mouse move task failed: {}", e)))??;

            self.update_mouse_position(x, y);
            Ok(ToolResult::success(format!("Mouse moved to ({}, {})", x, y)))
        }

        #[cfg(not(feature = "computer-use"))]
        {
            self.update_mouse_position(x, y);
            Ok(ToolResult::success(format!("Mouse moved to ({}, {}) [stub]", x, y)))
        }
    }

    /// Click at position (or current position if not specified)
    async fn click(&self, x: Option<i32>, y: Option<i32>, button: MouseButton, clicks: u32) -> Result<ToolResult, ToolError> {
        let (click_x, click_y) = match (x, y) {
            (Some(x), Some(y)) => (x, y),
            _ => self.get_mouse_position(),
        };

        #[cfg(feature = "computer-use")]
        {
            tokio::task::spawn_blocking(move || -> Result<(), ToolError> {
                let mut enigo = Self::create_enigo()?;
                enigo.move_mouse(click_x, click_y, Coordinate::Abs).map_err(|e| {
                    ToolError::ExecutionFailed(format!("Failed to move mouse: {}", e))
                })?;
                let btn: Button = button.into();
                for _ in 0..clicks {
                    enigo.button(btn, Direction::Click).map_err(|e| {
                        ToolError::ExecutionFailed(format!("Failed to click: {}", e))
                    })?;
                }
                Ok(())
            })
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Click task failed: {}", e)))??;

            self.update_mouse_position(click_x, click_y);
            Ok(ToolResult::success(format!("Clicked {} button {} time(s) at ({}, {})", button, clicks, click_x, click_y)))
        }

        #[cfg(not(feature = "computer-use"))]
        {
            self.update_mouse_position(click_x, click_y);
            Ok(ToolResult::success(format!("Clicked {} button {} time(s) at ({}, {}) [stub]", button, clicks, click_x, click_y)))
        }
    }

    /// Press and hold mouse button
    async fn mouse_down(&self, x: Option<i32>, y: Option<i32>, button: MouseButton) -> Result<ToolResult, ToolError> {
        let (pos_x, pos_y) = match (x, y) {
            (Some(x), Some(y)) => (x, y),
            _ => self.get_mouse_position(),
        };

        #[cfg(feature = "computer-use")]
        {
            tokio::task::spawn_blocking(move || -> Result<(), ToolError> {
                let mut enigo = Self::create_enigo()?;
                enigo.move_mouse(pos_x, pos_y, Coordinate::Abs).map_err(|e| {
                    ToolError::ExecutionFailed(format!("Failed to move mouse: {}", e))
                })?;
                let btn: Button = button.into();
                enigo.button(btn, Direction::Press).map_err(|e| {
                    ToolError::ExecutionFailed(format!("Failed to press mouse button: {}", e))
                })?;
                Ok(())
            })
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Mouse down task failed: {}", e)))??;

            self.update_mouse_position(pos_x, pos_y);
            Ok(ToolResult::success(format!("Mouse {} button pressed at ({}, {})", button, pos_x, pos_y)))
        }

        #[cfg(not(feature = "computer-use"))]
        {
            self.update_mouse_position(pos_x, pos_y);
            Ok(ToolResult::success(format!("Mouse {} button pressed at ({}, {}) [stub]", button, pos_x, pos_y)))
        }
    }

    /// Release mouse button
    async fn mouse_up(&self, x: Option<i32>, y: Option<i32>, button: MouseButton) -> Result<ToolResult, ToolError> {
        let (pos_x, pos_y) = match (x, y) {
            (Some(x), Some(y)) => (x, y),
            _ => self.get_mouse_position(),
        };

        #[cfg(feature = "computer-use")]
        {
            tokio::task::spawn_blocking(move || -> Result<(), ToolError> {
                let mut enigo = Self::create_enigo()?;
                enigo.move_mouse(pos_x, pos_y, Coordinate::Abs).map_err(|e| {
                    ToolError::ExecutionFailed(format!("Failed to move mouse: {}", e))
                })?;
                let btn: Button = button.into();
                enigo.button(btn, Direction::Release).map_err(|e| {
                    ToolError::ExecutionFailed(format!("Failed to release mouse button: {}", e))
                })?;
                Ok(())
            })
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Mouse up task failed: {}", e)))??;

            self.update_mouse_position(pos_x, pos_y);
            Ok(ToolResult::success(format!("Mouse {} button released at ({}, {})", button, pos_x, pos_y)))
        }

        #[cfg(not(feature = "computer-use"))]
        {
            self.update_mouse_position(pos_x, pos_y);
            Ok(ToolResult::success(format!("Mouse {} button released at ({}, {}) [stub]", button, pos_x, pos_y)))
        }
    }

    /// Drag from current position to target position
    async fn drag_to(&self, x: i32, y: i32, button: MouseButton) -> Result<ToolResult, ToolError> {
        let (start_x, start_y) = self.get_mouse_position();

        #[cfg(feature = "computer-use")]
        {
            tokio::task::spawn_blocking(move || -> Result<(), ToolError> {
                let mut enigo = Self::create_enigo()?;
                let btn: Button = button.into();

                enigo.move_mouse(start_x, start_y, Coordinate::Abs).map_err(|e| {
                    ToolError::ExecutionFailed(format!("Failed to move mouse: {}", e))
                })?;
                enigo.button(btn, Direction::Press).map_err(|e| {
                    ToolError::ExecutionFailed(format!("Failed to press mouse button: {}", e))
                })?;
                std::thread::sleep(std::time::Duration::from_millis(50));
                enigo.move_mouse(x, y, Coordinate::Abs).map_err(|e| {
                    ToolError::ExecutionFailed(format!("Failed to move mouse: {}", e))
                })?;
                std::thread::sleep(std::time::Duration::from_millis(50));
                enigo.button(btn, Direction::Release).map_err(|e| {
                    ToolError::ExecutionFailed(format!("Failed to release mouse button: {}", e))
                })?;
                Ok(())
            })
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Drag task failed: {}", e)))??;

            self.update_mouse_position(x, y);
            Ok(ToolResult::success(format!("Dragged from ({}, {}) to ({}, {}) with {} button", start_x, start_y, x, y, button)))
        }

        #[cfg(not(feature = "computer-use"))]
        {
            self.update_mouse_position(x, y);
            Ok(ToolResult::success(format!("Dragged from ({}, {}) to ({}, {}) with {} button [stub]", start_x, start_y, x, y, button)))
        }
    }

    /// Type text character by character
    async fn type_text(&self, text: &str) -> Result<ToolResult, ToolError> {
        #[cfg(feature = "computer-use")]
        {
            let text_owned = text.to_string();
            tokio::task::spawn_blocking(move || -> Result<(), ToolError> {
                let mut enigo = Self::create_enigo()?;
                enigo.text(&text_owned).map_err(|e| {
                    ToolError::ExecutionFailed(format!("Failed to type text: {}", e))
                })?;
                Ok(())
            })
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Type task failed: {}", e)))??;

            Ok(ToolResult::success(format!("Typed: {}", text)))
        }

        #[cfg(not(feature = "computer-use"))]
        {
            Ok(ToolResult::success(format!("Typed: {} [stub]", text)))
        }
    }

    /// Parse a key string into an enigo Key
    #[cfg(feature = "computer-use")]
    fn parse_key(key_str: &str) -> Option<Key> {
        let key_lower = key_str.to_lowercase();
        match key_lower.as_str() {
            "a" => Some(Key::Unicode('a')),
            "b" => Some(Key::Unicode('b')),
            "c" => Some(Key::Unicode('c')),
            "d" => Some(Key::Unicode('d')),
            "e" => Some(Key::Unicode('e')),
            "f" => Some(Key::Unicode('f')),
            "g" => Some(Key::Unicode('g')),
            "h" => Some(Key::Unicode('h')),
            "i" => Some(Key::Unicode('i')),
            "j" => Some(Key::Unicode('j')),
            "k" => Some(Key::Unicode('k')),
            "l" => Some(Key::Unicode('l')),
            "m" => Some(Key::Unicode('m')),
            "n" => Some(Key::Unicode('n')),
            "o" => Some(Key::Unicode('o')),
            "p" => Some(Key::Unicode('p')),
            "q" => Some(Key::Unicode('q')),
            "r" => Some(Key::Unicode('r')),
            "s" => Some(Key::Unicode('s')),
            "t" => Some(Key::Unicode('t')),
            "u" => Some(Key::Unicode('u')),
            "v" => Some(Key::Unicode('v')),
            "w" => Some(Key::Unicode('w')),
            "x" => Some(Key::Unicode('x')),
            "y" => Some(Key::Unicode('y')),
            "z" => Some(Key::Unicode('z')),
            "0" => Some(Key::Unicode('0')),
            "1" => Some(Key::Unicode('1')),
            "2" => Some(Key::Unicode('2')),
            "3" => Some(Key::Unicode('3')),
            "4" => Some(Key::Unicode('4')),
            "5" => Some(Key::Unicode('5')),
            "6" => Some(Key::Unicode('6')),
            "7" => Some(Key::Unicode('7')),
            "8" => Some(Key::Unicode('8')),
            "9" => Some(Key::Unicode('9')),
            "enter" | "return" => Some(Key::Return),
            "tab" => Some(Key::Tab),
            "space" => Some(Key::Space),
            "backspace" => Some(Key::Backspace),
            "delete" | "del" => Some(Key::Delete),
            "escape" | "esc" => Some(Key::Escape),
            "up" | "uparrow" => Some(Key::UpArrow),
            "down" | "downarrow" => Some(Key::DownArrow),
            "left" | "leftarrow" => Some(Key::LeftArrow),
            "right" | "rightarrow" => Some(Key::RightArrow),
            "shift" => Some(Key::Shift),
            "ctrl" | "control" => Some(Key::Control),
            "alt" | "option" => Some(Key::Alt),
            "meta" | "cmd" | "command" | "win" | "windows" | "super" => Some(Key::Meta),
            "f1" => Some(Key::F1),
            "f2" => Some(Key::F2),
            "f3" => Some(Key::F3),
            "f4" => Some(Key::F4),
            "f5" => Some(Key::F5),
            "f6" => Some(Key::F6),
            "f7" => Some(Key::F7),
            "f8" => Some(Key::F8),
            "f9" => Some(Key::F9),
            "f10" => Some(Key::F10),
            "f11" => Some(Key::F11),
            "f12" => Some(Key::F12),
            "home" => Some(Key::Home),
            "end" => Some(Key::End),
            "pageup" | "page_up" => Some(Key::PageUp),
            "pagedown" | "page_down" => Some(Key::PageDown),
            "capslock" | "caps_lock" => Some(Key::CapsLock),
            _ if key_str.chars().count() == 1 => key_str.chars().next().map(Key::Unicode),
            _ => None,
        }
    }

    /// Press a single key
    async fn press_key(&self, key: &str) -> Result<ToolResult, ToolError> {
        #[cfg(feature = "computer-use")]
        {
            let key_enum = Self::parse_key(key)
                .ok_or_else(|| ToolError::InvalidInput(format!("Unknown key: {}", key)))?;

            tokio::task::spawn_blocking(move || -> Result<(), ToolError> {
                let mut enigo = Self::create_enigo()?;
                enigo.key(key_enum, Direction::Click).map_err(|e| {
                    ToolError::ExecutionFailed(format!("Failed to press key: {}", e))
                })?;
                Ok(())
            })
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Key press task failed: {}", e)))??;

            Ok(ToolResult::success(format!("Pressed key: {}", key)))
        }

        #[cfg(not(feature = "computer-use"))]
        {
            Ok(ToolResult::success(format!("Pressed key: {} [stub]", key)))
        }
    }

    /// Press a key combination (hotkey)
    async fn press_hotkey(&self, keys: &str) -> Result<ToolResult, ToolError> {
        #[cfg(feature = "computer-use")]
        {
            let key_parts: Vec<&str> = keys.split('+').map(|s| s.trim()).collect();
            if key_parts.is_empty() {
                return Err(ToolError::InvalidInput("No keys specified".to_string()));
            }

            let parsed_keys: Vec<Key> = key_parts
                .iter()
                .map(|k| Self::parse_key(k).ok_or_else(|| ToolError::InvalidInput(format!("Unknown key: {}", k))))
                .collect::<Result<Vec<_>, _>>()?;

            tokio::task::spawn_blocking(move || -> Result<(), ToolError> {
                let mut enigo = Self::create_enigo()?;
                for key in &parsed_keys {
                    enigo.key(*key, Direction::Press).map_err(|e| {
                        ToolError::ExecutionFailed(format!("Failed to press key: {}", e))
                    })?;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
                for key in parsed_keys.iter().rev() {
                    enigo.key(*key, Direction::Release).map_err(|e| {
                        ToolError::ExecutionFailed(format!("Failed to release key: {}", e))
                    })?;
                }
                Ok(())
            })
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Hotkey task failed: {}", e)))??;

            Ok(ToolResult::success(format!("Pressed hotkey: {}", keys)))
        }

        #[cfg(not(feature = "computer-use"))]
        {
            Ok(ToolResult::success(format!("Pressed hotkey: {} [stub]", keys)))
        }
    }

    /// Scroll in a direction
    async fn scroll(&self, direction: ScrollDirection, amount: i32) -> Result<ToolResult, ToolError> {
        #[cfg(feature = "computer-use")]
        {
            tokio::task::spawn_blocking(move || -> Result<(), ToolError> {
                let mut enigo = Self::create_enigo()?;
                let (axis, scroll_amount) = match direction {
                    ScrollDirection::Up => (Axis::Vertical, amount),
                    ScrollDirection::Down => (Axis::Vertical, -amount),
                    ScrollDirection::Left => (Axis::Horizontal, -amount),
                    ScrollDirection::Right => (Axis::Horizontal, amount),
                };
                enigo.scroll(scroll_amount, axis).map_err(|e| {
                    ToolError::ExecutionFailed(format!("Failed to scroll: {}", e))
                })?;
                Ok(())
            })
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Scroll task failed: {}", e)))??;

            Ok(ToolResult::success(format!("Scrolled {} by {}", direction, amount)))
        }

        #[cfg(not(feature = "computer-use"))]
        {
            Ok(ToolResult::success(format!("Scrolled {} by {} [stub]", direction, amount)))
        }
    }

    /// Get screen size information
    async fn get_screen_size(&self) -> Result<ToolResult, ToolError> {
        Ok(ToolResult::success(format!(
            "Screen size: {}x{} (scale: {})",
            self.screen_info.width, self.screen_info.height, self.screen_info.scale_factor
        )))
    }

    /// Open an application
    async fn open_app(&self, app: &str) -> Result<ToolResult, ToolError> {
        #[cfg(target_os = "macos")]
        {
            let output = tokio::process::Command::new("open")
                .arg("-a")
                .arg(app)
                .output()
                .await
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to execute open: {}", e)))?;

            if output.status.success() {
                Ok(ToolResult::success(format!("Opened application: {}", app)))
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(ToolError::ExecutionFailed(format!("Failed to open {}: {}", app, stderr)))
            }
        }

        #[cfg(target_os = "linux")]
        {
            for launcher in &["xdg-open", "gnome-open", "kde-open"] {
                if let Ok(output) = tokio::process::Command::new(launcher).arg(app).output().await {
                    if output.status.success() {
                        return Ok(ToolResult::success(format!("Opened application: {}", app)));
                    }
                }
            }
            match tokio::process::Command::new(app).spawn() {
                Ok(_) => Ok(ToolResult::success(format!("Opened application: {}", app))),
                Err(e) => Err(ToolError::ExecutionFailed(format!("Failed to open {}: {}", app, e))),
            }
        }

        #[cfg(target_os = "windows")]
        {
            let output = tokio::process::Command::new("cmd")
                .args(["/C", "start", "", app])
                .output()
                .await
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to execute start: {}", e)))?;

            if output.status.success() {
                Ok(ToolResult::success(format!("Opened application: {}", app)))
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(ToolError::ExecutionFailed(format!("Failed to open {}: {}", app, stderr)))
            }
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        Err(ToolError::ExecutionFailed("Opening applications is not supported on this platform".to_string()))
    }

    /// Wait for a specified duration
    async fn wait(&self, duration: f64) -> Result<ToolResult, ToolError> {
        let clamped_duration = duration.clamp(0.0, 60.0);
        tokio::time::sleep(tokio::time::Duration::from_secs_f64(clamped_duration)).await;
        Ok(ToolResult::success(format!("Waited for {:.2} seconds", clamped_duration)))
    }
}

impl Default for ComputerUseTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ComputerUseTool {
    fn name(&self) -> &'static str {
        "computer"
    }

    fn description(&self) -> &'static str {
        "Computer control tool for desktop automation including mouse, keyboard, and screenshots"
    }

    fn parameters(&self) -> Option<ToolSchema> {
        Some(ToolSchema {
            schema_type: "object".to_string(),
            properties: {
                let mut props = HashMap::new();
                props.insert("action".to_string(), ToolParameter {
                    name: "action".to_string(),
                    param_type: "string".to_string(),
                    description: Some("Action to perform".to_string()),
                    required: Some(true),
                    enum_values: Some(vec![
                        "screenshot".to_string(), "mouse_move".to_string(), "click".to_string(),
                        "double_click".to_string(), "mouse_down".to_string(), "mouse_up".to_string(),
                        "drag_to".to_string(), "type".to_string(), "key_press".to_string(),
                        "hotkey".to_string(), "scroll".to_string(), "get_screen_size".to_string(),
                        "get_mouse_position".to_string(), "open_app".to_string(), "wait".to_string(),
                    ]),
                    default: None,
                });
                props.insert("x".to_string(), ToolParameter {
                    name: "x".to_string(), param_type: "integer".to_string(),
                    description: Some("X coordinate".to_string()), required: Some(false),
                    default: None, enum_values: None,
                });
                props.insert("y".to_string(), ToolParameter {
                    name: "y".to_string(), param_type: "integer".to_string(),
                    description: Some("Y coordinate".to_string()), required: Some(false),
                    default: None, enum_values: None,
                });
                props.insert("button".to_string(), ToolParameter {
                    name: "button".to_string(), param_type: "string".to_string(),
                    description: Some("Mouse button: left, right, middle".to_string()),
                    required: Some(false), default: Some(serde_json::json!("left")),
                    enum_values: Some(vec!["left".to_string(), "right".to_string(), "middle".to_string()]),
                });
                props.insert("clicks".to_string(), ToolParameter {
                    name: "clicks".to_string(), param_type: "integer".to_string(),
                    description: Some("Number of clicks".to_string()), required: Some(false),
                    default: Some(serde_json::json!(1)), enum_values: None,
                });
                props.insert("text".to_string(), ToolParameter {
                    name: "text".to_string(), param_type: "string".to_string(),
                    description: Some("Text to type".to_string()), required: Some(false),
                    default: None, enum_values: None,
                });
                props.insert("key".to_string(), ToolParameter {
                    name: "key".to_string(), param_type: "string".to_string(),
                    description: Some("Key to press".to_string()), required: Some(false),
                    default: None, enum_values: None,
                });
                props.insert("keys".to_string(), ToolParameter {
                    name: "keys".to_string(), param_type: "string".to_string(),
                    description: Some("Key combination (e.g., 'ctrl+c')".to_string()),
                    required: Some(false), default: None, enum_values: None,
                });
                props.insert("direction".to_string(), ToolParameter {
                    name: "direction".to_string(), param_type: "string".to_string(),
                    description: Some("Scroll direction".to_string()), required: Some(false),
                    default: None, enum_values: Some(vec!["up".to_string(), "down".to_string(), "left".to_string(), "right".to_string()]),
                });
                props.insert("amount".to_string(), ToolParameter {
                    name: "amount".to_string(), param_type: "integer".to_string(),
                    description: Some("Scroll amount".to_string()), required: Some(false),
                    default: Some(serde_json::json!(3)), enum_values: None,
                });
                props.insert("app".to_string(), ToolParameter {
                    name: "app".to_string(), param_type: "string".to_string(),
                    description: Some("Application name to open".to_string()), required: Some(false),
                    default: None, enum_values: None,
                });
                props.insert("duration".to_string(), ToolParameter {
                    name: "duration".to_string(), param_type: "number".to_string(),
                    description: Some("Wait duration in seconds".to_string()), required: Some(false),
                    default: Some(serde_json::json!(1.0)), enum_values: None,
                });
                props
            },
            required: Some(vec!["action".to_string()]),
        })
    }

    async fn execute(&self, input: &str, _ctx: &mut Context) -> Result<ToolResult, ToolError> {
        let action: ComputerAction = serde_json::from_str(input)
            .map_err(|e| ToolError::InvalidInput(format!("Invalid computer action: {}", e)))?;
        self.execute_action(action).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_computer_use_tool_new() {
        let tool = ComputerUseTool::new();
        assert_eq!(tool.name(), "computer");
    }

    #[test]
    fn test_screen_info_default() {
        let info = ScreenInfo::default();
        assert_eq!(info.width, 1920);
        assert_eq!(info.height, 1080);
    }

    #[tokio::test]
    async fn test_get_screen_size() {
        let tool = ComputerUseTool::new();
        let result = tool.get_screen_size().await.unwrap();
        assert!(!result.is_error());
    }

    #[tokio::test]
    async fn test_wait() {
        let tool = ComputerUseTool::new();
        let result = tool.wait(0.05).await.unwrap();
        assert!(!result.is_error());
    }

    #[test]
    fn test_computer_action_deserialize() {
        let json = r#"{"action": "click", "x": 100, "y": 200}"#;
        let action: ComputerAction = serde_json::from_str(json).unwrap();
        matches!(action, ComputerAction::Click { .. });
    }

    #[test]
    fn test_mouse_state_tracking() {
        let tool = ComputerUseTool::new();
        assert_eq!(tool.get_mouse_position(), (0, 0));
        tool.update_mouse_position(100, 200);
        assert_eq!(tool.get_mouse_position(), (100, 200));
    }

    #[cfg(feature = "computer-use")]
    mod feature_tests {
        use super::*;

        #[test]
        fn test_parse_key() {
            assert!(matches!(ComputerUseTool::parse_key("enter"), Some(Key::Return)));
            assert!(matches!(ComputerUseTool::parse_key("ctrl"), Some(Key::Control)));
            assert!(ComputerUseTool::parse_key("unknownkey").is_none());
        }
    }
}

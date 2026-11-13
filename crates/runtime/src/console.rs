//! Engine console: camera commands, diagnostics and game-defined commands.
//!
//! The console owns only text state and drawing. The runtime executes built-in
//! commands and delegates unknown ones to [`crate::Game::command`], so games do
//! not have to reimplement camera spawning or selection.

use ui::UiDrawList;

const MAX_MESSAGES: usize = 200;
const VISIBLE_MESSAGES: usize = 12;

struct Line {
    text: String,
    error: bool,
}

/// Console state, owned by the engine runtime.
pub struct Console {
    open: bool,
    input: String,
    messages: Vec<Line>,
    history: Vec<String>,
    history_index: usize,
    pending: Option<String>,
    /// Tab completion state: (prefix, matches, current_match_index)
    completion: Option<(String, Vec<String>, usize)>,
}

impl Default for Console {
    fn default() -> Self {
        Self::new()
    }
}

impl Console {
    pub fn new() -> Self {
        let mut console = Self {
            open: false,
            input: String::new(),
            messages: Vec::new(),
            history: Vec::new(),
            history_index: 0,
            pending: None,
            completion: None,
        };
        console.print("Soldatik3D console ready. Type `help` for commands.");
        console.print(
            "F1 / ` closes the console. Camera commands: camera.list, camera.spawn, camera.select.",
        );
        console
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn set_open(&mut self, open: bool) {
        self.open = open;
        if !open {
            self.input.clear();
            self.pending = None;
        }
        self.history_index = self.history.len();
    }

    pub fn push_text(&mut self, text: &str) {
        if self.open {
            self.input.push_str(text);
        }
    }

    pub fn backspace(&mut self) {
        if self.open {
            self.input.pop();
        }
    }

    /// Stores the current input as a pending command. The runtime consumes it
    /// on the next draw call, where the world and game are available.
    pub fn submit(&mut self) {
        if !self.open {
            return;
        }
        let line = self.input.trim().to_owned();
        self.input.clear();
        if line.is_empty() {
            return;
        }
        self.print(format!("> {line}"));
        self.history.push(line.clone());
        self.history_index = self.history.len();
        self.pending = Some(line);
    }

    pub fn take_pending(&mut self) -> Option<String> {
        self.pending.take()
    }

    pub fn history_previous(&mut self) {
        if self.history.is_empty() {
            return;
        }
        self.history_index = self.history_index.saturating_sub(1);
        self.input = self.history[self.history_index].clone();
    }

    pub fn history_next(&mut self) {
        if self.history.is_empty() {
            return;
        }
        self.history_index = (self.history_index + 1).min(self.history.len());
        self.input = if self.history_index == self.history.len() {
            String::new()
        } else {
            self.history[self.history_index].clone()
        };
    }

    /// Starts or advances tab completion using the provided registry.
    pub fn complete(&mut self, registry: &crate::commands::CommandRegistry) {
        if self.completion.is_none() {
            // First tab: find matches for current input
            let prefix = self.input.trim();
            let matches: Vec<String> = registry
                .complete(prefix)
                .into_iter()
                .map(|s| s.to_owned())
                .collect();
            if matches.is_empty() {
                return;
            }
            self.completion = Some((prefix.to_owned(), matches, 0));
        }
        // Cycle to next match
        if let Some((_prefix, matches, index)) = &mut self.completion {
            *index = (*index + 1) % matches.len();
            self.input = format!("{} ", matches[*index]);
        }
    }

    /// Resets completion state (call on non-tab key press).
    pub fn reset_completion(&mut self) {
        self.completion = None;
    }

    pub fn print(&mut self, text: impl Into<String>) {
        self.messages.push(Line {
            text: text.into(),
            error: false,
        });
        self.trim();
    }

    pub fn error(&mut self, text: impl Into<String>) {
        self.messages.push(Line {
            text: text.into(),
            error: true,
        });
        self.trim();
    }

    pub fn clear(&mut self) {
        self.messages.clear();
        self.print("Console cleared.");
    }

    fn trim(&mut self) {
        if self.messages.len() > MAX_MESSAGES {
            let excess = self.messages.len() - MAX_MESSAGES;
            self.messages.drain(..excess);
        }
    }

    /// Draws either the open console or a small shortcut hint.
    pub fn draw(&self, ui: &mut UiDrawList) {
        if !self.open {
            let hint = "F1 CONSOLE";
            let scale = 1.2;
            let width = UiDrawList::text_width(hint, scale) + 12.0;
            let x = ui.width() - width - 12.0;
            let y = 12.0;
            ui.rect(x, y, width, 20.0, [0.03, 0.035, 0.045, 0.65]);
            ui.text(hint, x + 6.0, y + 5.0, scale, [0.75, 0.75, 0.78, 0.9]);
            return;
        }

        let panel_width = (ui.width() - 24.0).min(780.0);
        let line_height = 16.0;
        let message_count = self.messages.len().min(VISIBLE_MESSAGES);
        let panel_height = 34.0 + message_count as f32 * line_height + 50.0;
        let x = 12.0;
        let y = 12.0;

        // Background with subtle gradient effect
        ui.rect(x, y, panel_width, panel_height, [0.02, 0.025, 0.035, 0.96]);
        ui.rect(x, y, panel_width, 2.0, [1.0, 0.55, 0.1, 1.0]);
        ui.rect(
            x,
            y + panel_height - 1.0,
            panel_width,
            1.0,
            [0.25, 0.25, 0.3, 1.0],
        );

        // Title bar
        ui.rect(x, y + 2.0, panel_width, 28.0, [0.05, 0.06, 0.08, 0.9]);
        ui.text(
            "SOLDATIK3D CONSOLE",
            x + 10.0,
            y + 7.0,
            1.4,
            [1.0, 0.75, 0.2, 1.0],
        );
        ui.text(
            "F1 CLOSE  |  TAB autocomplete  |  ↑↓ history",
            x + panel_width - 300.0,
            y + 9.0,
            1.0,
            [0.6, 0.65, 0.7, 1.0],
        );

        let start = self.messages.len().saturating_sub(VISIBLE_MESSAGES);
        for (row, line) in self.messages[start..].iter().enumerate() {
            let color = if line.error {
                [1.0, 0.4, 0.35, 1.0]
            } else if line.text.starts_with('>') {
                [1.0, 0.85, 0.4, 1.0]
            } else {
                [0.85, 0.88, 0.92, 1.0]
            };
            let text = fit(&line.text, panel_width - 20.0, 1.2);
            ui.text(
                &text,
                x + 10.0,
                y + 34.0 + row as f32 * line_height,
                1.2,
                color,
            );
        }

        // Autocomplete suggestions
        let messages_y = y + 34.0 + message_count as f32 * line_height;
        if let Some((_prefix, matches, index)) = &self.completion {
            let suggestion_y = messages_y + 4.0;
            let suggestion_count = matches.len().min(6);
            ui.rect(
                x + 8.0,
                suggestion_y,
                panel_width - 16.0,
                suggestion_count as f32 * 18.0 + 8.0,
                [0.08, 0.1, 0.14, 0.95],
            );
            for (i, m) in matches.iter().take(suggestion_count).enumerate() {
                let is_selected = i == *index;
                let color = if is_selected {
                    [1.0, 0.9, 0.3, 1.0]
                } else {
                    [0.7, 0.8, 0.9, 1.0]
                };
                let bg_color = if is_selected {
                    [0.15, 0.18, 0.25, 1.0]
                } else {
                    [0.0, 0.0, 0.0, 0.0]
                };
                ui.rect(
                    x + 10.0,
                    suggestion_y + 4.0 + i as f32 * 18.0,
                    panel_width - 20.0,
                    18.0,
                    bg_color,
                );
                ui.text(
                    m,
                    x + 14.0,
                    suggestion_y + 5.0 + i as f32 * 18.0,
                    1.1,
                    color,
                );
            }
        }

        let input_y = y + panel_height - 40.0;
        ui.rect(
            x + 8.0,
            input_y - 3.0,
            panel_width - 16.0,
            28.0,
            [0.06, 0.07, 0.09, 1.0],
        );
        ui.rect(
            x + 10.0,
            input_y + 2.0,
            panel_width - 20.0,
            22.0,
            [0.03, 0.04, 0.06, 1.0],
        );
        let prompt = format!("> {}_", self.input);
        ui.text(
            &prompt,
            x + 16.0,
            input_y + 6.0,
            1.3,
            [0.95, 0.97, 1.0, 1.0],
        );
    }
}

fn fit(text: &str, max_width: f32, scale: f32) -> String {
    if UiDrawList::text_width(text, scale) <= max_width {
        return text.to_owned();
    }
    let max_chars = (max_width / (6.0 * scale)).floor().max(1.0) as usize;
    let mut result: String = text.chars().take(max_chars.saturating_sub(3)).collect();
    result.push_str("...");
    result
}

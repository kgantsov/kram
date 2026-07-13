use ratatui::widgets::TableState;

use crate::metrics::Metric;
use crate::run::{ResourceSamples, TableData, build_table_data};
use crate::ui::HEADERS;

/// Whether keystrokes drive navigation (`Normal`) or edit the filter query
/// (`Filter`, entered with `/`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Filter,
}

/// Holds all UI state. The raw `samples` are fetched once at startup; every
/// interaction (switching metric, moving the sort column, flipping the order,
/// filtering, navigating rows) recomputes the derived `data` in memory without
/// touching the cluster.
pub struct App {
    samples: Vec<ResourceSamples>,
    pub metric: Metric,
    pub sort_col: usize,
    pub sort_desc: bool,
    pub filter: String,
    pub input_mode: InputMode,
    pub data: TableData,
    pub table_state: TableState,
    /// Number of data rows visible in the table body, updated on each render so
    /// that PageUp/PageDown jump by exactly one screenful.
    page_size: usize,
}

impl App {
    /// Build the initial view from raw samples and select the first row.
    pub fn new(
        samples: Vec<ResourceSamples>,
        metric: Metric,
        sort_col: usize,
        sort_desc: bool,
    ) -> Self {
        let data = build_table_data(&samples, metric, sort_col, sort_desc, "");
        let mut table_state = TableState::default();
        table_state.select_first();
        Self {
            samples,
            metric,
            sort_col,
            sort_desc,
            filter: String::new(),
            input_mode: InputMode::Normal,
            data,
            table_state,
            page_size: 1,
        }
    }

    /// Recompute the derived table for the current metric/sort/filter and reset
    /// the selection to the top (the row under the cursor is no longer
    /// meaningful once the ordering or the visible set changes).
    fn recompute(&mut self) {
        self.data = build_table_data(
            &self.samples,
            self.metric,
            self.sort_col,
            self.sort_desc,
            &self.filter,
        );
        self.table_state.select_first();
    }

    /// Whether the filter input prompt is currently open.
    pub fn is_filtering(&self) -> bool {
        self.input_mode == InputMode::Filter
    }

    /// Whether a non-empty filter is currently applied.
    pub fn has_filter(&self) -> bool {
        !self.filter.is_empty()
    }

    /// Open the filter prompt for editing.
    pub fn start_filter(&mut self) {
        self.input_mode = InputMode::Filter;
    }

    /// Append a character to the filter query and re-filter live.
    pub fn push_filter_char(&mut self, c: char) {
        self.filter.push(c);
        self.recompute();
    }

    /// Delete the last character of the filter query and re-filter live.
    pub fn pop_filter_char(&mut self) {
        self.filter.pop();
        self.recompute();
    }

    /// Accept the current filter and return to navigation, keeping it applied.
    pub fn confirm_filter(&mut self) {
        self.input_mode = InputMode::Normal;
    }

    /// Clear the filter and return to navigation.
    pub fn clear_filter(&mut self) {
        self.input_mode = InputMode::Normal;
        if !self.filter.is_empty() {
            self.filter.clear();
            self.recompute();
        }
    }

    /// Switch between memory and cpu views.
    pub fn toggle_metric(&mut self) {
        self.metric = self.metric.toggled();
        self.recompute();
    }

    /// Move the active sort column one to the right.
    pub fn sort_column_right(&mut self) {
        self.sort_col = (self.sort_col + 1).min(HEADERS.len() - 1);
        self.recompute();
    }

    /// Move the active sort column one to the left.
    pub fn sort_column_left(&mut self) {
        self.sort_col = self.sort_col.saturating_sub(1);
        self.recompute();
    }

    /// Flip ascending/descending on the active sort column.
    pub fn toggle_sort_order(&mut self) {
        self.sort_desc = !self.sort_desc;
        self.recompute();
    }

    pub fn select_next(&mut self) {
        self.table_state.select_next();
    }

    pub fn select_previous(&mut self) {
        self.table_state.select_previous();
    }

    pub fn select_first(&mut self) {
        self.table_state.select_first();
    }

    pub fn select_last(&mut self) {
        self.table_state.select_last();
    }

    /// Record the visible body height so page navigation matches the viewport.
    pub fn set_page_size(&mut self, rows: usize) {
        self.page_size = rows.max(1);
    }

    /// Move the selection down by one screenful.
    pub fn page_down(&mut self) {
        self.table_state.scroll_down_by(self.page_size as u16);
    }

    /// Move the selection up by one screenful.
    pub fn page_up(&mut self) {
        self.table_state.scroll_up_by(self.page_size as u16);
    }
}

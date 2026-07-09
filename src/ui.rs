use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, Cell, HighlightSpacing, Row, Table};

use crate::app::App;
use crate::metrics::{Metric, Statistics};
use crate::run::StatsRow;

/// Color of the filter prompt / active filter indicator.
const FILTER: Color = Color::Indexed(214);

/// Base background for odd (default) rows.
const ROW_BG: Color = Color::Reset;
/// Slightly lighter background for even rows (zebra striping).
const ALT_ROW_BG: Color = Color::Indexed(236);
/// Accent color used for borders and header.
const ACCENT: Color = Color::Cyan;
/// Color used to highlight the selected row (distinct from the header).
const SELECT: Color = Color::Indexed(214);

/// Column headers; the first is the resource, the rest are numeric stats.
pub const HEADERS: [&str; 7] = ["Resource", "min", "max", "mean", "p95", "count", "sum"];

/// Render the whole UI: title bar, table, and key hints.
pub fn render(frame: &mut Frame, app: &mut App) {
    let layout = Layout::vertical([
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
    ]);
    let [top, main, footer] = frame.area().layout(&layout);

    let title = Line::from_iter([
        " kram ".bold().fg(Color::Black).bg(ACCENT),
        Span::from(" Kubernetes pod resource stats").bold(),
    ]);
    frame.render_widget(title, top);

    render_table(frame, main, app);

    // While filtering, the footer becomes the query prompt; otherwise it lists
    // the key hints.
    if app.is_filtering() {
        let prompt = Line::from_iter([
            Span::from("/").fg(FILTER).bold(),
            Span::from(app.filter.clone()).fg(FILTER),
            Span::from("▌").fg(FILTER),
            Span::from("   Enter apply · Esc clear").dim(),
        ]);
        frame.render_widget(prompt, footer);
        return;
    }

    let hints = Line::from_iter([
        Span::from(" ↑/↓ ").fg(Color::Black).bg(ACCENT),
        Span::from(" navigate   "),
        Span::from(" ←/→ ").fg(Color::Black).bg(ACCENT),
        Span::from(" sort column   "),
        Span::from(" s ").fg(Color::Black).bg(ACCENT),
        Span::from(" asc/desc   "),
        Span::from(" m ").fg(Color::Black).bg(ACCENT),
        Span::from(" mem/cpu   "),
        Span::from(" / ").fg(Color::Black).bg(ACCENT),
        Span::from(" filter   "),
        Span::from(" g/G ").fg(Color::Black).bg(ACCENT),
        Span::from(" top/bottom   "),
        Span::from(" q ").fg(Color::Black).bg(ACCENT),
        Span::from(" quit"),
    ]);
    frame.render_widget(hints.dim(), footer);
}

/// Pick an accent color for a resource based on its owner kind.
fn kind_color(resource: &str) -> Color {
    match resource.split('/').next().unwrap_or_default() {
        "deployment" => Color::Green,
        "statefulset" => Color::Blue,
        "daemonset" => Color::Magenta,
        "job" => Color::Yellow,
        _ => Color::Gray,
    }
}

/// Format a `Statistics` into the six numeric cell strings for `metric`.
fn stat_cells(stats: &Statistics, metric: Metric) -> [String; 6] {
    [
        metric.format(stats.min),
        metric.format(stats.max),
        metric.format(stats.mean as u64),
        metric.format(stats.p95),
        stats.count.to_string(),
        metric.format(stats.sum),
    ]
}

/// Build a right-aligned numeric cell.
fn num_cell(value: String) -> Cell<'static> {
    Cell::from(Text::from(value).right_aligned())
}

/// Build a data row with zebra striping and right-aligned numeric cells.
fn data_row(index: usize, row: &StatsRow, metric: Metric) -> Row<'static> {
    let bg = if index.is_multiple_of(2) {
        ROW_BG
    } else {
        ALT_ROW_BG
    };

    let mut cells = vec![Cell::from(row.resource.clone()).fg(kind_color(&row.resource))];
    cells.extend(stat_cells(&row.stats, metric).into_iter().map(num_cell));

    Row::new(cells).style(Style::new().bg(bg))
}

/// Render the statistics table from the current `App` state.
pub fn render_table(frame: &mut Frame, area: Rect, app: &mut App) {
    let metric = app.metric;
    let sort_col = app.sort_col;
    let arrow = if app.sort_desc { " ▼" } else { " ▲" };
    let header = Row::new(HEADERS.iter().enumerate().map(|(i, h)| {
        let label = if i == sort_col {
            format!("{h}{arrow}")
        } else {
            h.to_string()
        };
        let text = Text::from(label);
        let cell = Cell::from(if i == 0 { text } else { text.right_aligned() });
        // Highlight the active sort column in the header, keeping the body calm.
        if i == sort_col {
            cell.style(Style::new().fg(Color::Black).bg(Color::White).bold())
        } else {
            cell
        }
    }))
    .style(
        Style::new()
            .fg(Color::Black)
            .bg(ACCENT)
            .add_modifier(Modifier::BOLD),
    )
    .height(1);

    let rows = app
        .data
        .rows
        .iter()
        .enumerate()
        .map(|(i, row)| data_row(i, row, metric));

    let mut footer_cells = vec![Cell::from("Summary")];
    footer_cells.extend(
        stat_cells(&app.data.summary, metric)
            .into_iter()
            .map(num_cell),
    );
    let footer = Row::new(footer_cells)
        .style(
            Style::new()
                .fg(ACCENT)
                .add_modifier(Modifier::BOLD | Modifier::ITALIC),
        )
        .top_margin(1);

    // Numeric columns must fit the widest formatted value ("1023.99 MiB" = 11
    // chars); right-aligned cells truncate on the left, so a too-narrow column
    // silently drops the most-significant digit.
    let widths = [
        Constraint::Fill(5),
        Constraint::Length(11),
        Constraint::Length(11),
        Constraint::Length(11),
        Constraint::Length(11),
        Constraint::Length(7),
        Constraint::Length(11),
    ];

    let mut block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(ACCENT))
        .title(Line::from(format!(" {} ", metric.title()).bold()).centered())
        .title_bottom(Line::from(format!(" {} workloads ", app.data.rows.len())).right_aligned());

    // Show the active filter on the bottom-left so it stays visible after the
    // prompt closes.
    if app.has_filter() {
        block = block.title_bottom(
            Line::from(format!(" filter: /{} ", app.filter))
                .fg(FILTER)
                .left_aligned(),
        );
    }

    let table = Table::new(rows, widths)
        .header(header)
        .footer(footer)
        .block(block)
        .column_spacing(2)
        .row_highlight_style(Style::new().bg(SELECT).fg(Color::Black).bold())
        .highlight_spacing(HighlightSpacing::Always)
        .highlight_symbol(Span::from(" ▍").fg(SELECT));

    frame.render_stateful_widget(table, area, &mut app.table_state);
}

use clap::Parser;
use crossterm::event::{self, KeyCode};
use kram::app::App;
use kram::command::{Cli, SortOrder};
use kram::run::{collect_raw, sort_by_to_column};
use kram::ui::render;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();
    let sort_col = sort_by_to_column(&cli.sort_by);
    let sort_desc = matches!(cli.sort_order, SortOrder::Desc);

    let samples = collect_raw(cli.namespace, cli.selector)
        .await
        .map_err(|e| color_eyre::eyre::eyre!("{e:#}"))?;

    let mut app = App::new(samples, kram::metrics::Metric::Memory, sort_col, sort_desc);

    ratatui::run(|terminal| {
        loop {
            terminal.draw(|frame| render(frame, &mut app))?;
            let Some(key) = event::read()?.as_key_press_event() else {
                continue;
            };

            // While the filter prompt is open, keystrokes edit the query.
            if app.is_filtering() {
                match key.code {
                    KeyCode::Esc => app.clear_filter(),
                    KeyCode::Enter => app.confirm_filter(),
                    KeyCode::Backspace => app.pop_filter_char(),
                    KeyCode::Char(c) => app.push_filter_char(c),
                    _ => {}
                }
                continue;
            }

            match key.code {
                KeyCode::Char('q') => return Ok(()),
                // Esc clears an active filter first, and only quits otherwise.
                KeyCode::Esc => {
                    if app.has_filter() {
                        app.clear_filter();
                    } else {
                        return Ok(());
                    }
                }
                // Open the filter prompt.
                KeyCode::Char('/') => app.start_filter(),
                KeyCode::Char('j') | KeyCode::Down => app.select_next(),
                KeyCode::Char('k') | KeyCode::Up => app.select_previous(),
                KeyCode::Char('g') => app.select_first(),
                KeyCode::Char('G') => app.select_last(),
                // Move the sort column and re-sort immediately.
                KeyCode::Char('l') | KeyCode::Right => app.sort_column_right(),
                KeyCode::Char('h') | KeyCode::Left => app.sort_column_left(),
                // Toggle ascending / descending on the current sort column.
                KeyCode::Char('s') | KeyCode::Char(' ') | KeyCode::Enter => app.toggle_sort_order(),
                // Switch between memory and cpu views.
                KeyCode::Char('m') => app.toggle_metric(),
                _ => {}
            }
        }
    })
}

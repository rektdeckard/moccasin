use crate::app::{App, LoadState, Tab, View};
use tui::{
    backend::Backend,
    layout::Alignment,
    prelude::*,
    style::{Color, Modifier, Style},
    widgets::{
        scrollbar, Block, BorderType, Borders, Clear, Gauge, List, ListItem, Padding, Paragraph,
        Scrollbar, Tabs, Wrap,
    },
    Frame,
};

pub mod browse;
pub mod detail;
pub mod themed;

/// Renders the user interface widgets.
pub fn render<B: Backend>(app: &mut App, frame: &mut Frame<'_, B>) {
    let wrapper = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(10),
            Constraint::Length(2),
        ])
        .split(frame.size());

    render_tabs_bar(app, frame, wrapper[0]);

    match app.active_tab {
        Tab::Browse => {
            browse::render_browse_area(app, frame, wrapper[1]);
        }
        _ => {}
    }

    render_status_bar(app, frame, wrapper[2]);

    if app.show_keybinds {
        render_keybinds_area(app, frame, frame.size());
    } else if app.should_render_feed_input() {
        render_input_feed_area(app, frame, frame.size())
    }
}

fn render_tabs_bar<B: Backend>(app: &mut App, frame: &mut Frame<'_, B>, area: Rect) {
    let browse = Tab::Browse.to_string().clone();
    let (b, rowse) = browse.split_at(1);
    let b = b.underlined().to_owned();
    let browse = Line::from(vec![b, rowse.reset()]);

    let favorites = Tab::Favorites.to_string().clone();
    let (f, avorites) = favorites.split_at(1);
    let f = f.underlined().to_owned();
    let favorites = Line::from(vec![f, avorites.reset()]);

    let tags = Tab::Tags.to_string().clone();
    let (t, ags) = tags.split_at(1);
    let t = t.underlined().to_owned();
    let tags = Line::from(vec![t, ags.reset()]);

    let tabs = Tabs::new(vec![browse, favorites, tags])
        .block(
            Block::default()
                .style(app.config.theme().status())
                .borders(Borders::BOTTOM)
                .border_style(app.config.theme().active_border()),
        )
        .select(app.active_tab.index_of())
        .highlight_style(app.config.theme().selection());
    frame.render_widget(tabs, area);
}

fn render_keybinds_area<B: Backend>(app: &mut App, frame: &mut Frame<'_, B>, area: Rect) {
    let area = centered_rect_ratio((5, 9), (5, 9), area);

    let block = Block::default()
        .title("Keybinds")
        .borders(Borders::ALL)
        .border_style(app.config.theme().overlay())
        .border_type(BorderType::Plain)
        .style(app.config.theme().overlay())
        .padding(Padding {
            top: 1,
            bottom: 1,
            left: 2,
            right: 2,
        });
    let lines = vec![
        Line::from("j/k    scroll down/up"),
        Line::from("h/l    focus previous/next panel"),
        Line::from("Ent    select current"),
        Line::from("Esc    deselect current"),
        Line::from("Tab    cycle tabs"),
        Line::from("b/f/t  go to Browse/Favorites/Tags tab"),
        Line::from("a      add a feed"),
        Line::from("r      refresh all feeds"),
        Line::from("q      quit"),
        Line::from("o      open feed/item in browser"),
        Line::from(",      open config file in default editor"),
        Line::from("?      toggle this help dialog"),
    ];
    let keybinds = Paragraph::new(lines).block(block);

    frame.render_widget(Clear, area);
    frame.render_widget(keybinds, area);
}

fn render_input_feed_area<B: Backend>(app: &mut App, frame: &mut Frame<'_, B>, area: Rect) {
    let area = centered_rect_sized(60, 3, area);

    let block = Block::default()
        .title("Add feed URL")
        .borders(Borders::ALL)
        .border_style(app.config.theme().overlay())
        .border_type(BorderType::Plain)
        .style(app.config.theme().overlay());

    let input_field = Paragraph::new(app.add_feed_state.input.as_str()).block(block);

    frame.render_widget(Clear, area);
    frame.render_widget(input_field, area);
    frame.set_cursor(
        // Draw the cursor at the current position in the input field.
        // This position is can be controlled via the left and right arrow key
        area.x + app.add_feed_state.cursor_position as u16 + 1,
        // Move one line down, from the border to the input line
        area.y + 1,
    )
}

fn render_status_bar<B: Backend>(app: &mut App, frame: &mut Frame<'_, B>, area: Rect) {
    let block = Block::default()
        .style(app.config.theme().status())
        .borders(Borders::TOP)
        .border_style(app.config.theme().active_border());

    match app.load_state {
        LoadState::Loading(n, count) => {
            if count > 0 {
                frame.render_widget(
                    Gauge::default()
                        .block(block)
                        .ratio(n as f64 / count as f64)
                        .label(format!("Loading {}/{}", n, count))
                        .use_unicode(true)
                        .gauge_style(app.config.theme().status()),
                    area,
                );
            }
        }
        LoadState::Done => {
            let text = match app.current_feed().cloned() {
                Some(feed) => {
                    let mut message = String::from("Last fetched: ");
                    let date = feed.last_fetched().unwrap_or("never").into();
                    message.push_str(date);
                    message
                }
                _ => "[no selection]".to_string(),
            };
            frame.render_widget(
                Paragraph::new(text)
                    .alignment(Alignment::Center)
                    .block(block),
                area,
            );
        }
        LoadState::Errored => {
            frame.render_widget(
                Paragraph::new("ERROR")
                    .alignment(Alignment::Center)
                    .block(block),
                area,
            );
        }
    }
}

fn centered_rect_ratio(ratio_x: (u32, u32), ratio_y: (u32, u32), r: Rect) -> Rect {
    let each_x = (ratio_x.1 - ratio_x.0) / 2;
    let each_y = (ratio_y.1 - ratio_y.0) / 2;

    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Ratio(each_y, ratio_y.1),
                Constraint::Ratio(ratio_y.0, ratio_y.1),
                Constraint::Ratio(each_y, ratio_y.1),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Ratio(each_x, ratio_x.1),
                Constraint::Ratio(ratio_x.0, ratio_x.1),
                Constraint::Ratio(each_x, ratio_x.1),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}

fn centered_rect_sized(width: u16, height: u16, r: Rect) -> Rect {
    let each_x = (r.width - width) / 2;
    let each_y = (r.height - height) / 2;

    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(each_y),
                Constraint::Length(height),
                Constraint::Length(each_y),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Length(each_x),
                Constraint::Length(width),
                Constraint::Length(each_x),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}

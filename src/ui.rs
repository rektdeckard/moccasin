use tui::{
    backend::Backend,
    layout::Alignment,
    prelude::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, List, ListItem, Padding, Paragraph, Wrap},
    Frame,
};

use crate::app::{ActiveView, App};

/// Renders the user interface widgets.
pub fn render<B: Backend>(app: &mut App, frame: &mut Frame<'_, B>) {
    // This is where you add new widgets.
    // See the following resources:
    // - https://docs.rs/ratatui/latest/ratatui/widgets/index.html
    // - https://github.com/ratatui-org/ratatui/tree/master/examples
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Max(40),
                Constraint::Min(60),
                Constraint::Min(60),
            ]
            .as_ref(),
        )
        .split(frame.size());

    let left = Block::default()
        .title("Feeds")
        .title_alignment(Alignment::Center)
        .title_style(Style::default().bg(Color::White).fg(Color::Red))
        .padding(Padding::uniform(1))
        .borders(Borders::ALL)
        .border_style(if app.active_view == ActiveView::Feeds {
            app.config.theme().active_panel()
        } else {
            app.config.theme().inactive_panel()
        })
        .border_type(BorderType::Plain);

    let feeds_list = List::new(
        app.feeds
            .items()
            .iter()
            .map(|channel| ListItem::new(format!("{} ({})", channel.title, channel.items().len())))
            .collect::<Vec<_>>(),
    )
    .block(left)
    .style(app.config.theme().base())
    .highlight_style(if app.active_view == ActiveView::Feeds {
        app.config.theme().active_selection()
    } else {
        app.config.theme().inactive_selection()
    });

    let current_feed = app
        .feeds
        .state
        .selected()
        .and_then(|i| app.feeds.items().get(i));

    if let Some(channel) = current_feed {
        let block = Block::default()
            .title(channel.title())
            .title_alignment(Alignment::Center)
            .padding(Padding::uniform(1))
            .borders(Borders::ALL)
            .border_style(if app.active_view == ActiveView::Items {
                app.config.theme().active_panel()
            } else {
                app.config.theme().inactive_panel()
            })
            .border_type(BorderType::Plain);

        let items_list = List::new(
            channel
                .items()
                .iter()
                .map(|item| {
                    let title = item.title.clone().unwrap_or("default".into());
                    ListItem::new(title)
                })
                .collect::<Vec<_>>(),
        )
        .block(block)
        .style(app.config.theme().base())
        .highlight_style(if app.active_view == ActiveView::Items {
            app.config.theme().active_selection()
        } else {
            app.config.theme().inactive_selection()
        });

        frame.render_stateful_widget(items_list, chunks[1], &mut app.items.state);

        if let Some(detail) = app.current_item() {
            let block = Block::default()
                .title("Detail")
                .title_alignment(Alignment::Center)
                .padding(Padding::uniform(1))
                .style(app.config.theme().base())
                .borders(Borders::ALL)
                .border_style(if app.active_view == ActiveView::Detail {
                    app.config.theme().active_panel()
                } else {
                    app.config.theme().inactive_panel()
                });
            let detail = Paragraph::new(detail.description().unwrap_or("EMPTY"))
                .wrap(Wrap { trim: true })
                .block(block);

            frame.render_widget(detail, chunks[2]);
        }
    } else {
        frame.render_widget(
            Block::default()
                .title("Items")
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .style(app.config.theme().base()),
            chunks[1],
        );
    }

    frame.render_stateful_widget(feeds_list, chunks[0], &mut app.feeds.state);
}

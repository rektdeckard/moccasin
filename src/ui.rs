use tui::{
    backend::Backend,
    layout::Alignment,
    prelude::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{
        scrollbar, Block, BorderType, Borders, List, ListItem, Padding, Paragraph, Scrollbar, Wrap,
    },
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
        .title_alignment(Alignment::Left)
        .title_style(Style::default().bg(Color::White).fg(Color::Red))
        .padding(Padding::uniform(1))
        .borders(Borders::ALL)
        .border_style(if app.active_view == ActiveView::Feeds {
            app.config.theme().active_border()
        } else {
            app.config.theme().border()
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
        app.config.theme().selection()
    });

    let current_feed = app
        .feeds
        .state
        .selected()
        .and_then(|i| app.feeds.items().get(i));

    if let Some(channel) = current_feed {
        let block = Block::default()
            .title(channel.title())
            .title_alignment(Alignment::Left)
            .padding(Padding::uniform(1))
            .borders(Borders::ALL)
            .border_style(if app.active_view == ActiveView::Items {
                app.config.theme().active_border()
            } else {
                app.config.theme().border()
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
            app.config.theme().selection()
        });

        frame.render_stateful_widget(items_list, chunks[1], &mut app.items.state);

        let block = Block::default()
            .title("Detail")
            .title_alignment(Alignment::Left)
            .padding(Padding::uniform(1))
            .style(app.config.theme().base())
            .borders(Borders::ALL)
            .border_style(if app.active_view == ActiveView::Detail {
                app.config.theme().active_border()
            } else {
                app.config.theme().border()
            });

        if let Some(detail) = &app.current_item() {
            frame.render_widget(block, chunks[2]);

            let content_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(1),
                    Constraint::Min(1),
                    Constraint::Min(1),
                    Constraint::Length(1),
                    Constraint::Min(1),
                ])
                .margin(2)
                .split(chunks[2]);

            let title = Paragraph::new(detail.title().unwrap_or("EMPTY"))
                .style(Style::default().add_modifier(Modifier::ITALIC))
                .wrap(Wrap { trim: true })
                .alignment(Alignment::Center);

            let author = Paragraph::new(
                detail
                    .author()
                    .and_then(|s| Some(s.to_owned()))
                    .or(detail
                        .itunes_ext()
                        .and_then(|it| it.author().and_then(|auth| Some(auth.to_owned()))))
                    .or(detail.dublin_core_ext().and_then(|dc| {
                        let creators = dc.creators().join(", ");
                        if creators.is_empty() {
                            None
                        } else {
                            Some(creators)
                        }
                    }))
                    .unwrap_or("[anonymous]".to_owned()),
            )
            .alignment(Alignment::Center);

            let date = Paragraph::new(detail.pub_date().unwrap_or("")).alignment(Alignment::Center);

            let body = Paragraph::new(detail.description().unwrap_or("EMPTY"))
                .wrap(Wrap { trim: true })
                .block(Block::default().padding(Padding {
                    top: 0,
                    bottom: 0,
                    left: 1,
                    right: 2,
                }))
                .scroll((app.detail_scroll_index, 0));

            frame.render_widget(title, content_chunks[0]);
            frame.render_widget(author, content_chunks[1]);
            frame.render_widget(date, content_chunks[2]);
            frame.render_widget(
                Block::default()
                    .borders(Borders::TOP)
                    .border_style(Style::default().add_modifier(Modifier::DIM))
                    .padding(Padding::vertical(1)),
                content_chunks[3],
            );
            frame.render_widget(body, content_chunks[4]);

            app.detail_scroll = app.detail_scroll.content_length(48);
            frame.render_stateful_widget(
                Scrollbar::default()
                    .begin_symbol(None)
                    .end_symbol(None)
                    .track_symbol(scrollbar::VERTICAL.thumb)
                    .track_style(Style::default().add_modifier(Modifier::DIM)),
                content_chunks[4],
                &mut app.detail_scroll,
            );
        }
    } else {
        frame.render_widget(
            Block::default()
                .title("Items")
                .title_alignment(Alignment::Left)
                .borders(Borders::ALL)
                .style(app.config.theme().base()),
            chunks[1],
        );
        frame.render_widget(
            Block::default()
                .title("Details")
                .title_alignment(Alignment::Left)
                .borders(Borders::ALL)
                .style(app.config.theme().base()),
            chunks[2],
        );
    }

    frame.render_stateful_widget(feeds_list, chunks[0], &mut app.feeds.state);
}

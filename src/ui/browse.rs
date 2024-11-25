use crate::app::{App, View};
use tui::{
    layout::Alignment,
    prelude::*,
    style::{Color, Modifier, Style},
    symbols::scrollbar,
    widgets::{Block, BorderType, Borders, List, ListItem, Padding, Paragraph, Scrollbar, Wrap},
    Frame,
};

pub fn render_browse_area(app: &mut App, frame: &mut Frame<'_>, area: Rect) {
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
        .split(area);

    let left = Block::default()
        .title_alignment(Alignment::Left)
        .title_style(Style::default().bg(Color::White).fg(Color::Red));

    let feeds_list = List::new(
        app.feeds
            .items()
            .iter()
            .map(|feed| ListItem::new(format!("{} ({})", feed.title(), feed.items().len())))
            .collect::<Vec<_>>(),
    )
    .block(left)
    .style(app.config.theme().base())
    .highlight_style(if app.active_view == View::MainList {
        app.config.theme().active_selection()
    } else {
        app.config.theme().selection()
    });

    let current_feed = app
        .feeds
        .state
        .selected()
        .and_then(|i| app.feeds.items().get(i));

    if let Some(feed) = current_feed {
        let block = Block::default().borders(Borders::LEFT);

        let items_list = List::new(
            feed.items()
                .iter()
                .map(|item| {
                    let title = item.title().clone().unwrap_or("default".into());
                    ListItem::new(title)
                })
                .collect::<Vec<_>>(),
        )
        .block(block)
        .style(app.config.theme().base())
        .highlight_style(if app.active_view == View::SubList {
            app.config.theme().active_selection()
        } else {
            app.config.theme().selection()
        });

        if app.current_item().is_some() {
            frame.render_stateful_widget(items_list, chunks[1], &mut app.items.state);
            if app.should_render_items_scroll() {
                frame.render_stateful_widget(
                    Scrollbar::default()
                        .begin_symbol(None)
                        .end_symbol(None)
                        .track_symbol(Some(scrollbar::VERTICAL.thumb))
                        .track_style(app.config.theme().scrollbar_track())
                        .thumb_style(app.config.theme().scrollbar_thumb()),
                    chunks[1].inner(Margin {
                        vertical: 1,
                        horizontal: 1,
                    }),
                    &mut app.items_scroll,
                );
            }
        } else {
            frame.render_stateful_widget(
                items_list,
                chunks[1].union(chunks[2]),
                &mut app.items.state,
            );
            if app.should_render_items_scroll() {
                frame.render_stateful_widget(
                    Scrollbar::default()
                        .begin_symbol(None)
                        .end_symbol(None)
                        .track_symbol(Some(scrollbar::VERTICAL.thumb))
                        .track_style(app.config.theme().scrollbar_track())
                        .thumb_style(app.config.theme().scrollbar_thumb()),
                    chunks[1].union(chunks[2]).inner(Margin {
                        vertical: 1,
                        horizontal: 1,
                    }),
                    &mut app.items_scroll,
                );
            }
        }

        if let Some(detail) = &app.current_item() {
            let block = Block::default()
                .style(app.config.theme().base())
                .padding(Padding {
                    left: 1,
                    right: if app.should_render_detail_scroll() {
                        2
                    } else {
                        1
                    },
                    ..Default::default()
                })
                .borders(Borders::LEFT);

            frame.render_widget(block, chunks[2]);

            let content_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(3), Constraint::Fill(10)])
                .horizontal_margin(2)
                .split(chunks[2]);

            let title = Line::from(detail.title().unwrap_or("[untitled]"))
                .style(Style::default().add_modifier(Modifier::ITALIC));
            let author = Line::from(format!("by {}", detail.author().unwrap_or("[anonymous]")));
            let date = Line::from(detail.pub_date().unwrap_or("[no date]"));

            let body = Paragraph::new(detail.description().unwrap_or("[no content]"))
                .wrap(Wrap { trim: true })
                .scroll((app.detail_scroll_index, 0));

            let metadata = Paragraph::new(vec![title, author, date])
                .wrap(Wrap { trim: true })
                .block(Block::default().borders(Borders::BOTTOM));

            frame.render_widget(metadata, content_chunks[0]);
            frame.render_widget(body, content_chunks[1]);

            app.detail_scroll = app.detail_scroll.content_length(48);
            if app.should_render_detail_scroll() {
                frame.render_stateful_widget(
                    Scrollbar::default()
                        .begin_symbol(None)
                        .end_symbol(None)
                        .track_symbol(Some(scrollbar::VERTICAL.thumb))
                        .track_style(app.config.theme().scrollbar_track())
                        .thumb_style(app.config.theme().scrollbar_thumb()),
                    content_chunks[4],
                    &mut app.detail_scroll,
                );
            }
        }

        frame.render_stateful_widget(feeds_list, chunks[0], &mut app.feeds.state);
        if app.should_render_feeds_scroll() {
            frame.render_stateful_widget(
                Scrollbar::default()
                    .begin_symbol(None)
                    .end_symbol(None)
                    .track_symbol(Some(scrollbar::VERTICAL.thumb))
                    .track_style(app.config.theme().scrollbar_track())
                    .thumb_style(app.config.theme().scrollbar_thumb()),
                chunks[0].inner(Margin {
                    vertical: 1,
                    horizontal: 1,
                }),
                &mut app.feeds_scroll,
            );
        }
    } else {
        frame.render_stateful_widget(feeds_list, area, &mut app.feeds.state);
        if app.should_render_feeds_scroll() {
            frame.render_stateful_widget(
                Scrollbar::default()
                    .begin_symbol(None)
                    .end_symbol(None)
                    .track_symbol(Some(scrollbar::VERTICAL.thumb))
                    .track_style(app.config.theme().scrollbar_track())
                    .thumb_style(app.config.theme().scrollbar_thumb()),
                area.inner(Margin {
                    vertical: 1,
                    horizontal: 1,
                }),
                &mut app.feeds_scroll,
            );
        }
    }
}

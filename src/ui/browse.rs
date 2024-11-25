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
        .title("Feeds")
        .title_alignment(Alignment::Left)
        .title_style(Style::default().bg(Color::White).fg(Color::Red))
        .padding(if app.should_render_feeds_scroll() {
            Padding {
                top: 1,
                bottom: 1,
                left: 1,
                right: 2,
            }
        } else {
            Padding::uniform(1)
        })
        .borders(Borders::ALL)
        .border_style(if app.active_view == View::MainList {
            app.config.theme().active_border()
        } else {
            app.config.theme().border()
        })
        .border_type(BorderType::Plain);

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
        let block = Block::default()
            .title(feed.title())
            .title_alignment(Alignment::Left)
            .padding(if app.should_render_items_scroll() {
                Padding {
                    top: 1,
                    bottom: 1,
                    left: 1,
                    right: 2,
                }
            } else {
                Padding::uniform(1)
            })
            .borders(Borders::ALL)
            .border_style(if app.active_view == View::SubList {
                app.config.theme().active_border()
            } else {
                app.config.theme().border()
            })
            .border_type(BorderType::Plain);

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
                .title("Detail")
                .title_alignment(Alignment::Left)
                .padding(Padding::uniform(1))
                .style(app.config.theme().base())
                .borders(Borders::ALL)
                .border_style(if app.active_view == View::Detail {
                    app.config.theme().active_border()
                } else {
                    app.config.theme().border()
                });

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

            let title = Paragraph::new(detail.title().unwrap_or("[no title]"))
                .style(Style::default().add_modifier(Modifier::ITALIC))
                .wrap(Wrap { trim: true })
                .alignment(Alignment::Center);

            let author = Paragraph::new(detail.author().unwrap_or("[anonymous]"))
                .alignment(Alignment::Center);

            let date = Paragraph::new(detail.pub_date().unwrap_or("[no date]"))
                .alignment(Alignment::Center);

            let body = Paragraph::new(detail.description().unwrap_or("[no content]"))
                .wrap(Wrap { trim: true })
                .block(Block::default().padding(Padding {
                    top: 0,
                    bottom: 0,
                    left: 1,
                    right: if app.should_render_detail_scroll() {
                        2
                    } else {
                        1
                    },
                }))
                .scroll((app.detail_scroll_index, 0));

            frame.render_widget(title, content_chunks[0]);
            frame.render_widget(author, content_chunks[1]);
            frame.render_widget(date, content_chunks[2]);
            frame.render_widget(
                Block::default()
                    .borders(Borders::TOP)
                    .border_style(app.config.theme().border())
                    .padding(Padding::vertical(1)),
                content_chunks[3],
            );
            frame.render_widget(body, content_chunks[4]);

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

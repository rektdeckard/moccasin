use html_parser::Dom;
use tui::layout::Alignment;
use tui::prelude::*;
use tui::style::Style;
use tui::widgets::{Block, Wrap};

fn get_line_offset(line_width: u16, text_area_width: u16, alignment: Alignment) -> u16 {
    match alignment {
        Alignment::Center => (text_area_width / 2).saturating_sub(line_width / 2),
        Alignment::Right => text_area_width.saturating_sub(line_width),
        Alignment::Left => 0,
    }
}

struct HTML<'a> {
    /// A block to wrap the widget in
    block: Option<Block<'a>>,
    /// Widget style
    style: Style,
    /// How to wrap the text
    wrap: Option<Wrap>,
    /// The text to display
    text: Text<'a>,
    /// Scroll
    scroll: (u16, u16),
    /// Alignment of the text
    alignment: Alignment,
}

impl<'a> HTML<'a> {
    fn from_dom(fragment: Dom) -> Self {
        // let text: Vec<Span<'a>> = unimplemented!();
        let text = Text::raw("asdd\n\nsdfae\nasdfasd");

        Self {
            block: None,
            style: Style::default(),
            wrap: None,
            text: text.into(),
            scroll: (0, 0),
            alignment: Alignment::Left,
        }
    }

    pub fn block(mut self, block: Block<'a>) -> HTML<'a> {
        self.block = Some(block);
        self
    }

    pub fn style(mut self, style: Style) -> HTML<'a> {
        self.style = style;
        self
    }

    pub fn wrap(mut self, wrap: Wrap) -> HTML<'a> {
        self.wrap = Some(wrap);
        self
    }

    pub fn scroll(mut self, offset: (u16, u16)) -> HTML<'a> {
        self.scroll = offset;
        self
    }

    pub fn alignment(mut self, alignment: Alignment) -> HTML<'a> {
        self.alignment = alignment;
        self
    }
}

// impl<'a> Widget for HTML<'a> {
//     fn render(mut self, area: Rect, buf: &mut Buffer) {
//         buf.set_style(area, self.style);
//         let text_area = match self.block.take() {
//             Some(b) => {
//                 let inner_area = b.inner(area);
//                 b.render(area, buf);
//                 inner_area
//             }
//             None => area,
//         };

//         if text_area.height < 1 {
//             return;
//         }

//         let style = self.style;
//         let styled = self.text.lines.iter().map(|line| {
//             (
//                 line.spans
//                     .iter()
//                     .flat_map(|span| span.styled_graphemes(style)),
//                 line.alignment.unwrap_or(self.alignment),
//             )
//         });

//         let mut line_composer: Box<dyn LineComposer> = if let Some(Wrap { trim }) = self.wrap {
//             Box::new(WordWrapper::new(styled, text_area.width, trim))
//         } else {
//             let mut line_composer = Box::new(LineTruncator::new(styled, text_area.width));
//             line_composer.set_horizontal_offset(self.scroll.1);
//             line_composer
//         };
//         let mut y = 0;
//         while let Some((current_line, current_line_width, current_line_alignment)) =
//             line_composer.next_line()
//         {
//             if y >= self.scroll.0 {
//                 let mut x =
//                     get_line_offset(current_line_width, text_area.width, current_line_alignment);
//                 for StyledGrapheme { symbol, style } in current_line {
//                     let width = symbol.width();
//                     if width == 0 {
//                         continue;
//                     }
//                     buf.get_mut(text_area.left() + x, text_area.top() + y - self.scroll.0)
//                         .set_symbol(if symbol.is_empty() {
//                             // If the symbol is empty, the last char which rendered last time will
//                             // leave on the line. It's a quick fix.
//                             " "
//                         } else {
//                             symbol
//                         })
//                         .set_style(*style);
//                     x += width as u16;
//                 }
//             }
//             y += 1;
//             if y >= text_area.height + self.scroll.0 {
//                 break;
//             }
//         }
//     }
// }

pub fn p(input: &str) -> Result<Dom, html_parser::Error> {
    Dom::parse(input)

    // Ok(Dom {
    //     tree_type: DomVariant::DocumentFragment,
    //     children: [
    //         Element(Element {
    //             id: None,
    //             name: "p",
    //             variant: Normal,
    //             attributes: {},
    //             classes: [],
    //             children: [
    //                 Text("We recently launched a new feature at Buffer, called "),
    //                 Element(Element {
    //                     id: None, name: "a",
    //                     variant: Normal,
    //                     attributes: {
    //                         "href": Some("https://buffer.com/ideas?ref=buffer.com"),
    //                         "rel": Some("noreferrer nofollow noopener")
    //                     },
    //                     classes: [],
    //                     children: [Text("Ideas")],
    //                     source_span: SourceSpan {
    //                         text: "<a href=\"https://buffer.com/ideas?ref=buffer.com\" rel=\"noreferrer nofollow noopener\">Ideas</a>",
    //                         start_line: 1,
    //                         end_line: 1,
    //                         start_column: 57,
    //                         end_column: 151
    //                     }
    //                 }),
    //                 Text(". With Ideas, you can store all your best ideas, tweak them until they&#x2019;re ready, and drop them straight into your Buffer queue. Now that Ideas has launched in our web and mobile apps, we have some time to")
    //             ],
    //             source_span: SourceSpan {
    //                 text: "<p>We recently launched a new feature at Buffer, called <a href=\"https://buffer.com/ideas?ref=buffer.com\" rel=\"noreferrer nofollow noopener\">Ideas</a>. With Ideas, you can store all your best ideas, tweak them until they&#x2019;re ready, and drop them straight into your Buffer queue. Now that Ideas has launched in our web and mobile apps, we have some time to</p>",
    //                 start_line: 1,
    //                 end_line: 1,
    //                 start_column: 1,
    //                 end_column: 366
    //             }
    //         })
    //     ],
    //     errors: []
    // })
}

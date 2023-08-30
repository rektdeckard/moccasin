use tui::style::Styled;
use tui::widgets::Block;

pub trait Themed<'a, T>
where
    T: Styled,
{
    fn theme(self) -> Self;
}

impl<'a> Themed<'a, Block<'_>> for Block<'a> {
    fn theme(self) -> Block<'a> {
        self
    }
}

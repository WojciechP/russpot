//! Module navigation defines common messages for keyboard navigation.
//! These are understood by multiple different layout-like components.

use crate::spotconn::model::SpotItem;

#[derive(Debug, Clone, Copy)]
pub enum NavCommand {
    Up,
    Down,
    Left,
    Right,
    ClearCursor,
}

#[derive(Debug, Clone)]
pub enum NavOutput {
    EscapedUp,
    EscapedDown,
    EscapedLeft,
    EscapedRight,
    CursorIsNowAt(SpotItem),
}

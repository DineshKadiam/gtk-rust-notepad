pub mod buffer;
pub mod completion;
pub mod dialog;
pub mod gmark;
pub mod notifications;
pub mod sidebar;
pub mod terminal;
pub mod welcome;
pub mod window;

/// Iter to LSP position
#[macro_export]
macro_rules! iter_position {
    ($iter: expr) => {
        Position {
            line: $iter.line() as u32,
            character: $iter.line_offset() as u32,
        }
    };
}

/// LSP position to iter
#[macro_export]
macro_rules! position_iter {
    ($buf: expr, $pos: expr) => {
        $buf.iter_at_line_offset(
            $pos.line.try_into().unwrap(),
            $pos.character.try_into().unwrap(),
        )
    };
}

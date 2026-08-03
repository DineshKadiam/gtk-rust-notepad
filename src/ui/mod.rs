pub mod buffer;
pub mod completion;
pub mod dialog;
pub mod notifications;
pub mod preferences;
pub mod recents;
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

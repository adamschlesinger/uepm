use crossterm::{
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
};
use std::io::stdout;

fn print_prefixed(color: Color, prefix: &str, msg: &str) {
    let _ = execute!(
        stdout(),
        SetForegroundColor(color),
        Print(prefix),
        ResetColor,
        Print(msg),
        Print("\n"),
    );
}

/// Print a green success message prefixed with ✓.
pub fn print_success(msg: &str) {
    print_prefixed(Color::Green, "✓ ", msg);
}

/// Print a yellow warning message prefixed with ⚠.
pub fn print_warn(msg: &str) {
    print_prefixed(Color::Yellow, "⚠ ", msg);
}

/// Print a red error message prefixed with ✗.
pub fn print_error(msg: &str) {
    print_prefixed(Color::Red, "✗ ", msg);
}

/// Print a cyan informational message prefixed with ℹ.
pub fn print_info(msg: &str) {
    print_prefixed(Color::Cyan, "ℹ ", msg);
}

use yansi::Paint;

/// Utility to format prompt strings as green and bold. Use yansi directly instead for custom colors.
pub fn paint_green_bold(input: &str) -> String {
    Paint::green(input).bold().to_string()
}

/// Utility to format prompt strings as yellow and bold. Use yansi directly instead for custom colors.
pub fn paint_yellow_bold(input: &str) -> String {
    Paint::yellow(input).bold().to_string()
}

use chrono::{DateTime, Utc};

pub fn time_diff_text(from: DateTime<Utc>) -> String {
    let now = Utc::now();
    let duration = now - from;

    let days = duration.num_days();
    let hours = duration.num_hours() % 24;
    let minutes = duration.num_minutes() % 60;
    let seconds = duration.num_seconds() % 60;

    match (days, hours, minutes) {
        (d, _, _) if d > 0 => format!(
            "{} days, {} hours, {} minutes, and {} seconds",
            days, hours, minutes, seconds
        ),
        (_, h, _) if h > 0 => format!(
            "{} hours, {} minutes, and {} seconds",
            hours, minutes, seconds
        ),
        (_, _, m) if m > 0 => format!("{} minutes and {} seconds", minutes, seconds),
        _ => format!("{} seconds", seconds),
    }
}

pub fn escape_markdown_v2(text: &str) -> String {
    let mut escaped_text = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '_' | '*' | '[' | ']' | '(' | ')' | '~' | '`' | '>' | '<' | '#' | '+' | '-' | '='
            | '|' | '{' | '}' | '.' | '!' => {
                escaped_text.push('\\');
                escaped_text.push(c);
            }
            _ => escaped_text.push(c),
        }
    }
    escaped_text
}

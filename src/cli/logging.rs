use colored::*;
use log::{Level, Record};
use std::io::Write;

pub(crate) fn formatter(
    buf: &mut env_logger::fmt::Formatter,
    record: &Record,
) -> Result<(), std::io::Error> {
    match record.level() {
        Level::Error => writeln!(buf, "{}: {}", "Error".red(), record.args()),
        Level::Warn => writeln!(buf, "{}: {}", "Warning".yellow(), record.args()),
        Level::Info => writeln!(buf, "{}", record.args()),
        Level::Debug => {
            let mut path = String::new();
            if let Some(file) = record.file() {
                path.push_str(&format!(" [{}", file));

                if let Some(line) = record.line() {
                    path.push_str(&format!(":{}", line));
                }

                path.push(']');
            }

            writeln!(buf, "{}{}: {}", "Debug".blue(), path.blue(), record.args())
        }
        Level::Trace => {
            let mut path = String::new();
            if let Some(file) = record.file() {
                path.push_str(&format!(" [{}", file));

                if let Some(line) = record.line() {
                    path.push_str(&format!(":{}", line));
                }

                path.push(']');
            }

            writeln!(
                buf,
                "{}{}: {}",
                "Trace".dimmed(),
                path.dimmed(),
                record.args()
            )
        }
    }
}

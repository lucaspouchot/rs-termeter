use std::fs;
use std::io::{Result, BufRead, stdin, BufReader};

/// Parse a file into columns of f64 values.
/// Supports 1 or 2 columns separated by spaces, commas, or semicolons.
/// Returns a Vec of columns (1 or 2 Vec<f64>).
pub fn read_values(path: &str) -> Result<Vec<Vec<f64>>> {
    let reader: Box<dyn BufRead> = if path == "-" {
        Box::new(stdin().lock())
    } else {
        let file = fs::File::open(path)?;
        Box::new(BufReader::new(file))
    };

    let mut columns: Vec<Vec<f64>> = Vec::new();
    for (line_num, line) in reader.lines().enumerate() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
            continue;
        }
        // Split by comma, semicolon, or whitespace
        let parts: Vec<&str> = trimmed
            .split(|c: char| c == ',' || c == ';' || c.is_ascii_whitespace())
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        let mut parsed = Vec::new();
        let mut ok = true;
        for part in &parts {
            match part.parse::<f64>() {
                Ok(v) => parsed.push(v),
                Err(_) => {
                    eprintln!(
                        "warning: skipping line {} (not a number): {:?}",
                        line_num + 1,
                        truncate(trimmed, 40),
                    );
                    ok = false;
                    break;
                }
            }
        }
        if !ok || parsed.is_empty() {
            continue;
        }

        // Initialize columns on first data line
        if columns.is_empty() {
            columns.resize(parsed.len(), Vec::new());
        }

        // If column count doesn't match, warn and skip
        if parsed.len() != columns.len() {
            eprintln!(
                "warning: skipping line {} (expected {} columns, got {})",
                line_num + 1,
                columns.len(),
                parsed.len(),
            );
            continue;
        }

        for (i, v) in parsed.into_iter().enumerate() {
            columns[i].push(v);
        }
    }
    Ok(columns)
}

pub fn truncate(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}…", &s[..max])
    } else {
        s.to_string()
    }
}
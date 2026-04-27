use crossterm::terminal;

pub struct PieSlice {
    pub name: String,
    pub value: f64,
}

pub struct Pie {
    pub slices: Vec<PieSlice>,
    pub diameter_cells: usize,
}

/// Foreground ANSI color codes used for pie slices.
const FG_COLORS: &[u8] = &[31, 32, 33, 34, 35, 36, 91, 92, 93, 94, 95, 96];
/// Background ANSI color codes (parallel to FG_COLORS).
const BG_COLORS: &[u8] = &[41, 42, 43, 44, 45, 46, 101, 102, 103, 104, 105, 106];

impl Pie {
    pub fn new(slices: Vec<PieSlice>) -> Self {
        let (term_w, term_h) = terminal::size().unwrap_or((80, 24));
        // The terminal cell aspect ratio is ~1:2 (w:h). One cell wide = 1 horizontal sub-pixel,
        // one cell tall = 2 vertical sub-pixels. We size the pie to fit in the terminal while
        // leaving room for a legend on the right and some margins.
        // Reserve up to 32 cols on the right for legend (we'll compute exact later, this is just
        // the rendering size of the pie itself).
        let max_w_cells = (term_w as usize).saturating_sub(36).max(20);
        // Height in cells we can use (reserve a few rows for title/blank lines/legend overflow).
        let max_h_cells = (term_h as usize).saturating_sub(4).max(10);
        // Diameter in cells (horizontal). Pie height in cells = diameter / 2 (since 2 sub-pixels per cell).
        let mut diameter_cells = max_w_cells.min(max_h_cells * 2);
        if diameter_cells < 10 {
            diameter_cells = 10;
        }
        // Make even for symmetry.
        if diameter_cells % 2 != 0 {
            diameter_cells -= 1;
        }
        Self {
            slices,
            diameter_cells,
        }
    }

    /// Total of all slice values.
    fn total(&self) -> f64 {
        self.slices.iter().map(|s| s.value).sum()
    }

    /// Render the pie chart as a string, including a legend on the right.
    pub fn render(&self) -> String {
        let total = self.total();
        if total <= 0.0 {
            return "  (no positive values to plot)\n".to_string();
        }

        // Cumulative angle boundaries (in radians) for each slice. Start at top (12 o'clock),
        // proceed clockwise.
        let mut bounds: Vec<f64> = Vec::with_capacity(self.slices.len() + 1);
        bounds.push(0.0);
        let mut acc = 0.0;
        for s in &self.slices {
            acc += (s.value.max(0.0)) / total * std::f64::consts::TAU;
            bounds.push(acc);
        }

        // Pie geometry (in sub-pixel units; horizontal: 1 per cell, vertical: 2 per cell).
        let d = self.diameter_cells;
        let radius = d as f64 / 2.0;
        let cx = radius - 0.5;
        let cy = radius - 0.5;
        let cells_h = d / 2; // rows of cells

        // Build a 2D color grid in sub-pixel space: width = d, height = d.
        // None = outside the circle.
        let mut grid: Vec<Vec<Option<usize>>> = vec![vec![None; d]; d];
        for sy in 0..d {
            for sx in 0..d {
                let dx = sx as f64 - cx;
                let dy = sy as f64 - cy;
                if dx * dx + dy * dy <= radius * radius {
                    // angle: 0 at top (12 o'clock), increasing clockwise, in [0, TAU)
                    let mut angle = (dx).atan2(-dy); // atan2(x, -y) -> 0 at top, +pi/2 at right
                    if angle < 0.0 {
                        angle += std::f64::consts::TAU;
                    }
                    // Find slice index: smallest i such that bounds[i+1] >= angle
                    let mut idx = self.slices.len() - 1;
                    for i in 0..self.slices.len() {
                        if angle < bounds[i + 1] {
                            idx = i;
                            break;
                        }
                    }
                    grid[sy][sx] = Some(idx);
                }
            }
        }

        // Pre-compute legend lines.
        let legend = self.build_legend(total);

        // Render row by row. Each terminal row covers 2 sub-pixel rows (upper and lower).
        let mut out = String::new();
        let reset = "\x1b[0m";
        for row in 0..cells_h {
            let y_upper = row * 2;
            let y_lower = row * 2 + 1;
            // Left margin
            out.push_str("  ");
            for x in 0..d {
                let upper = grid[y_upper][x];
                let lower = grid[y_lower][x];
                match (upper, lower) {
                    (None, None) => out.push(' '),
                    (Some(i), None) => {
                        let fg = FG_COLORS[i % FG_COLORS.len()];
                        out.push_str(&format!("\x1b[{}m▀{}", fg, reset));
                    }
                    (None, Some(j)) => {
                        let fg = FG_COLORS[j % FG_COLORS.len()];
                        out.push_str(&format!("\x1b[{}m▄{}", fg, reset));
                    }
                    (Some(i), Some(j)) if i == j => {
                        let fg = FG_COLORS[i % FG_COLORS.len()];
                        out.push_str(&format!("\x1b[{}m█{}", fg, reset));
                    }
                    (Some(i), Some(j)) => {
                        let fg = FG_COLORS[i % FG_COLORS.len()];
                        let bg = BG_COLORS[j % BG_COLORS.len()];
                        out.push_str(&format!("\x1b[{};{}m▀{}", fg, bg, reset));
                    }
                }
            }
            // Append legend entry on this row (if any)
            if let Some(line) = legend.get(row) {
                out.push_str("   ");
                out.push_str(line);
            }
            out.push('\n');
        }

        // If legend has more lines than rows, print remaining lines below.
        if legend.len() > cells_h {
            let pad = " ".repeat(2 + d + 3);
            for line in &legend[cells_h..] {
                out.push_str(&pad);
                out.push_str(line);
                out.push('\n');
            }
        }

        out
    }

    fn build_legend(&self, total: f64) -> Vec<String> {
        let reset = "\x1b[0m";
        let max_name_len = self
            .slices
            .iter()
            .map(|s| s.name.chars().count())
            .max()
            .unwrap_or(0);
        self.slices
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let fg = FG_COLORS[i % FG_COLORS.len()];
                let pct = s.value.max(0.0) / total * 100.0;
                format!(
                    "\x1b[{}m██{} {:<width$}  {:>6.2}%  ({})",
                    fg,
                    reset,
                    s.name,
                    pct,
                    format_value(s.value),
                    width = max_name_len
                )
            })
            .collect()
    }
}

fn format_value(v: f64) -> String {
    let a = v.abs();
    if a == 0.0 {
        "0".to_string()
    } else if (0.01..100_000.0).contains(&a) {
        format!("{:.2}", v)
    } else {
        format!("{:.3e}", v)
    }
}

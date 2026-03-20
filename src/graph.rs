use crossterm::terminal;

const BRAILLE_OFFSET: u32 = 0x2800;
// Braille dot mapping for a 2x4 cell:
//   dot1 (0,0) = 0x01   dot4 (1,0) = 0x08
//   dot2 (0,1) = 0x02   dot5 (1,1) = 0x10
//   dot3 (0,2) = 0x04   dot6 (1,2) = 0x20
//   dot7 (0,3) = 0x40   dot8 (1,3) = 0x80
const BRAILLE_MAP: [[u32; 4]; 2] = [
    [0x01, 0x02, 0x04, 0x40],
    [0x08, 0x10, 0x20, 0x80],
];

pub struct Series {
    pub data: Vec<f64>,
    pub name: String,
}

pub struct Graph {
    pub series: Vec<Series>,
    pub width: usize,
    pub height: usize,
    pub global_min: f64,
    pub global_max: f64,
    pub dual_axis: bool,
}

impl Graph {
    pub fn new(series: Vec<Series>, dual_axis: bool) -> Self {
        let (term_width, term_height) = terminal::size().unwrap_or((80, 24));
        // In dual mode, reserve space for right axis labels too
        let right_margin = if dual_axis { 12 } else { 1 };
        let width = (term_width as usize).saturating_sub(12 + right_margin).max(10);
        let reserve = 6 + series.len() * 3;
        let height = (term_height as usize).saturating_sub(reserve).max(4);

        let global_min = series
            .iter()
            .flat_map(|s| s.data.iter())
            .cloned()
            .fold(f64::INFINITY, f64::min);
        let global_max = series
            .iter()
            .flat_map(|s| s.data.iter())
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max);

        Self {
            series,
            width,
            height,
            global_min,
            global_max,
            dual_axis,
        }
    }

    pub fn render(&self) -> String {
        let mut output = String::new();

        let pixel_w = self.width * 2;
        let pixel_h = self.height * 4;

        // Per-series min/max/range (used in dual mode)
        let series_stats: Vec<(f64, f64, f64)> = self
            .series
            .iter()
            .map(|s| {
                let smin = s.data.iter().cloned().fold(f64::INFINITY, f64::min);
                let smax = s.data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                let srange = if (smax - smin).abs() < f64::EPSILON {
                    1.0
                } else {
                    smax - smin
                };
                (smin, smax, srange)
            })
            .collect();

        let global_range = if (self.global_max - self.global_min).abs() < f64::EPSILON {
            1.0
        } else {
            self.global_max - self.global_min
        };

        // One grid per series
        let mut grids: Vec<Vec<Vec<bool>>> = self
            .series
            .iter()
            .map(|_| vec![vec![false; pixel_h]; pixel_w])
            .collect();

        for (si, series) in self.series.iter().enumerate() {
            let (smin, _smax, srange) = if self.dual_axis {
                series_stats[si]
            } else {
                (self.global_min, self.global_max, global_range)
            };

            let grid = &mut grids[si];
            let n = series.data.len();
            for (i, &val) in series.data.iter().enumerate() {
                let col = if n == 1 {
                    pixel_w / 2
                } else {
                    (i as f64 / (n - 1) as f64 * (pixel_w - 1) as f64).round() as usize
                };
                let row = ((val - smin) / srange * (pixel_h - 1) as f64)
                    .round()
                    .clamp(0.0, (pixel_h - 1) as f64) as usize;
                if col < pixel_w {
                    grid[col][row] = true;
                }
            }

            for i in 1..n {
                let col0 = if n == 1 {
                    pixel_w / 2
                } else {
                    (((i - 1) as f64) / (n - 1) as f64 * (pixel_w - 1) as f64).round() as usize
                };
                let col1 =
                    ((i as f64) / (n - 1) as f64 * (pixel_w - 1) as f64).round() as usize;
                let row0 = ((series.data[i - 1] - smin) / srange * (pixel_h - 1) as f64)
                    .round()
                    .clamp(0.0, (pixel_h - 1) as f64) as usize;
                let row1 = ((series.data[i] - smin) / srange * (pixel_h - 1) as f64)
                    .round()
                    .clamp(0.0, (pixel_h - 1) as f64) as usize;

                draw_line(grid, col0, row0, col1, row1, pixel_w, pixel_h);
            }
        }

        // ANSI colors for series differentiation (when multiple series)
        let colors: &[&str] = &["\x1b[36m", "\x1b[33m", "\x1b[32m", "\x1b[35m"];
        let reset = "\x1b[0m";
        let use_colors = self.series.len() > 1;

        // Y-axis labels
        let label_count = self.height.min(5);
        let label_rows: Vec<usize> = (0..label_count)
            .map(|i| {
                if label_count == 1 {
                    0
                } else {
                    i * (self.height - 1) / (label_count - 1)
                }
            })
            .collect();

        for r in 0..self.height {
            let pixel_row_base = (self.height - 1 - r) * 4;

            // Left Y-axis label
            let left_label = if label_rows.contains(&(self.height - 1 - r)) {
                let frac = (self.height - 1 - r) as f64 / (self.height - 1).max(1) as f64;
                if self.dual_axis {
                    let (smin, _smax, srange) = series_stats[0];
                    let val = smin + frac * srange;
                    format!("{:>9}", format_eng(val))
                } else {
                    let val = self.global_min + frac * global_range;
                    format!("{:>9}", format_eng(val))
                }
            } else {
                "         ".to_string()
            };
            output.push_str(&left_label);
            output.push_str(" ┤");

            for c in 0..self.width {
                let pixel_col_base = c * 2;

                if use_colors {
                    // Determine which series contribute to this cell
                    // Build per-series braille codes, pick the one with most dots (or merge)
                    let mut codes: Vec<(usize, u32)> = Vec::new();
                    for (si, grid) in grids.iter().enumerate() {
                        let mut code: u32 = 0;
                        for (dx, braille_col) in BRAILLE_MAP.iter().enumerate() {
                            for (dy, &dot) in braille_col.iter().enumerate().take(4) {
                                let px = pixel_col_base + dx;
                                let py = pixel_row_base + (3 - dy);
                                if px < pixel_w && py < pixel_h && grid[px][py] {
                                    code |= dot;
                                }
                            }
                        }
                        if code != 0 {
                            codes.push((si, code));
                        }
                    }

                    if codes.is_empty() {
                        output.push(char::from_u32(BRAILLE_OFFSET).unwrap_or(' '));
                    } else if codes.len() == 1 {
                        let (si, code) = codes[0];
                        let color = colors[si % colors.len()];
                        let ch = char::from_u32(BRAILLE_OFFSET + code).unwrap_or(' ');
                        output.push_str(&format!("{}{}{}", color, ch, reset));
                    } else {
                        // Multiple series overlap: merge all dots, use white
                        let merged: u32 = codes.iter().map(|(_, c)| c).fold(0u32, |a, b| a | b);
                        let ch = char::from_u32(BRAILLE_OFFSET + merged).unwrap_or(' ');
                        output.push_str(&format!("\x1b[37m{}{}", ch, reset));
                    }
                } else {
                    let mut code: u32 = 0;
                    for (dx, braille_col) in BRAILLE_MAP.iter().enumerate() {
                        for (dy, &dot) in braille_col.iter().enumerate().take(4) {
                            let px = pixel_col_base + dx;
                            let py = pixel_row_base + (3 - dy);
                            if px < pixel_w && py < pixel_h && grids[0][px][py] {
                                code |= dot;
                            }
                        }
                    }
                    let ch = char::from_u32(BRAILLE_OFFSET + code).unwrap_or(' ');
                    output.push(ch);
                }
            }
            // Right Y-axis label (dual mode only)
            if self.dual_axis {
                output.push_str("├ ");
                if label_rows.contains(&(self.height - 1 - r)) {
                    let frac = (self.height - 1 - r) as f64 / (self.height - 1).max(1) as f64;
                    let (smin, _smax, srange) = series_stats[1];
                    let val = smin + frac * srange;
                    output.push_str(&format!("{:<9}", format_eng(val)));
                }
            }
            output.push('\n');
        }

        // Bottom axis
        output.push_str("          └");
        for _ in 0..self.width {
            output.push('─');
        }
        if self.dual_axis {
            output.push('┘');
        }
        output.push('\n');

        // X-axis labels
        let max_n = self.series.iter().map(|s| s.data.len()).max().unwrap_or(0);
        if max_n > 0 {
            let right_label = format!("{}", max_n - 1);
            let padding = self.width.saturating_sub(right_label.len());
            output.push_str(&format!(
                "          0{:>width$}\n",
                right_label,
                width = padding
            ));
        }

        // Legend for multi-series
        if use_colors {
            output.push('\n');
            for (i, s) in self.series.iter().enumerate() {
                let color = colors[i % colors.len()];
                output.push_str(&format!("  {}━━{} {}\n", color, reset, s.name));
            }
        }

        output
    }
}

/// Bresenham line drawing on the pixel grid.
fn draw_line(
    grid: &mut [Vec<bool>],
    x0: usize,
    y0: usize,
    x1: usize,
    y1: usize,
    max_x: usize,
    max_y: usize,
) {
    let (mut x0, mut y0) = (x0 as isize, y0 as isize);
    let (x1, y1) = (x1 as isize, y1 as isize);

    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx: isize = if x0 < x1 { 1 } else { -1 };
    let sy: isize = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        if x0 >= 0 && (x0 as usize) < max_x && y0 >= 0 && (y0 as usize) < max_y {
            grid[x0 as usize][y0 as usize] = true;
        }
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

/// Format a number for Y-axis labels using engineering notation (10^3 multiples)
/// when the value is too large or too small to fit in 9 characters.
fn format_eng(val: f64) -> String {
    let abs = val.abs();
    if abs < 1e-15 {
        return "0.00".to_string();
    }
    // Use plain format if it fits well in 9 chars
    if (0.01..100_000.0).contains(&abs) {
        return format!("{:.2}", val);
    }
    // Engineering notation: exponent is a multiple of 3
    let exp = (val.abs().log10().floor() as i32).div_euclid(3) * 3;
    let mantissa = val / 10f64.powi(exp);
    format!("{:.2}e{}", mantissa, exp)
}

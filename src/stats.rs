/// Compute statistics from a slice of f64 values.
pub struct Stats {
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub variance: f64,
    pub count: usize,
    sorted: Vec<f64>,
}

impl Stats {
    pub fn new(data: &[f64]) -> Option<Self> {
        if data.is_empty() {
            return None;
        }
        let count = data.len();
        let min = data.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let sum: f64 = data.iter().sum();
        let mean = sum / count as f64;
        let variance = data.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / count as f64;

        let mut sorted = data.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        Some(Self {
            min,
            max,
            mean,
            variance,
            count,
            sorted,
        })
    }

    /// Return the value at the given percentile (0–100).
    pub fn percentile(&self, p: f64) -> f64 {
        if self.sorted.len() == 1 {
            return self.sorted[0];
        }
        let p = p.clamp(0.0, 100.0);
        let rank = (p / 100.0) * (self.sorted.len() - 1) as f64;
        let lower = rank.floor() as usize;
        let upper = rank.ceil() as usize;
        let frac = rank - lower as f64;
        self.sorted[lower] * (1.0 - frac) + self.sorted[upper] * frac
    }

    pub fn stddev(&self) -> f64 {
        self.variance.sqrt()
    }
}

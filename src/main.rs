mod graph;
mod pie;
mod stats;
mod utils;

use clap::Parser;

use crate::graph::{Graph, Series};
use crate::pie::{Pie, PieSlice};
use crate::stats::Stats;
use crate::utils::{read_labeled_values, read_values};

#[derive(Parser)]
#[command(name = "rs-termeter", about = "ASCII graph renderer with statistics")]
struct Cli {
    /// Input file (one number per line). Use "-" or omit for stdin.
    #[arg(default_value = "-")]
    file: String,

    /// Title displayed above the graph
    #[arg(short, long, default_value = "")]
    title: String,

    /// Percentiles to display, comma-separated (e.g. "5,25,50,75,95")
    #[arg(short, long, value_delimiter = ',')]
    percentiles: Vec<f64>,

    /// Names for data columns, comma-separated (e.g. "latency,throughput,errors")
    /// Defaults to y1, y2, y3, ... for unnamed columns.
    #[arg(short = 'n', long = "names", value_delimiter = ',')]
    names: Vec<String>,

    /// Dual Y-axis mode: y1 scale on the left, y2 scale on the right (requires exactly 2 series)
    #[arg(short, long)]
    dual: bool,

    /// Pie chart mode. Input file format: one "label value" per line.
    /// Each slice is rendered with a percentage of the total.
    #[arg(short = 'P', long)]
    pie: bool,
}


fn main() {
    let cli = Cli::parse();

    if cli.pie {
        run_pie(&cli);
        return;
    }

    let columns = match read_values(&cli.file) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    };

    if columns.is_empty() || columns[0].is_empty() {
        eprintln!("error: no numeric data found");
        std::process::exit(1);
    }

    let names: Vec<String> = columns
        .iter()
        .enumerate()
        .map(|(i, _)| {
            cli.names
                .get(i)
                .cloned()
                .unwrap_or_else(|| format!("y{}", i + 1))
        })
        .collect();

    let series: Vec<Series> = columns
        .iter()
        .enumerate()
        .map(|(i, col)| Series {
            data: col.clone(),
            name: names[i].clone(),
        })
        .collect();

    // Render graph
    let title = if cli.title.is_empty() {
        if cli.file == "-" {
            "stdin".to_string()
        } else {
            cli.file.clone()
        }
    } else {
        cli.title.clone()
    };

    if cli.dual && columns.len() != 2 {
        eprintln!("error: --dual requires exactly 2 data columns");
        std::process::exit(1);
    }

    let g = Graph::new(series, cli.dual);
    let rendered = g.render();

    // Print title
    println!("\n  {}", title);
    println!();
    print!("{}", rendered);

    // Print stats per series
    println!();
    for (i, col) in columns.iter().enumerate() {
        let name = &names[i];
        let stats = Stats::new(col).expect("non-empty data");
        println!(
            "  [{}] count: {}   min: {:.4}   max: {:.4}   mean: {:.4}   stddev: {:.4}   var: {:.4}",
            name,
            stats.count,
            stats.min,
            stats.max,
            stats.mean,
            stats.stddev(),
            stats.variance,
        );

        if !cli.percentiles.is_empty() {
            let parts: Vec<String> = cli
                .percentiles
                .iter()
                .map(|&p| format!("P{}: {:.4}", p, stats.percentile(p)))
                .collect();
            println!("  [{}] {}", name, parts.join("   "));
        }
    }
    println!();
}

fn run_pie(cli: &Cli) {
    let entries = match read_labeled_values(&cli.file) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    };

    if entries.is_empty() {
        eprintln!("error: no labeled data found (expected lines like 'label value')");
        std::process::exit(1);
    }

    let total: f64 = entries.iter().map(|(_, v)| v.max(0.0)).sum();
    if total <= 0.0 {
        eprintln!("error: pie chart requires at least one positive value");
        std::process::exit(1);
    }

    let title = if cli.title.is_empty() {
        if cli.file == "-" {
            "stdin".to_string()
        } else {
            cli.file.clone()
        }
    } else {
        cli.title.clone()
    };

    let slices: Vec<PieSlice> = entries
        .into_iter()
        .map(|(name, value)| PieSlice { name, value })
        .collect();

    let pie = Pie::new(slices);
    let rendered = pie.render();

    println!("\n  {}", title);
    println!();
    print!("{}", rendered);
    println!("\n  total: {:.4}", total);
    println!();
}

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use telemetry_core::events::TelemetryEnvelope;

#[derive(Parser, Debug)]
#[command(name = "telemetry-viewer", about = "CME telemetry JSONL viewer")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Print summary statistics for a JSONL telemetry file.
    Summary {
        /// Path to the JSONL file.
        file: String,
    },
    /// Pretty-print each event in a JSONL telemetry file.
    Cat {
        /// Path to the JSONL file.
        file: String,
    },
    /// Filter events by event type.
    Filter {
        /// Path to the JSONL file.
        file: String,
        /// Event type to filter (e.g. "cme.market.l1").
        #[arg(long)]
        event_type: String,
    },
}

fn read_envelopes(path: &str) -> Result<Vec<TelemetryEnvelope>> {
    let file = File::open(path).with_context(|| format!("Cannot open {path}"))?;
    let reader = BufReader::new(file);
    let mut envelopes = Vec::new();
    for (i, line) in reader.lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let env: TelemetryEnvelope = serde_json::from_str(&line)
            .with_context(|| format!("Failed to parse line {}", i + 1))?;
        envelopes.push(env);
    }
    Ok(envelopes)
}

fn cmd_summary(file: &str) -> Result<()> {
    let envelopes = read_envelopes(file)?;
    let total = envelopes.len();
    let mut counts: HashMap<&'static str, usize> = HashMap::new();

    for env in &envelopes {
        *counts.entry(env.event_type_str()).or_insert(0) += 1;
    }

    println!("File   : {file}");
    println!("Total  : {total} events");
    println!();
    println!("{:<30} {:>8}", "Event Type", "Count");
    println!("{}", "─".repeat(40));

    let mut sorted: Vec<_> = counts.iter().collect();
    sorted.sort_by_key(|(k, _)| *k);
    for (k, v) in sorted {
        println!("{:<30} {:>8}", k, v);
    }

    Ok(())
}

fn cmd_cat(file: &str) -> Result<()> {
    let envelopes = read_envelopes(file)?;
    for env in &envelopes {
        let pretty = serde_json::to_string_pretty(env)?;
        println!("{}", pretty);
        println!("---");
    }
    Ok(())
}

fn cmd_filter(file: &str, event_type: &str) -> Result<()> {
    let envelopes = read_envelopes(file)?;
    let mut count = 0;
    for env in &envelopes {
        if env.event_type_str() == event_type {
            let pretty = serde_json::to_string_pretty(env)?;
            println!("{}", pretty);
            println!("---");
            count += 1;
        }
    }
    eprintln!("{count} events matched event_type = {event_type:?}");
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match &cli.command {
        Command::Summary { file } => cmd_summary(file),
        Command::Cat { file } => cmd_cat(file),
        Command::Filter { file, event_type } => cmd_filter(file, event_type),
    }
}

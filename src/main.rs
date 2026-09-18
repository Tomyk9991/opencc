mod client;
mod client_error;
mod model;

use std::thread;

use client::OpenCodeClient;

use indicatif::{MultiProgress, ProgressBar, ProgressFinish, ProgressStyle};

use crate::model::Model;

/// Full-width static rainbow bar (ANSI, one color per block).
fn rainbow_bar(width: usize) -> String {
    const CODES: [u8; 6] = [31, 33, 32, 36, 34, 35];
    let mut bar = String::with_capacity(width * 11);
    for i in 0..width {
        bar.push_str(&format!("\x1b[{}m■\x1b[0m", CODES[i % CODES.len()]));
    }
    bar
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = OpenCodeClient::new();
    let models = client.models().await?;

    let multi_progressbar = MultiProgress::new();
    let sty = ProgressStyle::with_template("{bar:40.yellow.transparent} {pos:>7} {msg}")?
        .progress_chars("■ ");

    let mut progress_bars: Vec<(ProgressBar, Model)> = vec![];

    let max_num_requests = models
        .iter()
        .max_by_key(|m| m.num_requests)
        .map_or_else(|| 0, |m| m.num_requests);

    for model in &models {
        let progress_bar = multi_progressbar
            .add(ProgressBar::new(max_num_requests as u64).with_finish(ProgressFinish::Abandon));
        progress_bar.set_style(sty.clone());
        let label = if model.unlimited {
            format!("∞ {}{}", model.name, model.markers)
        } else {
            format!("{}{}", model.name, model.markers)
        };
        progress_bar.set_message(label);

        progress_bars.push((progress_bar, model.clone()));
    }
    let handle = thread::spawn(move || {
        let inc = 200;
        let mut i = 0;
        while i < max_num_requests {
            i += inc;
            for (progress_bar, model) in &progress_bars {
                // Free models grow to full length, tied with the largest value.
                // Stays yellow like all others; rainbow applies only at final size.
                let target = if model.unlimited {
                    max_num_requests
                } else {
                    model.num_requests
                };
                progress_bar.set_position(i.min(target) as u64);
            }
            thread::sleep(std::time::Duration::from_millis(1));
        }

        for (progress_bar, model) in &progress_bars {
            if model.unlimited {
                progress_bar.set_style(
                    ProgressStyle::with_template("{msg}").expect("template didn't work."),
                );
                progress_bar.set_position(max_num_requests as u64);
                progress_bar.abandon_with_message(format!(
                    "{} {:>7} {}{}",
                    rainbow_bar(40),
                    "∞",
                    model.name,
                    model.markers
                ));
            } else {
                progress_bar.set_position(model.num_requests as u64);
                progress_bar.abandon();
            }
        }
    });

    let _ = handle.join();
    Ok(())
}

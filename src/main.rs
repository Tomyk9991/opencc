mod client;
mod client_error;
mod model;

use std::thread;

use client::OpenCodeClient;

use indicatif::{MultiProgress, ProgressBar, ProgressFinish, ProgressStyle};

use crate::model::Model;

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
        progress_bar.set_message(format!("{}{}", model.name.clone(), model.markers));

        progress_bars.push((progress_bar, model.clone()));
    }

    let handle = thread::spawn(move || {
        let inc = 200;
        let mut i = 0;
        while i < max_num_requests {
            i += inc;
            for (progress_bar, model) in &progress_bars {
                let pos = i.min(model.num_requests) as u64;
                progress_bar.set_position(pos);
            }
            thread::sleep(std::time::Duration::from_millis(1));
        }
        // Wichtig: Bars stehen lassen statt clearen.
        // abandon (nicht finish!): finish() würde alle Bars auf voll (= max) setzen.
        for (progress_bar, model) in &progress_bars {
            progress_bar.set_position(model.num_requests as u64);
            progress_bar.abandon();
        }
    });

    let _ = handle.join();
    Ok(())
}

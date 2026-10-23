use std::{
    io::{self, IsTerminal, Read},
    path::{Path, PathBuf},
    sync::Arc,
};
use veyl_lab::{fake_phone::FakePhone, recording::Input};

const USAGE: &str = "Usage:\n  veyl-lab scenarios\n  veyl-lab sim SCENARIO [--seed N] [--seconds S] --out DIR\n  veyl-lab analyze DIR [--config params.toml] [--out DIR2]\n  veyl-lab sweep DIR... --set parameter=value,value [--set ...] [--out sweep.csv]\n  veyl-lab fake-phone (--scenario NAME | --recording DIR) [--port 47820] [--realtime] [--seed N]";

#[tokio::main(flavor = "current_thread")]
async fn main() {
    if let Err(error) = run(std::env::args().skip(1).collect()).await {
        eprintln!("{error}");
        std::process::exit(2);
    }
}
async fn run(arguments: Vec<String>) -> io::Result<()> {
    if arguments.first().map(String::as_str) == Some("sim") {
        return simulate(&arguments);
    }
    if arguments.first().map(String::as_str) == Some("analyze") {
        return analyze_command(&arguments);
    }
    if arguments.first().map(String::as_str) == Some("sweep") {
        return sweep_command(&arguments);
    }
    if arguments.as_slice() == ["scenarios"] {
        for scenario in veyl_sim::SCENARIOS {
            println!("{scenario} (available)");
        }
        return Ok(());
    }
    if arguments.as_slice() == ["--help"] || arguments.as_slice() == ["help"] {
        println!("{USAGE}");
        return Ok(());
    }
    if arguments.first().map(String::as_str) != Some("fake-phone") {
        return Err(usage());
    }
    let (mut scenario, mut recording) = (None, None);
    let mut port = 47820;
    let mut seed = 1;
    let mut realtime = false;
    let mut options = std::collections::BTreeSet::new();
    let mut args = arguments.into_iter().skip(1);
    while let Some(option) = args.next() {
        if !options.insert(option.clone()) {
            return Err(usage());
        }
        match option.as_str() {
            "--scenario" => scenario = Some(args.next().ok_or_else(usage)?),
            "--recording" => recording = Some(args.next().ok_or_else(usage)?),
            "--port" => {
                port = args
                    .next()
                    .ok_or_else(usage)?
                    .parse()
                    .map_err(|_| usage())?
            }
            "--seed" => {
                seed = args
                    .next()
                    .ok_or_else(usage)?
                    .parse()
                    .map_err(|_| usage())?
            }
            "--realtime" => realtime = true,
            _ => return Err(usage()),
        }
    }
    let input = match (scenario.as_deref(), recording.as_deref()) {
        (Some(name), None) => Input::generated(veyl_sim::scene::render(
            &veyl_sim::scenarios::scene(name, seed, None)?,
        )?),
        (None, Some(directory)) if !options.contains("--seed") => {
            Input::read(Path::new(directory))?
        }
        _ => return Err(usage()),
    };
    let phone = FakePhone::bind(
        input,
        port,
        realtime,
        Arc::new(|code| {
            // This is the terminal's comparison UI. Redirected output never receives a code.
            if io::stdout().is_terminal() {
                println!("Pairing code: {code}");
            }
        }),
    )
    .await?;
    println!(
        "Veyl simulator is ready. Add by address: {}",
        phone.address()
    );
    println!("Pairings last until this process exits. Press Ctrl+C to stop.");
    if !io::stdout().is_terminal() {
        println!("Open an interactive terminal to compare a pairing code.");
    }
    tokio::signal::ctrl_c().await?;
    let report = phone.report();
    phone.shutdown().await;
    println!("Stopped. Sessions: {}; reference blocks: {}; microphone blocks: {}; decryption errors: {}.", report.sessions, report.blocks[0], report.blocks[1], report.decryption_errors);
    Ok(())
}
fn simulate(arguments: &[String]) -> io::Result<()> {
    let name = arguments.get(1).ok_or_else(usage)?;
    let mut seed = 1;
    let mut seconds = None;
    let mut output = None;
    let mut seen = std::collections::BTreeSet::new();
    let mut options = arguments.iter().skip(2);
    while let Some(option) = options.next() {
        if !seen.insert(option) {
            return Err(usage());
        }
        let value = options.next().ok_or_else(usage)?;
        match option.as_str() {
            "--seed" => seed = value.parse().map_err(|_| usage())?,
            "--seconds" => seconds = Some(value.parse().map_err(|_| usage())?),
            "--out" => output = Some(value),
            _ => return Err(usage()),
        }
    }
    let output = output.ok_or_else(usage)?;
    let scene = veyl_sim::scenarios::scene(name, seed, seconds)?;
    let mut input = Input::generated(veyl_sim::scene::render(&scene)?);
    input.meta.notes = format!("Acoustic simulator: {name}; seed {seed}.");
    input.meta.device["mode"] = if name.starts_with("pc_") {
        "pc"
    } else {
        "phone"
    }
    .into();
    input.meta.device["sound_source"] = if name == "blocked" {
        "capture"
    } else if name.starts_with("pc_") {
        "pc"
    } else {
        "test_sound"
    }
    .into();
    input.meta.device["route"] = if name == "pc_bluetooth" {
        "bluetooth"
    } else {
        "speaker"
    }
    .into();
    input.meta.device["media_volume"] = scene.volume.into();
    input.save_generated(Path::new(output))?;
    println!(
        "Generated {name}: {} blocks, {} truth pokes. Audio is synthetic and privacy-filtered.",
        input.blocks.len(),
        input.labels.len()
    );
    Ok(())
}
fn usage() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, USAGE)
}

fn analyze_command(arguments: &[String]) -> io::Result<()> {
    let directory = arguments.get(1).ok_or_else(usage)?;
    let mut output = directory;
    let mut config_path = None;
    let mut seen = std::collections::BTreeSet::new();
    let mut args = arguments.iter().skip(2);
    while let Some(option) = args.next() {
        if !seen.insert(option) {
            return Err(usage());
        }
        let value = args.next().ok_or_else(usage)?;
        match option.as_str() {
            "--out" => output = value,
            "--config" => config_path = Some(value),
            _ => return Err(usage()),
        }
    }
    let input = Input::read(Path::new(directory))?;
    let config = if let Some(path) = config_path {
        let mut text = String::new();
        std::fs::File::open(path)?
            .take(65537)
            .read_to_string(&mut text)?;
        veyl_dsp::config::EngineConfig::from_toml(&text).map_err(io::Error::other)?
    } else {
        veyl_lab::analysis::default_config(&input)
    };
    let analyzed = veyl_lab::analysis::evaluate(&input, config, true)?;
    veyl_lab::analysis::write_outputs(&input, &analyzed, Path::new(output))?;
    let metric = |value: Option<f64>| {
        value
            .map(|v| format!("{:.1}%", v * 100.0))
            .unwrap_or_else(|| "not defined".into())
    };
    println!(
        "Analysis saved. Pokes: {}; recall: {}; precision: {}.",
        analyzed.report.pokes.detected,
        metric(analyzed.report.pokes.recall),
        metric(analyzed.report.pokes.precision)
    );
    if analyzed.report.engine.lock_ms.is_none() {
        println!("No alignment lock in this recording.");
    }
    Ok(())
}
fn sweep_command(arguments: &[String]) -> io::Result<()> {
    let mut directories = vec![];
    let mut parameters = vec![];
    let mut output = PathBuf::from("sweep.csv");
    let mut output_seen = false;
    let mut args = arguments.iter().skip(1);
    while let Some(option) = args.next() {
        match option.as_str() {
            "--set" => parameters.push(veyl_lab::sweep::parameter(args.next().ok_or_else(usage)?)?),
            "--out" if !output_seen => {
                output = PathBuf::from(args.next().ok_or_else(usage)?);
                output_seen = true;
            }
            value if !value.starts_with('-') => directories.push(PathBuf::from(value)),
            _ => return Err(usage()),
        }
    }
    let report = veyl_lab::sweep::run(&directories, &parameters, &output)?;
    println!(
        "Saved {} parameter combinations across {} recording{}.",
        report.rows.len(),
        directories.len(),
        if directories.len() == 1 { "" } else { "s" }
    );
    if report.best.is_empty() {
        println!("No labelled result meets the 0.2 false-pokes-per-minute limit.");
    }
    for index in report.best.iter().take(5) {
        let row = &report.rows[*index];
        let settings = report
            .parameters
            .iter()
            .zip(&row.values)
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<_>>()
            .join(" ");
        println!(
            "{settings}: F1 {:.3}; recall {:.1}%; false pokes/min {:.3}",
            row.metrics.f1.unwrap(),
            row.metrics.recall.unwrap() * 100.0,
            row.metrics.false_pokes_per_minute.unwrap()
        );
    }
    Ok(())
}

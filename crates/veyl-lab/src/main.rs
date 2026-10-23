use std::{
    io::{self, IsTerminal},
    path::Path,
    sync::Arc,
};
use veyl_lab::{fake_phone::FakePhone, recording::Input};

const USAGE: &str = "Usage:\n  veyl-lab scenarios\n  veyl-lab sim SCENARIO [--seed N] [--seconds S] --out DIR\n  veyl-lab analyze DIR [--out DIR2]\n  veyl-lab fake-phone (--scenario NAME | --recording DIR) [--port 47820] [--realtime] [--seed N]";

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
        let directory = arguments.get(1).ok_or_else(usage)?;
        let output = match arguments.len() {
            2 => directory,
            4 if arguments[2] == "--out" => &arguments[3],
            _ => return Err(usage()),
        };
        let input = Input::read(Path::new(directory))?;
        let summary = veyl_lab::analyze::write_summary(&input, Path::new(output))?;
        println!("Stream statistics saved. Reference: {} blocks; microphone: {} blocks. Echo and gesture analysis arrive in M4.", summary.reference.blocks, summary.microphone.blocks);
        return Ok(());
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

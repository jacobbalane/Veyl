use std::{
    io::{self, IsTerminal},
    path::Path,
    sync::Arc,
};
use veyl_lab::{fake_phone::FakePhone, recording::Input};

const USAGE: &str = "Usage:\n  veyl-lab scenarios\n  veyl-lab fake-phone (--scenario poke_basic | --recording DIR) [--port 47820] [--realtime] [--seed N]";

#[tokio::main(flavor = "current_thread")]
async fn main() {
    if let Err(error) = run(std::env::args().skip(1).collect()).await {
        eprintln!("{error}");
        std::process::exit(2);
    }
}
async fn run(arguments: Vec<String>) -> io::Result<()> {
    if arguments.as_slice() == ["scenarios"] {
        for scenario in veyl_sim::SCENARIOS {
            println!(
                "{scenario}{}",
                if *scenario == "poke_basic" {
                    " (available)"
                } else {
                    " (planned for M4)"
                }
            );
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
        (Some("poke_basic"), None) => Input::generated(veyl_sim::poke_basic(seed, 40.0)?),
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
fn usage() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, USAGE)
}

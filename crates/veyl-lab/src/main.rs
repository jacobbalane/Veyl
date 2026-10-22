fn main() {
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    if arguments.as_slice() == ["scenarios"] {
        println!("Planned scenarios (rendering arrives in M4):");
        for scenario in veyl_sim::SCENARIOS {
            println!("{scenario}");
        }
    } else {
        eprintln!("Usage: veyl-lab scenarios");
        std::process::exit(2);
    }
}

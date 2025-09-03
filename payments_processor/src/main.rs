use std::env;
use std::error::Error;
use std::fs::File;
use csv::{StringRecord, Reader, ReaderBuilder};

mod transaction;
mod client;
mod ledger;
use ledger::Ledger;

fn open(file_path: &str) -> Result<Reader<File>, Box<dyn Error>> {
    let file = File::open(file_path)
        .map_err(|e| format!("Failed to open file '{}': {}", file_path, e))?;
    
    let rdr = ReaderBuilder::new()
        .flexible(true)
        .from_reader(file);

    Ok(rdr)
}

fn process(reader: &mut Reader<File>) -> Result<(), Box<dyn Error>> {
    let mut ledger = Ledger::new();

    for result in reader.records() {
        let record: StringRecord = result?;
        ledger.process(record);
    }

    ledger.print_summary()
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: cargo run -- <input.csv>");
        std::process::exit(1);
    }

    let file_path = &args[1];

    let mut reader = match open(file_path) {
        Ok(rdr) => rdr,
        Err(err) => {
            eprintln!("Error: {}", err);
            std::process::exit(1);
        }
    };

    if let Err(err) = process(&mut reader) {
        eprintln!("Error while processing: {}", err);
        std::process::exit(1);
    }
}

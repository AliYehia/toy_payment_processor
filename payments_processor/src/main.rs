use std::env;
use std::error::Error;
use std::fs::File;
use std::sync::Arc;
use tokio::sync::Mutex;
use csv::ReaderBuilder;

mod transaction;
mod client;
mod ledger;
use ledger::Ledger;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: cargo run -- <input1.csv> <input2.csv> ...");
        std::process::exit(1);
    }

    let ledger = Arc::new(Mutex::new(Ledger::new()));

    let mut handles = vec![];

    for file_path in &args[1..] {
        let ledger_clone = Arc::clone(&ledger);
        let file_path = file_path.clone();

        let handle = tokio::spawn(async move {
            match File::open(&file_path) {
                Ok(file) => {
                    let mut reader = ReaderBuilder::new()
                        .flexible(true)
                        .from_reader(file);

                    for result in reader.records() {
                        match result {
                            Ok(record) => {
                                let mut ledger_lock = ledger_clone.lock().await;
                                ledger_lock.process(record);
                            }
                            Err(e) => eprintln!("Error reading record in {}: {}", file_path, e),
                        }
                    }
                }
                Err(e) => eprintln!("Failed to open {}: {}", file_path, e),
            }
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.await?;
    }

    let ledger = ledger.lock().await;
    ledger.print_summary()?;

    Ok(())
}

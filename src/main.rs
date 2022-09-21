use std::error::Error;

use csv::Writer;

mod transaction_engine;

fn main() -> Result<(),Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    let path = match args.get(1) {
        Some(path) => path,
        None => {
            println!("Missing argument");
            std::process::exit(1)
        }
    };

    let mut engine = transaction_engine::TransactionEngine::new();

    let mut rdr = csv::Reader::from_path(path)?;
    rdr.deserialize()
        .for_each(|res: Result<transaction_engine::Transaction, csv::Error>|{
            if let Ok(transaction) = res {
                engine.compute_transaction(&transaction)
            }
        });

    let client = engine.get_client_list();
    
    let mut writer = Writer::from_writer(std::io::stdout());
    client.into_iter().for_each(|client| {
        writer.serialize(client);
    });

    writer.flush();

    Ok(())
}


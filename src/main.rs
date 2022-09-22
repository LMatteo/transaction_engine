use std::error::Error;

use csv::Writer;
use serde::Deserialize;

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
        .for_each(|res: Result<Transaction, csv::Error>|{
            match res {
                Ok(transaction) => {
                    match transaction.try_into() {
                        Ok(model) => engine.compute_transaction(model),
                        Err(_) => {}
                    }
                },
                Err(e) => eprintln!("Application error: {e}")
            }
        });

    let client = engine.get_client_list();
    
    let mut writer = Writer::from_writer(std::io::stdout());
    client.into_iter().for_each(|client| {
        writer.serialize(client);
    });

    writer.flush()?;

    Ok(())
}


#[derive(Debug, Deserialize,Clone,Copy)]
pub enum TransactionType {
    #[serde(rename = "deposit")]
    Deposit,
    #[serde(rename = "withdrawal")]
    Withdrawal,
    #[serde(rename = "dispute")]
    Dispute,
    #[serde(rename = "resolve")]
    Resolve,
}

#[derive(Debug, Deserialize, Clone,Copy)]
pub struct Transaction {
    #[serde(rename = "type")]
    transaction_type: TransactionType,
    client: u32,
    tx: u32,
    amount: Option<f32>
}

impl TryInto<transaction_engine::TransactionEnum> for Transaction {
    type Error = ();

    fn try_into(self) -> Result<transaction_engine::TransactionEnum, Self::Error> {
        match self.transaction_type {
            TransactionType::Deposit => {
                if let Some(amount) = self.amount  {
                    Ok(transaction_engine::TransactionEnum::Deposit { 
                        client_id: self.client, 
                        tx_id: self.tx, 
                        amount 
                    })
                } else {
                    Err(())
                }
            },
            TransactionType::Withdrawal => {
                if let Some(amount) = self.amount  {
                    Ok(transaction_engine::TransactionEnum::Withdrawal { 
                        client_id: self.client, 
                        tx_id: self.tx, 
                        amount 
                    })
                } else {
                    Err(())
                } 
            },
            TransactionType::Dispute => {
                Ok(transaction_engine::TransactionEnum::Dispute { 
                    client_id: self.client, 
                    tx_id: self.tx 
                })
            },
            TransactionType::Resolve => {
                Ok(transaction_engine::TransactionEnum::Resolve { 
                    client_id: self.client, 
                    tx_id: self.tx 
                })
            },
        }
    }
}
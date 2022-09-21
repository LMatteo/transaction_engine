use assert_cmd::prelude::*; // Add methods on commands
use predicates::prelude::*;
use serde::{Deserialize, Serialize}; // Used for writing assertions
use std::process::Command; // Run programs
use std::cmp::Ordering;

const BASE_PATH: &str = "/resources/tests";

fn get_base_path () -> String {
    let path = env!("CARGO_MANIFEST_DIR").to_string();
    return path + BASE_PATH
}

#[test]
fn deposit() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("transaction_engine")?;

    cmd.arg(get_base_path() + "/deposit.csv");

    let mut expected = vec![
        Client{client: 2,available: 2.0,held: 0.0,total: 2.0,locked: false},
        Client{client: 1,available: 3.0,held: 0.0,total: 3.0,locked: false},
    ];
    expected.sort();

    cmd.assert()
        .success()
        .stdout(predicate::function(compare_stdout(expected)));

    Ok(())
}

#[test]
fn withdrawal() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("transaction_engine")?;

    cmd.arg(get_base_path() + "/withdrawal.csv");

    let mut expected = vec![
        Client{client: 2,available: 10.0,held: 0.0,total: 10.0,locked: false},
        Client{client: 1,available: 5.0,held: 0.0,total: 5.0,locked: false},
    ];
    expected.sort();

    cmd.assert()
        .success()
        .stdout(predicate::function(compare_stdout(expected)));

    Ok(())
}
#[test]
fn dispute() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("transaction_engine")?;

    cmd.arg(get_base_path() + "/dispute.csv");

    let mut expected = vec![
        Client{client: 2,available: -40.0,held: 50.0,total: 10.0,locked: false},
        Client{client: 3,available: 0.0,held: 50.0,total: 50.0,locked: false},
        Client{client: 1,available:5.0,held: 0.0,total: 5.0,locked: false},
    ];
    expected.sort();

    cmd.assert()
        .success()
        .stdout(predicate::function(compare_stdout(expected)));

    Ok(())
}

fn compare_stdout(clients: Vec<Client>) -> impl Fn(&[u8]) -> bool {
    |x: &[u8]| {
        let mut rdr = csv::Reader::from_reader(x);
        let mut clients : Vec<Client> = rdr.deserialize()
            .filter(|client: &Result<Client, csv::Error>| client.is_ok())
            .map(|client|{
                client.unwrap()
            })
            .collect();
        clients.sort();

        clients == clients
    }
}

#[derive(Clone,Copy,Debug, Deserialize, Serialize, PartialOrd)]
struct Client {
    client: u32,
    available: f32,
    held: f32,
    total: f32,
    locked: bool
}

impl std::cmp::Ord for Client {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.client > other.client {
            Ordering::Greater
        } else if self.client < other.client {
            Ordering::Less
        } else {
            Ordering::Equal
        }
    }
}

impl PartialEq for Client {
    fn eq(&self, other: &Self) -> bool {
        self.client == other.client
    }
}

impl Eq for Client {}
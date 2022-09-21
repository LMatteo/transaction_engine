use std::{ error::Error, collections::HashMap};

use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Deserialize, Clone,Copy)]
pub struct Transaction {
    #[serde(rename = "type")]
    transaction_type: TransactionType,
    client: u16,
    tx: u32,
    amount: f32
}

#[derive(Debug, Deserialize,Clone,Copy)]
pub enum TransactionType {
    #[serde(rename = "deposit")]
    Deposit,
    #[serde(rename = "withdrawal")]
    Withdrawal,
    #[serde(rename = "dispute")]
    Dispute
}

pub enum TransactionEnum {
    Deposit(u32,u32,f32),
    Withdrawal(u32,u32,f32),
    Dispute(u32,u32)
} 

pub enum PersistedTransaction{
    Deposit(u32,u32,f32),
    Withdrawal(u32,u32,f32),
}

#[derive(Clone,Copy,Debug, Deserialize, Serialize)]
pub struct Client {
    client: u16,
    available: f32,
    held: f32,
    total: f32,
    locked: bool
}

struct ClientList{
    clients: HashMap<u16,Client>
}

impl ClientList {
    fn new() -> ClientList {
        ClientList { clients: HashMap::new() }
    }

    fn get_mut(&mut self,id: u16) -> &mut Client {
        self.clients
            .entry(id)
            .or_insert_with(|| Client{
                client: id,
                held: 0.0,
                total: 0.0,
                available: 0.0,
                locked: false
        })
    }

    fn get_all(&self) -> Vec<Client> {
        (&self.clients).into_iter()
            .map(|(_,client)| client )
            .copied()
            .collect()
    }
}

enum TransactionState {
    Disputed,
    None
}

pub struct TransactionEngine {
    client_list: ClientList,
    transactions: HashMap<u32,(Transaction,TransactionState)>,
    txs: HashMap<u32,(TransactionState)>,
}

impl TransactionEngine {
    pub fn new() -> TransactionEngine {
        TransactionEngine{
            client_list: ClientList::new(),
            transactions: HashMap::new()
        }
    }

    pub fn compute_transaction(&mut self, transaction: &Transaction) {
        match transaction.transaction_type {
            TransactionType::Deposit => self.handle_deposit(transaction),
            TransactionType::Withdrawal => self.handle_withdrawal(transaction),
            TransactionType::Dispute => self.handle_dispute(transaction),
        }
    }

    pub fn get_client_list(&self) -> Vec<Client> {
        (&self.client_list).get_all().clone()
    }

    fn handle_deposit(&mut self, transaction: &Transaction) {
        let client = self.client_list.get_mut(transaction.client);
    
        client.total += transaction.amount;
        client.available += transaction.amount;

        self.transactions.insert(transaction.tx, (transaction.clone(),TransactionState::None));
    }

    fn handle_withdrawal(&mut self, transaction: &Transaction) {
        let client = self.client_list.get_mut(transaction.client);
        
        if client.total >= transaction.amount || client.available >= transaction.amount {
            client.total -= transaction.amount;
            client.available -= transaction.amount;

            self.transactions.insert(transaction.tx, (transaction.clone(),TransactionState::None));
        }
    }

    fn handle_dispute(&mut self, transaction: &Transaction) {
        let (disputed,state) = match self.transactions.get(&transaction.tx){
            Some(tx) => tx,
            None => return,
        };

        if !matches!(disputed.transaction_type, TransactionType::Deposit) {
                return
        }

        if matches!(state, &TransactionState::Disputed) {
            return
        }
        
        let client = self.client_list.get_mut(transaction.client);

        client.available -= disputed.amount;
        client.held += disputed.amount;

        self.transactions.insert(transaction.tx, (disputed.clone(),TransactionState::Disputed));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn when_deposit_should_increase_total_and_available() {
        let mut engine = TransactionEngine::new();
        let transaction = Transaction{
            transaction_type: TransactionType::Deposit,
            client: 1,
            tx: 1,
            amount: 10.0
        };

        engine.compute_transaction(&transaction);
        let clients = engine.get_client_list();

        assert_eq!(clients.len(),1);
        let client = clients.get(0).unwrap();
        assert_eq!(client.available,10.0);
        assert_eq!(client.total,10.0);
        assert_eq!(client.client,1);
    }

    #[test]
    fn when_withdrawal_and_fund_available_should_decrease_total_and_available() {
        let mut engine = TransactionEngine::new();

        engine.compute_transaction(&Transaction{
            transaction_type: TransactionType::Deposit,
            client: 1,
            tx: 1,
            amount: 30.0
        });
        engine.compute_transaction(&Transaction{
            transaction_type: TransactionType::Withdrawal,
            client: 1,
            tx: 2,
            amount: 20.0
        });
        let clients = engine.get_client_list();

        assert_eq!(clients.len(),1);
        let client = clients.get(0).unwrap();
        assert_eq!(client.available,10.0);
        assert_eq!(client.total,10.0);
        assert_eq!(client.client,1);
    }

    #[test]
    fn when_withdrawal_and_fund_not_available_should_do_nothing() {
        let mut engine = TransactionEngine::new();

        engine.compute_transaction(&Transaction{
            transaction_type: TransactionType::Deposit,
            client: 1,
            tx: 1,
            amount: 50.0
        });
        engine.compute_transaction(&Transaction{
            transaction_type: TransactionType::Withdrawal,
            client: 1,
            tx: 2,
            amount: 60.0
        });
        let clients = engine.get_client_list();

        assert_eq!(clients.len(),1);
        let client = clients.get(0).unwrap();
        assert_eq!(client.available,50.0);
        assert_eq!(client.total,50.0);
        assert_eq!(client.client,1);
    }

    #[test]
    fn when_deposit_should_copy_it_with_state_none() {
        let mut engine = TransactionEngine::new();
        let transaction = Transaction{
            transaction_type: TransactionType::Deposit,
            client: 1,
            tx: 1,
            amount: 10.0
        };

        engine.compute_transaction(&transaction);
        
        assert_eq!(engine.transactions.len(),1);
        let (tx,state) = engine.transactions.get(&transaction.tx).unwrap();

        assert!(matches!(state,TransactionState::None));
        assert_eq!(tx.tx,transaction.tx);
        assert!(matches!(tx.transaction_type,TransactionType::Deposit));
        assert_eq!(tx.client,transaction.client);
        assert_eq!(tx.amount,transaction.amount);
    }

    #[test]
    fn when_succesful_withdrawal_should_copy_it_with_state_none() {
        let mut engine = TransactionEngine::new();

        engine.compute_transaction(&Transaction{
            transaction_type: TransactionType::Deposit,
            client: 1,
            tx: 1,
            amount: 30.0
        });
        engine.compute_transaction(&Transaction{
            transaction_type: TransactionType::Withdrawal,
            client: 1,
            tx: 2,
            amount: 20.0
        });
        let (tx,state) = engine.transactions.get(&2).unwrap();

        assert!(matches!(state,TransactionState::None));
        assert_eq!(tx.tx,2);
        assert!(matches!(tx.transaction_type,TransactionType::Withdrawal));
        assert_eq!(tx.client,1);
        assert_eq!(tx.amount,20.0);
    }

    #[test]
    fn when_dispute_on_deposit_should_decrease_available_increase_held() {
        let mut engine = TransactionEngine::new();

        engine.compute_transaction(&Transaction{
            transaction_type: TransactionType::Deposit,
            client: 1,
            tx: 1,
            amount: 10.0
        });
        engine.compute_transaction(&Transaction{
            transaction_type: TransactionType::Dispute,
            client: 1,
            tx: 1,
            amount: 0.0
        });

        let (_, state) = engine.transactions.get(&1).unwrap();
        assert!(matches!(state,TransactionState::Disputed));

        let client = engine.client_list.get_mut(1);
        assert_eq!(client.held,10.0);
        assert_eq!(client.available,0.0);
        assert_eq!(client.total,10.0);
    }
    
    #[test]
    fn when_dispute_on_already_disputed_should_do_nothing() {
        let mut engine = TransactionEngine::new();

        engine.compute_transaction(&Transaction{
            transaction_type: TransactionType::Deposit,
            client: 1,
            tx: 1,
            amount: 10.0
        });
        engine.compute_transaction(&Transaction{
            transaction_type: TransactionType::Dispute,
            client: 1,
            tx: 1,
            amount: 0.0
        });
        engine.compute_transaction(&Transaction{
            transaction_type: TransactionType::Dispute,
            client: 1,
            tx: 1,
            amount: 0.0
        });

        let (_, state) = engine.transactions.get(&1).unwrap();
        assert!(matches!(state,TransactionState::Disputed));

        let client = engine.client_list.get_mut(1);
        assert_eq!(client.held,10.0);
        assert_eq!(client.available,0.0);
        assert_eq!(client.total,10.0);
    }

    #[test]
    fn when_dispute_on_missing_tx_should_do_nothing() {
        let mut engine = TransactionEngine::new();

        engine.compute_transaction(&Transaction{
            transaction_type: TransactionType::Dispute,
            client: 1,
            tx: 1,
            amount: 0.0
        });

        assert_eq!(0,engine.transactions.len());
        assert_eq!(0,engine.client_list.get_all().len())
    }
}




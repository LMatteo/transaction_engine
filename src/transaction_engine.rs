use std::{ collections::HashMap};

use serde::Deserialize;
use serde::Serialize;


pub enum TransactionEnum {
    Deposit{client_id: u32, tx_id : u32, amount: f32},
    Withdrawal{client_id: u32, tx_id : u32, amount: f32},
    Dispute{client_id: u32, tx_id : u32},
    Resolve{client_id: u32, tx_id : u32}
} 

#[derive(Clone)]
pub enum PersistedTransaction{
    Deposit{client_id: u32, tx_id : u32, amount: f32},
}

#[derive(Clone,Copy,Debug, Deserialize, Serialize)]
pub struct Client {
    client: u32,
    available: f32,
    held: f32,
    total: f32,
    locked: bool
}

struct ClientList{
    clients: HashMap<u32,Client>
}

impl ClientList {
    fn new() -> ClientList {
        ClientList { clients: HashMap::new() }
    }

    fn get_mut(&mut self,id: u32) -> &mut Client {
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
    transactions: HashMap<u32,(PersistedTransaction,TransactionState)>,
}

impl TransactionEngine {
    pub fn new() -> TransactionEngine {
        TransactionEngine{
            client_list: ClientList::new(),
            transactions: HashMap::new()
        }
    }

    pub fn compute_transaction(&mut self, transaction: TransactionEnum) {
        match transaction {
            TransactionEnum::Deposit{client_id,tx_id,amount} => self.handle_deposit(client_id,tx_id,amount),
            TransactionEnum::Withdrawal{client_id,tx_id,amount} => self.handle_withdrawal(client_id,tx_id,amount),
            TransactionEnum::Dispute{client_id,tx_id} => self.handle_dispute(client_id,tx_id),
            TransactionEnum::Resolve{client_id,tx_id} => self.handle_resolve(client_id,tx_id),
        }
    }

    pub fn get_client_list(&self) -> Vec<Client> {
        (&self.client_list).get_all().clone()
    }

    fn handle_deposit(&mut self, client_id: u32, tx_id : u32, amount: f32) {
        let client = self.client_list.get_mut(client_id);
    
        client.total += amount;
        client.available += amount;

        self.transactions.insert(tx_id, 
            (PersistedTransaction::Deposit { client_id, tx_id,  amount },TransactionState::None));
    }

    fn handle_withdrawal(&mut self, client_id: u32, _ : u32, amount: f32) {
        let client = self.client_list.get_mut(client_id);
        
        if client.total >= amount || client.available >= amount {
            client.total -= amount;
            client.available -= amount;

        }
    }

    fn handle_dispute(&mut self, _: u32, tx_id : u32) {
        let (disputed,state) = match self.transactions.get(&tx_id){
            Some(tx) => tx,
            None => return,
        };

        if matches!(state, &TransactionState::Disputed) {
            return
        }
        
        match disputed {
            PersistedTransaction::Deposit { client_id, tx_id: _, amount } => {
                let client = self.client_list.get_mut(*client_id);
                client.available -= amount;
                client.held += amount;
            }
        }

        self.transactions.insert(tx_id, (disputed.clone(),TransactionState::Disputed));
    }

    fn handle_resolve(&mut self, _: u32, tx_id : u32) {
        let (disputed,state) = match self.transactions.get(&tx_id){
            Some(tx) => tx,
            None => return,
        };

        if matches!(state, &TransactionState::None) {
            return
        }

        match disputed {
            PersistedTransaction::Deposit { client_id, tx_id: _, amount } => {
                let client = self.client_list.get_mut(*client_id);
                client.available += amount;
                client.held -= amount;
            }
        }

        self.transactions.insert(tx_id, (disputed.clone(),TransactionState::None));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn when_deposit_should_increase_total_and_available() {
        let mut engine = TransactionEngine::new();

        engine.compute_transaction(TransactionEnum::Deposit { 
            client_id: 1, 
            tx_id: 1, 
            amount: 10.0 
        });
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

        engine.compute_transaction(TransactionEnum::Deposit{
            client_id: 1,
            tx_id: 1,
            amount: 30.0
        });
        engine.compute_transaction(TransactionEnum::Withdrawal{
            client_id: 1,
            tx_id: 2,
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

        engine.compute_transaction(TransactionEnum::Deposit{
            client_id: 1,
            tx_id: 1,
            amount: 50.0
        });
        engine.compute_transaction(TransactionEnum::Withdrawal{
            client_id: 1,
            tx_id: 2,
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
        let transaction = TransactionEnum::Deposit{
            client_id: 1,
            tx_id: 1,
            amount: 10.0
        };

        engine.compute_transaction(transaction);
        
        assert_eq!(engine.transactions.len(),1);
        let (tx,state) = engine.transactions.get(&1).unwrap();

        if let PersistedTransaction::Deposit { client_id, tx_id, amount } = tx {
            assert!(matches!(state,TransactionState::None));
            assert_eq!(*tx_id,1);
            assert_eq!(*client_id,1);
            assert_eq!(*amount,10.0);
        } else {
            panic!()
        }   
        
    }


    #[test]
    fn when_dispute_on_deposit_should_decrease_available_increase_held() {
        let mut engine = TransactionEngine::new();

        engine.compute_transaction(TransactionEnum::Deposit{
            client_id: 1,
            tx_id: 1,
            amount: 10.0
        });
        engine.compute_transaction(TransactionEnum::Dispute{
            client_id: 1,
            tx_id: 1,
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

        engine.compute_transaction(TransactionEnum::Deposit{
            client_id: 1,
            tx_id: 1,
            amount: 10.0
        });
        engine.compute_transaction(TransactionEnum::Dispute{
            client_id: 1,
            tx_id: 1
        });
        engine.compute_transaction(TransactionEnum::Dispute{
            client_id: 1,
            tx_id: 1
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

        engine.compute_transaction(TransactionEnum::Dispute{
            client_id: 1,
            tx_id: 1,
        });

        assert_eq!(0,engine.transactions.len());
        assert_eq!(0,engine.client_list.get_all().len())
    }
    #[test]
    fn when_resolve_on_missing_tx_should_do_nothing() {
        let mut engine = TransactionEngine::new();

        engine.compute_transaction(TransactionEnum::Resolve {
            client_id: 1,
            tx_id: 1,
        });

        assert_eq!(0,engine.transactions.len());
        assert_eq!(0,engine.client_list.get_all().len())
    }

    #[test]
    fn when_resolve_on_not_disputed_tx_should_do_nothing() {
        let mut engine = TransactionEngine::new();

        engine.compute_transaction(TransactionEnum::Deposit{
            client_id: 1,
            tx_id: 1,
            amount: 10.0
        });
        engine.compute_transaction(TransactionEnum::Resolve {
            client_id: 1,
            tx_id: 1,
        });

        assert_eq!(1,engine.transactions.len());

        let (_,state) = engine.transactions.get(&1).unwrap();
        assert!(matches!(state,TransactionState::None));

        let client = engine.client_list.get_mut(1);
        assert_eq!(client.total,10.0);
        assert_eq!(client.available,10.0);
        assert_eq!(client.held,0.0)
    }

    #[test]
    fn when_resolve_should_revert_dispute() {
        let mut engine = TransactionEngine::new();

        engine.compute_transaction(TransactionEnum::Deposit{
            client_id: 1,
            tx_id: 1,
            amount: 10.0
        });
        engine.compute_transaction(TransactionEnum::Dispute{
            client_id: 1,
            tx_id: 1,
        });
        engine.compute_transaction(TransactionEnum::Resolve {
            client_id: 1,
            tx_id: 1,
        });

        assert_eq!(1,engine.transactions.len());

        let (_,state) = engine.transactions.get(&1).unwrap();
        assert!(matches!(state,TransactionState::None));

        let client = engine.client_list.get_mut(1);
        assert_eq!(client.total,10.0);
        assert_eq!(client.available,10.0);
        assert_eq!(client.held,0.0)
    }
}




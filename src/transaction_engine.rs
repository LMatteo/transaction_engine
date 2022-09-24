use std::{ collections::HashMap};

use serde::Deserialize;
use serde::Serialize;


pub enum Transaction {
    Deposit{client_id: u16, tx_id : u32, amount: f64},
    Withdrawal{client_id: u16, tx_id : u32, amount: f64},
    Dispute{client_id: u16, tx_id : u32},
    Resolve{client_id: u16, tx_id : u32},
    Chargeback{client_id: u16, tx_id : u32},
} 

#[derive(Clone)]
pub enum PersistedTransaction{
    Deposit{client_id: u16, tx_id : u32, amount: f64},
}

#[derive(Clone,Copy,Debug, Deserialize, Serialize)]
pub struct Client {
    client: u16,
    available: f64,
    held: f64,
    total: f64,
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
    transactions: HashMap<u32,(PersistedTransaction,TransactionState)>,
}

impl TransactionEngine {
    pub fn new() -> TransactionEngine {
        TransactionEngine{
            client_list: ClientList::new(),
            transactions: HashMap::new()
        }
    }

    pub fn compute_transaction(&mut self, transaction: Transaction) {
        match transaction {
            Transaction::Deposit{client_id,tx_id,amount} => self.handle_deposit(client_id,tx_id,amount),
            Transaction::Withdrawal{client_id,tx_id,amount} => self.handle_withdrawal(client_id,tx_id,amount),
            Transaction::Dispute{client_id,tx_id} => self.handle_dispute(client_id,tx_id),
            Transaction::Resolve{client_id,tx_id} => self.handle_resolve(client_id,tx_id),
            Transaction::Chargeback{client_id,tx_id} => self.handle_chargeback(client_id,tx_id),
        }
    }

    pub fn get_client_list(&self) -> Vec<Client> {
        (&self.client_list).get_all().clone()
    }

    fn handle_deposit(&mut self, client_id: u16, tx_id : u32, amount: f64) {
        let client = self.client_list.get_mut(client_id);

        if client.locked {
            return
        }
    
        client.total += amount;
        client.available += amount;

        self.transactions.insert(tx_id, 
            (PersistedTransaction::Deposit { client_id, tx_id,  amount },TransactionState::None));
    }

    fn handle_withdrawal(&mut self, client_id: u16, _ : u32, amount: f64) {
        let client = self.client_list.get_mut(client_id);

        if client.locked {
            return
        }
        
        if client.total >= amount || client.available >= amount {
            client.total -= amount;
            client.available -= amount;

        }
    }

    fn handle_dispute(&mut self, _: u16, tx_id : u32) {
        let (disputed,state) = match self.transactions.get(&tx_id){
            Some(tx) => tx,
            None => return,
        };

        if !matches!(state, &TransactionState::None) {
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

    fn handle_resolve(&mut self, _: u16, tx_id : u32) {
        let (disputed,state) = match self.transactions.get(&tx_id){
            Some(tx) => tx,
            None => return,
        };

        if !matches!(state, &TransactionState::Disputed) {
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

    fn handle_chargeback(&mut self, _: u16, tx_id : u32) {
        let (disputed,state) = match self.transactions.get(&tx_id){
            Some(tx) => tx,
            None => return,
        };

        if !matches!(state, &TransactionState::Disputed) {
            return
        }

        match disputed {
            PersistedTransaction::Deposit { client_id, tx_id: _, amount } => {
                let client = self.client_list.get_mut(*client_id);
                client.total -= amount;
                client.held -= amount;
                client.locked = true;
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

        engine.compute_transaction(Transaction::Deposit { 
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
    fn when_deposit_on_client_locked_should_do_nothing() {
        let mut engine = TransactionEngine::new();

        let locked = engine.client_list.get_mut(1);
        locked.locked = true;

        engine.compute_transaction(Transaction::Deposit { 
            client_id: 1, 
            tx_id: 1, 
            amount: 10.0 
        });
        let clients = engine.get_client_list();

        assert_eq!(clients.len(),1);
        let client = clients.get(0).unwrap();
        assert_eq!(client.available,0.0);
        assert_eq!(client.total,0.0);
        assert_eq!(client.client,1);
    }

    #[test]
    fn when_withdrawal_and_fund_available_should_decrease_total_and_available() {
        let mut engine = TransactionEngine::new();

        engine.compute_transaction(Transaction::Deposit{
            client_id: 1,
            tx_id: 1,
            amount: 30.0
        });

        let locked = engine.client_list.get_mut(1);
        locked.locked = true;

        engine.compute_transaction(Transaction::Withdrawal{
            client_id: 1,
            tx_id: 2,
            amount: 20.0
        });
        let clients = engine.get_client_list();

        assert_eq!(clients.len(),1);
        let client = clients.get(0).unwrap();
        assert_eq!(client.available,30.0);
        assert_eq!(client.total,30.0);
        assert_eq!(client.client,1);
    }

    #[test]
    fn when_withdrawal_on_locked_client_should_do_nothing() {
        let mut engine = TransactionEngine::new();

        engine.compute_transaction(Transaction::Deposit{
            client_id: 1,
            tx_id: 1,
            amount: 30.0
        });
        engine.compute_transaction(Transaction::Withdrawal{
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

        engine.compute_transaction(Transaction::Deposit{
            client_id: 1,
            tx_id: 1,
            amount: 50.0
        });
        engine.compute_transaction(Transaction::Withdrawal{
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
        let transaction = Transaction::Deposit{
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

        engine.compute_transaction(Transaction::Deposit{
            client_id: 1,
            tx_id: 1,
            amount: 10.0
        });
        engine.compute_transaction(Transaction::Dispute{
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

        engine.compute_transaction(Transaction::Deposit{
            client_id: 1,
            tx_id: 1,
            amount: 10.0
        });
        engine.compute_transaction(Transaction::Dispute{
            client_id: 1,
            tx_id: 1
        });
        engine.compute_transaction(Transaction::Dispute{
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

        engine.compute_transaction(Transaction::Dispute{
            client_id: 1,
            tx_id: 1,
        });

        assert_eq!(0,engine.transactions.len());
        assert_eq!(0,engine.client_list.get_all().len())
    }
    #[test]
    fn when_resolve_on_missing_tx_should_do_nothing() {
        let mut engine = TransactionEngine::new();

        engine.compute_transaction(Transaction::Resolve {
            client_id: 1,
            tx_id: 1,
        });

        assert_eq!(0,engine.transactions.len());
        assert_eq!(0,engine.client_list.get_all().len())
    }

    #[test]
    fn when_resolve_on_not_disputed_tx_should_do_nothing() {
        let mut engine = TransactionEngine::new();

        engine.compute_transaction(Transaction::Deposit{
            client_id: 1,
            tx_id: 1,
            amount: 10.0
        });
        engine.compute_transaction(Transaction::Resolve {
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

        engine.compute_transaction(Transaction::Deposit{
            client_id: 1,
            tx_id: 1,
            amount: 10.0
        });
        engine.compute_transaction(Transaction::Dispute{
            client_id: 1,
            tx_id: 1,
        });
        engine.compute_transaction(Transaction::Resolve {
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
    fn when_chargeback_on_missing_tx_should_do_nothing() {
        let mut engine = TransactionEngine::new();

        engine.compute_transaction(Transaction::Chargeback{
            client_id: 1,
            tx_id: 1,
        });

        assert_eq!(0,engine.transactions.len());
        assert_eq!(0,engine.client_list.get_all().len())
    }

    #[test]
    fn when_chargeback_on_not_disputed_tx_should_do_nothing() {
        let mut engine = TransactionEngine::new();

        engine.compute_transaction(Transaction::Deposit{
            client_id: 1,
            tx_id: 1,
            amount: 10.0
        });
        engine.compute_transaction(Transaction::Chargeback {
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
    fn when_chargeback_should_freeze_and_withdraw() {
        let mut engine = TransactionEngine::new();

        engine.compute_transaction(Transaction::Deposit{
            client_id: 1,
            tx_id: 1,
            amount: 10.0
        });
        engine.compute_transaction(Transaction::Dispute{
            client_id: 1,
            tx_id: 1,
        });
        engine.compute_transaction(Transaction::Chargeback {
            client_id: 1,
            tx_id: 1,
        });

        assert_eq!(1,engine.transactions.len());

        let (_,state) = engine.transactions.get(&1).unwrap();
        assert!(matches!(state,TransactionState::None));

        let client = engine.client_list.get_mut(1);
        assert_eq!(client.total,0.0);
        assert_eq!(client.available,0.0);
        assert_eq!(client.held,0.0);
        assert_eq!(client.locked,true);
    }
}




use std::collections::HashMap;
use csv::{StringRecord, Writer};
use std::error::Error;
use std::fmt;

use crate::transaction::{Transaction, TxType, PaymentStatus};
use crate::client::Clients;

#[derive(Debug, PartialEq)]
pub enum LedgerError {
    ClientNotFound(u16),
    MalformedRequest,
    NotEnoughFunds { client: u16, requested: f64, available: f64 },
    InvalidDispute(u32),
}
impl fmt::Display for LedgerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LedgerError::ClientNotFound(id) => write!(f, "Client {} not found", id),
            LedgerError::MalformedRequest => write!(f, "Malformed transaction request"),
            LedgerError::NotEnoughFunds { client, requested, available } =>
                write!(f, "Client {}: insufficient funds (requested {}, available {})", client, requested, available),
            LedgerError::InvalidDispute(tx) => write!(f, "Invalid dispute for tx {}", tx),
        }
    }
}
impl std::error::Error for LedgerError {}

pub struct Ledger {
    ledger: HashMap<u32, Transaction>,
    clients: Clients,
}

impl Ledger {
    pub fn new() -> Ledger {
        Ledger { 
            ledger: HashMap::new(),
            clients: Clients::new(), 
        }
    }

    pub fn print_summary(&self) -> Result<(), Box<dyn Error>> {
        let mut wtr = Writer::from_writer(std::io::stdout());

        wtr.write_record(&["client", "available", "held", "total", "locked"])?;

        for client in self.clients.clients.values() {
            wtr.write_record(&[
                client.id.to_string(),
                format!("{:.4}", client.available),
                format!("{:.4}", client.held),
                format!("{:.4}", client.total),
                client.locked.to_string(),
            ])?;
        }

        wtr.flush()?;
        Ok(())
    }

    pub fn process(&mut self, record: StringRecord) {
        match Transaction::create_transaction(&record) {
            Ok(tx) => {
                if let Err(e) = self.process_transaction(tx) {
                    eprintln!("Error applying transaction: {}", e);
                }
            }
            Err(e) => eprintln!("Error processing record: {}", e),
        }
    }

    fn process_transaction(&mut self, tx: Transaction) -> Result<(), LedgerError> {
        match tx.tx_type {
            TxType::Deposit => self.deposit(&tx),
            TxType::Withdrawal => self.withdraw( &tx),
            TxType::Dispute => self.dispute(&tx),
            TxType::Resolve => self.resolve(&tx),
            TxType::Chargeback => self.chargeback(&tx),
        }
    }

    fn deposit(&mut self, t: &Transaction) -> Result<(), LedgerError> {
        let client = self.clients.add_client(t.client_id);

        if let Some(amount) = t.amount {
            client.available += amount;
            client.total += amount;
            self.ledger.insert(t.tx_id, t.clone());
            return Ok(())
        } else {
            return Err(LedgerError::MalformedRequest); // should never happen - double check azy
        }
    }

    fn withdraw(&mut self, t: &Transaction) -> Result<(), LedgerError> {
        let client = self.clients.add_client(t.client_id);

        if let Some(amount) = t.amount {
            // Assumption-1: Only withdraw if available > tx amount, so we don't end up with negative balances - please comment 'if statement' below if incorrect
            if client.available >= amount {
                client.available -= amount;
                client.total -= amount;
                self.ledger.insert(t.tx_id, t.clone());
                return Ok(())
            } else {
                return Err(LedgerError::NotEnoughFunds { client: (t.client_id), requested: (amount), available: (client.available) });
            }
        } else {
            return Err(LedgerError::MalformedRequest);
        }
    }

    fn dispute(&mut self, t: &Transaction) -> Result<(), LedgerError> {
        let client = match self.clients.find_client(t.client_id) {
            Some(c) => c,
            None => return Err(LedgerError::ClientNotFound(t.client_id)),
        };
        let tx = match self.ledger.get_mut(&t.tx_id) {
            Some(tx) => tx,
            None => return Err(LedgerError::InvalidDispute(t.tx_id)),
        };
        if let Some(amount) = tx.amount {
            client.held += amount;
            client.available -= amount;
            tx.status = PaymentStatus::Disputed;
            return Ok(());
        } else {
            return Err(LedgerError::MalformedRequest)
        }
    }

    fn resolve(&mut self, t: &Transaction) -> Result<(), LedgerError> {
        let client = match self.clients.find_client(t.client_id) {
            Some(c) => c,
            None => return Err(LedgerError::ClientNotFound(t.client_id)),
        };
        let tx = match self.ledger.get_mut(&t.tx_id) {
            Some(tx) => tx,
            None => return Err(LedgerError::InvalidDispute(t.tx_id)),
        };
        if !matches!(tx.status, PaymentStatus::Disputed) {
            return Err(LedgerError::InvalidDispute(t.tx_id))
        }
        if let Some(amount) = tx.amount {
            client.held -= amount;
            client.available += amount;
            // Assumption-2: Mark transaction as no longer disputed - please comment line below if incorrect
            tx.status = PaymentStatus::Undisputed;
            return Ok(());
        } else { return Err(LedgerError::MalformedRequest) } // should never happen
    }

    fn chargeback(&mut self, t: &Transaction) -> Result<(), LedgerError> {
        let client = match self.clients.find_client(t.client_id) {
            Some(c) => c,
            None => return Err(LedgerError::ClientNotFound(t.client_id)),
        };
        let tx = match self.ledger.get_mut(&t.tx_id) {
            Some(tx) => tx,
            None => return Err(LedgerError::InvalidDispute(t.tx_id)),
        };
        if !matches!(tx.status, PaymentStatus::Disputed) {
            return Err(LedgerError::InvalidDispute(t.tx_id))
        }
        if let Some(amount) = tx.amount {
            client.held -= amount;
            client.total -= amount;
            client.locked = true; 
            // my gut feeling tells me that this is still a disputed charge, so I wont do the same (switch tx.status) 
            // as I did in resolve and change the PaymentStatus - please add if incorrect? :)
            return Ok(());
        } else { return Err(LedgerError::MalformedRequest) } // should never happen
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{transaction::{PaymentStatus, Transaction}};

    fn create_tx(tx_type: TxType, client_id: u16, tx_id: u32, amount: Option<f64>) -> Transaction {
        Transaction {
            tx_type,
            client_id,
            tx_id,
            amount,
            status: PaymentStatus::Undisputed,
        }
    }

    #[test]
    fn test_deposit_increases_balance() {
        let mut ledger = Ledger::new();
        let tx = create_tx(TxType::Deposit, 1, 1, Some(1.0));
        assert!(ledger.deposit(&tx).is_ok());

        let client = ledger.clients.find_client(1).unwrap();
        assert_eq!(client.available, 1.0);
        assert_eq!(client.total, 1.0);
    }

    #[test]
    fn test_withdraw_decreases_balance() {
        let mut ledger = Ledger::new();
        let tx = create_tx(TxType::Deposit, 1, 1, Some(10.0));
        ledger.deposit(&tx).unwrap();

        let tx = create_tx(TxType::Withdrawal, 1, 2, Some(4.0));
        assert!(ledger.withdraw(&tx).is_ok());

        let client = ledger.clients.find_client(1).unwrap();
        assert_eq!(client.available, 6.0);
        assert_eq!(client.total, 6.0);
    }

    #[test]
    fn test_disputes_and_resolve_work_correctly() {
        let mut ledger = Ledger::new();
        let tx = create_tx(TxType::Deposit, 1, 1, Some(1.0));
        assert!(ledger.deposit(&tx).is_ok());

        let tx = create_tx(TxType::Dispute, 1, 1, None);
        assert!(ledger.dispute(&tx).is_ok());

        let client = ledger.clients.find_client(1).unwrap();
        let transaction = ledger.ledger.get(&1).unwrap();

        assert_eq!(client.available, 0.0);
        assert_eq!(client.held, 1.0);
        assert_eq!(client.total, 1.0);
        assert!(matches!(transaction.status, PaymentStatus::Disputed));

        let tx = create_tx(TxType::Resolve, 1, 1, None);
        assert!(ledger.resolve(&tx).is_ok());
        let client = ledger.clients.find_client(1).unwrap();
        let transaction = ledger.ledger.get(&1).unwrap();
        assert_eq!(client.available, 1.0);
        assert_eq!(client.held, 0.0);
        assert_eq!(client.total, 1.0);
        assert!(matches!(transaction.status, PaymentStatus::Undisputed));
    }

    #[test]
    fn test_chargeback_works_correctly() {
        let mut ledger = Ledger::new();
        let tx = create_tx(TxType::Deposit, 1, 1, Some(1.0));
        assert!(ledger.deposit(&tx).is_ok());

        let tx = create_tx(TxType::Dispute, 1, 1, None);
        assert!(ledger.dispute(&tx).is_ok());

        let tx = create_tx(TxType::Chargeback, 1, 1, None);
        assert!(ledger.chargeback(&tx).is_ok());

        let client = ledger.clients.find_client(1).unwrap();
        let transaction = ledger.ledger.get(&1).unwrap();

        assert_eq!(client.available, 0.0);
        assert_eq!(client.held, 0.0);
        assert_eq!(client.total, 0.0);
        assert!(client.locked);
        assert!(matches!(transaction.status, PaymentStatus::Disputed));
    }

    #[test]
    fn test_withdraw_over_balance_fails() {
        let mut ledger = Ledger::new();
        let tx = create_tx(TxType::Deposit, 1, 1, Some(1.0));
        assert!(ledger.deposit(&tx).is_ok());

        let tx= create_tx(TxType::Withdrawal, 1, 2, Some(1.1));
        let res = ledger.withdraw(&tx);

        match res {
            Err(LedgerError::NotEnoughFunds { client, requested, available }) => {
                assert_eq!(client, 1);
                assert_eq!(requested, 1.1);
                assert_eq!(available, 1.0);
            } other => panic!("Expected NotEnoughFunds error, got {:?}", other),
        }
    }

    #[test]
    fn test_deposit_or_withdraw_with_no_amount_fails() {
        let mut ledger = Ledger::new();
        let tx = create_tx(TxType::Deposit, 1, 1, None);
        let res = ledger.deposit(&tx);

        match res {
            Err(LedgerError::MalformedRequest) => {},
            other => panic!("Expected MalformedRequest error, got {:?}", other),
        }

        let tx = create_tx(TxType::Withdrawal, 1, 1, None);
        let res = ledger.withdraw(&tx);

        match res {
            Err(LedgerError::MalformedRequest) => {},
            other => panic!("Expected MalformedRequest error, got {:?}", other),
        }
    }

    #[test]
    fn test_disputes_fails() {
        let mut ledger = Ledger::new();
        let tx = create_tx(TxType::Deposit, 1, 1, Some(1.0));
        assert!(ledger.deposit(&tx).is_ok());

        let tx = create_tx(TxType::Dispute, 2, 1, None);
        let res = ledger.dispute(&tx);
        match res {
            Err(LedgerError::ClientNotFound(2)) => {},
            other => panic!("Expected ClientNotFound error, got {:?}", other),
        }

        let tx = create_tx(TxType::Dispute, 1, 2, None);
        let res = ledger.dispute(&tx);
        match res {
            Err(LedgerError::InvalidDispute(2)) => {},
            other => panic!("Expected InvalidDispute error, got {:?}", other),
        }
    }

    #[test]
    fn test_resolve_chargeback_undisputed_tx_fails() {
        let mut ledger = Ledger::new();
        let tx = create_tx(TxType::Deposit, 1, 1, Some(5.0));
        ledger.deposit(&tx).unwrap();

        let tx = create_tx(TxType::Resolve, 1, 1, None);
        let res = ledger.chargeback(&tx);
        assert!(matches!(res, Err(LedgerError::InvalidDispute(1))));

        let tx = create_tx(TxType::Chargeback, 1, 1, None);
        let res = ledger.chargeback(&tx);
        assert!(matches!(res, Err(LedgerError::InvalidDispute(1))));
    }

}
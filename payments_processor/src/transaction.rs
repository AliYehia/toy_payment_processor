use std::fmt;
use std::error::Error;
use csv::StringRecord;

#[derive(Clone, PartialEq, Debug)]
pub enum TxType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

impl TxType {
    fn from_str(s: &str) -> Result<TxType, TransactionError> {
        match s.trim().to_lowercase().as_str() {
            "deposit" => Ok(TxType::Deposit),
            "withdrawal" => Ok(TxType::Withdrawal),
            "dispute" => Ok(TxType::Dispute),
            "resolve" => Ok(TxType::Resolve),
            "chargeback" => Ok(TxType::Chargeback),
            other => Err(TransactionError::UnknownTxType(other.to_string())),
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum PaymentStatus {
    Disputed,
    Undisputed,
}

#[derive(Clone, Debug)]
pub struct Transaction {
    pub tx_type: TxType,
    pub tx_id: u32,
    pub client_id: u16,
    pub amount: Option<f64>,
    pub status: PaymentStatus,
}

#[derive(Debug)]
pub enum TransactionError {
    TooFewFields(Vec<String>),
    UnknownTxType(String),
    ParseError { field: String, source: Box<dyn Error> },
}

impl fmt::Display for TransactionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransactionError::TooFewFields(fields) => write!(f, "Too few fields: {:?}", fields),
            TransactionError::UnknownTxType(s) => write!(f, "Unknown transaction type: {}", s),
            TransactionError::ParseError { field, source } => write!(f, "Failed to parse {}: {}", field, source),
        }
    }
}

impl Error for TransactionError {}

impl Transaction {
    pub fn create_transaction(record: &StringRecord) -> Result<Transaction, TransactionError> {
        let fields: Vec<String> = record.iter().map(|f| f.trim().to_string()).collect();

        if fields.len() < 3 {
            return Err(TransactionError::TooFewFields(fields));
        }

        let tx_type = TxType::from_str(&fields[0])?;
        let client_id = fields[1].parse()
            .map_err(|e| TransactionError::ParseError { field: "client_id".to_string(), source: Box::new(e) })?;
        let tx_id = fields[2].parse()
            .map_err(|e| TransactionError::ParseError { field: "tx_id".to_string(), source: Box::new(e) })?;

        let amount = if fields.len() >= 4 && !fields[3].is_empty() {
            Some(fields[3].parse()
                .map_err(|e| TransactionError::ParseError { field: "amount".to_string(), source: Box::new(e) })?)
        } else {
            None
        };

        Ok(Transaction { tx_type, client_id, tx_id, amount, status: PaymentStatus::Undisputed })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use csv::StringRecord;

    #[test]
    fn test_create_transaction_valid() {
        let record = StringRecord::from(vec!["deposit", "1", "1",
                                                  "100.0"]);
        let tx = Transaction::create_transaction(&record).unwrap();
        assert_eq!(tx.tx_type, TxType::Deposit);
        assert_eq!(tx.client_id, 1);
        assert_eq!(tx.tx_id, 1);
        assert_eq!(tx.amount, Some(100.0));
    }

    #[test]
    fn test_create_transaction_invalid_tx_type() {
        let record = StringRecord::from(vec!["invalid", "1", "1",
                                                    "100.0"]);
        let err = Transaction::create_transaction(&record).unwrap_err();
        match err {
            TransactionError::UnknownTxType(s) => assert_eq!(s, "invalid"),
            _ => panic!("Expected UnknownTxType error"),
        }
    }
    #[test]
    fn test_create_transaction_too_few_fields() {
        let record = StringRecord::from(vec!["deposit", "1"]);
        let err = Transaction::create_transaction(&record).unwrap_err();
        match err {
            TransactionError::TooFewFields(fields) => assert_eq!(fields, vec!["deposit", "1"]),
            _ => panic!("Expected TooFewFields error"),
        }
    }

    #[test]
    fn test_create_transaction_parse_error() {
        let record = StringRecord::from(vec!["deposit", "abc", "1",
                                                    "100.0"]);
        let err = Transaction::create_transaction(&record).unwrap_err();
        match err {
            TransactionError::ParseError { field, .. } => assert_eq!(field, "client_id"),
            _ => panic!("Expected ParseError error"),
        }
    }

}
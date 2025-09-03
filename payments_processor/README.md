## Paymet processor

Rust project that reads a CSV of transactions, processes it, then outputs a CSV of the clients summary at the end

### Usage

cargo run -- transactions.csv > accounts.csv

### Functional Requirements
* Reads CSV files and processes each line
* Processes all requests: Deposit, Withdrawal, Dispute, Resolve, Chargeback
* Prints out a CSV with the status of all the clients after processing

### Non functional requirements
* Modular code organization
* Clean error handling
* Unit tests for all major functions
* Avoid panics and crashes
* Streaming values through memory using csv::Reader

### Design

I chose to keep the design modular. Each concept in it's own file.

transaction.rs:
* Define the enum of the types of transactions (Deposit, Dispute, Resolve..)
* Define the struct Transaction which will hold the transaction type, the transaction id, the client id, the amount. Basically all the info from each line will be transformed into that struct.

client.rs:
* Define a struct for Client (the id, the available amount in their account, held amount in their account, whether it is locked or not)
* Define a struct for Clients, a wrapper around Clinet that contains a hashmap for quick lookup of clients, it will be u16 (client id) to Client (Client struct)

ledger.rs:
* Define a struct that will hold a hashmap to store all the transactions for quick lookup. Used this mostly for disputes
* This will be the main logical engine which will perform the actions of each transaction. It will also update the Clients struct

main.rs:
* Open the file, read the contents, create a ledger and send each transaction to be processed

### Assumptions Made During Implementation

* When doing a withdrawal, I check if the balance allows by checking available funds and not processing that request all together. If incorrect, please change by following the comment <Assumption-1:> 
* When going from Disputed to Resolved/Chargeback, I changed the transaction type internally to undisputed, but it's not stated explicitly in the requirements. Might affect tests on it if we have double resolve or something..If incorrect, please change by following the comment <Assumption-2:> 

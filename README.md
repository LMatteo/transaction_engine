# Transaction Engine

The engine can be run with 

```bash
cargo run -- input.csv
```

Sample input files can be found under ./resources/tests

## Feature

Deposit, withdrawal, dispute, resolve and chargeback are implemented.

Deposit and withdrawal can be applied only to client whose account is not locked.

Dispute, resolve and chargeback can only be applied on a deposit.

## Testing 

The application is tested through unit tests and integration tests. 

Integration test the whole application by launching the binary and checking the output.
Sample input files used in the tests can be found under resources/tests  

## Error

Error are printed to stderr, they do not interrupt the application.
If there is an error whend handling a transaction, the transaction in ignored. 

## Data Read and memory

The data read from the input are streamed. They are read, handled and then dropped.
Only a single transaction is kept in memory at once. 
No history is kept, except for deposits which can be disputed and need to be retrieved. 
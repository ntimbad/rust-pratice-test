# Exercise

## Implementation Details: 

- State 1 : Transaction is in a Deposit or Withdrawal state which is the initial state
- State 2 : Transaction is in the Dispute state
- State 3 : Transaction is either in Resolved state or in Chargeback state

### Concurrency : 

- To achieve performance concurrency is used to run transactions in parallel.
- Not all transactions can be run in parallel, obviously. 
- For a single client transactions need to be run one after the other ensuring Serializability and atomicity for transactions.
- For overcoming this I have used batching and grouping transactions by client and then running them.
- Batch size is adjustable but the program doesn't want external parameters so yeet that. 

### No automated tests : 
- Running well over prescribed time of 2 to 3 hours for this project. 
- Couldn't find time to get this. Although some cases I tested against are included in data folder.

## Assumptions: 

### Write Transaction states to file :
- Need to do this since can't fit all in memory. Otherwise I would use a DB which by the looks I dont have the previlege for.

### Possible Transaction states :
- Only State1 -> State2 -> State3
- None of the statements indicate to me otherwise.

### Withdrawal disputes : 
- This flow is not described properly and is a figment of my imagination

### Withdrawals above balance: 
- These are ignored

### All other balance issues:
- Program just halts. Give it good input. If you want to ignore an issue you can consider throwing RuntimeError::Recoverable instead of
  RuntimeError::NonRecoverable
- Tons of examples of doing this can be found in client_account.rs file


# Final thoughts: 
- Could do some more type safety magic in client_account.rs but 2 things repetitive code and have written a proc macro before this myself. 
  Secondly the Serializable Transaction type is isolated within the module and hence complexity is hidden. 
- Project felt like a 1 hour thing when I read it, but I feel I still can spend 3 4 hours improving minor stuff here and there and rethinking how I did concurrency here. 
- Spent some holiday time on this, Reminds me why I love this rusty language.
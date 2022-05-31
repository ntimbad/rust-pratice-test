use crate::client_account::ClientAccount;
use crate::constants::{BATCH_SIZE, TEMP_DIRECTORY_LOC};
use crate::error::RuntimeError::{NonRecoverable, Recoverable};
use crate::error::{RuntimeError, RuntimeErrorType};
use crate::transaction::{CSVTransaction, CSVTransactionType};
use itertools::Itertools;
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct Runner {
    file: PathBuf,
    client_map: HashMap<u16, Arc<Mutex<ClientAccount>>>, // threaded interior mutability
}

impl Runner {
    pub fn new(file: PathBuf) -> Self {
        Self {
            file,
            client_map: HashMap::new(),
        }
    }

    pub async fn run(&mut self) -> Result<(), RuntimeError> {
        tokio::fs::remove_dir_all(PathBuf::from(TEMP_DIRECTORY_LOC)).await; // do if possible
        tokio::fs::create_dir_all(PathBuf::from(TEMP_DIRECTORY_LOC))
            .await
            .map_err(|e| NonRecoverable(RuntimeErrorType::TransactionFileOps(e.to_string())))?;

        let mut result = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_path(self.file.clone())
            .map_err(|e| NonRecoverable(RuntimeErrorType::CSVFileReadWriteError(e.to_string())))?;

        for res in &result.records().chunks(BATCH_SIZE) {
            // process BATCH_SIZE records at once
            let transactions: Vec<CSVTransaction> = res
                .map(|line_result| match line_result {
                    Ok(line) => {
                        let csv_transaction_result = CSVTransaction::try_from(line);
                        match csv_transaction_result {
                            Ok(csv_transaction) => Some(csv_transaction),
                            Err(e) => {
                                println!("{:?}", e);
                                None
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Result invalid {:?}", e);
                        None
                    }
                })
                .filter(|x| x.is_some())
                .map(|e| e.expect("Filtered out none values"))
                .collect();

            let csv_transactions_grouped_by_client = {
                // doing this as group by apparently is unstable
                let mut grouped_transactions: HashMap<u16, Vec<CSVTransaction>> = HashMap::new();
                for t in transactions {
                    grouped_transactions
                        .entry(t.client_id)
                        .or_insert_with(Vec::new);
                    grouped_transactions
                        .get_mut(&t.client_id)
                        .expect("above line makes sure we have a value")
                        .push(t);
                }
                grouped_transactions
            };

            let mut handles = vec![];
            for (key, client_transactions) in csv_transactions_grouped_by_client.into_iter() {
                let account = {
                    self.client_map
                        .entry(key)
                        .or_insert_with(|| Arc::new(Mutex::new(ClientAccount::new_account(key))));
                    self.client_map.get(&key)
                }
                .expect("Will be present since is defaulted if not present")
                .clone();

                let handle = tokio::spawn(async move {
                    for transaction in client_transactions {
                        match transaction.transaction_type {
                            CSVTransactionType::Deposit => {
                                //takes a state 1 transaction and writes it
                                if let Err(NonRecoverable(e_type)) = account
                                    .lock()
                                    .await
                                    .execute_deposit(transaction.try_into().unwrap())
                                    .await
                                {
                                    panic!("{:?}", e_type);
                                }
                            }
                            CSVTransactionType::Withdrawal => {
                                //takes a state 1 transaction and writes it
                                if let Err(NonRecoverable(e_type)) = account
                                    .lock()
                                    .await
                                    .execute_withdrawal(transaction.try_into().unwrap())
                                    .await
                                {
                                    panic!("{:?}", e_type);
                                }
                            }
                            CSVTransactionType::Dispute => {
                                //Finds a state1 transaction in file
                                // Converts it into state 2
                                if let Err(NonRecoverable(e_type)) = account
                                    .lock()
                                    .await
                                    .execute_dispute(transaction.try_into().unwrap())
                                    .await
                                {
                                    panic!("{:?}", e_type);
                                }
                            }
                            CSVTransactionType::Resolve => {
                                //Finds a state2 transaction in file
                                //Writes it back to state 3
                                if let Err(NonRecoverable(e_type)) = account
                                    .lock()
                                    .await
                                    .execute_resolve(transaction.try_into().expect(""))
                                    .await
                                {
                                    panic!("{:?}", e_type);
                                }
                            }
                            CSVTransactionType::Chargeback => {
                                //Finds a state2 transaction in file
                                //Writes it back to state 3
                                if let Err(NonRecoverable(e_type)) = account
                                    .lock()
                                    .await
                                    .execute_chargeback(transaction.try_into().expect(""))
                                    .await
                                {
                                    panic!("{:?}", e_type);
                                }
                            }
                        }
                    }
                });
                handles.push(handle);
            }
            //await before starting the next batch
            for x in futures::future::join_all(handles).await {
                x.map_err(|e| {
                    RuntimeError::NonRecoverable(RuntimeErrorType::JoinError(
                        "Not all futures succeeded".to_string(),
                    ))
                })?
            }
        }

        tokio::fs::remove_dir_all(PathBuf::from(TEMP_DIRECTORY_LOC))
            .await
            .map_err(|e| Recoverable(RuntimeErrorType::CSVFileReadWriteError(e.to_string())));
        Ok(())
    }

    pub async fn print_all_accounts(&self) {
        println!("client, available, held, total, locked");
        for (client, value) in &self.client_map {
            let guard = value.lock().await;
            println!(
                "{},{},{},{},{}",
                client,
                guard.available,
                guard.held,
                (guard.available.clone() + guard.held.clone()),
                guard.locked
            );
        }
    }

    /// Intended for tests only. Helps verify with automated tests.
    pub async fn get_cloned_account_snapshot(&self, account_id: u16) -> Option<ClientAccount> {
        match &self.client_map.get(&account_id) {
            None => None,
            Some(x) => {
                let guard = x.lock().await;
                let guard = &*guard;
                Some(guard.clone())
            }
        }
    }
}

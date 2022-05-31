use crate::constants::TEMP_DIRECTORY_LOC;
use crate::error::RuntimeErrorType::BalanceIssues;
use crate::error::{RuntimeError, RuntimeErrorType};
use crate::transaction::{
    CSVTransactionType, ChargeBackRequest, DepositRequest, DisputeRequest, ResolveRequest, State1,
    WithdrawalRequest,
};
use bigdecimal::BigDecimal;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone)]
pub struct ClientAccount {
    pub(crate) id: u16,
    pub(crate) available: BigDecimal,
    pub(crate) held: BigDecimal,
    pub(crate) locked: bool,
}

impl ClientAccount {
    pub fn new_account(id: u16) -> Self {
        ClientAccount {
            id,
            available: BigDecimal::from(0),
            held: BigDecimal::from(0),
            locked: false,
        }
    }

    /// Takes a state 1 transaction and writes it
    pub(crate) async fn execute_deposit(&mut self, r: DepositRequest) -> Result<(), RuntimeError> {
        self.ensure_unlocked()?;
        let result = SerializableTransaction::new_from_state1(Box::new(&r))?;
        if let Ok(_) = SerializableTransaction::read(r.0.tx_id).await {
            return Err(RuntimeError::Recoverable(
                RuntimeErrorType::TransactionAlreadyPresent,
            ));
        }
        result.overwrite_state().await?;
        self.available +=
            &r.0.amount
                .expect("Deposit Request makes sure this is there");
        Ok(())
    }

    /// Takes a state 1 transaction and writes it
    pub(crate) async fn execute_withdrawal(
        &mut self,
        r: WithdrawalRequest,
    ) -> Result<(), RuntimeError> {
        self.ensure_unlocked()?;
        self.ensure_balance(
            r.0.amount
                .as_ref()
                .expect("Withdrawal wrapper makes sure amount is present and is positive"),
        )
        .map_err(|e| {
            // Make sure to ignore balance issues here
            if let RuntimeError::NonRecoverable(e) = e {
                RuntimeError::Recoverable(e)
            } else {
                e
            }
        })?;
        if let Ok(_) = SerializableTransaction::read(r.0.tx_id).await {
            return Err(RuntimeError::Recoverable(
                RuntimeErrorType::TransactionAlreadyPresent,
            ));
        }
        let result = SerializableTransaction::new_from_state1(Box::new(&r))?;
        result.overwrite_state().await?;

        self.available -=
            r.0.amount
                .as_ref()
                .expect("Withdrawal wrapper makes sure amount is present and is positive");
        Ok(())
    }

    ///Finds a state1 transaction in file
    /// Converts it into state 2
    pub(crate) async fn execute_dispute(&mut self, r: DisputeRequest) -> Result<(), RuntimeError> {
        self.ensure_unlocked()?;
        let s = match SerializableTransaction::read(r.0.tx_id).await {
            Ok(x) => x,
            Err(_) => {
                return Err(RuntimeError::Recoverable(
                    RuntimeErrorType::WrongTransactionState("Transaction not present".to_string()),
                ));
            }
        };

        match s.state {
            SerializableState::State1 => {}
            _ => {
                return Err(RuntimeError::Recoverable(
                    RuntimeErrorType::WrongTransactionState(
                        "Transaction in wrong state".to_string(),
                    ),
                ));
            }
        }

        let s = s.upgrade_state();
        s.overwrite_state().await?;

        match s.transaction_type {
            SerializableTransactionType::Deposit => {
                self.ensure_balance(&s.amount)?;
                self.available -= &s.amount;
                self.held += &s.amount;
            }
            SerializableTransactionType::Withdrawal => {
                self.held += &s.amount;
            }
        }
        Ok(())
    }

    ///Finds a state2 transaction in file
    ///Writes it back to state 3
    pub(crate) async fn execute_resolve(&mut self, r: ResolveRequest) -> Result<(), RuntimeError> {
        self.ensure_unlocked()?;
        let s = match SerializableTransaction::read(r.0.tx_id).await {
            Ok(x) => x,
            Err(_) => {
                return Err(RuntimeError::Recoverable(
                    RuntimeErrorType::WrongTransactionState("Transaction not present".to_string()),
                ));
            }
        };

        match s.state {
            SerializableState::State2 => {}
            _ => {
                return Err(RuntimeError::Recoverable(
                    RuntimeErrorType::WrongTransactionState(
                        "Transaction in wrong state".to_string(),
                    ),
                ));
            }
        }

        let s = s.upgrade_state();
        s.overwrite_state().await?;
        match s.transaction_type {
            SerializableTransactionType::Deposit => {
                self.available += &s.amount;
                self.ensure_hold_balance(&s.amount)?;
                self.held -= &s.amount;
            }
            SerializableTransactionType::Withdrawal => {
                self.available += &s.amount;
                self.ensure_hold_balance(&s.amount)?;
                self.held -= &s.amount;
            }
        }

        Ok(())
    }

    ///Finds a state2 transaction in file
    ///Writes it back to state 3
    pub(crate) async fn execute_chargeback(
        &mut self,
        r: ChargeBackRequest,
    ) -> Result<(), RuntimeError> {
        self.ensure_unlocked()?;
        let s = match SerializableTransaction::read(r.0.tx_id).await {
            Ok(x) => x,
            Err(_) => {
                return Err(RuntimeError::Recoverable(
                    RuntimeErrorType::WrongTransactionState("Transaction not present".to_string()),
                ));
            }
        };
        match s.state {
            SerializableState::State2 => {}
            _ => {
                return Err(RuntimeError::Recoverable(
                    RuntimeErrorType::WrongTransactionState(
                        "Transaction in wrong state".to_string(),
                    ),
                ));
            }
        }
        let s = s.upgrade_state();
        s.overwrite_state().await?;
        match s.transaction_type {
            SerializableTransactionType::Deposit | SerializableTransactionType::Withdrawal => {
                self.ensure_hold_balance(&s.amount)?;
                self.held -= &s.amount;
                self.locked = true;
            }
        }
        Ok(())
    }

    fn ensure_balance(&self, amount: &BigDecimal) -> Result<(), RuntimeError> {
        match &self.available >= amount {
            true => Ok(()),
            false => {
                let err_string = format!("Given chain of transactions is erroneous account: {:?} required balance {:?} but found {:?}", &self.id, &amount, &self.available);
                Err(RuntimeError::NonRecoverable(BalanceIssues(err_string)))
            }
        }
    }

    fn ensure_hold_balance(&self, amount: &BigDecimal) -> Result<(), RuntimeError> {
        match &self.held >= amount {
            true => Ok(()),
            false => {
                let err_string = format!("Given chain of transactions is erroneous account: {:?} required held balance {:?} but found {:?}", &self.id, &amount, &self.held);
                Err(RuntimeError::NonRecoverable(BalanceIssues(err_string)))
            }
        }
    }

    fn ensure_unlocked(&self) -> Result<(), RuntimeError> {
        if self.locked {
            let err_string = format!("Account is locked {}", self.id);
            return Err(RuntimeError::Recoverable(RuntimeErrorType::LockedAccount(
                err_string,
            )));
        }
        Ok(())
    }
}

#[derive(
    Copy,
    Clone,
    Debug,
    Eq,
    PartialEq,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    strum::Display,
    strum::EnumIter,
    strum::EnumString,
    strum::EnumVariantNames,
    strum::IntoStaticStr,
)]
enum SerializableTransactionType {
    Deposit,
    Withdrawal,
}

#[derive(
    Copy,
    Clone,
    Debug,
    Eq,
    PartialEq,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    strum::Display,
    strum::EnumIter,
    strum::EnumString,
    strum::EnumVariantNames,
    strum::IntoStaticStr,
)]
enum SerializableState {
    State1, // Deposit or Withdrawal
    State2, // Dispute
    State3, // Resolve or Chargeback
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct SerializableTransaction {
    amount: BigDecimal,
    client_id: u16,
    tx_id: u32,
    transaction_type: SerializableTransactionType,
    state: SerializableState,
}

impl SerializableTransaction {
    pub fn new_from_state1(transaction: Box<&dyn State1>) -> Result<Self, RuntimeError> {
        let csv_transaction = transaction.inner();
        let transaction_type = match csv_transaction.transaction_type {
            CSVTransactionType::Deposit => SerializableTransactionType::Deposit,
            CSVTransactionType::Withdrawal => SerializableTransactionType::Withdrawal,
            _ => {
                return Err(RuntimeError::Recoverable(
                    RuntimeErrorType::WrongTransactionState(
                        "Ignoring this transaction".to_string(),
                    ),
                ))
            }
        };
        Ok(Self {
            amount: csv_transaction
                .amount
                .clone()
                .expect("State 1 transactions have an amount"),
            client_id: csv_transaction.client_id,
            tx_id: csv_transaction.tx_id,
            transaction_type,
            state: SerializableState::State1,
        })
    }

    pub(crate) async fn overwrite_state(&self) -> Result<(), RuntimeError> {
        let path = format!("{}{}", TEMP_DIRECTORY_LOC, self.tx_id);
        tokio::fs::remove_file(path.clone()).await.map_err(|_| {
            RuntimeError::NonRecoverable(RuntimeErrorType::TransactionFileOps(
                "Remove file failed".to_string(),
            ))
        });
        let contents = {
            match serde_json::to_string(&self) {
                Ok(x) => x,
                Err(e) => {
                    return Err(RuntimeError::NonRecoverable(RuntimeErrorType::ParseError(
                        e.to_string(),
                    )));
                }
            }
        };
        let contents = contents.as_bytes();
        tokio::fs::write(path, contents).await.map_err(|_| {
            RuntimeError::NonRecoverable(RuntimeErrorType::TransactionFileOps(
                "Write to file failed".to_string(),
            ))
        })?;
        Ok(())
    }

    pub(crate) async fn read(tx_id: u32) -> Result<Self, RuntimeError> {
        let path = format!("{}{}", TEMP_DIRECTORY_LOC, tx_id);
        let result = tokio::fs::read(path).await.map_err(|e| {
            RuntimeError::NonRecoverable(RuntimeErrorType::TransactionFileOps(e.to_string()))
        })?;
        let result =
            serde_json::from_str::<SerializableTransaction>(&*String::from_utf8(result).expect(""))
                .expect("");
        Ok(result)
    }

    pub(crate) fn upgrade_state(self) -> Self {
        let mut t = self.clone();
        t.state = match self.state {
            SerializableState::State1 => SerializableState::State2,
            SerializableState::State2 => SerializableState::State3,
            SerializableState::State3 => SerializableState::State3,
        };
        t
    }
}

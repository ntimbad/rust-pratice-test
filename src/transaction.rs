use crate::error::RuntimeError::NonRecoverable;
use crate::error::{RuntimeError, RuntimeErrorType};
use bigdecimal::BigDecimal;
use csv::StringRecord;
use std::convert::TryFrom;
use std::str::FromStr;

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
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
#[allow(missing_docs)]
pub enum CSVTransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Debug)]
pub struct CSVTransaction {
    // Represents a read transaction from file
    pub(crate) amount: Option<BigDecimal>,
    pub(crate) client_id: u16,
    pub(crate) tx_id: u32,
    pub(crate) transaction_type: CSVTransactionType,
}

impl TryFrom<StringRecord> for CSVTransaction {
    type Error = RuntimeError;

    fn try_from(record: StringRecord) -> Result<Self, Self::Error> {
        //type, client, tx, amount
        let transaction_type = {
            match record.get(0) {
                None => {
                    return Result::Err(NonRecoverable(RuntimeErrorType::CSVLineParseError(
                        "Transaction type not present".to_string(),
                    )));
                }
                Some(x) => x,
            }
        };
        let transaction_type = {
            match CSVTransactionType::from_str(transaction_type) {
                Ok(x) => x,
                Err(_) => {
                    return Result::Err(NonRecoverable(RuntimeErrorType::CSVLineParseError(
                        "Transaction type not correct".to_string(),
                    )));
                }
            }
        };
        let client_id = {
            let client_id = record
                .get(1)
                .ok_or_else(|| {
                    NonRecoverable(RuntimeErrorType::CSVLineParseError(
                        "client id not present".to_string(),
                    ))
                })?
                .trim();
            u16::from_str(client_id).map_err(|e| {
                NonRecoverable(RuntimeErrorType::CSVLineParseError(
                    "Parse client_id".to_string(),
                ))
            })?
        };
        let tx_id = {
            let tx_id = record
                .get(2)
                .ok_or_else(|| {
                    NonRecoverable(RuntimeErrorType::CSVLineParseError(
                        "tx_id not present".to_string(),
                    ))
                })?
                .trim();
            u32::from_str(tx_id)
                .map_err(|e| NonRecoverable(RuntimeErrorType::CSVLineParseError(e.to_string())))?
        };
        let amount = {
            match record.get(3) {
                None => None,
                Some("") => None,
                Some(x) => Some(BigDecimal::from_str(x.trim()).map_err(|e| {
                    NonRecoverable(RuntimeErrorType::CSVLineParseError(e.to_string()))
                })?),
            }
        };
        Ok(Self {
            amount,
            client_id,
            tx_id,
            transaction_type,
        })
    }
}

pub(crate) trait State1 {
    fn inner(&self) -> &CSVTransaction;
}

pub struct WithdrawalRequest(pub(crate) CSVTransaction);
pub struct DepositRequest(pub(crate) CSVTransaction);
pub struct DisputeRequest(pub(crate) CSVTransaction);
pub struct ResolveRequest(pub(crate) CSVTransaction);
pub struct ChargeBackRequest(pub(crate) CSVTransaction);

impl State1 for DepositRequest {
    fn inner(&self) -> &CSVTransaction {
        &self.0
    }
}
impl State1 for WithdrawalRequest {
    fn inner(&self) -> &CSVTransaction {
        &self.0
    }
}
//
// impl State2 for DisputeRequest {}
//
// impl State3 for ResolveRequest {}
// impl State3 for ChargeBackRequest {}

impl TryFrom<CSVTransaction> for WithdrawalRequest {
    type Error = RuntimeError;
    fn try_from(value: CSVTransaction) -> Result<Self, Self::Error> {
        match value.transaction_type {
            CSVTransactionType::Withdrawal => Ok(WithdrawalRequest(value)),
            _ => Err(RuntimeError::Recoverable(RuntimeErrorType::ParseError(
                "Not a withdrawal".to_string(),
            ))),
        }
    }
}

impl TryFrom<CSVTransaction> for DepositRequest {
    type Error = RuntimeError;
    fn try_from(value: CSVTransaction) -> Result<Self, Self::Error> {
        match value.transaction_type {
            CSVTransactionType::Deposit => Ok(DepositRequest(value)),
            _ => Err(RuntimeError::Recoverable(RuntimeErrorType::ParseError(
                "Not a deposit".to_string(),
            ))),
        }
    }
}

impl TryFrom<CSVTransaction> for DisputeRequest {
    type Error = RuntimeError;
    fn try_from(value: CSVTransaction) -> Result<Self, Self::Error> {
        match value.transaction_type {
            CSVTransactionType::Dispute => Ok(DisputeRequest(value)),
            _ => Err(RuntimeError::Recoverable(RuntimeErrorType::ParseError(
                "Not a dispute".to_string(),
            ))),
        }
    }
}

impl TryFrom<CSVTransaction> for ResolveRequest {
    type Error = RuntimeError;
    fn try_from(value: CSVTransaction) -> Result<Self, Self::Error> {
        match value.transaction_type {
            CSVTransactionType::Resolve => Ok(ResolveRequest(value)),
            _ => Err(RuntimeError::Recoverable(RuntimeErrorType::ParseError(
                "Not a resolve".to_string(),
            ))),
        }
    }
}

impl TryFrom<CSVTransaction> for ChargeBackRequest {
    type Error = RuntimeError;
    fn try_from(value: CSVTransaction) -> Result<Self, Self::Error> {
        match value.transaction_type {
            CSVTransactionType::Chargeback => Ok(ChargeBackRequest(value)),
            _ => Err(RuntimeError::Recoverable(RuntimeErrorType::ParseError(
                "Not a chargeback".to_string(),
            ))),
        }
    }
}

use cosmwasm_std::StdError;
use thiserror::Error;

use super::asset::AssetOnboardingStatus;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Bech32Error(#[from] bech32::Error),

    #[error("Semver parsing error: {0}")]
    SemVer(#[from] semver::Error),

    #[error("duplicate/existing validator address provided as input")]
    DuplicateValidatorProvided,

    #[error("Invalid address provided [{address}]: {explanation}")]
    InvalidAddress {
        address: String,
        explanation: String,
    },

    #[error("Current contract name [{current_contract}] does not match provided migration name [{migration_contract}]")]
    InvalidContractName {
        current_contract: String,
        migration_contract: String,
    },

    #[error("Current contract version [{current_version}] is higher than provided migration version [{migration_version}]")]
    InvalidContractVersion {
        current_version: String,
        migration_version: String,
    },

    #[error("{0}")]
    InvalidFunds(String),

    #[error("Message of type [{message_type}] was invalid. Invalid fields: {invalid_fields:?}")]
    InvalidMessageFields {
        message_type: String,
        invalid_fields: Vec<String>,
    },

    #[error("Invalid message type provided. Expected message type {expected_message_type}")]
    InvalidMessageType { expected_message_type: String },

    #[error("Resource not found: {explanation}")]
    NotFound { explanation: String },

    #[error("Unsupported asset type [{asset_type}]")]
    UnsupportedAssetType { asset_type: String },

    #[error("Unsupported validator [{validator_address}] for asset type [{asset_type}]")]
    UnsupportedValidator {
        validator_address: String,
        asset_type: String,
    },

    #[error("Asset {scope_address} has already been fully onboarded")]
    AssetAlreadyOnboarded { scope_address: String },

    #[error(
        "Asset {scope_address} is currently awaiting validation for address {validator_address}"
    )]
    AssetPendingValidation {
        scope_address: String,
        validator_address: String,
    },

    #[error("Asset {scope_address} not found")]
    AssetNotFound { scope_address: String },

    #[error("Error onboarding asset (type: {asset_type}, address: {scope_address}): {message}")]
    AssetOnboardingError {
        asset_type: String,
        scope_address: String,
        message: String,
    },

    #[error("Asset identifier not supplied, please provide either asset_uuid or scope_address")]
    AssetIdentifierNotSupplied,

    #[error("Asset identifier mismatch, both asset_uuid and scope_address provided, but provided scope_address [{scope_address}] cannot be derived from asset_uuid [{asset_uuid}]")]
    AssetIdentifierMismatch {
        asset_uuid: String,
        scope_address: String,
    },

    #[error("Asset type {asset_type} is currently disabled")]
    AssetTypeDisabled { asset_type: String },

    #[error("Unauthorized validator [{validator_address}] for scope [{scope_address}], expected validator [{expected_validator_address}]")]
    UnathorizedAssetValidator {
        scope_address: String,
        validator_address: String,
        expected_validator_address: String,
    },

    #[error("Asset [{scope_address}] already validated and has status [{status}]")]
    AssetAlreadyValidated {
        scope_address: String,
        status: AssetOnboardingStatus,
    },

    #[error("Invalid scope: {explanation}")]
    InvalidScope { explanation: String },

    #[error("Expected only a single asset attribute on scope {scope_address}, but found {attribute_amount}")]
    InvalidScopeAttribute {
        scope_address: String,
        attribute_amount: usize,
    },

    #[error("Existing record found: {explanation}")]
    RecordAlreadyExists { explanation: String },

    #[error("Record not found: {explanation}")]
    RecordNotFound { explanation: String },

    #[error("Unauthorized: {explanation}")]
    Unauthorized { explanation: String },

    #[error("Unexpected state: {explanation}")]
    UnexpectedState { explanation: String },

    #[error("Requested functionality is not yet implemented")]
    Unimplemented,

    #[error("{0}")]
    UuidError(#[from] uuid::Error),
}
impl ContractError {
    pub fn std_err<S: Into<String>>(msg: S) -> ContractError {
        ContractError::Std(StdError::generic_err(msg))
    }
}

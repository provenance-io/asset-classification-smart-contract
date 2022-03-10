use crate::core::asset::AssetOnboardingStatus;
use crate::core::error::ContractError;
use crate::core::msg::{AssetIdentifier, ExecuteMsg};
use crate::util::aliases::{ContractResponse, ContractResult, DepsMutC};
use crate::util::asset_meta_repository::AssetMetaRepository;
use crate::util::event_attributes::{EventAttributes, EventType};
use crate::util::traits::ResultExtensions;
use cosmwasm_std::{Env, MessageInfo, Response};

#[derive(Clone, PartialEq)]
pub struct ValidateAssetV1 {
    pub identifier: AssetIdentifier,
    pub success: bool,
    pub message: Option<String>,
}
impl ValidateAssetV1 {
    pub fn from_execute_msg(msg: ExecuteMsg) -> ContractResult<ValidateAssetV1> {
        match msg {
            ExecuteMsg::ValidateAsset {
                identifier,
                success,
                message,
            } => ValidateAssetV1 {
                identifier,
                success,
                message,
            }
            .to_ok(),
            _ => ContractError::InvalidMessageType {
                expected_message_type: "ExecuteMsg::ValidateAsset".to_string(),
            }
            .to_err(),
        }
    }
}

pub fn validate_asset(
    deps: DepsMutC,
    _env: Env,
    info: MessageInfo,
    msg: ValidateAssetV1,
) -> ContractResponse {
    let asset_meta_repository = AssetMetaRepository::new(deps);
    let asset_identifiers = msg.identifier.parse_identifiers()?;
    // look up asset in repository
    let meta = asset_meta_repository.get_asset(&asset_identifiers.scope_address)?;

    // verify sender is requested validator
    if info.sender != meta.validator_address {
        return ContractError::UnathorizedAssetValidator {
            scope_address: asset_identifiers.scope_address,
            validator_address: info.sender.into(),
            expected_validator_address: meta.validator_address.into_string(),
        }
        .to_err();
    }

    if meta.onboarding_status == AssetOnboardingStatus::Approved {
        return ContractError::AssetAlreadyValidated {
            scope_address: asset_identifiers.scope_address,
        }
        .to_err();
    }

    asset_meta_repository.validate_asset(
        &asset_identifiers.scope_address,
        msg.success,
        msg.message,
    )?;

    // construct/emit validation attribute
    Ok(Response::new()
        .add_attributes(
            EventAttributes::for_asset_event(
                EventType::ValidateAsset,
                &meta.asset_type,
                &asset_identifiers.scope_address,
            )
            .set_validator(info.sender),
        )
        .add_messages(asset_meta_repository.messages.get()))
}

#[cfg(test)]
#[cfg(feature = "enable-test-utils")]
mod tests {
    use cosmwasm_std::testing::mock_env;
    use provwasm_mocks::mock_dependencies;

    use crate::{
        core::{error::ContractError, msg::AssetIdentifier},
        testutil::{
            onboard_asset_helpers::{test_onboard_asset, TestOnboardAsset},
            test_constants::{
                DEFAULT_ONBOARDING_COST, DEFAULT_SCOPE_ADDRESS, DEFAULT_VALIDATOR_ADDRESS,
            },
            test_utilities::{mock_info_with_nhash, setup_test_suite, InstArgs},
        },
    };

    use super::{validate_asset, ValidateAssetV1};

    #[test]
    fn test_validate_asset_not_found_error() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());

        let err = validate_asset(
            deps.as_mut(),
            mock_env(),
            mock_info_with_nhash(DEFAULT_VALIDATOR_ADDRESS, DEFAULT_ONBOARDING_COST),
            ValidateAssetV1 {
                identifier: AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
                success: true,
                message: None,
            },
        )
        .unwrap_err();

        match err {
            ContractError::NotFound { explanation } => {
                assert_eq!(
                    format!(
                        "scope at address [{}] did not include an asset scope attribute",
                        DEFAULT_SCOPE_ADDRESS
                    ),
                    explanation,
                    "the asset not found message should reflect that the asset was not found"
                );
            }
            _ => panic!(
                "unexpected error when non-onboarded asset provided: {:?}",
                err
            ),
        }
    }

    #[test]
    fn test_validate_asset_wrong_validator_error() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());

        test_onboard_asset(&mut deps, TestOnboardAsset::default()).unwrap();

        let info = mock_info_with_nhash(
            "tp129z88fpzthllrdzktw98cck3ypd34wv77nqfyl",
            DEFAULT_ONBOARDING_COST,
        );
        let err = validate_asset(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ValidateAssetV1 {
                identifier: AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
                success: true,
                message: None,
            },
        )
        .unwrap_err();

        match err {
            ContractError::UnathorizedAssetValidator {
                scope_address,
                validator_address,
                expected_validator_address,
            } => {
                assert_eq!(
                    DEFAULT_SCOPE_ADDRESS, scope_address,
                    "the unauthorized validator message should reflect the scope address"
                );
                assert_eq!(
                    info.sender.to_string(), validator_address,
                    "the unauthorized validator message should reflect the provided (sender) validator address"
                );
                assert_eq!(
                    DEFAULT_VALIDATOR_ADDRESS, expected_validator_address,
                    "the unauthorized validator message should reflect the expected validator address (from onboarding)"
                );
            }
            _ => panic!(
                "unexpected error when unauthorized validator submits validation: {:?}",
                err
            ),
        }
    }

    #[test]
    fn test_validate_asset_adds_error_message_on_negative_validation() {
        let mut deps = mock_dependencies(&[]);
        setup_test_suite(&mut deps, InstArgs::default());
        test_onboard_asset(&mut deps, TestOnboardAsset::default()).unwrap();

        let info = mock_info_with_nhash(DEFAULT_VALIDATOR_ADDRESS, DEFAULT_ONBOARDING_COST);

        let result = validate_asset(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ValidateAssetV1 {
                identifier: AssetIdentifier::scope_address(DEFAULT_SCOPE_ADDRESS),
                success: true,
                message: Some("Your data sucks".to_string()),
            },
        )
        .unwrap();

        assert_eq!(
            2,
            result.messages.len(),
            "validate asset should produce two messages (attribute delete/update combo)"
        );
    }
}

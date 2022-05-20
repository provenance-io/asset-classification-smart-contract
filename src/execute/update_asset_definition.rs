use crate::core::error::ContractError;
use crate::core::msg::ExecuteMsg;
use crate::core::state::replace_asset_definition;
use crate::core::types::asset_definition::AssetDefinition;
use crate::util::aliases::{AssetResult, DepsMutC, EntryPointResponse};
use crate::util::contract_helpers::{check_admin_only, check_funds_are_empty};
use crate::util::event_attributes::{EventAttributes, EventType};
use crate::util::traits::ResultExtensions;
use cosmwasm_std::{MessageInfo, Response};

/// A transformation of [ExecuteMsg::UpdateAssetDefinition](crate::core::msg::ExecuteMsg::UpdateAssetDefinition)
/// for ease of use in the underlying [update_asset_definition](self::update_asset_definition) function.
///
/// # Parameters
///
/// * `asset_definition` The asset definition instance to update.  Must have an [asset_type](crate::core::types::asset_definition::AssetDefinition::asset_type)
/// property that matches an existing asset definition in contract storage.
#[derive(Clone, PartialEq)]
pub struct UpdateAssetDefinitionV1 {
    pub asset_definition: AssetDefinition,
}
impl UpdateAssetDefinitionV1 {
    /// Constructs a new instance of this struct.
    ///
    /// # Parameters
    ///
    /// * `asset_definition` The asset definition instance to update.  Must have an [asset_type](crate::core::types::asset_definition::AssetDefinition::asset_type)
    /// property that matches an existing asset definition in contract storage.
    pub fn new(asset_definition: AssetDefinition) -> Self {
        UpdateAssetDefinitionV1 { asset_definition }
    }

    /// Attempts to create an instance of this struct from a provided execute msg.  If the provided
    /// value is not of the [UpdateAssetDefinition](crate::core::msg::ExecuteMsg::UpdateAssetDefinition)
    /// variant, then an [InvalidMessageType](crate::core::error::ContractError::InvalidMessageType)
    /// error will be returned.
    ///
    /// # Parameters
    ///
    /// * `msg` An execute msg provided by the contract's [execute](crate::contract::execute) function.
    pub fn from_execute_msg(msg: ExecuteMsg) -> AssetResult<UpdateAssetDefinitionV1> {
        match msg {
            ExecuteMsg::UpdateAssetDefinition { asset_definition } => Self {
                asset_definition: asset_definition.into_asset_definition()?,
            }
            .to_ok(),
            _ => ContractError::InvalidMessageType {
                expected_message_type: "ExecuteMsg::UpdateAssetDefinition".to_string(),
            }
            .to_err(),
        }
    }
}

/// The function used by [execute](crate::contract::execute) when an [ExecuteMsg::UpdateAssetDefinition](crate::core::msg::ExecuteMsg::UpdateAssetDefinition)
/// message is provided.  Attempts to replace an existing [AssetDefinition](crate::core::types::asset_definition::AssetDefinition)
/// value based on a matching [asset_type](crate::core::types::asset_definition::AssetDefinition::asset_type)
/// property.  If no matching type is present, the request will be rejected.
///
/// # Parameters
///
/// * `deps` A dependencies object provided by the cosmwasm framework.  Allows access to useful
/// resources like contract internal storage and a querier to retrieve blockchain objects.
/// * `info` A message information object provided by the cosmwasm framework.  Describes the sender
/// of the instantiation message, as well as the funds provided as an amount during the transaction.
/// * `msg` An instance of the update asset definition v1 struct, provided by conversion from an
/// [ExecuteMsg](crate::core::msg::ExecuteMsg).
pub fn update_asset_definition(
    deps: DepsMutC,
    info: MessageInfo,
    msg: UpdateAssetDefinitionV1,
) -> EntryPointResponse {
    check_admin_only(&deps.as_ref(), &info)?;
    check_funds_are_empty(&info)?;
    // Overwrite the existing asset definition with the new one
    replace_asset_definition(deps.storage, &msg.asset_definition)?;
    Response::new()
        .add_attributes(
            EventAttributes::new(EventType::UpdateAssetDefinition)
                .set_asset_type(&msg.asset_definition.asset_type),
        )
        .to_ok()
}

#[cfg(test)]
#[cfg(feature = "enable-test-utils")]
mod tests {
    use crate::contract::execute;
    use crate::core::error::ContractError;
    use crate::core::msg::ExecuteMsg;
    use crate::core::state::load_asset_definition_by_type;
    use crate::core::types::asset_definition::{AssetDefinition, AssetDefinitionInput};
    use crate::core::types::fee_destination::FeeDestination;
    use crate::core::types::scope_spec_identifier::ScopeSpecIdentifier;
    use crate::core::types::verifier_detail::VerifierDetail;
    use crate::execute::update_asset_definition::{
        update_asset_definition, UpdateAssetDefinitionV1,
    };
    use crate::testutil::test_constants::{
        DEFAULT_ADMIN_ADDRESS, DEFAULT_ASSET_TYPE, DEFAULT_SCOPE_SPEC_ADDRESS,
        DEFAULT_SENDER_ADDRESS,
    };
    use crate::testutil::test_utilities::{
        empty_mock_info, get_default_entity_detail, single_attribute_for_key,
        test_instantiate_success, InstArgs,
    };
    use crate::util::aliases::DepsC;
    use crate::util::constants::{ASSET_EVENT_TYPE_KEY, ASSET_TYPE_KEY, NHASH};
    use crate::util::event_attributes::EventType;
    use crate::util::traits::OptionExtensions;
    use crate::validation::validate_init_msg::validate_asset_definition_input;
    use cosmwasm_std::testing::{mock_env, mock_info};
    use cosmwasm_std::{coin, Decimal, Uint128};
    use provwasm_mocks::mock_dependencies;

    #[test]
    fn test_valid_update_asset_definition_via_execute() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let asset_definition = get_update_asset_definition();
        let response = execute(
            deps.as_mut(),
            mock_env(),
            empty_mock_info(DEFAULT_ADMIN_ADDRESS),
            ExecuteMsg::UpdateAssetDefinition {
                asset_definition: asset_definition.clone(),
            },
        )
        .expect("expected the update asset checks to work correctly");
        assert!(
            response.messages.is_empty(),
            "updating an asset definition should not require messages",
        );
        assert_eq!(
            2,
            response.attributes.len(),
            "updating an asset definition should produce the correct number of attributes",
        );
        assert_eq!(
            EventType::UpdateAssetDefinition.event_name().as_str(),
            single_attribute_for_key(&response, ASSET_EVENT_TYPE_KEY),
            "the correct event type should be emitted",
        );
        assert_eq!(
            DEFAULT_ASSET_TYPE,
            single_attribute_for_key(&response, ASSET_TYPE_KEY),
            "the asset type attribute should be added correctly",
        );
        test_asset_definition_was_updated_for_input(&asset_definition, &deps.as_ref());
    }

    #[test]
    fn test_valid_update_asset_definition_via_internal() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let msg = get_valid_update_asset_definition();
        update_asset_definition(
            deps.as_mut(),
            mock_info(DEFAULT_ADMIN_ADDRESS, &[]),
            msg.clone(),
        )
        .expect("expected the update asset definition function to return properly");
        test_asset_definition_was_updated(&msg.asset_definition, &deps.as_ref());
    }

    #[test]
    fn test_invalid_update_asset_definition_for_invalid_msg() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let msg = ExecuteMsg::UpdateAssetDefinition {
            asset_definition: AssetDefinitionInput::new(
                DEFAULT_ASSET_TYPE,
                ScopeSpecIdentifier::address(DEFAULT_SCOPE_SPEC_ADDRESS).to_serialized_enum(),
                vec![],
                None,
                None,
            ),
        };
        let error = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(DEFAULT_ADMIN_ADDRESS, &[]),
            msg,
        )
        .unwrap_err();
        assert!(
            matches!(error, ContractError::InvalidMessageFields { .. }),
            "expected an invalid asset definition to cause an InvalidMessageFields error, but got {:?}",
            error,
        );
    }

    #[test]
    fn test_invalid_update_asset_definition_for_invalid_sender() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let error = update_asset_definition(
            deps.as_mut(),
            // Send from the "sender address" which is the address of the account that does onboarding in tests
            mock_info(DEFAULT_SENDER_ADDRESS, &[]),
            get_valid_update_asset_definition(),
        )
        .unwrap_err();
        assert!(
            matches!(error, ContractError::Unauthorized { .. }),
            "expected the unauthorized response to be returned when a different address than the admin is the sender, but got error: {:?}",
            error,
        );
    }

    #[test]
    fn test_invalid_update_asset_definition_for_provided_funds() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let error = update_asset_definition(
            deps.as_mut(),
            mock_info(DEFAULT_ADMIN_ADDRESS, &[coin(420, "usdf")]),
            get_valid_update_asset_definition(),
        )
        .unwrap_err();
        assert!(
            matches!(error, ContractError::InvalidFunds(_)),
            "expected the invalid funds response to be returned when funds are provided to the function, but got: {:?}",
            error,
        );
    }

    #[test]
    fn test_invalid_update_asset_definition_for_missing_loan_type() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let missing_asset_definition = AssetDefinition::new(
            "nonexistent-type",
            DEFAULT_SCOPE_SPEC_ADDRESS,
            vec![VerifierDetail::new(
                "verifier",
                Uint128::new(100),
                NHASH,
                Decimal::percent(25),
                vec![FeeDestination::new("fee-guy", Decimal::percent(100))],
                get_default_entity_detail().to_some(),
            )],
        );
        let error = update_asset_definition(
            deps.as_mut(),
            mock_info(DEFAULT_ADMIN_ADDRESS, &[]),
            UpdateAssetDefinitionV1::new(missing_asset_definition),
        )
        .unwrap_err();
        assert!(
            matches!(error, ContractError::RecordNotFound { .. }),
            "expected the not found response to be returned when an update is attempted for a definition that does not exist, but got: {:?}",
            error,
        );
    }

    fn test_asset_definition_was_updated_for_input(input: &AssetDefinitionInput, deps: &DepsC) {
        test_asset_definition_was_updated(
            &input
                .as_asset_definition()
                .expect("conversion should succeed"),
            deps,
        )
    }

    fn test_asset_definition_was_updated(asset_definition: &AssetDefinition, deps: &DepsC) {
        let state_def = load_asset_definition_by_type(deps.storage, &asset_definition.asset_type)
            .expect("expected the updated asset definition to be stored in the state");
        assert_eq!(
            asset_definition, &state_def,
            "the value in state should directly equate to the added value",
        );
    }

    // This builds off of the existing default asset definition in test_utilities and adds/tweaks
    // details.  This uses randomly-generated bech32 provenance testnet addresses to be different than
    // the default values
    fn get_update_asset_definition() -> AssetDefinitionInput {
        let def = AssetDefinitionInput::new(
            DEFAULT_ASSET_TYPE,
            ScopeSpecIdentifier::address(DEFAULT_SCOPE_SPEC_ADDRESS).to_serialized_enum(),
            vec![VerifierDetail::new(
                "tp1y67rma23nplzy8rpvfqsztvktvp85hnmnjvzxs",
                Uint128::new(1500000),
                NHASH,
                Decimal::percent(50),
                vec![
                    FeeDestination::new(
                        "tp1knh6n2kafm78mfv0c6d6y3x3en3pcdph23r2e7",
                        Decimal::percent(70),
                    ),
                    FeeDestination::new(
                        "tp1uqx5fcrx0nkcak52tt794p03d5tju62qfnwc52",
                        Decimal::percent(30),
                    ),
                ],
                get_default_entity_detail().to_some(),
            )],
            None,
            None,
        );
        validate_asset_definition_input(&def).expect("expected the asset definition to be valid");
        def
    }

    fn get_valid_update_asset_definition() -> UpdateAssetDefinitionV1 {
        UpdateAssetDefinitionV1 {
            asset_definition: get_update_asset_definition()
                .into_asset_definition()
                .expect("asset definition input conversion should succeed"),
        }
    }
}

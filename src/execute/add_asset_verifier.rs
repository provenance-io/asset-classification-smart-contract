use crate::core::asset::VerifierDetail;
use crate::core::error::ContractError;
use crate::core::msg::ExecuteMsg;
use crate::core::state::{load_asset_definition_by_type, replace_asset_definition};
use crate::util::aliases::{AssetResult, DepsMutC, EntryPointResponse};
use crate::util::contract_helpers::{check_admin_only, check_funds_are_empty};
use crate::util::event_attributes::{EventAttributes, EventType};
use crate::util::traits::ResultExtensions;
use cosmwasm_std::{MessageInfo, Response};

#[derive(Clone, PartialEq)]
pub struct AddAssetVerifierV1 {
    pub asset_type: String,
    pub verifier: VerifierDetail,
}
impl AddAssetVerifierV1 {
    pub fn new<S: Into<String>>(asset_type: S, verifier: VerifierDetail) -> Self {
        AddAssetVerifierV1 {
            asset_type: asset_type.into(),
            verifier,
        }
    }

    pub fn from_execute_msg(msg: ExecuteMsg) -> AssetResult<AddAssetVerifierV1> {
        match msg {
            ExecuteMsg::AddAssetVerifier {
                asset_type,
                verifier,
            } => AddAssetVerifierV1::new(asset_type, verifier).to_ok(),
            _ => ContractError::InvalidMessageType {
                expected_message_type: "ExecuteMsg::AddAssetVerifier".to_string(),
            }
            .to_err(),
        }
    }
}

pub fn add_asset_verifier(
    deps: DepsMutC,
    info: MessageInfo,
    msg: AddAssetVerifierV1,
) -> EntryPointResponse {
    check_admin_only(&deps.as_ref(), &info)?;
    check_funds_are_empty(&info)?;
    let mut asset_definition = load_asset_definition_by_type(deps.storage, &msg.asset_type)?;
    // If the asset definition has any verifiers on it (only ever should be 1 max) with a matching
    // address to the new verifier, this request should be an update, not an add
    if asset_definition
        .verifiers
        .iter()
        .any(|verifier| verifier.address == msg.verifier.address)
    {
        return ContractError::DuplicateVerifierProvided.to_err();
    }
    // Declare all attributes before values are moved
    let attributes = EventAttributes::new(EventType::AddAssetVerifier)
        .set_asset_type(&asset_definition.asset_type)
        .set_verifier(&msg.verifier.address);
    // Store the new verifier in the definition and save it to storage
    asset_definition.verifiers.push(msg.verifier);
    replace_asset_definition(deps.storage, &asset_definition)?;
    // Respond with emitted attributes
    Response::new().add_attributes(attributes).to_ok()
}

#[cfg(test)]
#[cfg(feature = "enable-test-utils")]
mod tests {
    use crate::contract::execute;
    use crate::core::asset::{FeeDestination, VerifierDetail};
    use crate::core::error::ContractError;
    use crate::core::msg::ExecuteMsg;
    use crate::core::state::load_asset_definition_by_type;
    use crate::execute::add_asset_verifier::{add_asset_verifier, AddAssetVerifierV1};
    use crate::testutil::test_constants::{
        DEFAULT_ADMIN_ADDRESS, DEFAULT_ASSET_TYPE, DEFAULT_VERIFIER_ADDRESS,
    };
    use crate::testutil::test_utilities::{
        single_attribute_for_key, test_instantiate_success, InstArgs,
    };
    use crate::util::aliases::DepsC;
    use crate::util::constants::{
        ASSET_EVENT_TYPE_KEY, ASSET_TYPE_KEY, NHASH, VERIFIER_ADDRESS_KEY,
    };
    use crate::util::event_attributes::EventType;
    use crate::validation::validate_init_msg::validate_verifier;
    use cosmwasm_std::testing::{mock_env, mock_info};
    use cosmwasm_std::{coin, Decimal, Uint128};
    use provwasm_mocks::mock_dependencies;

    // Addresses must be valid bech32, so these are valid randomly-generated values for testing
    const TEST_VERIFIER_ADDRESS: &str = "tp1g83pm46c8wxsnlra2ytruec7nuy95ttc8yy5n3";
    const TEST_FEE_ADDRESS: &str = "tp1jz6mk0mfxd7heqhveezd2yf8ht0m3nekm6xve6";

    #[test]
    fn test_valid_add_asset_verifier_via_execute() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let verifier = get_valid_new_verifier();
        let response = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(DEFAULT_ADMIN_ADDRESS, &[]),
            ExecuteMsg::AddAssetVerifier {
                asset_type: DEFAULT_ASSET_TYPE.to_string(),
                verifier: verifier.clone(),
            },
        )
        .expect("expected the add verifier function to execute properly");
        assert!(
            response.messages.is_empty(),
            "adding an asset verifier should not require messages",
        );
        assert_eq!(
            3,
            response.attributes.len(),
            "adding an asset verifier should produce the correct number of attributes",
        );
        assert_eq!(
            EventType::AddAssetVerifier.event_name().as_str(),
            single_attribute_for_key(&response, ASSET_EVENT_TYPE_KEY),
            "expected the correct event type to be emitted",
        );
        assert_eq!(
            DEFAULT_ASSET_TYPE,
            single_attribute_for_key(&response, ASSET_TYPE_KEY),
            "expected the default asset type to be used for the main add key",
        );
        assert_eq!(
            &verifier.address,
            single_attribute_for_key(&response, VERIFIER_ADDRESS_KEY),
            "expected the new verifier's address to be emitted as an attribute",
        );
        test_default_verifier_was_added(&verifier, &deps.as_ref());
    }

    #[test]
    fn test_valid_add_asset_verifier_via_internal() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let msg = get_add_verifier();
        add_asset_verifier(
            deps.as_mut(),
            mock_info(DEFAULT_ADMIN_ADDRESS, &[]),
            msg.clone(),
        )
        .expect("expected the add verifier function to return properly");
        test_default_verifier_was_added(&msg.verifier, &deps.as_ref());
    }

    #[test]
    fn test_invalid_add_asset_verifier_for_invalid_asset_type() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let error = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(DEFAULT_ADMIN_ADDRESS, &[]),
            ExecuteMsg::AddAssetVerifier {
                // Invalid because the asset type is missing
                asset_type: String::new(),
                verifier: get_valid_new_verifier(),
            },
        )
        .unwrap_err();
        assert!(
            matches!(error, ContractError::InvalidMessageFields { .. }),
            "when an invalid asset type is provided to execute, the invalid message fields error should be returned, but got: {:?}",
            error,
        );
    }

    #[test]
    fn test_invalid_add_asset_verifier_for_invalid_msg() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let error = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(DEFAULT_ADMIN_ADDRESS, &[]),
            ExecuteMsg::AddAssetVerifier {
                asset_type: DEFAULT_ASSET_TYPE.to_string(),
                // Invalid because the address is blank
                verifier: VerifierDetail::new(
                    "",
                    Uint128::new(0),
                    NHASH,
                    Decimal::percent(0),
                    vec![],
                ),
            },
        )
        .unwrap_err();
        assert!(
            matches!(error, ContractError::InvalidMessageFields { .. }),
            "when an invalid verifier is provided to execute, the invalid message fields error should be returned, but got: {:?}",
            error,
        );
    }

    #[test]
    fn test_invalid_add_asset_verifier_for_invalid_sender() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let error = add_asset_verifier(
            deps.as_mut(),
            mock_info("non-admin-person", &[]),
            get_add_verifier(),
        )
        .unwrap_err();
        assert!(
            matches!(error, ContractError::Unauthorized { .. }),
            "expected the unauthorized response to be returned when a different address than the admin is the sender, but got: {:?}",
            error,
        );
    }

    #[test]
    fn test_invalid_add_asset_verifier_for_provided_funds() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let error = add_asset_verifier(
            deps.as_mut(),
            mock_info(DEFAULT_ADMIN_ADDRESS, &[coin(6900, "nhash")]),
            get_add_verifier(),
        )
        .unwrap_err();
        assert!(
            matches!(error, ContractError::InvalidFunds(_)),
            "expected the invalid funds response to be returned when funds are provided to the function, but got: {:?}",
            error,
        );
    }

    #[test]
    fn test_invalid_add_asset_verifier_for_duplicate_verifier_address() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let error = add_asset_verifier(
            deps.as_mut(),
            mock_info(DEFAULT_ADMIN_ADDRESS, &[]),
            AddAssetVerifierV1::new(
                DEFAULT_ASSET_TYPE,
                VerifierDetail::new(
                    DEFAULT_VERIFIER_ADDRESS,
                    Uint128::new(100),
                    NHASH,
                    Decimal::percent(0),
                    vec![],
                ),
            ),
        )
        .unwrap_err();
        assert!(
            matches!(error, ContractError::DuplicateVerifierProvided),
            "expected the duplcate verifier error to be returned when the verifier to be added is already placed on the asset definition, but got: {:?}",
            error,
        );
    }

    // Checks that the verifier passed in was added to the default asset type's definition
    fn test_default_verifier_was_added(verifier: &VerifierDetail, deps: &DepsC) {
        let state_def = load_asset_definition_by_type(deps.storage, DEFAULT_ASSET_TYPE)
            .expect("expected the default asset type to be stored in the state");
        let target_verifier = state_def.verifiers.into_iter().find(|v| v.address == verifier.address)
            .expect("expected a single verifier to be produced when searching for the new verifier's address");
        assert_eq!(
            verifier, &target_verifier,
            "expected the verifier stored in state to equate to the newly-added verifier",
        );
    }

    fn get_valid_new_verifier() -> VerifierDetail {
        let verifier = VerifierDetail::new(
            TEST_VERIFIER_ADDRESS,
            Uint128::new(500000),
            NHASH,
            Decimal::percent(10),
            vec![FeeDestination::new(TEST_FEE_ADDRESS, Decimal::percent(100))],
        );
        validate_verifier(&verifier).expect("expected the new verifier to pass validation");
        verifier
    }

    fn get_add_verifier() -> AddAssetVerifierV1 {
        AddAssetVerifierV1 {
            asset_type: DEFAULT_ASSET_TYPE.to_string(),
            verifier: get_valid_new_verifier(),
        }
    }
}
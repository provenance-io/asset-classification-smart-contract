use cosmwasm_std::{to_binary, Binary};

use crate::{
    migrate::version_info::get_version_info,
    util::{
        aliases::{AssetResult, DepsC},
        traits::ResultExtensions,
    },
};

/// Pulls the version info for the contract out of the version store.
/// On a success, serializes the value to a cosmwasm Binary and responses Ok.
pub fn query_version(deps: &DepsC) -> AssetResult<Binary> {
    to_binary(&get_version_info(deps.storage)?)?.to_ok()
}

#[cfg(test)]
#[cfg(feature = "enable-test-utils")]
mod tests {
    use cosmwasm_std::from_binary;
    use provwasm_mocks::mock_dependencies;

    use crate::{
        migrate::version_info::{VersionInfoV1, CONTRACT_NAME, CONTRACT_VERSION},
        testutil::test_utilities::{test_instantiate_success, InstArgs},
    };

    use super::query_version;

    #[test]
    fn test_default_instantiate_and_fetch_version() {
        let mut deps = mock_dependencies(&[]);
        test_instantiate_success(deps.as_mut(), InstArgs::default());
        let version_bin = query_version(&deps.as_ref()).expect("failed to receive version info");
        let version_info = from_binary::<VersionInfoV1>(&version_bin)
            .expect("failed to deserialize version info binary");
        // These values should always follow the env declared in Cargo.toml
        assert_eq!(
            CONTRACT_NAME, version_info.contract,
            "unexpected contract name value"
        );
        assert_eq!(
            CONTRACT_VERSION, version_info.version,
            "unexpected contract version value"
        );
    }
}

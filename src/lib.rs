//! The HELA EVM.
use std::collections::{BTreeMap, BTreeSet};

use oasis_runtime_sdk::{
    self as sdk, config, modules,
    types::{
        token::{BaseUnits, Denomination},
        address::Address,
    },
    Module, Version,
};

/// Configuration of the various modules.
pub struct Config;

/// Determine whether the build is for Testnet.
///
/// If the crate version has a pre-release component (e.g. 3.0.0-alpha) then the build is classified
/// as Testnet. If there is no such component (e.g. 5.0.0) then it is classified as Mainnet.
const fn is_testnet() -> bool {
    !env!("CARGO_PKG_VERSION_PRE").is_empty()
}

const fn is_localnet() -> bool {
    option_env!("HELA_USE_LOCALNET_CHAINID").is_some()
}

/// Determine EVM chain ID to use depending on whether the build is for Localnet, Testnet or Mainnet.
const fn chain_id() -> u64 {
    if is_localnet() {
        // Localnet.
        0x1a20 //6688
    } else if is_testnet() {
        // Testnet.
        0xa2d08 //666888
    } else {
        // Mainnet.
        0x21dc //8668
    }
}

/// Determine state version on weather the build is for Testnet or Mainnet.
#[allow(clippy::if_same_then_else)]
const fn state_version() -> u32 {
    if is_testnet() {
        // Testnet.
        3
    } else {
        // Mainnet.
        3
    }
}

impl modules::core::Config for Config {
    /// Default local minimum gas price configuration that is used in case no overrides are set in
    /// local per-node configuration.

    const DEFAULT_LOCAL_MIN_GAS_PRICE: once_cell::unsync::Lazy<BTreeMap<Denomination, u128>> =
        once_cell::unsync::Lazy::new(|| BTreeMap::from([(Denomination::NATIVE, 1_000_000_000_000)]));

    // GB: exempt accounts.MintST from transaction fee requirements.
    /// Methods which are exempt from minimum gas price requirements.
    const MIN_GAS_PRICE_EXEMPT_METHODS: once_cell::unsync::Lazy<BTreeSet<&'static str>> =
        once_cell::unsync::Lazy::new(|| BTreeSet::from(["consensus.Deposit", "accounts.MintST", 
            "accounts.BurnST", "accounts.InitOwners", "accounts.Propose", "accounts.VoteST"]));

    /// Default local estimate gas max search iterations configuration that is used in case no overrides
    /// are set in the local per-node configuration.
    const DEFAULT_LOCAL_ESTIMATE_GAS_SEARCH_MAX_ITERS: u64 = if is_testnet() { 25 } else { 23 }; // log2(max_batch_gas) for exact estimates.
}

impl module_evm::Config for Config {
    type Accounts = modules::accounts::Module;

    const CHAIN_ID: u64 = chain_id();

    const TOKEN_DENOMINATION: Denomination = Denomination::NATIVE;
}

/// The EVM ParaTime.
pub struct Runtime;

impl sdk::Runtime for Runtime {
    /// Version of the runtime.
    const VERSION: Version = sdk::version_from_cargo!();
    /// Current version of the global state (e.g. parameters). Any parameter updates should bump
    /// this version in order for the migrations to be executed.
    const STATE_VERSION: u32 = state_version();

    /// Schedule control configuration.
    const SCHEDULE_CONTROL: config::ScheduleControl = config::ScheduleControl {
        initial_batch_size: 10_000,// 50
        batch_size: 10_000,        // 50
        min_remaining_gas: 1_000, // accounts.Transfer method calls.
        max_tx_count: 10_000,      // Consistent with runtime descriptor.
    };

    type Core = modules::core::Module<Config>;

    #[allow(clippy::type_complexity)]
    type Modules = (
        // Core.
        modules::core::Module<Config>,
        // Accounts.
        modules::accounts::Module,
        // Consensus layer interface.
        modules::consensus::Module,
        // Consensus layer accounts.
        modules::consensus_accounts::Module<modules::accounts::Module, modules::consensus::Module>,
        // Rewards.
        modules::rewards::Module<modules::accounts::Module>,
        // EVM.
        module_evm::Module<Config>,
    );

    fn genesis_state() -> <Self::Modules as sdk::module::MigrationHandler>::Genesis {
        (
            modules::core::Genesis {
                parameters: modules::core::Parameters {
                    min_gas_price: {
                        let mut mgp = BTreeMap::new();
                        mgp.insert(Denomination::NATIVE, 1_000_000_000_000);
                        mgp
                    },
                    max_batch_gas: if is_testnet() { 30_000_000 } else { 10_000_000 },
                    max_tx_signers: 1,
                    max_multisig_signers: 8,
                    gas_costs: modules::core::GasCosts {
                        tx_byte: 1,
                        auth_signature: 1_000,
                        auth_multisig_signer: 1_000,
                        callformat_x25519_deoxysii: 10_000,
                    },
                    max_tx_size: 1024 * 300, // 300 KiB.
                },
            },
            modules::accounts::Genesis {
                parameters: modules::accounts::Parameters {
                    // GB: replace chain_initiator with SNAP's address, or introduce field in 
                    // oasis-core/runtime/src/consensus/registry.rs/Runtime (line 623)

                    chain_initiator: Address::from_bech32("hela01qp3fu89xmt5s3trszky6k6z0e2qka9hklsj5h6jn").unwrap(),

                    // GB: define initial value for transfers_disabled, mintst_disabled, burnst_disabled.
                    // transfers_disabled: false,
                    mintst_disabled: true,
                    burnst_disabled: true,

                    // GB: add initial parameters.
                    gas_costs: modules::accounts::GasCosts { 
                        tx_transfer: 1_000, 
                        tx_managest: 0
                    },
                    denomination_infos: {
                        let mut denomination_infos = BTreeMap::new();
                        denomination_infos.insert(
                            Denomination::NATIVE,
                            modules::accounts::types::DenominationInfo {
                                // Consistent with EVM ecosystem.
                                decimals: 18,
                            },
                        );
                        denomination_infos
                    },
                    ..Default::default()
                },

                // MZ
                // Add some test accounts for testing, cannot be used in mainnet
                balances: BTreeMap::from([
                    (
                        sdk::testing::keys::alice::address(),
                        BTreeMap::from([(Denomination::NATIVE, 
                            if is_localnet() || is_testnet() {
                                100_000_000_000_000_000_000_000
                            } else {
                                0
                            }
                        )]),
                    ),
                    (
                        sdk::testing::keys::bob::address(),
                        BTreeMap::from([(Denomination::NATIVE, 
                            if is_localnet() || is_testnet() {
                                200_000_000_000_000_000_000_000
                            } else {
                                0
                            }
                        )]),
                    ),
                ]),
                total_supplies: BTreeMap::from([(Denomination::NATIVE,
                    if is_localnet() || is_testnet() {
                        300_000_000_000_000_000_000_000
                    } else {
                        0
                    }
                )]),
                ..Default::default()
            },
            modules::consensus::Genesis {
                parameters: modules::consensus::Parameters {
                    // Consensus layer denomination is the native denomination of this runtime.
                    consensus_denomination: Denomination::NATIVE,
                    // Scale to 18 decimal places as this is what is expected in the EVM ecosystem.
                    consensus_scaling_factor: 1_000_000_000,
                },
            },
            modules::consensus_accounts::Genesis {
                parameters: modules::consensus_accounts::Parameters {
                    gas_costs: modules::consensus_accounts::GasCosts {
                        tx_deposit: 10_000,
                        tx_withdraw: 10_000,
                    },
                },
            },
            modules::rewards::Genesis {
                parameters: modules::rewards::Parameters {
                    schedule: modules::rewards::types::RewardSchedule {
                        steps: vec![modules::rewards::types::RewardStep {
                            until: 27_500,
                            amount: BaseUnits::new(3_000_000_000_000_000_000, Denomination::NATIVE),
                        }],
                    },
                    participation_threshold_numerator: 3,
                    participation_threshold_denominator: 4,
                },
            },
            module_evm::Genesis {
                parameters: module_evm::Parameters {
                    gas_costs: module_evm::GasCosts {},
                },
            },
        )
    }

    fn migrate_state<C: sdk::Context>(ctx: &mut C) {
        // State migration from by copying over parameters from updated genesis state.
        let genesis = Self::genesis_state();

        // Core.
        modules::core::Module::<Config>::set_params(ctx.runtime_state(), genesis.0.parameters);
        // Accounts.
        modules::accounts::Module::set_params(ctx.runtime_state(), genesis.1.parameters);
        // Consensus layer interface.
        modules::consensus::Module::set_params(ctx.runtime_state(), genesis.2.parameters);
        // Consensus layer accounts.
        modules::consensus_accounts::Module::<modules::accounts::Module, modules::consensus::Module>::set_params(
            ctx.runtime_state(),
            genesis.3.parameters,
        );
        // Rewards.
        modules::rewards::Module::<modules::accounts::Module>::set_params(
            ctx.runtime_state(),
            genesis.4.parameters,
        );
        // EVM.
        module_evm::Module::<Config>::set_params(ctx.runtime_state(), genesis.5.parameters);
    }
}

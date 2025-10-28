/// Constants for important contract addresses across different chains
///
/// This module contains addresses for commonly used contracts and services
/// that are needed across the WAVS ecosystem. Addresses are organized by
/// chain and service type.
///
/// Skip swap entry point contract address on Neutron
///
/// This is the main Skip entry point contract used for executing swaps
/// on the Neutron chain via the Skip protocol.
pub const SKIP_SWAP_ENTRY_POINT_NEUTRON: &str =
    "neutron1zvesudsdfxusz06jztpph4d3h5x6veglqsspxns2v2jqml9nhywskcc923";

/// Get the Skip swap entry point address for a specific chain
///
/// # Arguments
///
/// * `chain_id` - The chain identifier (e.g., "neutron")
///
/// # Returns
///
/// Returns the Skip swap entry point address for the specified chain,
/// or None if the chain is not supported.
pub fn skip_swap_entry_point(chain_id: &str) -> Option<&'static str> {
    match chain_id {
        "neutron-1" => Some(SKIP_SWAP_ENTRY_POINT_NEUTRON),
        _ => None,
    }
}

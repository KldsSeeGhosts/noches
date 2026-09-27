import Foundation

/// Harness catalog semantics from `crates/engine/src/registry.rs` and
/// `crates/ui/src/pickers.rs`.
extension HostHarness {
    /// `descriptor_enabled`: a missing `enabled` falls back to detection; the
    /// mock rig and Antigravity never switch themselves on.
    var enabledByDefault: Bool {
        if let enabled { return enabled }
        return (installed ?? true) && id != "mock" && id != "antigravity"
    }

    /// `offered_harnesses`: installed and enabled.
    var isOffered: Bool { (installed ?? true) && enabledByDefault }

    /// Non-interrupting steer (`HarnessDescriptor::steers_mid_turn`).
    var steersMidTurn: Bool { (supportsSteering ?? false) && steeringMode == "step-boundary" }
}

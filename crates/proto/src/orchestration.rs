//! Generated T3 V2 wire contracts. See docs/orchestration/contracts.md.
//! Regenerate: node crates/proto/tests/t3_oracle/generate.mjs
//! Source: T3 Tools Inc., MIT, pinned in tests/t3_oracle/fixtures/provenance.json.
#![allow(unused_imports)]
use crate::orchestration_mcp::*;
use crate::provider_instance::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AttachmentCreateUploadUrlResult {
    #[serde(rename = "attachmentId")]
    pub attachment_id: String,
    #[serde(rename = "relativeUrl")]
    pub relative_url: String,
    #[serde(rename = "expiresAt")]
    pub expires_at: JsonNumber,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BackgroundActivityProfile {
    #[serde(rename = "balanced")]
    Balanced,
    #[serde(rename = "performance")]
    Performance,
    #[serde(rename = "battery-saver")]
    BatterySaver,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BackgroundActivityProfileSelection {
    #[serde(rename = "balanced")]
    Balanced,
    #[serde(rename = "performance")]
    Performance,
    #[serde(rename = "battery-saver")]
    BatterySaver,
    #[serde(rename = "custom")]
    Custom,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BrowserProfileId(pub String);
impl From<String> for BrowserProfileId {
    fn from(value: String) -> Self {
        Self(value)
    }
}
impl From<&str> for BrowserProfileId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}
impl AsRef<str> for BrowserProfileId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
impl std::fmt::Display for BrowserProfileId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ChatAttachment {
    Variant1(Box<ChatImageAttachment>),
    Variant2(Box<ChatFileAttachment>),
    Variant3(Box<ChatUnknownAttachment>),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ChatAttachmentId(pub String);
impl From<String> for ChatAttachmentId {
    fn from(value: String) -> Self {
        Self(value)
    }
}
impl From<&str> for ChatAttachmentId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}
impl AsRef<str> for ChatAttachmentId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
impl std::fmt::Display for ChatAttachmentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ChatFileAttachmentType {
    #[serde(rename = "file")]
    File,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatFileAttachment {
    #[serde(rename = "type")]
    pub r#type: ChatFileAttachmentType,
    #[serde(rename = "id")]
    pub id: ChatAttachmentId,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    #[serde(rename = "sizeBytes")]
    pub size_bytes: i64,
    #[serde(
        rename = "source",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub source: Optional<PastedTextAttachmentSource>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ChatImageAttachmentType {
    #[serde(rename = "image")]
    Image,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatImageAttachment {
    #[serde(rename = "type")]
    pub r#type: ChatImageAttachmentType,
    #[serde(rename = "id")]
    pub id: ChatAttachmentId,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    #[serde(rename = "sizeBytes")]
    pub size_bytes: i64,
    #[serde(
        rename = "source",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub source: Optional<SnapShotSource>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatUnknownAttachment {
    #[serde(rename = "type")]
    pub r#type: String,
    #[serde(rename = "id")]
    pub id: ChatAttachmentId,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    #[serde(rename = "sizeBytes")]
    pub size_bytes: NonNegativeInt,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CheckpointId(pub String);
impl From<String> for CheckpointId {
    fn from(value: String) -> Self {
        Self(value)
    }
}
impl From<&str> for CheckpointId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}
impl AsRef<str> for CheckpointId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
impl std::fmt::Display for CheckpointId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CheckpointRef(pub String);
impl From<String> for CheckpointRef {
    fn from(value: String) -> Self {
        Self(value)
    }
}
impl From<&str> for CheckpointRef {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}
impl AsRef<str> for CheckpointRef {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
impl std::fmt::Display for CheckpointRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CheckpointScopeId(pub String);
impl From<String> for CheckpointScopeId {
    fn from(value: String) -> Self {
        Self(value)
    }
}
impl From<&str> for CheckpointScopeId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}
impl AsRef<str> for CheckpointScopeId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
impl std::fmt::Display for CheckpointScopeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CommandId(pub String);
impl From<String> for CommandId {
    fn from(value: String) -> Self {
        Self(value)
    }
}
impl From<&str> for CommandId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}
impl AsRef<str> for CommandId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
impl std::fmt::Display for CommandId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ComposerContextId(pub String);
impl From<String> for ComposerContextId {
    fn from(value: String) -> Self {
        Self(value)
    }
}
impl From<&str> for ComposerContextId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}
impl AsRef<str> for ComposerContextId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
impl std::fmt::Display for ComposerContextId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ComposerContextRecord {
    Variant1(ImageContextRecord),
    Variant2(FileContextRecord),
    Variant3(TerminalContextRecord),
    Variant4(ElementContextRecord),
    Variant5(PreviewAnnotationContextRecord),
    Variant6(ReviewCommentContextRecord),
    Variant7(MentionContextRecord),
    Variant8(SkillContextRecord),
    Variant9(ThreadContextRecord),
    Variant10(UnknownContextRecord),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ContextHandoffId(pub String);
impl From<String> for ContextHandoffId {
    fn from(value: String) -> Self {
        Self(value)
    }
}
impl From<&str> for ContextHandoffId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}
impl AsRef<str> for ContextHandoffId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
impl std::fmt::Display for ContextHandoffId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ContextTransferId(pub String);
impl From<String> for ContextTransferId {
    fn from(value: String) -> Self {
        Self(value)
    }
}
impl From<&str> for ContextTransferId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}
impl AsRef<str> for ContextTransferId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
impl std::fmt::Display for ContextTransferId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeviceActionUnavailableErrorTag {
    #[serde(rename = "DeviceActionUnavailableError")]
    DeviceActionUnavailableError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeviceActionUnavailableErrorReason {
    #[serde(rename = "unsupported")]
    Unsupported,
    #[serde(rename = "helper_missing")]
    HelperMissing,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceActionUnavailableError {
    #[serde(rename = "_tag")]
    pub _tag: DeviceActionUnavailableErrorTag,
    #[serde(rename = "operation")]
    pub operation: IsoDateTime,
    #[serde(rename = "platform")]
    pub platform: DevicePlatform,
    #[serde(rename = "reason")]
    pub reason: DeviceActionUnavailableErrorReason,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeviceBootErrorTag {
    #[serde(rename = "DeviceBootError")]
    DeviceBootError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeviceBootErrorReason {
    #[serde(rename = "disk_space")]
    DiskSpace,
    #[serde(rename = "timeout")]
    Timeout,
    #[serde(rename = "launch_failed")]
    LaunchFailed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceBootError {
    #[serde(rename = "_tag")]
    pub _tag: DeviceBootErrorTag,
    #[serde(rename = "hostId")]
    pub host_id: DeviceHostId,
    #[serde(rename = "deviceId")]
    pub device_id: DeviceId,
    #[serde(rename = "reason")]
    pub reason: DeviceBootErrorReason,
    #[serde(rename = "cause")]
    pub cause: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DeviceHostId(pub String);
impl From<String> for DeviceHostId {
    fn from(value: String) -> Self {
        Self(value)
    }
}
impl From<&str> for DeviceHostId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}
impl AsRef<str> for DeviceHostId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
impl std::fmt::Display for DeviceHostId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeviceHostStatus {
    #[serde(rename = "disabled")]
    Disabled,
    #[serde(rename = "idle")]
    Idle,
    #[serde(rename = "installing")]
    Installing,
    #[serde(rename = "starting")]
    Starting,
    #[serde(rename = "ready")]
    Ready,
    #[serde(rename = "failed")]
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeviceHostSummaryKind {
    #[serde(rename = "local")]
    Local,
    #[serde(rename = "ssh")]
    Ssh,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceHostSummary {
    #[serde(rename = "id")]
    pub id: DeviceHostId,
    #[serde(rename = "kind")]
    pub kind: DeviceHostSummaryKind,
    #[serde(rename = "label")]
    pub label: TrimmedNonEmptyString,
    #[serde(rename = "platforms")]
    pub platforms: Vec<DevicePlatformAvailability>,
    #[serde(rename = "tools", default, skip_serializing_if = "Optional::is_absent")]
    pub tools: Optional<DeviceToolVersions>,
    #[serde(
        rename = "toolInspectionError",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_inspection_error: Optional<IsoDateTime>,
    #[serde(rename = "hubInstalled")]
    pub hub_installed: bool,
    #[serde(rename = "agentDeviceInstalled")]
    pub agent_device_installed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeviceHostUnavailableErrorTag {
    #[serde(rename = "DeviceHostUnavailableError")]
    DeviceHostUnavailableError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceHostUnavailableError {
    #[serde(rename = "_tag")]
    pub _tag: DeviceHostUnavailableErrorTag,
    #[serde(rename = "hostId")]
    pub host_id: DeviceHostId,
    #[serde(rename = "reason")]
    pub reason: IsoDateTime,
    #[serde(rename = "cause", default, skip_serializing_if = "Optional::is_absent")]
    pub cause: Optional<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DeviceId(pub String);
impl From<String> for DeviceId {
    fn from(value: String) -> Self {
        Self(value)
    }
}
impl From<&str> for DeviceId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}
impl AsRef<str> for DeviceId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
impl std::fmt::Display for DeviceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeviceNotFoundErrorTag {
    #[serde(rename = "DeviceNotFoundError")]
    DeviceNotFoundError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceNotFoundError {
    #[serde(rename = "_tag")]
    pub _tag: DeviceNotFoundErrorTag,
    #[serde(rename = "hostId")]
    pub host_id: DeviceHostId,
    #[serde(rename = "deviceId")]
    pub device_id: DeviceId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeviceOperationErrorTag {
    #[serde(rename = "DeviceOperationError")]
    DeviceOperationError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeviceOperationErrorReason {
    #[serde(rename = "command_failed")]
    CommandFailed,
    #[serde(rename = "request_failed")]
    RequestFailed,
    #[serde(rename = "invalid_payload")]
    InvalidPayload,
    #[serde(rename = "settings_failed")]
    SettingsFailed,
    #[serde(rename = "hub_rejected")]
    HubRejected,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceOperationError {
    #[serde(rename = "_tag")]
    pub _tag: DeviceOperationErrorTag,
    #[serde(rename = "operation")]
    pub operation: IsoDateTime,
    #[serde(rename = "reason")]
    pub reason: DeviceOperationErrorReason,
    #[serde(
        rename = "exitCode",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub exit_code: Optional<JsonNumber>,
    #[serde(rename = "cause")]
    pub cause: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DevicePlatform {
    #[serde(rename = "ios")]
    Ios,
    #[serde(rename = "android")]
    Android,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DevicePlatformAvailability {
    #[serde(rename = "platform")]
    pub platform: DevicePlatform,
    #[serde(rename = "available")]
    pub available: bool,
    #[serde(
        rename = "reason",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub reason: Optional<IsoDateTime>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DevicePlatformUnavailableErrorTag {
    #[serde(rename = "DevicePlatformUnavailableError")]
    DevicePlatformUnavailableError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DevicePlatformUnavailableError {
    #[serde(rename = "_tag")]
    pub _tag: DevicePlatformUnavailableErrorTag,
    #[serde(rename = "hostId")]
    pub host_id: DeviceHostId,
    #[serde(rename = "platform")]
    pub platform: DevicePlatform,
    #[serde(rename = "reason")]
    pub reason: IsoDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceSummary {
    #[serde(rename = "hostId")]
    pub host_id: DeviceHostId,
    #[serde(rename = "id")]
    pub id: DeviceId,
    #[serde(rename = "platform")]
    pub platform: DevicePlatform,
    #[serde(rename = "name")]
    pub name: TrimmedNonEmptyString,
    #[serde(rename = "version")]
    pub version: IsoDateTime,
    #[serde(rename = "booted")]
    pub booted: bool,
    #[serde(rename = "physical")]
    pub physical: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceToolErrorDeviceToolUnavailableError {
    #[serde(rename = "reason")]
    pub reason: IsoDateTime,
    #[serde(rename = "cause", default, skip_serializing_if = "Optional::is_absent")]
    pub cause: Optional<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceToolErrorDeviceHostUnavailableError {
    #[serde(rename = "hostId")]
    pub host_id: DeviceHostId,
    #[serde(rename = "reason")]
    pub reason: IsoDateTime,
    #[serde(rename = "cause", default, skip_serializing_if = "Optional::is_absent")]
    pub cause: Optional<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceToolErrorDevicePlatformUnavailableError {
    #[serde(rename = "hostId")]
    pub host_id: DeviceHostId,
    #[serde(rename = "platform")]
    pub platform: DevicePlatform,
    #[serde(rename = "reason")]
    pub reason: IsoDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceToolErrorDeviceNotFoundError {
    #[serde(rename = "hostId")]
    pub host_id: DeviceHostId,
    #[serde(rename = "deviceId")]
    pub device_id: DeviceId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeviceToolErrorDeviceBootErrorReason {
    #[serde(rename = "disk_space")]
    DiskSpace,
    #[serde(rename = "timeout")]
    Timeout,
    #[serde(rename = "launch_failed")]
    LaunchFailed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceToolErrorDeviceBootError {
    #[serde(rename = "hostId")]
    pub host_id: DeviceHostId,
    #[serde(rename = "deviceId")]
    pub device_id: DeviceId,
    #[serde(rename = "reason")]
    pub reason: DeviceToolErrorDeviceBootErrorReason,
    #[serde(rename = "cause")]
    pub cause: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeviceToolErrorDeviceOperationErrorReason {
    #[serde(rename = "command_failed")]
    CommandFailed,
    #[serde(rename = "request_failed")]
    RequestFailed,
    #[serde(rename = "invalid_payload")]
    InvalidPayload,
    #[serde(rename = "settings_failed")]
    SettingsFailed,
    #[serde(rename = "hub_rejected")]
    HubRejected,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceToolErrorDeviceOperationError {
    #[serde(rename = "operation")]
    pub operation: IsoDateTime,
    #[serde(rename = "reason")]
    pub reason: DeviceToolErrorDeviceOperationErrorReason,
    #[serde(
        rename = "exitCode",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub exit_code: Optional<JsonNumber>,
    #[serde(rename = "cause")]
    pub cause: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeviceToolErrorDeviceActionUnavailableErrorReason {
    #[serde(rename = "unsupported")]
    Unsupported,
    #[serde(rename = "helper_missing")]
    HelperMissing,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceToolErrorDeviceActionUnavailableError {
    #[serde(rename = "operation")]
    pub operation: IsoDateTime,
    #[serde(rename = "platform")]
    pub platform: DevicePlatform,
    #[serde(rename = "reason")]
    pub reason: DeviceToolErrorDeviceActionUnavailableErrorReason,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "_tag")]
pub enum DeviceToolError {
    #[serde(rename = "DeviceToolUnavailableError")]
    DeviceToolUnavailableError(Box<DeviceToolErrorDeviceToolUnavailableError>),
    #[serde(rename = "DeviceHostUnavailableError")]
    DeviceHostUnavailableError(Box<DeviceToolErrorDeviceHostUnavailableError>),
    #[serde(rename = "DevicePlatformUnavailableError")]
    DevicePlatformUnavailableError(Box<DeviceToolErrorDevicePlatformUnavailableError>),
    #[serde(rename = "DeviceNotFoundError")]
    DeviceNotFoundError(Box<DeviceToolErrorDeviceNotFoundError>),
    #[serde(rename = "DeviceBootError")]
    DeviceBootError(Box<DeviceToolErrorDeviceBootError>),
    #[serde(rename = "DeviceOperationError")]
    DeviceOperationError(Box<DeviceToolErrorDeviceOperationError>),
    #[serde(rename = "DeviceActionUnavailableError")]
    DeviceActionUnavailableError(Box<DeviceToolErrorDeviceActionUnavailableError>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceToolListResultHostStatusesValue {
    #[serde(rename = "status")]
    pub status: DeviceHostStatus,
    #[serde(
        rename = "detail",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub detail: Optional<IsoDateTime>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceToolListResultOpenItem {
    #[serde(rename = "hostId")]
    pub host_id: DeviceHostId,
    #[serde(rename = "deviceId")]
    pub device_id: DeviceId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceToolListResult {
    #[serde(rename = "hostStatuses")]
    pub host_statuses: BTreeMap<DeviceHostId, DeviceToolListResultHostStatusesValue>,
    #[serde(rename = "hosts")]
    pub hosts: Vec<DeviceHostSummary>,
    #[serde(rename = "devices")]
    pub devices: Vec<DeviceSummary>,
    #[serde(rename = "open")]
    pub open: Vec<DeviceToolListResultOpenItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceToolOpenResultAgentDevice {
    #[serde(rename = "command")]
    pub command: IsoDateTime,
    #[serde(rename = "targetArgs")]
    pub target_args: Vec<IsoDateTime>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceToolOpenResult {
    #[serde(rename = "device")]
    pub device: DeviceSummary,
    #[serde(rename = "agentDevice")]
    pub agent_device: DeviceToolOpenResultAgentDevice,
    #[serde(rename = "quickStart")]
    pub quick_start: IsoDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeviceToolScreenshotResultScreenshotMimeType {
    #[serde(rename = "image/png")]
    ImagePng,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceToolScreenshotResultScreenshot {
    #[serde(rename = "mimeType")]
    pub mime_type: DeviceToolScreenshotResultScreenshotMimeType,
    #[serde(rename = "data")]
    pub data: IsoDateTime,
    #[serde(rename = "width")]
    pub width: i64,
    #[serde(rename = "height")]
    pub height: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceToolScreenshotResult {
    #[serde(rename = "device")]
    pub device: DeviceSummary,
    #[serde(rename = "screenshot")]
    pub screenshot: DeviceToolScreenshotResultScreenshot,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeviceToolUnavailableErrorTag {
    #[serde(rename = "DeviceToolUnavailableError")]
    DeviceToolUnavailableError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceToolUnavailableError {
    #[serde(rename = "_tag")]
    pub _tag: DeviceToolUnavailableErrorTag,
    #[serde(rename = "reason")]
    pub reason: IsoDateTime,
    #[serde(rename = "cause", default, skip_serializing_if = "Optional::is_absent")]
    pub cause: Optional<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceToolVersion {
    #[serde(rename = "requiredVersion")]
    pub required_version: IsoDateTime,
    #[serde(rename = "installedVersions")]
    pub installed_versions: Vec<IsoDateTime>,
    #[serde(rename = "runningVersion", deserialize_with = "required_nullable")]
    pub running_version: Option<IsoDateTime>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceToolVersions {
    #[serde(rename = "hub")]
    pub hub: DeviceToolVersion,
    #[serde(rename = "agent")]
    pub agent: DeviceToolVersion,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElementContextDetails {
    #[serde(rename = "pageUrl")]
    pub page_url: String,
    #[serde(rename = "pageTitle", deserialize_with = "required_nullable")]
    pub page_title: Option<String>,
    #[serde(rename = "tagName")]
    pub tag_name: String,
    #[serde(rename = "selector", deserialize_with = "required_nullable")]
    pub selector: Option<String>,
    #[serde(rename = "htmlPreview")]
    pub html_preview: String,
    #[serde(rename = "componentName", deserialize_with = "required_nullable")]
    pub component_name: Option<String>,
    #[serde(rename = "source", deserialize_with = "required_nullable")]
    pub source: Option<ElementContextSource>,
    #[serde(rename = "styles")]
    pub styles: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ElementContextRecordVersion;
impl Serialize for ElementContextRecordVersion {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serde_json::json!(1).serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for ElementContextRecordVersion {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        if value == serde_json::json!(1) {
            Ok(Self)
        } else {
            Err(serde::de::Error::custom("expected literal 1"))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ElementContextRecordKind {
    #[serde(rename = "element")]
    Element,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElementContextRecord {
    #[serde(rename = "version")]
    pub version: ElementContextRecordVersion,
    #[serde(rename = "contextId")]
    pub context_id: ComposerContextId,
    #[serde(rename = "label")]
    pub label: String,
    #[serde(rename = "kind")]
    pub kind: ElementContextRecordKind,
    #[serde(rename = "pageUrl")]
    pub page_url: String,
    #[serde(rename = "pageTitle", deserialize_with = "required_nullable")]
    pub page_title: Option<String>,
    #[serde(rename = "tagName")]
    pub tag_name: String,
    #[serde(rename = "selector", deserialize_with = "required_nullable")]
    pub selector: Option<String>,
    #[serde(rename = "htmlPreview")]
    pub html_preview: String,
    #[serde(rename = "componentName", deserialize_with = "required_nullable")]
    pub component_name: Option<String>,
    #[serde(rename = "source", deserialize_with = "required_nullable")]
    pub source: Option<ElementContextSource>,
    #[serde(rename = "styles")]
    pub styles: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElementContextSource {
    #[serde(rename = "functionName", deserialize_with = "required_nullable")]
    pub function_name: Option<String>,
    #[serde(rename = "fileName", deserialize_with = "required_nullable")]
    pub file_name: Option<String>,
    #[serde(rename = "lineNumber", deserialize_with = "required_nullable")]
    pub line_number: Option<NonNegativeInt>,
    #[serde(rename = "columnNumber", deserialize_with = "required_nullable")]
    pub column_number: Option<NonNegativeInt>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EnvironmentId(pub String);
impl From<String> for EnvironmentId {
    fn from(value: String) -> Self {
        Self(value)
    }
}
impl From<&str> for EnvironmentId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}
impl AsRef<str> for EnvironmentId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
impl std::fmt::Display for EnvironmentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EnvironmentMachineKind {
    #[serde(rename = "server")]
    Server,
    #[serde(rename = "cloud")]
    Cloud,
    #[serde(rename = "linux")]
    Linux,
    #[serde(rename = "desktop")]
    Desktop,
    #[serde(rename = "laptop")]
    Laptop,
    #[serde(rename = "mac-mini")]
    MacMini,
    #[serde(rename = "mac-studio")]
    MacStudio,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EventId(pub String);
impl From<String> for EventId {
    fn from(value: String) -> Self {
        Self(value)
    }
}
impl From<&str> for EventId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}
impl AsRef<str> for EventId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
impl std::fmt::Display for EventId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionEnvironmentPlatform {
    #[serde(rename = "os")]
    pub os: ExecutionEnvironmentPlatformOs,
    #[serde(rename = "arch")]
    pub arch: ExecutionEnvironmentPlatformArch,
    #[serde(
        rename = "machine",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub machine: Optional<EnvironmentMachineKind>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ExecutionEnvironmentPlatformArch {
    #[serde(rename = "arm64")]
    Arm64,
    #[serde(rename = "x64")]
    X64,
    #[serde(rename = "other")]
    Other,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ExecutionEnvironmentPlatformOs {
    #[serde(rename = "darwin")]
    Darwin,
    #[serde(rename = "linux")]
    Linux,
    #[serde(rename = "windows")]
    Windows,
    #[serde(rename = "unknown")]
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FileContextRecordVersion;
impl Serialize for FileContextRecordVersion {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serde_json::json!(1).serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for FileContextRecordVersion {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        if value == serde_json::json!(1) {
            Ok(Self)
        } else {
            Err(serde::de::Error::custom("expected literal 1"))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FileContextRecordKind {
    #[serde(rename = "file")]
    File,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileContextRecord {
    #[serde(rename = "version")]
    pub version: FileContextRecordVersion,
    #[serde(rename = "contextId")]
    pub context_id: ComposerContextId,
    #[serde(rename = "label")]
    pub label: String,
    #[serde(rename = "kind")]
    pub kind: FileContextRecordKind,
    #[serde(rename = "attachmentId")]
    pub attachment_id: String,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    #[serde(rename = "sizeBytes")]
    pub size_bytes: NonNegativeInt,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImageContextRecordVersion;
impl Serialize for ImageContextRecordVersion {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serde_json::json!(1).serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for ImageContextRecordVersion {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        if value == serde_json::json!(1) {
            Ok(Self)
        } else {
            Err(serde::de::Error::custom("expected literal 1"))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ImageContextRecordKind {
    #[serde(rename = "image")]
    Image,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageContextRecord {
    #[serde(rename = "version")]
    pub version: ImageContextRecordVersion,
    #[serde(rename = "contextId")]
    pub context_id: ComposerContextId,
    #[serde(rename = "label")]
    pub label: String,
    #[serde(rename = "kind")]
    pub kind: ImageContextRecordKind,
    #[serde(rename = "attachmentId")]
    pub attachment_id: String,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    #[serde(rename = "sizeBytes")]
    pub size_bytes: NonNegativeInt,
}

pub type IsoDateTime = String;

#[derive(Debug, Clone, PartialEq)]
pub struct MentionContextRecordVersion;
impl Serialize for MentionContextRecordVersion {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serde_json::json!(1).serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for MentionContextRecordVersion {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        if value == serde_json::json!(1) {
            Ok(Self)
        } else {
            Err(serde::de::Error::custom("expected literal 1"))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MentionContextRecordKind {
    #[serde(rename = "mention")]
    Mention,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MentionContextRecord {
    #[serde(rename = "version")]
    pub version: MentionContextRecordVersion,
    #[serde(rename = "contextId")]
    pub context_id: ComposerContextId,
    #[serde(rename = "label")]
    pub label: String,
    #[serde(rename = "kind")]
    pub kind: MentionContextRecordKind,
    #[serde(rename = "path")]
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MessageId(pub String);
impl From<String> for MessageId {
    fn from(value: String) -> Self {
        Self(value)
    }
}
impl From<&str> for MessageId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}
impl AsRef<str> for MessageId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
impl std::fmt::Display for MessageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NodeId(pub String);
impl From<String> for NodeId {
    fn from(value: String) -> Self {
        Self(value)
    }
}
impl From<&str> for NodeId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}
impl AsRef<str> for NodeId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

pub type NonNegativeInt = i64;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationGetWorkflowScriptErrorTag {
    #[serde(rename = "OrchestrationGetWorkflowScriptError")]
    OrchestrationGetWorkflowScriptError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationGetWorkflowScriptErrorReason {
    #[serde(rename = "invalid-path")]
    InvalidPath,
    #[serde(rename = "root-unavailable")]
    RootUnavailable,
    #[serde(rename = "not-found")]
    NotFound,
    #[serde(rename = "outside-root")]
    OutsideRoot,
    #[serde(rename = "not-js")]
    NotJs,
    #[serde(rename = "not-regular-file")]
    NotRegularFile,
    #[serde(rename = "changed-during-read")]
    ChangedDuringRead,
    #[serde(rename = "read-failed")]
    ReadFailed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationGetWorkflowScriptError {
    #[serde(rename = "_tag")]
    pub _tag: OrchestrationGetWorkflowScriptErrorTag,
    #[serde(rename = "reason")]
    pub reason: OrchestrationGetWorkflowScriptErrorReason,
    #[serde(rename = "scriptPath")]
    pub script_path: IsoDateTime,
    #[serde(rename = "cause", default, skip_serializing_if = "Optional::is_absent")]
    pub cause: Optional<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OrchestrationMessageContextVersion;
impl Serialize for OrchestrationMessageContextVersion {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serde_json::json!(1).serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for OrchestrationMessageContextVersion {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        if value == serde_json::json!(1) {
            Ok(Self)
        } else {
            Err(serde::de::Error::custom("expected literal 1"))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationMessageContext {
    #[serde(rename = "version")]
    pub version: OrchestrationMessageContextVersion,
    #[serde(rename = "records")]
    pub records: Vec<ComposerContextRecord>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationProjectShell {
    #[serde(rename = "id")]
    pub id: ProjectId,
    #[serde(rename = "title")]
    pub title: TrimmedNonEmptyString,
    #[serde(rename = "workspaceRoot")]
    pub workspace_root: TrimmedNonEmptyString,
    #[serde(
        rename = "repositoryIdentity",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub repository_identity: Optional<Option<RepositoryIdentity>>,
    #[serde(
        rename = "defaultModelSelection",
        deserialize_with = "required_nullable"
    )]
    pub default_model_selection: Option<ModelSelection>,
    #[serde(
        rename = "defaultThreadEnvMode",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub default_thread_env_mode: Optional<Option<ThreadEnvMode>>,
    #[serde(
        rename = "autoPull",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub auto_pull: Optional<bool>,
    #[serde(
        rename = "faviconPath",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub favicon_path: Optional<Option<TrimmedNonEmptyString>>,
    #[serde(
        rename = "projectIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub project_icon: Optional<Option<ProjectIconOverride>>,
    #[serde(rename = "scripts")]
    pub scripts: Vec<ProjectScript>,
    #[serde(rename = "createdAt")]
    pub created_at: IsoDateTime,
    #[serde(rename = "updatedAt")]
    pub updated_at: IsoDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationSearchThreadsResult {
    #[serde(rename = "matches")]
    pub matches: Vec<OrchestrationThreadSearchMatch>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationThreadSearchMatch {
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "projectId")]
    pub project_id: ProjectId,
    #[serde(rename = "source")]
    pub source: OrchestrationThreadSearchSource,
    #[serde(rename = "snippet")]
    pub snippet: String,
    #[serde(rename = "messageCreatedAt", deserialize_with = "required_nullable")]
    pub message_created_at: Option<IsoDateTime>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationThreadSearchSource {
    #[serde(rename = "user")]
    User,
    #[serde(rename = "assistant")]
    Assistant,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2Actor {
    #[serde(rename = "user")]
    User,
    #[serde(rename = "agent")]
    Agent,
    #[serde(rename = "system")]
    System,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2AppThreadForkedFromRun {
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId")]
    pub run_id: RunId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2AppThreadForkedFromNode {
    #[serde(rename = "nodeId")]
    pub node_id: NodeId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2AppThreadForkedFromProviderThread {
    #[serde(rename = "providerThreadId")]
    pub provider_thread_id: ProviderThreadId,
    #[serde(
        rename = "providerTurnId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_turn_id: Optional<ProviderTurnId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum OrchestrationV2AppThreadForkedFrom {
    #[serde(rename = "run")]
    Run(Box<OrchestrationV2AppThreadForkedFromRun>),
    #[serde(rename = "node")]
    Node(Box<OrchestrationV2AppThreadForkedFromNode>),
    #[serde(rename = "provider_thread")]
    ProviderThread(Box<OrchestrationV2AppThreadForkedFromProviderThread>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2AppThreadSettledOverride {
    #[serde(rename = "settled")]
    Settled,
    #[serde(rename = "active")]
    Active,
}

fn orchestration_v2_app_thread_settled_override_default()
-> Option<OrchestrationV2AppThreadSettledOverride> {
    serde_json::from_str("null").expect("upstream default")
}

fn orchestration_v2_app_thread_settled_at_default() -> Option<String> {
    serde_json::from_str("null").expect("upstream default")
}

fn orchestration_v2_app_thread_last_visited_at_default() -> Option<String> {
    serde_json::from_str("null").expect("upstream default")
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2AppThreadTitleRegeneration {
    #[serde(rename = "requestId")]
    pub request_id: CommandId,
    #[serde(rename = "startedAt")]
    pub started_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2AppThreadRollbackFailure {
    #[serde(rename = "requestId")]
    pub request_id: CommandId,
    #[serde(rename = "message")]
    pub message: TrimmedNonEmptyString,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2AppThread {
    #[serde(rename = "createdBy")]
    pub created_by: OrchestrationV2Actor,
    #[serde(rename = "creationSource")]
    pub creation_source: OrchestrationV2CreationSource,
    #[serde(rename = "id")]
    pub id: ThreadId,
    #[serde(rename = "projectId")]
    pub project_id: ProjectId,
    #[serde(rename = "title")]
    pub title: TrimmedNonEmptyString,
    #[serde(rename = "providerInstanceId")]
    pub provider_instance_id: ProviderInstanceId,
    #[serde(rename = "modelSelection")]
    pub model_selection: ModelSelection,
    #[serde(rename = "runtimeMode")]
    pub runtime_mode: RuntimeMode,
    #[serde(rename = "interactionMode")]
    pub interaction_mode: ProviderInteractionMode,
    #[serde(rename = "branch", deserialize_with = "required_nullable")]
    pub branch: Option<TrimmedNonEmptyString>,
    #[serde(rename = "worktreePath", deserialize_with = "required_nullable")]
    pub worktree_path: Option<TrimmedNonEmptyString>,
    #[serde(
        rename = "linkedPullRequest",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub linked_pull_request: Optional<Option<ThreadLinkedPullRequest>>,
    #[serde(
        rename = "pullRequests",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub pull_requests: Optional<Vec<ThreadPullRequestLink>>,
    #[serde(
        rename = "branchPullRequest",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub branch_pull_request: Optional<Option<ThreadLinkedPullRequest>>,
    #[serde(
        rename = "activeProviderThreadId",
        deserialize_with = "required_nullable"
    )]
    pub active_provider_thread_id: Option<ProviderThreadId>,
    #[serde(
        rename = "historyOrigin",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub history_origin: Optional<OrchestrationV2ThreadHistoryOrigin>,
    #[serde(rename = "lineage")]
    pub lineage: OrchestrationV2AppThreadLineage,
    #[serde(rename = "forkedFrom", deserialize_with = "required_nullable")]
    pub forked_from: Option<OrchestrationV2AppThreadForkedFrom>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "archivedAt", deserialize_with = "required_nullable")]
    pub archived_at: Option<String>,
    #[serde(
        rename = "settledOverride",
        default = "orchestration_v2_app_thread_settled_override_default"
    )]
    pub settled_override: Option<OrchestrationV2AppThreadSettledOverride>,
    #[serde(
        rename = "settledAt",
        default = "orchestration_v2_app_thread_settled_at_default"
    )]
    pub settled_at: Option<String>,
    #[serde(
        rename = "unsettledAt",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub unsettled_at: Optional<Option<String>>,
    #[serde(
        rename = "snoozedUntil",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub snoozed_until: Optional<Option<String>>,
    #[serde(
        rename = "snoozedAt",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub snoozed_at: Optional<Option<String>>,
    #[serde(
        rename = "limitRecovery",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub limit_recovery: Optional<Option<OrchestrationV2LimitRecovery>>,
    #[serde(
        rename = "pinnedAt",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub pinned_at: Optional<Option<String>>,
    #[serde(
        rename = "autoSettleDisabledAt",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub auto_settle_disabled_at: Optional<Option<String>>,
    #[serde(
        rename = "pinOrderKey",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub pin_order_key: Optional<Option<TrimmedNonEmptyString>>,
    #[serde(
        rename = "activeOrderKey",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub active_order_key: Optional<Option<TrimmedNonEmptyString>>,
    #[serde(
        rename = "lastVisitedAt",
        default = "orchestration_v2_app_thread_last_visited_at_default"
    )]
    pub last_visited_at: Option<String>,
    #[serde(
        rename = "titleRegeneration",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub title_regeneration: Optional<Option<OrchestrationV2AppThreadTitleRegeneration>>,
    #[serde(
        rename = "rollbackRequestId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub rollback_request_id: Optional<CommandId>,
    #[serde(
        rename = "rollbackFailure",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub rollback_failure: Optional<Option<OrchestrationV2AppThreadRollbackFailure>>,
    #[serde(rename = "deletedAt", deserialize_with = "required_nullable")]
    pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2AppThreadJsonForkedFromRun {
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId")]
    pub run_id: RunId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2AppThreadJsonForkedFromNode {
    #[serde(rename = "nodeId")]
    pub node_id: NodeId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2AppThreadJsonForkedFromProviderThread {
    #[serde(rename = "providerThreadId")]
    pub provider_thread_id: ProviderThreadId,
    #[serde(
        rename = "providerTurnId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_turn_id: Optional<ProviderTurnId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum OrchestrationV2AppThreadJsonForkedFrom {
    #[serde(rename = "run")]
    Run(Box<OrchestrationV2AppThreadJsonForkedFromRun>),
    #[serde(rename = "node")]
    Node(Box<OrchestrationV2AppThreadJsonForkedFromNode>),
    #[serde(rename = "provider_thread")]
    ProviderThread(Box<OrchestrationV2AppThreadJsonForkedFromProviderThread>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2AppThreadJsonSettledOverride {
    #[serde(rename = "settled")]
    Settled,
    #[serde(rename = "active")]
    Active,
}

fn orchestration_v2_app_thread_json_settled_override_default()
-> Option<OrchestrationV2AppThreadJsonSettledOverride> {
    serde_json::from_str("null").expect("upstream default")
}

fn orchestration_v2_app_thread_json_settled_at_default() -> Option<String> {
    serde_json::from_str("null").expect("upstream default")
}

fn orchestration_v2_app_thread_json_last_visited_at_default() -> Option<String> {
    serde_json::from_str("null").expect("upstream default")
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2AppThreadJsonTitleRegeneration {
    #[serde(rename = "requestId")]
    pub request_id: CommandId,
    #[serde(rename = "startedAt")]
    pub started_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2AppThreadJsonRollbackFailure {
    #[serde(rename = "requestId")]
    pub request_id: CommandId,
    #[serde(rename = "message")]
    pub message: TrimmedNonEmptyString,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2AppThreadJson {
    #[serde(rename = "createdBy")]
    pub created_by: OrchestrationV2Actor,
    #[serde(rename = "creationSource")]
    pub creation_source: OrchestrationV2CreationSource,
    #[serde(rename = "id")]
    pub id: ThreadId,
    #[serde(rename = "projectId")]
    pub project_id: ProjectId,
    #[serde(rename = "title")]
    pub title: TrimmedNonEmptyString,
    #[serde(rename = "providerInstanceId")]
    pub provider_instance_id: ProviderInstanceId,
    #[serde(rename = "modelSelection")]
    pub model_selection: ModelSelection,
    #[serde(rename = "runtimeMode")]
    pub runtime_mode: RuntimeMode,
    #[serde(rename = "interactionMode")]
    pub interaction_mode: ProviderInteractionMode,
    #[serde(rename = "branch", deserialize_with = "required_nullable")]
    pub branch: Option<TrimmedNonEmptyString>,
    #[serde(rename = "worktreePath", deserialize_with = "required_nullable")]
    pub worktree_path: Option<TrimmedNonEmptyString>,
    #[serde(
        rename = "linkedPullRequest",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub linked_pull_request: Optional<Option<ThreadLinkedPullRequest>>,
    #[serde(
        rename = "pullRequests",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub pull_requests: Optional<Vec<ThreadPullRequestLink>>,
    #[serde(
        rename = "branchPullRequest",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub branch_pull_request: Optional<Option<ThreadLinkedPullRequest>>,
    #[serde(
        rename = "activeProviderThreadId",
        deserialize_with = "required_nullable"
    )]
    pub active_provider_thread_id: Option<ProviderThreadId>,
    #[serde(
        rename = "historyOrigin",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub history_origin: Optional<OrchestrationV2ThreadHistoryOrigin>,
    #[serde(rename = "lineage")]
    pub lineage: OrchestrationV2AppThreadLineage,
    #[serde(rename = "forkedFrom", deserialize_with = "required_nullable")]
    pub forked_from: Option<OrchestrationV2AppThreadJsonForkedFrom>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "archivedAt", deserialize_with = "required_nullable")]
    pub archived_at: Option<String>,
    #[serde(
        rename = "settledOverride",
        default = "orchestration_v2_app_thread_json_settled_override_default"
    )]
    pub settled_override: Option<OrchestrationV2AppThreadJsonSettledOverride>,
    #[serde(
        rename = "settledAt",
        default = "orchestration_v2_app_thread_json_settled_at_default"
    )]
    pub settled_at: Option<String>,
    #[serde(
        rename = "unsettledAt",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub unsettled_at: Optional<Option<String>>,
    #[serde(
        rename = "snoozedUntil",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub snoozed_until: Optional<Option<String>>,
    #[serde(
        rename = "snoozedAt",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub snoozed_at: Optional<Option<String>>,
    #[serde(
        rename = "limitRecovery",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub limit_recovery: Optional<Option<OrchestrationV2LimitRecovery>>,
    #[serde(
        rename = "pinnedAt",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub pinned_at: Optional<Option<String>>,
    #[serde(
        rename = "autoSettleDisabledAt",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub auto_settle_disabled_at: Optional<Option<String>>,
    #[serde(
        rename = "pinOrderKey",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub pin_order_key: Optional<Option<TrimmedNonEmptyString>>,
    #[serde(
        rename = "activeOrderKey",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub active_order_key: Optional<Option<TrimmedNonEmptyString>>,
    #[serde(
        rename = "lastVisitedAt",
        default = "orchestration_v2_app_thread_json_last_visited_at_default"
    )]
    pub last_visited_at: Option<String>,
    #[serde(
        rename = "titleRegeneration",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub title_regeneration: Optional<Option<OrchestrationV2AppThreadJsonTitleRegeneration>>,
    #[serde(
        rename = "rollbackRequestId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub rollback_request_id: Optional<CommandId>,
    #[serde(
        rename = "rollbackFailure",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub rollback_failure: Optional<Option<OrchestrationV2AppThreadJsonRollbackFailure>>,
    #[serde(rename = "deletedAt", deserialize_with = "required_nullable")]
    pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2AppThreadLineageRelationshipToParent {
    #[serde(rename = "fork")]
    Fork,
    #[serde(rename = "subagent")]
    Subagent,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2AppThreadLineage {
    #[serde(rename = "parentThreadId", deserialize_with = "required_nullable")]
    pub parent_thread_id: Option<ThreadId>,
    #[serde(
        rename = "relationshipToParent",
        deserialize_with = "required_nullable"
    )]
    pub relationship_to_parent: Option<OrchestrationV2AppThreadLineageRelationshipToParent>,
    #[serde(rename = "rootThreadId")]
    pub root_thread_id: ThreadId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ApprovalCapabilities {
    #[serde(rename = "supportsCommandApproval")]
    pub supports_command_approval: bool,
    #[serde(rename = "supportsFileReadApproval")]
    pub supports_file_read_approval: bool,
    #[serde(rename = "supportsFileChangeApproval")]
    pub supports_file_change_approval: bool,
    #[serde(rename = "supportsApplyPatchApproval")]
    pub supports_apply_patch_approval: bool,
    #[serde(rename = "approvalsHaveNativeRequestIds")]
    pub approvals_have_native_request_ids: bool,
    #[serde(rename = "approvalCallbacksAreLiveOnly")]
    pub approval_callbacks_are_live_only: bool,
    #[serde(rename = "approvalsCanOriginateFromSubagents")]
    pub approvals_can_originate_from_subagents: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ArchivedShellSnapshot {
    #[serde(rename = "schemaVersion")]
    pub schema_version: PositiveInt,
    #[serde(rename = "snapshotSequence")]
    pub snapshot_sequence: NonNegativeInt,
    #[serde(rename = "projects")]
    pub projects: Vec<OrchestrationProjectShell>,
    #[serde(rename = "threads")]
    pub threads: Vec<OrchestrationV2ThreadShell>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ArchivedShellStreamItemSnapshot {
    #[serde(rename = "snapshot")]
    pub snapshot: OrchestrationV2ArchivedShellSnapshot,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ArchivedShellStreamItemThreadUpdated {
    #[serde(rename = "sequence")]
    pub sequence: NonNegativeInt,
    #[serde(rename = "thread")]
    pub thread: OrchestrationV2ThreadShell,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ArchivedShellStreamItemThreadRemoved {
    #[serde(rename = "sequence")]
    pub sequence: NonNegativeInt,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum OrchestrationV2ArchivedShellStreamItem {
    #[serde(rename = "snapshot")]
    Snapshot(Box<OrchestrationV2ArchivedShellStreamItemSnapshot>),
    #[serde(rename = "thread.updated")]
    ThreadUpdated(Box<OrchestrationV2ArchivedShellStreamItemThreadUpdated>),
    #[serde(rename = "thread.removed")]
    ThreadRemoved(Box<OrchestrationV2ArchivedShellStreamItemThreadRemoved>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2CheckpointStatus {
    #[serde(rename = "ready")]
    Ready,
    #[serde(rename = "missing")]
    Missing,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "stale")]
    Stale,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2Checkpoint {
    #[serde(rename = "id")]
    pub id: CheckpointId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "scopeId")]
    pub scope_id: CheckpointScopeId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "nodeId")]
    pub node_id: NodeId,
    #[serde(rename = "parentCheckpointId", deserialize_with = "required_nullable")]
    pub parent_checkpoint_id: Option<CheckpointId>,
    #[serde(rename = "ordinalWithinScope")]
    pub ordinal_within_scope: NonNegativeInt,
    #[serde(rename = "appRunOrdinal", deserialize_with = "required_nullable")]
    pub app_run_ordinal: Option<PositiveInt>,
    #[serde(rename = "ref")]
    pub r#ref: CheckpointRef,
    #[serde(rename = "status")]
    pub status: OrchestrationV2CheckpointStatus,
    #[serde(rename = "files")]
    pub files: Vec<OrchestrationV2CheckpointFileSummary>,
    #[serde(rename = "capturedAt")]
    pub captured_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CheckpointCapabilities {
    #[serde(rename = "appCanCheckpointFilesystem")]
    pub app_can_checkpoint_filesystem: bool,
    #[serde(rename = "supportsNestedCheckpointScopes")]
    pub supports_nested_checkpoint_scopes: bool,
    #[serde(rename = "providerCanRollbackConversation")]
    pub provider_can_rollback_conversation: bool,
    #[serde(rename = "providerRollbackReturnsSnapshot")]
    pub provider_rollback_returns_snapshot: bool,
    #[serde(rename = "providerCanReadConversationSnapshot")]
    pub provider_can_read_conversation_snapshot: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CheckpointFileSummary {
    #[serde(rename = "path")]
    pub path: TrimmedNonEmptyString,
    #[serde(rename = "kind")]
    pub kind: TrimmedNonEmptyString,
    #[serde(rename = "additions")]
    pub additions: NonNegativeInt,
    #[serde(rename = "deletions")]
    pub deletions: NonNegativeInt,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2CheckpointJsonStatus {
    #[serde(rename = "ready")]
    Ready,
    #[serde(rename = "missing")]
    Missing,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "stale")]
    Stale,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CheckpointJson {
    #[serde(rename = "id")]
    pub id: CheckpointId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "scopeId")]
    pub scope_id: CheckpointScopeId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "nodeId")]
    pub node_id: NodeId,
    #[serde(rename = "parentCheckpointId", deserialize_with = "required_nullable")]
    pub parent_checkpoint_id: Option<CheckpointId>,
    #[serde(rename = "ordinalWithinScope")]
    pub ordinal_within_scope: NonNegativeInt,
    #[serde(rename = "appRunOrdinal", deserialize_with = "required_nullable")]
    pub app_run_ordinal: Option<PositiveInt>,
    #[serde(rename = "ref")]
    pub r#ref: CheckpointRef,
    #[serde(rename = "status")]
    pub status: OrchestrationV2CheckpointJsonStatus,
    #[serde(rename = "files")]
    pub files: Vec<OrchestrationV2CheckpointFileSummary>,
    #[serde(rename = "capturedAt")]
    pub captured_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CheckpointRollbackRequest {
    #[serde(rename = "scopeId")]
    pub scope_id: CheckpointScopeId,
    #[serde(rename = "checkpointId")]
    pub checkpoint_id: CheckpointId,
    #[serde(rename = "requestedAt")]
    pub requested_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CheckpointRollbackRequestJson {
    #[serde(rename = "scopeId")]
    pub scope_id: CheckpointScopeId,
    #[serde(rename = "checkpointId")]
    pub checkpoint_id: CheckpointId,
    #[serde(rename = "requestedAt")]
    pub requested_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2CheckpointScopeKind {
    #[serde(rename = "root_run")]
    RootRun,
    #[serde(rename = "subagent")]
    Subagent,
    #[serde(rename = "tool")]
    Tool,
    #[serde(rename = "provider_thread")]
    ProviderThread,
    #[serde(rename = "manual")]
    Manual,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CheckpointScope {
    #[serde(rename = "id")]
    pub id: CheckpointScopeId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "nodeId")]
    pub node_id: NodeId,
    #[serde(rename = "parentScopeId", deserialize_with = "required_nullable")]
    pub parent_scope_id: Option<CheckpointScopeId>,
    #[serde(rename = "providerThreadId", deserialize_with = "required_nullable")]
    pub provider_thread_id: Option<ProviderThreadId>,
    #[serde(rename = "kind")]
    pub kind: OrchestrationV2CheckpointScopeKind,
    #[serde(rename = "ordinalWithinParent")]
    pub ordinal_within_parent: NonNegativeInt,
    #[serde(rename = "advancesAppRunCount")]
    pub advances_app_run_count: bool,
    #[serde(rename = "cwd")]
    pub cwd: TrimmedNonEmptyString,
    #[serde(rename = "createdAt")]
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2CheckpointScopeJsonKind {
    #[serde(rename = "root_run")]
    RootRun,
    #[serde(rename = "subagent")]
    Subagent,
    #[serde(rename = "tool")]
    Tool,
    #[serde(rename = "provider_thread")]
    ProviderThread,
    #[serde(rename = "manual")]
    Manual,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CheckpointScopeJson {
    #[serde(rename = "id")]
    pub id: CheckpointScopeId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "nodeId")]
    pub node_id: NodeId,
    #[serde(rename = "parentScopeId", deserialize_with = "required_nullable")]
    pub parent_scope_id: Option<CheckpointScopeId>,
    #[serde(rename = "providerThreadId", deserialize_with = "required_nullable")]
    pub provider_thread_id: Option<ProviderThreadId>,
    #[serde(rename = "kind")]
    pub kind: OrchestrationV2CheckpointScopeJsonKind,
    #[serde(rename = "ordinalWithinParent")]
    pub ordinal_within_parent: NonNegativeInt,
    #[serde(rename = "advancesAppRunCount")]
    pub advances_app_run_count: bool,
    #[serde(rename = "cwd")]
    pub cwd: TrimmedNonEmptyString,
    #[serde(rename = "createdAt")]
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2CheckpointUnavailableErrorTag {
    #[serde(rename = "OrchestrationV2CheckpointUnavailableError")]
    OrchestrationV2CheckpointUnavailableError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CheckpointUnavailableError {
    #[serde(rename = "_tag")]
    pub _tag: OrchestrationV2CheckpointUnavailableErrorTag,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "target")]
    pub target: IsoDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2CommandThreadCreateImportedNativeThreadRefStrength {
    #[serde(rename = "strong")]
    Strong,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandThreadCreateImportedNativeThreadRef {
    #[serde(rename = "driver")]
    pub driver: ProviderDriverKind,
    #[serde(rename = "nativeId")]
    pub native_id: TrimmedNonEmptyString,
    #[serde(rename = "strength")]
    pub strength: OrchestrationV2CommandThreadCreateImportedNativeThreadRefStrength,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandThreadCreateImportedNativeThread {
    #[serde(rename = "ref")]
    pub r#ref: OrchestrationV2CommandThreadCreateImportedNativeThreadRef,
    #[serde(
        rename = "metadata",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub metadata: Optional<OrchestrationV2ProviderThreadNativeMetadata>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandThreadCreate {
    #[serde(rename = "createdBy")]
    pub created_by: OrchestrationV2Actor,
    #[serde(rename = "creationSource")]
    pub creation_source: OrchestrationV2CreationSource,
    #[serde(rename = "commandId")]
    pub command_id: CommandId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "projectId")]
    pub project_id: ProjectId,
    #[serde(rename = "title")]
    pub title: TrimmedNonEmptyString,
    #[serde(rename = "modelSelection")]
    pub model_selection: ModelSelection,
    #[serde(rename = "runtimeMode")]
    pub runtime_mode: RuntimeMode,
    #[serde(rename = "interactionMode")]
    pub interaction_mode: ProviderInteractionMode,
    #[serde(rename = "branch", deserialize_with = "required_nullable")]
    pub branch: Option<TrimmedNonEmptyString>,
    #[serde(rename = "worktreePath", deserialize_with = "required_nullable")]
    pub worktree_path: Option<TrimmedNonEmptyString>,
    #[serde(
        rename = "importedNativeThread",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub imported_native_thread: Optional<OrchestrationV2CommandThreadCreateImportedNativeThread>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandThreadArchive {
    #[serde(rename = "commandId")]
    pub command_id: CommandId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandThreadUnarchive {
    #[serde(rename = "commandId")]
    pub command_id: CommandId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandThreadDelete {
    #[serde(rename = "commandId")]
    pub command_id: CommandId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandThreadSettle {
    #[serde(rename = "commandId")]
    pub command_id: CommandId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(
        rename = "settledAt",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub settled_at: Optional<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandThreadAutoSettle {
    #[serde(rename = "commandId")]
    pub command_id: CommandId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "snapshotAt")]
    pub snapshot_at: String,
    #[serde(
        rename = "settledAt",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub settled_at: Optional<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2CommandThreadUnsettleReason {
    #[serde(rename = "user")]
    User,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandThreadUnsettle {
    #[serde(rename = "commandId")]
    pub command_id: CommandId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "reason")]
    pub reason: OrchestrationV2CommandThreadUnsettleReason,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandThreadSnooze {
    #[serde(rename = "commandId")]
    pub command_id: CommandId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "snoozedUntil")]
    pub snoozed_until: IsoDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2CommandThreadUnsnoozeReason {
    #[serde(rename = "user")]
    User,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandThreadUnsnooze {
    #[serde(rename = "commandId")]
    pub command_id: CommandId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "reason")]
    pub reason: OrchestrationV2CommandThreadUnsnoozeReason,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandThreadAutoSettleSet {
    #[serde(rename = "commandId")]
    pub command_id: CommandId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "enabled")]
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandThreadPin {
    #[serde(rename = "commandId")]
    pub command_id: CommandId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(
        rename = "orderKey",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub order_key: Optional<TrimmedNonEmptyString>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandThreadUnpin {
    #[serde(rename = "commandId")]
    pub command_id: CommandId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandThreadPinReorder {
    #[serde(rename = "commandId")]
    pub command_id: CommandId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "orderKey")]
    pub order_key: TrimmedNonEmptyString,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandThreadActiveReorder {
    #[serde(rename = "commandId")]
    pub command_id: CommandId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "orderKey")]
    pub order_key: TrimmedNonEmptyString,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandThreadVisit {
    #[serde(rename = "commandId")]
    pub command_id: CommandId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "visitedAt")]
    pub visited_at: IsoDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandThreadMarkUnread {
    #[serde(rename = "commandId")]
    pub command_id: CommandId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandThreadMetadataUpdate {
    #[serde(rename = "commandId")]
    pub command_id: CommandId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "title", default, skip_serializing_if = "Optional::is_absent")]
    pub title: Optional<TrimmedNonEmptyString>,
    #[serde(
        rename = "regenerateTitle",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub regenerate_title: Optional<bool>,
    #[serde(
        rename = "branch",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub branch: Optional<Option<TrimmedNonEmptyString>>,
    #[serde(
        rename = "worktreePath",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub worktree_path: Optional<Option<TrimmedNonEmptyString>>,
    #[serde(
        rename = "expectedWorktreePath",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub expected_worktree_path: Optional<Option<TrimmedNonEmptyString>>,
    #[serde(
        rename = "expectedEmpty",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub expected_empty: Optional<bool>,
    #[serde(
        rename = "limitRecovery",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub limit_recovery: Optional<Option<OrchestrationV2LimitRecoveryUpdate>>,
    #[serde(
        rename = "linkedPullRequest",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub linked_pull_request: Optional<Option<ThreadLinkedPullRequest>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandThreadPullRequestLink {
    #[serde(rename = "commandId")]
    pub command_id: CommandId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "host")]
    pub host: TrimmedNonEmptyString,
    #[serde(rename = "repository")]
    pub repository: TrimmedNonEmptyString,
    #[serde(rename = "number")]
    pub number: PositiveInt,
    #[serde(rename = "url")]
    pub url: TrimmedNonEmptyString,
    #[serde(rename = "source")]
    pub source: ThreadPullRequestLinkSource,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandThreadPullRequestUnlink {
    #[serde(rename = "commandId")]
    pub command_id: CommandId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "host")]
    pub host: TrimmedNonEmptyString,
    #[serde(rename = "repository")]
    pub repository: TrimmedNonEmptyString,
    #[serde(rename = "number")]
    pub number: PositiveInt,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandThreadPullRequestLinkSync {
    #[serde(rename = "commandId")]
    pub command_id: CommandId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "host")]
    pub host: TrimmedNonEmptyString,
    #[serde(rename = "repository")]
    pub repository: TrimmedNonEmptyString,
    #[serde(rename = "number")]
    pub number: PositiveInt,
    #[serde(rename = "snapshot")]
    pub snapshot: ThreadPullRequestSnapshot,
    #[serde(rename = "stack", deserialize_with = "required_nullable")]
    pub stack: Option<ThreadPullRequestStack>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandThreadPullRequestWatchLink {
    #[serde(rename = "url")]
    pub url: TrimmedNonEmptyString,
    #[serde(rename = "source")]
    pub source: ThreadPullRequestLinkSource,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandThreadPullRequestWatch {
    #[serde(rename = "commandId")]
    pub command_id: CommandId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "host")]
    pub host: TrimmedNonEmptyString,
    #[serde(rename = "repository")]
    pub repository: TrimmedNonEmptyString,
    #[serde(rename = "number")]
    pub number: PositiveInt,
    #[serde(rename = "watching")]
    pub watching: bool,
    #[serde(rename = "link", default, skip_serializing_if = "Optional::is_absent")]
    pub link: Optional<OrchestrationV2CommandThreadPullRequestWatchLink>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandThreadPullRequestSyncExpected {
    #[serde(rename = "workspaceRoot")]
    pub workspace_root: TrimmedNonEmptyString,
    #[serde(rename = "branch", deserialize_with = "required_nullable")]
    pub branch: Option<TrimmedNonEmptyString>,
    #[serde(rename = "worktreePath", deserialize_with = "required_nullable")]
    pub worktree_path: Option<TrimmedNonEmptyString>,
    #[serde(rename = "linkedPullRequest", deserialize_with = "required_nullable")]
    pub linked_pull_request: Option<ThreadLinkedPullRequest>,
    #[serde(rename = "branchPullRequest", deserialize_with = "required_nullable")]
    pub branch_pull_request: Option<ThreadLinkedPullRequest>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandThreadPullRequestSync {
    #[serde(rename = "commandId")]
    pub command_id: CommandId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "projectId")]
    pub project_id: ProjectId,
    #[serde(rename = "snapshotSequence")]
    pub snapshot_sequence: NonNegativeInt,
    #[serde(rename = "expected")]
    pub expected: OrchestrationV2CommandThreadPullRequestSyncExpected,
    #[serde(rename = "branchPullRequest", deserialize_with = "required_nullable")]
    pub branch_pull_request: Option<ThreadLinkedPullRequest>,
    #[serde(
        rename = "linkedPullRequest",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub linked_pull_request: Optional<ThreadLinkedPullRequest>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandThreadTitleRegenerationComplete {
    #[serde(rename = "commandId")]
    pub command_id: CommandId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "requestId")]
    pub request_id: CommandId,
    #[serde(rename = "title", default, skip_serializing_if = "Optional::is_absent")]
    pub title: Optional<TrimmedNonEmptyString>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandThreadRuntimeModeSet {
    #[serde(rename = "commandId")]
    pub command_id: CommandId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runtimeMode")]
    pub runtime_mode: RuntimeMode,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandThreadInteractionModeSet {
    #[serde(rename = "commandId")]
    pub command_id: CommandId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "interactionMode")]
    pub interaction_mode: ProviderInteractionMode,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandThreadModelSelectionSet {
    #[serde(rename = "commandId")]
    pub command_id: CommandId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "modelSelection")]
    pub model_selection: ModelSelection,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandProviderSessionDetach {
    #[serde(rename = "commandId")]
    pub command_id: CommandId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "providerSessionId")]
    pub provider_session_id: ProviderSessionId,
    #[serde(
        rename = "reason",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub reason: Optional<IsoDateTime>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandMessageDispatchSourcePlanRef {
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "planId")]
    pub plan_id: PlanId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2CommandMessageDispatchDeliveryIntent {
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "steer")]
    Steer,
    #[serde(rename = "restart")]
    Restart,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandMessageDispatchDelegatedCompletion {
    #[serde(rename = "parentRunId")]
    pub parent_run_id: RunId,
    #[serde(rename = "generation")]
    pub generation: PositiveInt,
    #[serde(rename = "taskIds")]
    pub task_ids: Vec<NodeId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandMessageDispatchDispatchModeDeferStart {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandMessageDispatchDispatchModeSteerActive {
    #[serde(rename = "targetRunId")]
    pub target_run_id: RunId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandMessageDispatchDispatchModeRestartActive {
    #[serde(rename = "targetRunId")]
    pub target_run_id: RunId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandMessageDispatchDispatchModeQueueAfterActive {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandMessageDispatchDispatchModeStartImmediately {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum OrchestrationV2CommandMessageDispatchDispatchMode {
    #[serde(rename = "defer_start")]
    DeferStart(Box<OrchestrationV2CommandMessageDispatchDispatchModeDeferStart>),
    #[serde(rename = "steer_active")]
    SteerActive(Box<OrchestrationV2CommandMessageDispatchDispatchModeSteerActive>),
    #[serde(rename = "restart_active")]
    RestartActive(Box<OrchestrationV2CommandMessageDispatchDispatchModeRestartActive>),
    #[serde(rename = "queue_after_active")]
    QueueAfterActive(Box<OrchestrationV2CommandMessageDispatchDispatchModeQueueAfterActive>),
    #[serde(rename = "start_immediately")]
    StartImmediately(Box<OrchestrationV2CommandMessageDispatchDispatchModeStartImmediately>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandMessageDispatch {
    #[serde(
        rename = "notification",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub notification: Optional<OrchestrationV2Notification>,
    #[serde(rename = "createdBy")]
    pub created_by: OrchestrationV2Actor,
    #[serde(rename = "creationSource")]
    pub creation_source: OrchestrationV2CreationSource,
    #[serde(
        rename = "scheduledTaskId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub scheduled_task_id: Optional<ScheduledTaskId>,
    #[serde(
        rename = "senderThreadId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub sender_thread_id: Optional<ThreadId>,
    #[serde(rename = "commandId")]
    pub command_id: CommandId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "messageId")]
    pub message_id: MessageId,
    #[serde(rename = "text")]
    pub text: IsoDateTime,
    #[serde(
        rename = "context",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub context: Optional<OrchestrationMessageContext>,
    #[serde(rename = "attachments")]
    pub attachments: Vec<ChatAttachment>,
    #[serde(
        rename = "titleSeed",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub title_seed: Optional<TrimmedNonEmptyString>,
    #[serde(
        rename = "modelSelection",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub model_selection: Optional<ModelSelection>,
    #[serde(
        rename = "sourcePlanRef",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub source_plan_ref: Optional<OrchestrationV2CommandMessageDispatchSourcePlanRef>,
    #[serde(
        rename = "restartContinuationOfRunId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub restart_continuation_of_run_id: Optional<RunId>,
    #[serde(
        rename = "usageLimitContinuationOfRunId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub usage_limit_continuation_of_run_id: Optional<RunId>,
    #[serde(
        rename = "manualContinuationOfRunId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub manual_continuation_of_run_id: Optional<RunId>,
    #[serde(
        rename = "usageLimitRecoveryRequestId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub usage_limit_recovery_request_id: Optional<CommandId>,
    #[serde(
        rename = "deliveryIntent",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub delivery_intent: Optional<OrchestrationV2CommandMessageDispatchDeliveryIntent>,
    #[serde(
        rename = "delegatedCompletion",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub delegated_completion: Optional<OrchestrationV2CommandMessageDispatchDelegatedCompletion>,
    #[serde(rename = "dispatchMode")]
    pub dispatch_mode: OrchestrationV2CommandMessageDispatchDispatchMode,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandPreparedRunRelease {
    #[serde(rename = "commandId")]
    pub command_id: CommandId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId")]
    pub run_id: RunId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandNotificationDeliveryAccept {
    #[serde(rename = "commandId")]
    pub command_id: CommandId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "messageId")]
    pub message_id: MessageId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2CommandPreparedRunProgressPhase {
    #[serde(rename = "worktree")]
    Worktree,
    #[serde(rename = "setup")]
    Setup,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandPreparedRunProgress {
    #[serde(rename = "commandId")]
    pub command_id: CommandId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId")]
    pub run_id: RunId,
    #[serde(rename = "phase")]
    pub phase: OrchestrationV2CommandPreparedRunProgressPhase,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandPreparedRunFail {
    #[serde(rename = "commandId")]
    pub command_id: CommandId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId")]
    pub run_id: RunId,
    #[serde(rename = "failure")]
    pub failure: OrchestrationV2ProviderFailure,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandRunInterrupt {
    #[serde(rename = "commandId")]
    pub command_id: CommandId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId")]
    pub run_id: RunId,
    #[serde(
        rename = "reason",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub reason: Optional<IsoDateTime>,
    #[serde(
        rename = "holdQueue",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub hold_queue: Optional<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandQueuedMessagePromoteToSteer {
    #[serde(rename = "commandId")]
    pub command_id: CommandId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "queuedRunId")]
    pub queued_run_id: RunId,
    #[serde(rename = "targetRunId")]
    pub target_run_id: RunId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandQueueResume {
    #[serde(rename = "commandId")]
    pub command_id: CommandId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandQueuedRunReorder {
    #[serde(rename = "commandId")]
    pub command_id: CommandId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId")]
    pub run_id: RunId,
    #[serde(rename = "beforeRunId", deserialize_with = "required_nullable")]
    pub before_run_id: Option<RunId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandQueuedRunCancel {
    #[serde(rename = "commandId")]
    pub command_id: CommandId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId")]
    pub run_id: RunId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandQueuedRunEdit {
    #[serde(
        rename = "context",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub context: Optional<OrchestrationMessageContext>,
    #[serde(rename = "commandId")]
    pub command_id: CommandId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId")]
    pub run_id: RunId,
    #[serde(rename = "text")]
    pub text: IsoDateTime,
    #[serde(
        rename = "attachments",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub attachments: Optional<Vec<ChatAttachment>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandRuntimeRequestRespond {
    #[serde(rename = "commandId")]
    pub command_id: CommandId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "requestId")]
    pub request_id: RuntimeRequestId,
    #[serde(
        rename = "decision",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub decision: Optional<ProviderApprovalDecision>,
    #[serde(
        rename = "answers",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub answers: Optional<ProviderUserInputAnswers>,
    #[serde(
        rename = "attachmentsByQuestionId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub attachments_by_question_id: Optional<UserInputAttachments>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandThreadUserInputDismiss {
    #[serde(rename = "commandId")]
    pub command_id: CommandId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "requestId")]
    pub request_id: RuntimeRequestId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandCheckpointRollback {
    #[serde(
        rename = "restoreFiles",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub restore_files: Optional<bool>,
    #[serde(rename = "commandId")]
    pub command_id: CommandId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "scopeId")]
    pub scope_id: CheckpointScopeId,
    #[serde(rename = "checkpointId")]
    pub checkpoint_id: CheckpointId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandThreadFork {
    #[serde(rename = "createdBy")]
    pub created_by: OrchestrationV2Actor,
    #[serde(rename = "creationSource")]
    pub creation_source: OrchestrationV2CreationSource,
    #[serde(rename = "commandId")]
    pub command_id: CommandId,
    #[serde(rename = "sourceThreadId")]
    pub source_thread_id: ThreadId,
    #[serde(rename = "targetThreadId")]
    pub target_thread_id: ThreadId,
    #[serde(rename = "sourcePoint")]
    pub source_point: OrchestrationV2ThreadForkSourcePoint,
    #[serde(rename = "title", default, skip_serializing_if = "Optional::is_absent")]
    pub title: Optional<TrimmedNonEmptyString>,
    #[serde(
        rename = "createdAt",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub created_at: Optional<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandThreadMergeBack {
    #[serde(rename = "createdBy")]
    pub created_by: OrchestrationV2Actor,
    #[serde(rename = "creationSource")]
    pub creation_source: OrchestrationV2CreationSource,
    #[serde(rename = "commandId")]
    pub command_id: CommandId,
    #[serde(rename = "sourceThreadId")]
    pub source_thread_id: ThreadId,
    #[serde(rename = "targetThreadId")]
    pub target_thread_id: ThreadId,
    #[serde(rename = "sourcePoint")]
    pub source_point: OrchestrationV2ThreadForkSourcePoint,
    #[serde(
        rename = "createdAt",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub created_at: Optional<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2CommandDelegatedTaskRequestCompletionWake {
    #[serde(rename = "always")]
    Always,
    #[serde(rename = "settled_only")]
    SettledOnly,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandDelegatedTaskRequest {
    #[serde(rename = "createdBy")]
    pub created_by: OrchestrationV2Actor,
    #[serde(rename = "creationSource")]
    pub creation_source: OrchestrationV2CreationSource,
    #[serde(rename = "commandId")]
    pub command_id: CommandId,
    #[serde(rename = "parentThreadId")]
    pub parent_thread_id: ThreadId,
    #[serde(rename = "parentRunId")]
    pub parent_run_id: RunId,
    #[serde(rename = "parentNodeId")]
    pub parent_node_id: NodeId,
    #[serde(rename = "task")]
    pub task: TrimmedNonEmptyString,
    #[serde(rename = "title", default, skip_serializing_if = "Optional::is_absent")]
    pub title: Optional<TrimmedNonEmptyString>,
    #[serde(rename = "modelSelection")]
    pub model_selection: ModelSelection,
    #[serde(rename = "runtimeMode")]
    pub runtime_mode: RuntimeMode,
    #[serde(rename = "interactionMode")]
    pub interaction_mode: ProviderInteractionMode,
    #[serde(
        rename = "completionWake",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub completion_wake: Optional<OrchestrationV2CommandDelegatedTaskRequestCompletionWake>,
    #[serde(
        rename = "createdAt",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub created_at: Optional<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2CommandDelegatedTaskWakePolicyCompletionWake {
    #[serde(rename = "always")]
    Always,
    #[serde(rename = "settled_only")]
    SettledOnly,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandDelegatedTaskWakePolicy {
    #[serde(rename = "commandId")]
    pub command_id: CommandId,
    #[serde(rename = "parentThreadId")]
    pub parent_thread_id: ThreadId,
    #[serde(rename = "taskId")]
    pub task_id: NodeId,
    #[serde(rename = "completionWake")]
    pub completion_wake: OrchestrationV2CommandDelegatedTaskWakePolicyCompletionWake,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandDelegatedTaskCompletionDeliveryAcknowledge {
    #[serde(rename = "commandId")]
    pub command_id: CommandId,
    #[serde(rename = "parentThreadId")]
    pub parent_thread_id: ThreadId,
    #[serde(rename = "taskId")]
    pub task_id: NodeId,
    #[serde(rename = "observedByRunId", deserialize_with = "required_nullable")]
    pub observed_by_run_id: Option<RunId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandDelegatedTaskCompletionDeliveryDispose {
    #[serde(rename = "commandId")]
    pub command_id: CommandId,
    #[serde(rename = "parentThreadId")]
    pub parent_thread_id: ThreadId,
    #[serde(rename = "taskId")]
    pub task_id: NodeId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandThreadCreatedRecord {
    #[serde(rename = "commandId")]
    pub command_id: CommandId,
    #[serde(rename = "parentThreadId")]
    pub parent_thread_id: ThreadId,
    #[serde(rename = "parentRunId")]
    pub parent_run_id: RunId,
    #[serde(rename = "parentNodeId")]
    pub parent_node_id: NodeId,
    #[serde(rename = "targetThreadId")]
    pub target_thread_id: ThreadId,
    #[serde(rename = "targetRunId", deserialize_with = "required_nullable")]
    pub target_run_id: Option<RunId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2CommandProviderSwitch {
    #[serde(rename = "commandId")]
    pub command_id: CommandId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "modelSelection")]
    pub model_selection: ModelSelection,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum OrchestrationV2Command {
    #[serde(rename = "thread.create")]
    ThreadCreate(Box<OrchestrationV2CommandThreadCreate>),
    #[serde(rename = "thread.archive")]
    ThreadArchive(Box<OrchestrationV2CommandThreadArchive>),
    #[serde(rename = "thread.unarchive")]
    ThreadUnarchive(Box<OrchestrationV2CommandThreadUnarchive>),
    #[serde(rename = "thread.delete")]
    ThreadDelete(Box<OrchestrationV2CommandThreadDelete>),
    #[serde(rename = "thread.settle")]
    ThreadSettle(Box<OrchestrationV2CommandThreadSettle>),
    #[serde(rename = "thread.auto-settle")]
    ThreadAutoSettle(Box<OrchestrationV2CommandThreadAutoSettle>),
    #[serde(rename = "thread.unsettle")]
    ThreadUnsettle(Box<OrchestrationV2CommandThreadUnsettle>),
    #[serde(rename = "thread.snooze")]
    ThreadSnooze(Box<OrchestrationV2CommandThreadSnooze>),
    #[serde(rename = "thread.unsnooze")]
    ThreadUnsnooze(Box<OrchestrationV2CommandThreadUnsnooze>),
    #[serde(rename = "thread.auto-settle.set")]
    ThreadAutoSettleSet(Box<OrchestrationV2CommandThreadAutoSettleSet>),
    #[serde(rename = "thread.pin")]
    ThreadPin(Box<OrchestrationV2CommandThreadPin>),
    #[serde(rename = "thread.unpin")]
    ThreadUnpin(Box<OrchestrationV2CommandThreadUnpin>),
    #[serde(rename = "thread.pin.reorder")]
    ThreadPinReorder(Box<OrchestrationV2CommandThreadPinReorder>),
    #[serde(rename = "thread.active.reorder")]
    ThreadActiveReorder(Box<OrchestrationV2CommandThreadActiveReorder>),
    #[serde(rename = "thread.visit")]
    ThreadVisit(Box<OrchestrationV2CommandThreadVisit>),
    #[serde(rename = "thread.mark-unread")]
    ThreadMarkUnread(Box<OrchestrationV2CommandThreadMarkUnread>),
    #[serde(rename = "thread.metadata.update")]
    ThreadMetadataUpdate(Box<OrchestrationV2CommandThreadMetadataUpdate>),
    #[serde(rename = "thread.pull-request.link")]
    ThreadPullRequestLink(Box<OrchestrationV2CommandThreadPullRequestLink>),
    #[serde(rename = "thread.pull-request.unlink")]
    ThreadPullRequestUnlink(Box<OrchestrationV2CommandThreadPullRequestUnlink>),
    #[serde(rename = "thread.pull-request-link.sync")]
    ThreadPullRequestLinkSync(Box<OrchestrationV2CommandThreadPullRequestLinkSync>),
    #[serde(rename = "thread.pull-request.watch")]
    ThreadPullRequestWatch(Box<OrchestrationV2CommandThreadPullRequestWatch>),
    #[serde(rename = "thread.pull-request.sync")]
    ThreadPullRequestSync(Box<OrchestrationV2CommandThreadPullRequestSync>),
    #[serde(rename = "thread.title.regeneration.complete")]
    ThreadTitleRegenerationComplete(Box<OrchestrationV2CommandThreadTitleRegenerationComplete>),
    #[serde(rename = "thread.runtime-mode.set")]
    ThreadRuntimeModeSet(Box<OrchestrationV2CommandThreadRuntimeModeSet>),
    #[serde(rename = "thread.interaction-mode.set")]
    ThreadInteractionModeSet(Box<OrchestrationV2CommandThreadInteractionModeSet>),
    #[serde(rename = "thread.model-selection.set")]
    ThreadModelSelectionSet(Box<OrchestrationV2CommandThreadModelSelectionSet>),
    #[serde(rename = "provider-session.detach")]
    ProviderSessionDetach(Box<OrchestrationV2CommandProviderSessionDetach>),
    #[serde(rename = "message.dispatch")]
    MessageDispatch(Box<OrchestrationV2CommandMessageDispatch>),
    #[serde(rename = "prepared-run.release")]
    PreparedRunRelease(Box<OrchestrationV2CommandPreparedRunRelease>),
    #[serde(rename = "notification.delivery.accept")]
    NotificationDeliveryAccept(Box<OrchestrationV2CommandNotificationDeliveryAccept>),
    #[serde(rename = "prepared-run.progress")]
    PreparedRunProgress(Box<OrchestrationV2CommandPreparedRunProgress>),
    #[serde(rename = "prepared-run.fail")]
    PreparedRunFail(Box<OrchestrationV2CommandPreparedRunFail>),
    #[serde(rename = "run.interrupt")]
    RunInterrupt(Box<OrchestrationV2CommandRunInterrupt>),
    #[serde(rename = "queued-message.promote-to-steer")]
    QueuedMessagePromoteToSteer(Box<OrchestrationV2CommandQueuedMessagePromoteToSteer>),
    #[serde(rename = "queue.resume")]
    QueueResume(Box<OrchestrationV2CommandQueueResume>),
    #[serde(rename = "queued-run.reorder")]
    QueuedRunReorder(Box<OrchestrationV2CommandQueuedRunReorder>),
    #[serde(rename = "queued-run.cancel")]
    QueuedRunCancel(Box<OrchestrationV2CommandQueuedRunCancel>),
    #[serde(rename = "queued-run.edit")]
    QueuedRunEdit(Box<OrchestrationV2CommandQueuedRunEdit>),
    #[serde(rename = "runtime-request.respond")]
    RuntimeRequestRespond(Box<OrchestrationV2CommandRuntimeRequestRespond>),
    #[serde(rename = "thread.user-input.dismiss")]
    ThreadUserInputDismiss(Box<OrchestrationV2CommandThreadUserInputDismiss>),
    #[serde(rename = "checkpoint.rollback")]
    CheckpointRollback(Box<OrchestrationV2CommandCheckpointRollback>),
    #[serde(rename = "thread.fork")]
    ThreadFork(Box<OrchestrationV2CommandThreadFork>),
    #[serde(rename = "thread.merge_back")]
    ThreadMergeBack(Box<OrchestrationV2CommandThreadMergeBack>),
    #[serde(rename = "delegated_task.request")]
    DelegatedTaskRequest(Box<OrchestrationV2CommandDelegatedTaskRequest>),
    #[serde(rename = "delegated_task.wake-policy")]
    DelegatedTaskWakePolicy(Box<OrchestrationV2CommandDelegatedTaskWakePolicy>),
    #[serde(rename = "delegated_task.completion-delivery.acknowledge")]
    DelegatedTaskCompletionDeliveryAcknowledge(
        Box<OrchestrationV2CommandDelegatedTaskCompletionDeliveryAcknowledge>,
    ),
    #[serde(rename = "delegated_task.completion-delivery.dispose")]
    DelegatedTaskCompletionDeliveryDispose(
        Box<OrchestrationV2CommandDelegatedTaskCompletionDeliveryDispose>,
    ),
    #[serde(rename = "thread.created.record")]
    ThreadCreatedRecord(Box<OrchestrationV2CommandThreadCreatedRecord>),
    #[serde(rename = "provider.switch")]
    ProviderSwitch(Box<OrchestrationV2CommandProviderSwitch>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ContextCapabilities {
    #[serde(rename = "acceptsSystemContext")]
    pub accepts_system_context: bool,
    #[serde(rename = "acceptsDeveloperContext")]
    pub accepts_developer_context: bool,
    #[serde(rename = "acceptsSyntheticUserContext")]
    pub accepts_synthetic_user_context: bool,
    #[serde(rename = "canGenerateSummaries")]
    pub can_generate_summaries: bool,
    #[serde(rename = "canConsumeHandoffSummaries")]
    pub can_consume_handoff_summaries: bool,
    #[serde(rename = "supportsDeltaHandoff")]
    pub supports_delta_handoff: bool,
    #[serde(rename = "supportsFullThreadHandoff")]
    pub supports_full_thread_handoff: bool,
    #[serde(
        rename = "maxRecommendedHandoffChars",
        deserialize_with = "required_nullable"
    )]
    pub max_recommended_handoff_chars: Option<PositiveInt>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ContextHandoffCoveredRunOrdinals {
    #[serde(rename = "from")]
    pub from: PositiveInt,
    #[serde(rename = "to")]
    pub to: PositiveInt,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2ContextHandoffStrategy {
    #[serde(rename = "delta_since_target_last_seen")]
    DeltaSinceTargetLastSeen,
    #[serde(rename = "fork_delta_summary")]
    ForkDeltaSummary,
    #[serde(rename = "full_thread_summary")]
    FullThreadSummary,
    #[serde(rename = "checkpoint_summary")]
    CheckpointSummary,
    #[serde(rename = "manual_context")]
    ManualContext,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2ContextHandoffStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "ready")]
    Ready,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "superseded")]
    Superseded,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ContextHandoffHistory {
    #[serde(rename = "messages")]
    pub messages: Vec<OrchestrationV2HistoricalMessage>,
    #[serde(rename = "coverage")]
    pub coverage: IsoDateTime,
    #[serde(rename = "omittedItems")]
    pub omitted_items: NonNegativeInt,
    #[serde(
        rename = "omittedItemIds",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub omitted_item_ids: Optional<Vec<TurnItemId>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2ContextHandoffDeliveryStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "injected")]
    Injected,
    #[serde(rename = "inline")]
    Inline,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ContextHandoffDelivery {
    #[serde(rename = "nativeThreadId")]
    pub native_thread_id: IsoDateTime,
    #[serde(rename = "status")]
    pub status: OrchestrationV2ContextHandoffDeliveryStatus,
    #[serde(rename = "itemIds")]
    pub item_ids: Vec<TurnItemId>,
    #[serde(
        rename = "omittedItemIds",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub omitted_item_ids: Optional<Vec<TurnItemId>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OrchestrationV2ContextHandoffDetailInTurnItem;
impl Serialize for OrchestrationV2ContextHandoffDetailInTurnItem {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serde_json::json!(true).serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for OrchestrationV2ContextHandoffDetailInTurnItem {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        if value == serde_json::json!(true) {
            Ok(Self)
        } else {
            Err(serde::de::Error::custom("expected literal true"))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ContextHandoff {
    #[serde(rename = "id")]
    pub id: ContextHandoffId,
    #[serde(
        rename = "transferId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub transfer_id: Optional<Option<ContextTransferId>>,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "targetRunId")]
    pub target_run_id: RunId,
    #[serde(rename = "fromProviderThreadIds")]
    pub from_provider_thread_ids: Vec<ProviderThreadId>,
    #[serde(rename = "toProviderThreadId")]
    pub to_provider_thread_id: ProviderThreadId,
    #[serde(rename = "coveredRunOrdinals")]
    pub covered_run_ordinals: OrchestrationV2ContextHandoffCoveredRunOrdinals,
    #[serde(rename = "strategy")]
    pub strategy: OrchestrationV2ContextHandoffStrategy,
    #[serde(rename = "status")]
    pub status: OrchestrationV2ContextHandoffStatus,
    #[serde(rename = "summaryMessageId", deserialize_with = "required_nullable")]
    pub summary_message_id: Option<MessageId>,
    #[serde(rename = "summaryText")]
    pub summary_text: IsoDateTime,
    #[serde(
        rename = "history",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub history: Optional<OrchestrationV2ContextHandoffHistory>,
    #[serde(
        rename = "delivery",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub delivery: Optional<OrchestrationV2ContextHandoffDelivery>,
    #[serde(
        rename = "detailInTurnItem",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub detail_in_turn_item: Optional<OrchestrationV2ContextHandoffDetailInTurnItem>,
    #[serde(
        rename = "createdByProviderInstanceId",
        deserialize_with = "required_nullable"
    )]
    pub created_by_provider_instance_id: Option<ProviderInstanceId>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ContextHandoffJsonCoveredRunOrdinals {
    #[serde(rename = "from")]
    pub from: PositiveInt,
    #[serde(rename = "to")]
    pub to: PositiveInt,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2ContextHandoffJsonStrategy {
    #[serde(rename = "delta_since_target_last_seen")]
    DeltaSinceTargetLastSeen,
    #[serde(rename = "fork_delta_summary")]
    ForkDeltaSummary,
    #[serde(rename = "full_thread_summary")]
    FullThreadSummary,
    #[serde(rename = "checkpoint_summary")]
    CheckpointSummary,
    #[serde(rename = "manual_context")]
    ManualContext,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2ContextHandoffJsonStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "ready")]
    Ready,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "superseded")]
    Superseded,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ContextHandoffJsonHistory {
    #[serde(rename = "messages")]
    pub messages: Vec<OrchestrationV2HistoricalMessage>,
    #[serde(rename = "coverage")]
    pub coverage: IsoDateTime,
    #[serde(rename = "omittedItems")]
    pub omitted_items: NonNegativeInt,
    #[serde(
        rename = "omittedItemIds",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub omitted_item_ids: Optional<Vec<TurnItemId>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2ContextHandoffJsonDeliveryStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "injected")]
    Injected,
    #[serde(rename = "inline")]
    Inline,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ContextHandoffJsonDelivery {
    #[serde(rename = "nativeThreadId")]
    pub native_thread_id: IsoDateTime,
    #[serde(rename = "status")]
    pub status: OrchestrationV2ContextHandoffJsonDeliveryStatus,
    #[serde(rename = "itemIds")]
    pub item_ids: Vec<TurnItemId>,
    #[serde(
        rename = "omittedItemIds",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub omitted_item_ids: Optional<Vec<TurnItemId>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OrchestrationV2ContextHandoffJsonDetailInTurnItem;
impl Serialize for OrchestrationV2ContextHandoffJsonDetailInTurnItem {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serde_json::json!(true).serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for OrchestrationV2ContextHandoffJsonDetailInTurnItem {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        if value == serde_json::json!(true) {
            Ok(Self)
        } else {
            Err(serde::de::Error::custom("expected literal true"))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ContextHandoffJson {
    #[serde(rename = "id")]
    pub id: ContextHandoffId,
    #[serde(
        rename = "transferId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub transfer_id: Optional<Option<ContextTransferId>>,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "targetRunId")]
    pub target_run_id: RunId,
    #[serde(rename = "fromProviderThreadIds")]
    pub from_provider_thread_ids: Vec<ProviderThreadId>,
    #[serde(rename = "toProviderThreadId")]
    pub to_provider_thread_id: ProviderThreadId,
    #[serde(rename = "coveredRunOrdinals")]
    pub covered_run_ordinals: OrchestrationV2ContextHandoffJsonCoveredRunOrdinals,
    #[serde(rename = "strategy")]
    pub strategy: OrchestrationV2ContextHandoffJsonStrategy,
    #[serde(rename = "status")]
    pub status: OrchestrationV2ContextHandoffJsonStatus,
    #[serde(rename = "summaryMessageId", deserialize_with = "required_nullable")]
    pub summary_message_id: Option<MessageId>,
    #[serde(rename = "summaryText")]
    pub summary_text: IsoDateTime,
    #[serde(
        rename = "history",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub history: Optional<OrchestrationV2ContextHandoffJsonHistory>,
    #[serde(
        rename = "delivery",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub delivery: Optional<OrchestrationV2ContextHandoffJsonDelivery>,
    #[serde(
        rename = "detailInTurnItem",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub detail_in_turn_item: Optional<OrchestrationV2ContextHandoffJsonDetailInTurnItem>,
    #[serde(
        rename = "createdByProviderInstanceId",
        deserialize_with = "required_nullable"
    )]
    pub created_by_provider_instance_id: Option<ProviderInstanceId>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ContextSourcePoint {
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "checkpointId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub checkpoint_id: Optional<CheckpointId>,
    #[serde(
        rename = "turnItemId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub turn_item_id: Optional<TurnItemId>,
    #[serde(
        rename = "providerThreadRef",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_thread_ref: Optional<OrchestrationV2ProviderRef>,
    #[serde(
        rename = "providerTurnRef",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_turn_ref: Optional<OrchestrationV2ProviderRef>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2ContextTransferStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "resolved_native")]
    ResolvedNative,
    #[serde(rename = "resolved_portable")]
    ResolvedPortable,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "consumed")]
    Consumed,
    #[serde(rename = "superseded")]
    Superseded,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ContextTransfer {
    #[serde(rename = "id")]
    pub id: ContextTransferId,
    #[serde(rename = "type")]
    pub r#type: OrchestrationV2ContextTransferType,
    #[serde(rename = "sourceThreadId")]
    pub source_thread_id: ThreadId,
    #[serde(rename = "targetThreadId")]
    pub target_thread_id: ThreadId,
    #[serde(rename = "sourcePoint")]
    pub source_point: OrchestrationV2ContextSourcePoint,
    #[serde(rename = "basePoint", deserialize_with = "required_nullable")]
    pub base_point: Option<OrchestrationV2ContextSourcePoint>,
    #[serde(
        rename = "sourceProviderInstanceId",
        deserialize_with = "required_nullable"
    )]
    pub source_provider_instance_id: Option<ProviderInstanceId>,
    #[serde(
        rename = "targetProviderInstanceId",
        deserialize_with = "required_nullable"
    )]
    pub target_provider_instance_id: Option<ProviderInstanceId>,
    #[serde(rename = "targetRunId", deserialize_with = "required_nullable")]
    pub target_run_id: Option<RunId>,
    #[serde(rename = "status")]
    pub status: OrchestrationV2ContextTransferStatus,
    #[serde(rename = "resolution", deserialize_with = "required_nullable")]
    pub resolution: Option<OrchestrationV2ContextTransferResolution>,
    #[serde(rename = "createdBy")]
    pub created_by: OrchestrationV2Actor,
    #[serde(rename = "error", deserialize_with = "required_nullable")]
    pub error: Option<IsoDateTime>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "consumedAt", deserialize_with = "required_nullable")]
    pub consumed_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2ContextTransferJsonStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "resolved_native")]
    ResolvedNative,
    #[serde(rename = "resolved_portable")]
    ResolvedPortable,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "consumed")]
    Consumed,
    #[serde(rename = "superseded")]
    Superseded,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ContextTransferJson {
    #[serde(rename = "id")]
    pub id: ContextTransferId,
    #[serde(rename = "type")]
    pub r#type: OrchestrationV2ContextTransferType,
    #[serde(rename = "sourceThreadId")]
    pub source_thread_id: ThreadId,
    #[serde(rename = "targetThreadId")]
    pub target_thread_id: ThreadId,
    #[serde(rename = "sourcePoint")]
    pub source_point: OrchestrationV2ContextSourcePoint,
    #[serde(rename = "basePoint", deserialize_with = "required_nullable")]
    pub base_point: Option<OrchestrationV2ContextSourcePoint>,
    #[serde(
        rename = "sourceProviderInstanceId",
        deserialize_with = "required_nullable"
    )]
    pub source_provider_instance_id: Option<ProviderInstanceId>,
    #[serde(
        rename = "targetProviderInstanceId",
        deserialize_with = "required_nullable"
    )]
    pub target_provider_instance_id: Option<ProviderInstanceId>,
    #[serde(rename = "targetRunId", deserialize_with = "required_nullable")]
    pub target_run_id: Option<RunId>,
    #[serde(rename = "status")]
    pub status: OrchestrationV2ContextTransferJsonStatus,
    #[serde(rename = "resolution", deserialize_with = "required_nullable")]
    pub resolution: Option<OrchestrationV2ContextTransferResolution>,
    #[serde(rename = "createdBy")]
    pub created_by: OrchestrationV2Actor,
    #[serde(rename = "error", deserialize_with = "required_nullable")]
    pub error: Option<IsoDateTime>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "consumedAt", deserialize_with = "required_nullable")]
    pub consumed_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ContextTransferResolutionNativeFork {
    #[serde(rename = "providerThreadRef")]
    pub provider_thread_ref: OrchestrationV2ProviderRef,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ContextTransferResolutionPortableContext {
    #[serde(rename = "contextHandoffId")]
    pub context_handoff_id: ContextHandoffId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ContextTransferResolutionDeltaContext {
    #[serde(rename = "contextHandoffId")]
    pub context_handoff_id: ContextHandoffId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ContextTransferResolutionForkDeltaContext {
    #[serde(rename = "contextHandoffId")]
    pub context_handoff_id: ContextHandoffId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ContextTransferResolutionCheckpointContext {
    #[serde(rename = "contextHandoffId")]
    pub context_handoff_id: ContextHandoffId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "strategy")]
pub enum OrchestrationV2ContextTransferResolution {
    #[serde(rename = "native_fork")]
    NativeFork(Box<OrchestrationV2ContextTransferResolutionNativeFork>),
    #[serde(rename = "portable_context")]
    PortableContext(Box<OrchestrationV2ContextTransferResolutionPortableContext>),
    #[serde(rename = "delta_context")]
    DeltaContext(Box<OrchestrationV2ContextTransferResolutionDeltaContext>),
    #[serde(rename = "fork_delta_context")]
    ForkDeltaContext(Box<OrchestrationV2ContextTransferResolutionForkDeltaContext>),
    #[serde(rename = "checkpoint_context")]
    CheckpointContext(Box<OrchestrationV2ContextTransferResolutionCheckpointContext>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2ContextTransferType {
    #[serde(rename = "fork")]
    Fork,
    #[serde(rename = "provider_handoff")]
    ProviderHandoff,
    #[serde(rename = "merge_back")]
    MergeBack,
    #[serde(rename = "subagent_spawn")]
    SubagentSpawn,
    #[serde(rename = "subagent_result")]
    SubagentResult,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2ConversationMessageRole {
    #[serde(rename = "user")]
    User,
    #[serde(rename = "assistant")]
    Assistant,
    #[serde(rename = "system")]
    System,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ConversationMessageDelegatedCompletion {
    #[serde(rename = "parentRunId")]
    pub parent_run_id: RunId,
    #[serde(rename = "generation")]
    pub generation: PositiveInt,
    #[serde(rename = "taskIds")]
    pub task_ids: Vec<NodeId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ConversationMessage {
    #[serde(
        rename = "notification",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub notification: Optional<OrchestrationV2Notification>,
    #[serde(rename = "createdBy")]
    pub created_by: OrchestrationV2Actor,
    #[serde(rename = "creationSource")]
    pub creation_source: OrchestrationV2CreationSource,
    #[serde(
        rename = "scheduledTaskId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub scheduled_task_id: Optional<ScheduledTaskId>,
    #[serde(
        rename = "senderThreadId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub sender_thread_id: Optional<ThreadId>,
    #[serde(rename = "id")]
    pub id: MessageId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "nodeId", deserialize_with = "required_nullable")]
    pub node_id: Option<NodeId>,
    #[serde(rename = "role")]
    pub role: OrchestrationV2ConversationMessageRole,
    #[serde(rename = "text")]
    pub text: IsoDateTime,
    #[serde(
        rename = "context",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub context: Optional<OrchestrationMessageContext>,
    #[serde(rename = "attachments")]
    pub attachments: Vec<ChatAttachment>,
    #[serde(rename = "streaming")]
    pub streaming: bool,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(
        rename = "delegatedCompletion",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub delegated_completion: Optional<OrchestrationV2ConversationMessageDelegatedCompletion>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2ConversationMessageJsonRole {
    #[serde(rename = "user")]
    User,
    #[serde(rename = "assistant")]
    Assistant,
    #[serde(rename = "system")]
    System,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ConversationMessageJsonDelegatedCompletion {
    #[serde(rename = "parentRunId")]
    pub parent_run_id: RunId,
    #[serde(rename = "generation")]
    pub generation: PositiveInt,
    #[serde(rename = "taskIds")]
    pub task_ids: Vec<NodeId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ConversationMessageJson {
    #[serde(
        rename = "notification",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub notification: Optional<OrchestrationV2Notification>,
    #[serde(rename = "createdBy")]
    pub created_by: OrchestrationV2Actor,
    #[serde(rename = "creationSource")]
    pub creation_source: OrchestrationV2CreationSource,
    #[serde(
        rename = "scheduledTaskId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub scheduled_task_id: Optional<ScheduledTaskId>,
    #[serde(
        rename = "senderThreadId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub sender_thread_id: Optional<ThreadId>,
    #[serde(rename = "id")]
    pub id: MessageId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "nodeId", deserialize_with = "required_nullable")]
    pub node_id: Option<NodeId>,
    #[serde(rename = "role")]
    pub role: OrchestrationV2ConversationMessageJsonRole,
    #[serde(rename = "text")]
    pub text: IsoDateTime,
    #[serde(
        rename = "context",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub context: Optional<OrchestrationMessageContext>,
    #[serde(rename = "attachments")]
    pub attachments: Vec<ChatAttachment>,
    #[serde(rename = "streaming")]
    pub streaming: bool,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(
        rename = "delegatedCompletion",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub delegated_completion: Optional<OrchestrationV2ConversationMessageJsonDelegatedCompletion>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2CreationSource {
    #[serde(rename = "web")]
    Web,
    #[serde(rename = "mobile")]
    Mobile,
    #[serde(rename = "mcp")]
    Mcp,
    #[serde(rename = "provider")]
    Provider,
    #[serde(rename = "server")]
    Server,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2DelegatedCompletionCohortDisposition {
    #[serde(rename = "open")]
    Open,
    #[serde(rename = "stopped")]
    Stopped,
    #[serde(rename = "disposed")]
    Disposed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DelegatedCompletionCohort {
    #[serde(rename = "disposition")]
    pub disposition: OrchestrationV2DelegatedCompletionCohortDisposition,
    #[serde(rename = "nextGeneration")]
    pub next_generation: PositiveInt,
    #[serde(rename = "delivery", deserialize_with = "required_nullable")]
    pub delivery: Option<OrchestrationV2DelegatedCompletionDelivery>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DelegatedCompletionDelivery {
    #[serde(rename = "generation")]
    pub generation: PositiveInt,
    #[serde(rename = "messageId")]
    pub message_id: MessageId,
    #[serde(rename = "taskIds")]
    pub task_ids: Vec<NodeId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DelegatedCompletionTaskDelivery {
    #[serde(rename = "state")]
    pub state: OrchestrationV2DelegatedCompletionTaskDeliveryState,
    #[serde(rename = "observedByRunId", deserialize_with = "required_nullable")]
    pub observed_by_run_id: Option<RunId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2DelegatedCompletionTaskDeliveryState {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "claimed")]
    Claimed,
    #[serde(rename = "acknowledged")]
    Acknowledged,
    #[serde(rename = "delivered")]
    Delivered,
    #[serde(rename = "disposed")]
    Disposed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2DispatchCommandErrorTag {
    #[serde(rename = "OrchestrationV2DispatchCommandError")]
    OrchestrationV2DispatchCommandError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DispatchCommandError {
    #[serde(rename = "_tag")]
    pub _tag: OrchestrationV2DispatchCommandErrorTag,
    #[serde(rename = "commandId")]
    pub command_id: CommandId,
    #[serde(rename = "commandType")]
    pub command_type: IsoDateTime,
    #[serde(rename = "message")]
    pub message: IsoDateTime,
    #[serde(
        rename = "detail",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub detail: Optional<IsoDateTime>,
    #[serde(rename = "cause", default, skip_serializing_if = "Optional::is_absent")]
    pub cause: Optional<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DispatchCommandResult {
    #[serde(rename = "sequence")]
    pub sequence: NonNegativeInt,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventThreadCreated {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2AppThread,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventThreadArchived {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2AppThread,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventThreadUnarchived {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2AppThread,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventThreadDeleted {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2AppThread,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventThreadSettled {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2AppThread,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventThreadUnsettled {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2AppThread,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventThreadSnoozed {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2AppThread,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventThreadUnsnoozed {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2AppThread,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventThreadPinned {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2AppThread,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventThreadAutoSettleSet {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2AppThread,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventThreadUnpinned {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2AppThread,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventThreadPinReordered {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2AppThread,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventThreadActiveReordered {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2AppThread,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventThreadVisited {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2AppThread,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventThreadMarkedUnread {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2AppThread,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventThreadMetadataUpdated {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2AppThread,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventThreadPullRequestSynced {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2AppThread,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventThreadRuntimeModeUpdated {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2AppThread,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventThreadInteractionModeUpdated {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2AppThread,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventThreadModelSelectionUpdated {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2AppThread,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventThreadProviderSwitched {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2AppThread,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventRunCreated {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2Run,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventRunUpdated {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2Run,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventRunBackgroundWorkCancelled {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2RunBackgroundWorkCancelled,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventRunAttemptCreated {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2RunAttempt,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventRunAttemptUpdated {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2RunAttempt,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventNodeUpdated {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2ExecutionNode,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventSubagentUpdated {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2Subagent,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventProviderSessionAttached {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2ProviderSession,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventProviderSessionUpdated {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2ProviderSession,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventProviderSessionDetached {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2ProviderSessionDetached,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventProviderThreadUpdated {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2ProviderThread,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventProviderTurnUpdated {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2ProviderTurn,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventRuntimeRequestUpdated {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2RuntimeRequest,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventMessageUpdated {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2ConversationMessage,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventTurnItemUpdated {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2TurnItem,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventPlanUpdated {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2PlanArtifact,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventCheckpointScopeCreated {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2CheckpointScope,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventCheckpointCaptured {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2Checkpoint,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventCheckpointRollbackRequested {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2CheckpointRollbackRequest,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventContextHandoffUpdated {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2ContextHandoff,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventContextTransferCreated {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2ContextTransfer,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventContextTransferUpdated {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2ContextTransfer,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum OrchestrationV2DomainEvent {
    #[serde(rename = "thread.created")]
    ThreadCreated(Box<OrchestrationV2DomainEventThreadCreated>),
    #[serde(rename = "thread.archived")]
    ThreadArchived(Box<OrchestrationV2DomainEventThreadArchived>),
    #[serde(rename = "thread.unarchived")]
    ThreadUnarchived(Box<OrchestrationV2DomainEventThreadUnarchived>),
    #[serde(rename = "thread.deleted")]
    ThreadDeleted(Box<OrchestrationV2DomainEventThreadDeleted>),
    #[serde(rename = "thread.settled")]
    ThreadSettled(Box<OrchestrationV2DomainEventThreadSettled>),
    #[serde(rename = "thread.unsettled")]
    ThreadUnsettled(Box<OrchestrationV2DomainEventThreadUnsettled>),
    #[serde(rename = "thread.snoozed")]
    ThreadSnoozed(Box<OrchestrationV2DomainEventThreadSnoozed>),
    #[serde(rename = "thread.unsnoozed")]
    ThreadUnsnoozed(Box<OrchestrationV2DomainEventThreadUnsnoozed>),
    #[serde(rename = "thread.pinned")]
    ThreadPinned(Box<OrchestrationV2DomainEventThreadPinned>),
    #[serde(rename = "thread.auto-settle-set")]
    ThreadAutoSettleSet(Box<OrchestrationV2DomainEventThreadAutoSettleSet>),
    #[serde(rename = "thread.unpinned")]
    ThreadUnpinned(Box<OrchestrationV2DomainEventThreadUnpinned>),
    #[serde(rename = "thread.pin-reordered")]
    ThreadPinReordered(Box<OrchestrationV2DomainEventThreadPinReordered>),
    #[serde(rename = "thread.active-reordered")]
    ThreadActiveReordered(Box<OrchestrationV2DomainEventThreadActiveReordered>),
    #[serde(rename = "thread.visited")]
    ThreadVisited(Box<OrchestrationV2DomainEventThreadVisited>),
    #[serde(rename = "thread.marked-unread")]
    ThreadMarkedUnread(Box<OrchestrationV2DomainEventThreadMarkedUnread>),
    #[serde(rename = "thread.metadata-updated")]
    ThreadMetadataUpdated(Box<OrchestrationV2DomainEventThreadMetadataUpdated>),
    #[serde(rename = "thread.pull-request-synced")]
    ThreadPullRequestSynced(Box<OrchestrationV2DomainEventThreadPullRequestSynced>),
    #[serde(rename = "thread.runtime-mode-updated")]
    ThreadRuntimeModeUpdated(Box<OrchestrationV2DomainEventThreadRuntimeModeUpdated>),
    #[serde(rename = "thread.interaction-mode-updated")]
    ThreadInteractionModeUpdated(Box<OrchestrationV2DomainEventThreadInteractionModeUpdated>),
    #[serde(rename = "thread.model-selection-updated")]
    ThreadModelSelectionUpdated(Box<OrchestrationV2DomainEventThreadModelSelectionUpdated>),
    #[serde(rename = "thread.provider-switched")]
    ThreadProviderSwitched(Box<OrchestrationV2DomainEventThreadProviderSwitched>),
    #[serde(rename = "run.created")]
    RunCreated(Box<OrchestrationV2DomainEventRunCreated>),
    #[serde(rename = "run.updated")]
    RunUpdated(Box<OrchestrationV2DomainEventRunUpdated>),
    #[serde(rename = "run.background-work-cancelled")]
    RunBackgroundWorkCancelled(Box<OrchestrationV2DomainEventRunBackgroundWorkCancelled>),
    #[serde(rename = "run-attempt.created")]
    RunAttemptCreated(Box<OrchestrationV2DomainEventRunAttemptCreated>),
    #[serde(rename = "run-attempt.updated")]
    RunAttemptUpdated(Box<OrchestrationV2DomainEventRunAttemptUpdated>),
    #[serde(rename = "node.updated")]
    NodeUpdated(Box<OrchestrationV2DomainEventNodeUpdated>),
    #[serde(rename = "subagent.updated")]
    SubagentUpdated(Box<OrchestrationV2DomainEventSubagentUpdated>),
    #[serde(rename = "provider-session.attached")]
    ProviderSessionAttached(Box<OrchestrationV2DomainEventProviderSessionAttached>),
    #[serde(rename = "provider-session.updated")]
    ProviderSessionUpdated(Box<OrchestrationV2DomainEventProviderSessionUpdated>),
    #[serde(rename = "provider-session.detached")]
    ProviderSessionDetached(Box<OrchestrationV2DomainEventProviderSessionDetached>),
    #[serde(rename = "provider-thread.updated")]
    ProviderThreadUpdated(Box<OrchestrationV2DomainEventProviderThreadUpdated>),
    #[serde(rename = "provider-turn.updated")]
    ProviderTurnUpdated(Box<OrchestrationV2DomainEventProviderTurnUpdated>),
    #[serde(rename = "runtime-request.updated")]
    RuntimeRequestUpdated(Box<OrchestrationV2DomainEventRuntimeRequestUpdated>),
    #[serde(rename = "message.updated")]
    MessageUpdated(Box<OrchestrationV2DomainEventMessageUpdated>),
    #[serde(rename = "turn-item.updated")]
    TurnItemUpdated(Box<OrchestrationV2DomainEventTurnItemUpdated>),
    #[serde(rename = "plan.updated")]
    PlanUpdated(Box<OrchestrationV2DomainEventPlanUpdated>),
    #[serde(rename = "checkpoint-scope.created")]
    CheckpointScopeCreated(Box<OrchestrationV2DomainEventCheckpointScopeCreated>),
    #[serde(rename = "checkpoint.captured")]
    CheckpointCaptured(Box<OrchestrationV2DomainEventCheckpointCaptured>),
    #[serde(rename = "checkpoint.rollback-requested")]
    CheckpointRollbackRequested(Box<OrchestrationV2DomainEventCheckpointRollbackRequested>),
    #[serde(rename = "context-handoff.updated")]
    ContextHandoffUpdated(Box<OrchestrationV2DomainEventContextHandoffUpdated>),
    #[serde(rename = "context-transfer.created")]
    ContextTransferCreated(Box<OrchestrationV2DomainEventContextTransferCreated>),
    #[serde(rename = "context-transfer.updated")]
    ContextTransferUpdated(Box<OrchestrationV2DomainEventContextTransferUpdated>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventJsonThreadCreated {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2AppThreadJson,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventJsonThreadArchived {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2AppThreadJson,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventJsonThreadUnarchived {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2AppThreadJson,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventJsonThreadDeleted {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2AppThreadJson,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventJsonThreadSettled {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2AppThreadJson,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventJsonThreadUnsettled {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2AppThreadJson,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventJsonThreadSnoozed {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2AppThreadJson,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventJsonThreadUnsnoozed {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2AppThreadJson,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventJsonThreadPinned {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2AppThreadJson,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventJsonThreadAutoSettleSet {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2AppThreadJson,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventJsonThreadUnpinned {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2AppThreadJson,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventJsonThreadPinReordered {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2AppThreadJson,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventJsonThreadActiveReordered {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2AppThreadJson,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventJsonThreadVisited {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2AppThreadJson,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventJsonThreadMarkedUnread {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2AppThreadJson,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventJsonThreadPullRequestSynced {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2AppThreadJson,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventJsonThreadMetadataUpdated {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2AppThreadJson,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventJsonThreadRuntimeModeUpdated {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2AppThreadJson,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventJsonThreadInteractionModeUpdated {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2AppThreadJson,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventJsonThreadModelSelectionUpdated {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2AppThreadJson,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventJsonThreadProviderSwitched {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2AppThreadJson,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventJsonRunCreated {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2RunJson,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventJsonRunUpdated {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2RunJson,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventJsonRunBackgroundWorkCancelled {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2RunBackgroundWorkCancelled,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventJsonRunAttemptCreated {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2RunAttemptJson,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventJsonRunAttemptUpdated {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2RunAttemptJson,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventJsonNodeUpdated {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2ExecutionNodeJson,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventJsonSubagentUpdated {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2SubagentJson,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventJsonProviderSessionAttached {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2ProviderSessionJson,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventJsonProviderSessionUpdated {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2ProviderSessionJson,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventJsonProviderSessionDetached {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2ProviderSessionDetachedJson,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventJsonProviderThreadUpdated {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2ProviderThreadJson,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventJsonProviderTurnUpdated {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2ProviderTurnJson,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventJsonRuntimeRequestUpdated {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2RuntimeRequestJson,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventJsonMessageUpdated {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2ConversationMessageJson,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventJsonTurnItemUpdated {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2TurnItemJson,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventJsonPlanUpdated {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2PlanArtifact,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventJsonCheckpointScopeCreated {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2CheckpointScopeJson,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventJsonCheckpointCaptured {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2CheckpointJson,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventJsonCheckpointRollbackRequested {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2CheckpointRollbackRequestJson,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventJsonContextHandoffUpdated {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2ContextHandoffJson,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventJsonContextTransferCreated {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2ContextTransferJson,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2DomainEventJsonContextTransferUpdated {
    #[serde(rename = "id")]
    pub id: EventId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "nodeId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub node_id: Optional<NodeId>,
    #[serde(
        rename = "driver",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver: Optional<ProviderDriverKind>,
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "rawEventId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub raw_event_id: Optional<RawEventId>,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(rename = "payload")]
    pub payload: OrchestrationV2ContextTransferJson,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum OrchestrationV2DomainEventJson {
    #[serde(rename = "thread.created")]
    ThreadCreated(Box<OrchestrationV2DomainEventJsonThreadCreated>),
    #[serde(rename = "thread.archived")]
    ThreadArchived(Box<OrchestrationV2DomainEventJsonThreadArchived>),
    #[serde(rename = "thread.unarchived")]
    ThreadUnarchived(Box<OrchestrationV2DomainEventJsonThreadUnarchived>),
    #[serde(rename = "thread.deleted")]
    ThreadDeleted(Box<OrchestrationV2DomainEventJsonThreadDeleted>),
    #[serde(rename = "thread.settled")]
    ThreadSettled(Box<OrchestrationV2DomainEventJsonThreadSettled>),
    #[serde(rename = "thread.unsettled")]
    ThreadUnsettled(Box<OrchestrationV2DomainEventJsonThreadUnsettled>),
    #[serde(rename = "thread.snoozed")]
    ThreadSnoozed(Box<OrchestrationV2DomainEventJsonThreadSnoozed>),
    #[serde(rename = "thread.unsnoozed")]
    ThreadUnsnoozed(Box<OrchestrationV2DomainEventJsonThreadUnsnoozed>),
    #[serde(rename = "thread.pinned")]
    ThreadPinned(Box<OrchestrationV2DomainEventJsonThreadPinned>),
    #[serde(rename = "thread.auto-settle-set")]
    ThreadAutoSettleSet(Box<OrchestrationV2DomainEventJsonThreadAutoSettleSet>),
    #[serde(rename = "thread.unpinned")]
    ThreadUnpinned(Box<OrchestrationV2DomainEventJsonThreadUnpinned>),
    #[serde(rename = "thread.pin-reordered")]
    ThreadPinReordered(Box<OrchestrationV2DomainEventJsonThreadPinReordered>),
    #[serde(rename = "thread.active-reordered")]
    ThreadActiveReordered(Box<OrchestrationV2DomainEventJsonThreadActiveReordered>),
    #[serde(rename = "thread.visited")]
    ThreadVisited(Box<OrchestrationV2DomainEventJsonThreadVisited>),
    #[serde(rename = "thread.marked-unread")]
    ThreadMarkedUnread(Box<OrchestrationV2DomainEventJsonThreadMarkedUnread>),
    #[serde(rename = "thread.pull-request-synced")]
    ThreadPullRequestSynced(Box<OrchestrationV2DomainEventJsonThreadPullRequestSynced>),
    #[serde(rename = "thread.metadata-updated")]
    ThreadMetadataUpdated(Box<OrchestrationV2DomainEventJsonThreadMetadataUpdated>),
    #[serde(rename = "thread.runtime-mode-updated")]
    ThreadRuntimeModeUpdated(Box<OrchestrationV2DomainEventJsonThreadRuntimeModeUpdated>),
    #[serde(rename = "thread.interaction-mode-updated")]
    ThreadInteractionModeUpdated(Box<OrchestrationV2DomainEventJsonThreadInteractionModeUpdated>),
    #[serde(rename = "thread.model-selection-updated")]
    ThreadModelSelectionUpdated(Box<OrchestrationV2DomainEventJsonThreadModelSelectionUpdated>),
    #[serde(rename = "thread.provider-switched")]
    ThreadProviderSwitched(Box<OrchestrationV2DomainEventJsonThreadProviderSwitched>),
    #[serde(rename = "run.created")]
    RunCreated(Box<OrchestrationV2DomainEventJsonRunCreated>),
    #[serde(rename = "run.updated")]
    RunUpdated(Box<OrchestrationV2DomainEventJsonRunUpdated>),
    #[serde(rename = "run.background-work-cancelled")]
    RunBackgroundWorkCancelled(Box<OrchestrationV2DomainEventJsonRunBackgroundWorkCancelled>),
    #[serde(rename = "run-attempt.created")]
    RunAttemptCreated(Box<OrchestrationV2DomainEventJsonRunAttemptCreated>),
    #[serde(rename = "run-attempt.updated")]
    RunAttemptUpdated(Box<OrchestrationV2DomainEventJsonRunAttemptUpdated>),
    #[serde(rename = "node.updated")]
    NodeUpdated(Box<OrchestrationV2DomainEventJsonNodeUpdated>),
    #[serde(rename = "subagent.updated")]
    SubagentUpdated(Box<OrchestrationV2DomainEventJsonSubagentUpdated>),
    #[serde(rename = "provider-session.attached")]
    ProviderSessionAttached(Box<OrchestrationV2DomainEventJsonProviderSessionAttached>),
    #[serde(rename = "provider-session.updated")]
    ProviderSessionUpdated(Box<OrchestrationV2DomainEventJsonProviderSessionUpdated>),
    #[serde(rename = "provider-session.detached")]
    ProviderSessionDetached(Box<OrchestrationV2DomainEventJsonProviderSessionDetached>),
    #[serde(rename = "provider-thread.updated")]
    ProviderThreadUpdated(Box<OrchestrationV2DomainEventJsonProviderThreadUpdated>),
    #[serde(rename = "provider-turn.updated")]
    ProviderTurnUpdated(Box<OrchestrationV2DomainEventJsonProviderTurnUpdated>),
    #[serde(rename = "runtime-request.updated")]
    RuntimeRequestUpdated(Box<OrchestrationV2DomainEventJsonRuntimeRequestUpdated>),
    #[serde(rename = "message.updated")]
    MessageUpdated(Box<OrchestrationV2DomainEventJsonMessageUpdated>),
    #[serde(rename = "turn-item.updated")]
    TurnItemUpdated(Box<OrchestrationV2DomainEventJsonTurnItemUpdated>),
    #[serde(rename = "plan.updated")]
    PlanUpdated(Box<OrchestrationV2DomainEventJsonPlanUpdated>),
    #[serde(rename = "checkpoint-scope.created")]
    CheckpointScopeCreated(Box<OrchestrationV2DomainEventJsonCheckpointScopeCreated>),
    #[serde(rename = "checkpoint.captured")]
    CheckpointCaptured(Box<OrchestrationV2DomainEventJsonCheckpointCaptured>),
    #[serde(rename = "checkpoint.rollback-requested")]
    CheckpointRollbackRequested(Box<OrchestrationV2DomainEventJsonCheckpointRollbackRequested>),
    #[serde(rename = "context-handoff.updated")]
    ContextHandoffUpdated(Box<OrchestrationV2DomainEventJsonContextHandoffUpdated>),
    #[serde(rename = "context-transfer.created")]
    ContextTransferCreated(Box<OrchestrationV2DomainEventJsonContextTransferCreated>),
    #[serde(rename = "context-transfer.updated")]
    ContextTransferUpdated(Box<OrchestrationV2DomainEventJsonContextTransferUpdated>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2ExecutionNodeKind {
    #[serde(rename = "root_turn")]
    RootTurn,
    #[serde(rename = "assistant_message")]
    AssistantMessage,
    #[serde(rename = "reasoning")]
    Reasoning,
    #[serde(rename = "plan")]
    Plan,
    #[serde(rename = "todo_list")]
    TodoList,
    #[serde(rename = "tool_call")]
    ToolCall,
    #[serde(rename = "approval_request")]
    ApprovalRequest,
    #[serde(rename = "user_input_request")]
    UserInputRequest,
    #[serde(rename = "subagent")]
    Subagent,
    #[serde(rename = "hook")]
    Hook,
    #[serde(rename = "system")]
    System,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2ExecutionNodeStatus {
    #[serde(rename = "idle")]
    Idle,
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "waiting")]
    Waiting,
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "interrupted")]
    Interrupted,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "cancelled")]
    Cancelled,
    #[serde(rename = "rolled_back")]
    RolledBack,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ExecutionNode {
    #[serde(rename = "id")]
    pub id: NodeId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "parentNodeId", deserialize_with = "required_nullable")]
    pub parent_node_id: Option<NodeId>,
    #[serde(rename = "rootNodeId")]
    pub root_node_id: NodeId,
    #[serde(rename = "kind")]
    pub kind: OrchestrationV2ExecutionNodeKind,
    #[serde(rename = "status")]
    pub status: OrchestrationV2ExecutionNodeStatus,
    #[serde(rename = "countsForRun")]
    pub counts_for_run: bool,
    #[serde(rename = "providerThreadId", deserialize_with = "required_nullable")]
    pub provider_thread_id: Option<ProviderThreadId>,
    #[serde(rename = "providerTurnId", deserialize_with = "required_nullable")]
    pub provider_turn_id: Option<ProviderTurnId>,
    #[serde(rename = "nativeItemRef", deserialize_with = "required_nullable")]
    pub native_item_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(rename = "runtimeRequestId", deserialize_with = "required_nullable")]
    pub runtime_request_id: Option<RuntimeRequestId>,
    #[serde(rename = "checkpointScopeId", deserialize_with = "required_nullable")]
    pub checkpoint_scope_id: Option<CheckpointScopeId>,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<String>,
    #[serde(rename = "completedAt", deserialize_with = "required_nullable")]
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2ExecutionNodeJsonKind {
    #[serde(rename = "root_turn")]
    RootTurn,
    #[serde(rename = "assistant_message")]
    AssistantMessage,
    #[serde(rename = "reasoning")]
    Reasoning,
    #[serde(rename = "plan")]
    Plan,
    #[serde(rename = "todo_list")]
    TodoList,
    #[serde(rename = "tool_call")]
    ToolCall,
    #[serde(rename = "approval_request")]
    ApprovalRequest,
    #[serde(rename = "user_input_request")]
    UserInputRequest,
    #[serde(rename = "subagent")]
    Subagent,
    #[serde(rename = "hook")]
    Hook,
    #[serde(rename = "system")]
    System,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2ExecutionNodeJsonStatus {
    #[serde(rename = "idle")]
    Idle,
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "waiting")]
    Waiting,
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "interrupted")]
    Interrupted,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "cancelled")]
    Cancelled,
    #[serde(rename = "rolled_back")]
    RolledBack,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ExecutionNodeJson {
    #[serde(rename = "id")]
    pub id: NodeId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "parentNodeId", deserialize_with = "required_nullable")]
    pub parent_node_id: Option<NodeId>,
    #[serde(rename = "rootNodeId")]
    pub root_node_id: NodeId,
    #[serde(rename = "kind")]
    pub kind: OrchestrationV2ExecutionNodeJsonKind,
    #[serde(rename = "status")]
    pub status: OrchestrationV2ExecutionNodeJsonStatus,
    #[serde(rename = "countsForRun")]
    pub counts_for_run: bool,
    #[serde(rename = "providerThreadId", deserialize_with = "required_nullable")]
    pub provider_thread_id: Option<ProviderThreadId>,
    #[serde(rename = "providerTurnId", deserialize_with = "required_nullable")]
    pub provider_turn_id: Option<ProviderTurnId>,
    #[serde(rename = "nativeItemRef", deserialize_with = "required_nullable")]
    pub native_item_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(rename = "runtimeRequestId", deserialize_with = "required_nullable")]
    pub runtime_request_id: Option<RuntimeRequestId>,
    #[serde(rename = "checkpointScopeId", deserialize_with = "required_nullable")]
    pub checkpoint_scope_id: Option<CheckpointScopeId>,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<String>,
    #[serde(rename = "completedAt", deserialize_with = "required_nullable")]
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2FileChangeDetail {
    #[serde(rename = "operation")]
    pub operation: TrimmedNonEmptyString,
    #[serde(rename = "path")]
    pub path: TrimmedNonEmptyString,
    #[serde(
        rename = "oldPath",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub old_path: Optional<TrimmedNonEmptyString>,
    #[serde(
        rename = "fileType",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub file_type: Optional<TrimmedNonEmptyString>,
    #[serde(
        rename = "mimeType",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub mime_type: Optional<TrimmedNonEmptyString>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2FileSearchResult {
    #[serde(rename = "fileName")]
    pub file_name: TrimmedNonEmptyString,
    #[serde(rename = "line", default, skip_serializing_if = "Optional::is_absent")]
    pub line: Optional<PositiveInt>,
    #[serde(
        rename = "column",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub column: Optional<PositiveInt>,
    #[serde(
        rename = "preview",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub preview: Optional<IsoDateTime>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2GetShellSnapshotErrorTag {
    #[serde(rename = "OrchestrationV2GetShellSnapshotError")]
    OrchestrationV2GetShellSnapshotError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2GetShellSnapshotError {
    #[serde(rename = "_tag")]
    pub _tag: OrchestrationV2GetShellSnapshotErrorTag,
    #[serde(rename = "message")]
    pub message: IsoDateTime,
    #[serde(rename = "cause", default, skip_serializing_if = "Optional::is_absent")]
    pub cause: Optional<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2GetThreadProjectionErrorTag {
    #[serde(rename = "OrchestrationV2GetThreadProjectionError")]
    OrchestrationV2GetThreadProjectionError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2GetThreadProjectionError {
    #[serde(rename = "_tag")]
    pub _tag: OrchestrationV2GetThreadProjectionErrorTag,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "message")]
    pub message: IsoDateTime,
    #[serde(rename = "cause", default, skip_serializing_if = "Optional::is_absent")]
    pub cause: Optional<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2GetThreadProjectionInput {
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2GetWorkflowScriptInput {
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "scriptPath")]
    pub script_path: TrimmedNonEmptyString,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2GetWorkflowScriptResult {
    #[serde(rename = "scriptPath")]
    pub script_path: TrimmedNonEmptyString,
    #[serde(rename = "contents")]
    pub contents: IsoDateTime,
    #[serde(rename = "truncated")]
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2HistoricalMessageRole {
    #[serde(rename = "user")]
    User,
    #[serde(rename = "assistant")]
    Assistant,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2HistoricalMessage {
    #[serde(rename = "role")]
    pub role: OrchestrationV2HistoricalMessageRole,
    #[serde(rename = "text")]
    pub text: IsoDateTime,
    #[serde(
        rename = "runStatus",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub run_status: Optional<IsoDateTime>,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "itemId")]
    pub item_id: TurnItemId,
    #[serde(rename = "providerThreadId", deserialize_with = "required_nullable")]
    pub provider_thread_id: Option<ProviderThreadId>,
    #[serde(rename = "status")]
    pub status: IsoDateTime,
    #[serde(rename = "kind")]
    pub kind: IsoDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2IdentityCapabilities {
    #[serde(rename = "nativeThreadIds")]
    pub native_thread_ids: OrchestrationV2NativeRefStrength,
    #[serde(rename = "nativeTurnIds")]
    pub native_turn_ids: OrchestrationV2NativeRefStrength,
    #[serde(rename = "nativeItemIds")]
    pub native_item_ids: OrchestrationV2NativeRefStrength,
    #[serde(rename = "nativeRequestIds")]
    pub native_request_ids: OrchestrationV2NativeRefStrength,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2LatestVisibleMessageSummaryRole {
    #[serde(rename = "user")]
    User,
    #[serde(rename = "assistant")]
    Assistant,
    #[serde(rename = "system")]
    System,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2LatestVisibleMessageSummary {
    #[serde(rename = "id")]
    pub id: MessageId,
    #[serde(rename = "role")]
    pub role: OrchestrationV2LatestVisibleMessageSummaryRole,
    #[serde(rename = "text")]
    pub text: IsoDateTime,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2LatestVisibleMessageSummaryJsonRole {
    #[serde(rename = "user")]
    User,
    #[serde(rename = "assistant")]
    Assistant,
    #[serde(rename = "system")]
    System,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2LatestVisibleMessageSummaryJson {
    #[serde(rename = "id")]
    pub id: MessageId,
    #[serde(rename = "role")]
    pub role: OrchestrationV2LatestVisibleMessageSummaryJsonRole,
    #[serde(rename = "text")]
    pub text: IsoDateTime,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2LimitRecovery {
    #[serde(
        rename = "requestId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub request_id: Optional<CommandId>,
    #[serde(rename = "runId")]
    pub run_id: RunId,
    #[serde(rename = "resetAt")]
    pub reset_at: IsoDateTime,
    #[serde(rename = "autoResume")]
    pub auto_resume: bool,
    #[serde(
        rename = "snooze",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub snooze: Optional<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2LimitRecoveryUpdate {
    #[serde(rename = "runId")]
    pub run_id: RunId,
    #[serde(rename = "resetAt")]
    pub reset_at: IsoDateTime,
    #[serde(
        rename = "autoResume",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub auto_resume: Optional<bool>,
    #[serde(
        rename = "snooze",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub snooze: Optional<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2NativeRefStrength {
    #[serde(rename = "strong")]
    Strong,
    #[serde(rename = "weak")]
    Weak,
    #[serde(rename = "none")]
    None,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2NotificationOutcome {
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "cancelled")]
    Cancelled,
    #[serde(rename = "updated")]
    Updated,
    #[serde(rename = "unknown")]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2Notification {
    #[serde(rename = "source")]
    pub source: OrchestrationV2NotificationSource,
    #[serde(rename = "outcome")]
    pub outcome: OrchestrationV2NotificationOutcome,
    #[serde(rename = "summary")]
    pub summary: TrimmedNonEmptyString,
    #[serde(
        rename = "detail",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub detail: Optional<IsoDateTime>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2NotificationSourceDelegatedTask {
    #[serde(rename = "taskIds")]
    pub task_ids: Vec<NodeId>,
    #[serde(
        rename = "childThreadId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub child_thread_id: Optional<ThreadId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2NotificationSourceSubagent {
    #[serde(
        rename = "childThreadId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub child_thread_id: Optional<ThreadId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2NotificationSourceCommand {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2NotificationSourceMonitor {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2NotificationSourceBackgroundTask {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
enum OrchestrationV2NotificationSourceCanonical {
    #[serde(rename = "delegated_task")]
    DelegatedTask(Box<OrchestrationV2NotificationSourceDelegatedTask>),
    #[serde(rename = "subagent")]
    Subagent(Box<OrchestrationV2NotificationSourceSubagent>),
    #[serde(rename = "command")]
    Command(Box<OrchestrationV2NotificationSourceCommand>),
    #[serde(rename = "monitor")]
    Monitor(Box<OrchestrationV2NotificationSourceMonitor>),
    #[serde(rename = "background_task")]
    BackgroundTask(Box<OrchestrationV2NotificationSourceBackgroundTask>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "serde_json::Value", into = "serde_json::Value")]
#[serde(tag = "kind")]
pub enum OrchestrationV2NotificationSource {
    #[serde(rename = "delegated_task")]
    DelegatedTask(Box<OrchestrationV2NotificationSourceDelegatedTask>),
    #[serde(rename = "subagent")]
    Subagent(Box<OrchestrationV2NotificationSourceSubagent>),
    #[serde(rename = "command")]
    Command(Box<OrchestrationV2NotificationSourceCommand>),
    #[serde(rename = "monitor")]
    Monitor(Box<OrchestrationV2NotificationSourceMonitor>),
    #[serde(rename = "background_task")]
    BackgroundTask(Box<OrchestrationV2NotificationSourceBackgroundTask>),
}
impl TryFrom<serde_json::Value> for OrchestrationV2NotificationSource {
    type Error = String;
    fn try_from(value: serde_json::Value) -> Result<Self, String> {
        let value: OrchestrationV2NotificationSourceCanonical = serde_json::from_value(
            normalize_contract("OrchestrationV2NotificationSource", value)?,
        )
        .map_err(|e| e.to_string())?;
        Ok(match value {
            OrchestrationV2NotificationSourceCanonical::DelegatedTask(value) => {
                Self::DelegatedTask(value)
            }
            OrchestrationV2NotificationSourceCanonical::Subagent(value) => Self::Subagent(value),
            OrchestrationV2NotificationSourceCanonical::Command(value) => Self::Command(value),
            OrchestrationV2NotificationSourceCanonical::Monitor(value) => Self::Monitor(value),
            OrchestrationV2NotificationSourceCanonical::BackgroundTask(value) => {
                Self::BackgroundTask(value)
            }
        })
    }
}
impl From<OrchestrationV2NotificationSource> for serde_json::Value {
    fn from(value: OrchestrationV2NotificationSource) -> Self {
        let canonical = match value {
            OrchestrationV2NotificationSource::DelegatedTask(value) => {
                OrchestrationV2NotificationSourceCanonical::DelegatedTask(value)
            }
            OrchestrationV2NotificationSource::Subagent(value) => {
                OrchestrationV2NotificationSourceCanonical::Subagent(value)
            }
            OrchestrationV2NotificationSource::Command(value) => {
                OrchestrationV2NotificationSourceCanonical::Command(value)
            }
            OrchestrationV2NotificationSource::Monitor(value) => {
                OrchestrationV2NotificationSourceCanonical::Monitor(value)
            }
            OrchestrationV2NotificationSource::BackgroundTask(value) => {
                OrchestrationV2NotificationSourceCanonical::BackgroundTask(value)
            }
        };
        encode_notification_source(serde_json::to_value(canonical).expect("notification encoding"))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2PendingBackgroundTaskSubagent {
    #[serde(rename = "taskId")]
    pub task_id: TrimmedNonEmptyString,
    #[serde(
        rename = "description",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub description: Optional<TrimmedNonEmptyString>,
    #[serde(
        rename = "childThreadId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub child_thread_id: Optional<ThreadId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2PendingBackgroundTaskCommand {
    #[serde(rename = "taskId")]
    pub task_id: TrimmedNonEmptyString,
    #[serde(
        rename = "description",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub description: Optional<TrimmedNonEmptyString>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2PendingBackgroundTaskMonitor {
    #[serde(rename = "taskId")]
    pub task_id: TrimmedNonEmptyString,
    #[serde(
        rename = "description",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub description: Optional<TrimmedNonEmptyString>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2PendingBackgroundTaskBackgroundTask {
    #[serde(rename = "taskId")]
    pub task_id: TrimmedNonEmptyString,
    #[serde(
        rename = "description",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub description: Optional<TrimmedNonEmptyString>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
enum OrchestrationV2PendingBackgroundTaskCanonical {
    #[serde(rename = "subagent")]
    Subagent(Box<OrchestrationV2PendingBackgroundTaskSubagent>),
    #[serde(rename = "command")]
    Command(Box<OrchestrationV2PendingBackgroundTaskCommand>),
    #[serde(rename = "monitor")]
    Monitor(Box<OrchestrationV2PendingBackgroundTaskMonitor>),
    #[serde(rename = "background_task")]
    BackgroundTask(Box<OrchestrationV2PendingBackgroundTaskBackgroundTask>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "serde_json::Value")]
#[serde(tag = "kind")]
pub enum OrchestrationV2PendingBackgroundTask {
    #[serde(rename = "subagent")]
    Subagent(Box<OrchestrationV2PendingBackgroundTaskSubagent>),
    #[serde(rename = "command")]
    Command(Box<OrchestrationV2PendingBackgroundTaskCommand>),
    #[serde(rename = "monitor")]
    Monitor(Box<OrchestrationV2PendingBackgroundTaskMonitor>),
    #[serde(rename = "background_task")]
    BackgroundTask(Box<OrchestrationV2PendingBackgroundTaskBackgroundTask>),
}
impl TryFrom<serde_json::Value> for OrchestrationV2PendingBackgroundTask {
    type Error = String;
    fn try_from(value: serde_json::Value) -> Result<Self, String> {
        let value: OrchestrationV2PendingBackgroundTaskCanonical = serde_json::from_value(
            normalize_contract("OrchestrationV2PendingBackgroundTask", value)?,
        )
        .map_err(|e| e.to_string())?;
        Ok(match value {
            OrchestrationV2PendingBackgroundTaskCanonical::Subagent(value) => Self::Subagent(value),
            OrchestrationV2PendingBackgroundTaskCanonical::Command(value) => Self::Command(value),
            OrchestrationV2PendingBackgroundTaskCanonical::Monitor(value) => Self::Monitor(value),
            OrchestrationV2PendingBackgroundTaskCanonical::BackgroundTask(value) => {
                Self::BackgroundTask(value)
            }
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2PendingRuntimeRequestSummaryKind {
    #[serde(rename = "command")]
    Command,
    #[serde(rename = "file-read")]
    FileRead,
    #[serde(rename = "file-change")]
    FileChange,
    #[serde(rename = "mcp-elicitation")]
    McpElicitation,
    #[serde(rename = "permission")]
    Permission,
    #[serde(rename = "dynamic_tool_call")]
    DynamicToolCall,
    #[serde(rename = "user_input")]
    UserInput,
    #[serde(rename = "auth_refresh")]
    AuthRefresh,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2PendingRuntimeRequestSummary {
    #[serde(rename = "id")]
    pub id: RuntimeRequestId,
    #[serde(rename = "kind")]
    pub kind: OrchestrationV2PendingRuntimeRequestSummaryKind,
    #[serde(rename = "createdAt")]
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2PendingRuntimeRequestSummaryJsonKind {
    #[serde(rename = "command")]
    Command,
    #[serde(rename = "file-read")]
    FileRead,
    #[serde(rename = "file-change")]
    FileChange,
    #[serde(rename = "mcp-elicitation")]
    McpElicitation,
    #[serde(rename = "permission")]
    Permission,
    #[serde(rename = "dynamic_tool_call")]
    DynamicToolCall,
    #[serde(rename = "user_input")]
    UserInput,
    #[serde(rename = "auth_refresh")]
    AuthRefresh,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2PendingRuntimeRequestSummaryJson {
    #[serde(rename = "id")]
    pub id: RuntimeRequestId,
    #[serde(rename = "kind")]
    pub kind: OrchestrationV2PendingRuntimeRequestSummaryJsonKind,
    #[serde(rename = "createdAt")]
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2PlanArtifactProposedPlanStatus {
    #[serde(rename = "draft")]
    Draft,
    #[serde(rename = "active")]
    Active,
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "superseded")]
    Superseded,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OrchestrationV2PlanArtifactProposedPlanDetailInTurnItem;
impl Serialize for OrchestrationV2PlanArtifactProposedPlanDetailInTurnItem {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serde_json::json!(true).serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for OrchestrationV2PlanArtifactProposedPlanDetailInTurnItem {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        if value == serde_json::json!(true) {
            Ok(Self)
        } else {
            Err(serde::de::Error::custom("expected literal true"))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2PlanArtifactProposedPlan {
    #[serde(rename = "id")]
    pub id: PlanId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "nodeId")]
    pub node_id: NodeId,
    #[serde(rename = "status")]
    pub status: OrchestrationV2PlanArtifactProposedPlanStatus,
    #[serde(
        rename = "detailInTurnItem",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub detail_in_turn_item: Optional<OrchestrationV2PlanArtifactProposedPlanDetailInTurnItem>,
    #[serde(rename = "markdown")]
    pub markdown: IsoDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2PlanArtifactTodoListStatus {
    #[serde(rename = "draft")]
    Draft,
    #[serde(rename = "active")]
    Active,
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "superseded")]
    Superseded,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OrchestrationV2PlanArtifactTodoListDetailInTurnItem;
impl Serialize for OrchestrationV2PlanArtifactTodoListDetailInTurnItem {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serde_json::json!(true).serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for OrchestrationV2PlanArtifactTodoListDetailInTurnItem {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        if value == serde_json::json!(true) {
            Ok(Self)
        } else {
            Err(serde::de::Error::custom("expected literal true"))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2PlanArtifactTodoList {
    #[serde(rename = "id")]
    pub id: PlanId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "nodeId")]
    pub node_id: NodeId,
    #[serde(rename = "status")]
    pub status: OrchestrationV2PlanArtifactTodoListStatus,
    #[serde(
        rename = "detailInTurnItem",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub detail_in_turn_item: Optional<OrchestrationV2PlanArtifactTodoListDetailInTurnItem>,
    #[serde(rename = "steps")]
    pub steps: Vec<OrchestrationV2PlanStep>,
    #[serde(
        rename = "explanation",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub explanation: Optional<IsoDateTime>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum OrchestrationV2PlanArtifact {
    #[serde(rename = "proposed_plan")]
    ProposedPlan(Box<OrchestrationV2PlanArtifactProposedPlan>),
    #[serde(rename = "todo_list")]
    TodoList(Box<OrchestrationV2PlanArtifactTodoList>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2PlanStepStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "completed")]
    Completed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2PlanStep {
    #[serde(rename = "id")]
    pub id: TrimmedNonEmptyString,
    #[serde(rename = "text")]
    pub text: TrimmedNonEmptyString,
    #[serde(rename = "status")]
    pub status: OrchestrationV2PlanStepStatus,
    #[serde(
        rename = "durationAnchorAt",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub duration_anchor_at: Optional<IsoDateTime>,
    #[serde(
        rename = "durationMs",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub duration_ms: Optional<NonNegativeInt>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2PlanningCapabilities {
    #[serde(rename = "emitsPlanUpdated")]
    pub emits_plan_updated: bool,
    #[serde(rename = "emitsTodoList")]
    pub emits_todo_list: bool,
    #[serde(rename = "emitsProposedPlan")]
    pub emits_proposed_plan: bool,
    #[serde(rename = "supportsStructuredQuestions")]
    pub supports_structured_questions: bool,
    #[serde(rename = "planDeltasHaveItemIds")]
    pub plan_deltas_have_item_ids: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2ProjectedTurnItemVisibility {
    #[serde(rename = "local")]
    Local,
    #[serde(rename = "inherited")]
    Inherited,
    #[serde(rename = "synthetic")]
    Synthetic,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ProjectedTurnItem {
    #[serde(rename = "position")]
    pub position: NonNegativeInt,
    #[serde(rename = "visibility")]
    pub visibility: OrchestrationV2ProjectedTurnItemVisibility,
    #[serde(rename = "sourceThreadId")]
    pub source_thread_id: ThreadId,
    #[serde(rename = "sourceItemId")]
    pub source_item_id: TurnItemId,
    #[serde(rename = "item")]
    pub item: OrchestrationV2TurnItem,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2ProjectedTurnItemJsonVisibility {
    #[serde(rename = "local")]
    Local,
    #[serde(rename = "inherited")]
    Inherited,
    #[serde(rename = "synthetic")]
    Synthetic,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ProjectedTurnItemJson {
    #[serde(rename = "position")]
    pub position: NonNegativeInt,
    #[serde(rename = "visibility")]
    pub visibility: OrchestrationV2ProjectedTurnItemJsonVisibility,
    #[serde(rename = "sourceThreadId")]
    pub source_thread_id: ThreadId,
    #[serde(rename = "sourceItemId")]
    pub source_item_id: TurnItemId,
    #[serde(rename = "item")]
    pub item: OrchestrationV2TurnItemJson,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2ProviderCapabilitiesRuntimePolicyEnforcement {
    #[serde(rename = "native")]
    Native,
    #[serde(rename = "client-boundary")]
    ClientBoundary,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ProviderCapabilitiesRuntimePolicy {
    #[serde(rename = "enforcement")]
    pub enforcement: OrchestrationV2ProviderCapabilitiesRuntimePolicyEnforcement,
}

fn orchestration_v2_provider_capabilities_runtime_policy_default()
-> OrchestrationV2ProviderCapabilitiesRuntimePolicy {
    serde_json::from_str("{\"enforcement\":\"client-boundary\"}").expect("upstream default")
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ProviderCapabilities {
    #[serde(rename = "sessions")]
    pub sessions: OrchestrationV2SessionCapabilities,
    #[serde(rename = "threads")]
    pub threads: OrchestrationV2ThreadCapabilities,
    #[serde(rename = "turns")]
    pub turns: OrchestrationV2TurnCapabilities,
    #[serde(rename = "streaming")]
    pub streaming: OrchestrationV2StreamingCapabilities,
    #[serde(rename = "tools")]
    pub tools: OrchestrationV2ToolCapabilities,
    #[serde(rename = "approvals")]
    pub approvals: OrchestrationV2ApprovalCapabilities,
    #[serde(rename = "planning")]
    pub planning: OrchestrationV2PlanningCapabilities,
    #[serde(rename = "subagents")]
    pub subagents: OrchestrationV2SubagentCapabilities,
    #[serde(rename = "context")]
    pub context: OrchestrationV2ContextCapabilities,
    #[serde(rename = "checkpointing")]
    pub checkpointing: OrchestrationV2CheckpointCapabilities,
    #[serde(rename = "identity")]
    pub identity: OrchestrationV2IdentityCapabilities,
    #[serde(
        rename = "runtimePolicy",
        default = "orchestration_v2_provider_capabilities_runtime_policy_default"
    )]
    pub runtime_policy: OrchestrationV2ProviderCapabilitiesRuntimePolicy,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ProviderFailure {
    #[serde(rename = "class")]
    pub class: OrchestrationV2ProviderFailureClass,
    #[serde(rename = "message")]
    pub message: String,
    #[serde(rename = "code", deserialize_with = "required_nullable")]
    pub code: Option<String>,
    #[serde(rename = "retryable", deserialize_with = "required_nullable")]
    pub retryable: Option<bool>,
    #[serde(
        rename = "resetAt",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub reset_at: Optional<Option<IsoDateTime>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2ProviderFailureClass {
    #[serde(rename = "usage_limit")]
    UsageLimit,
    #[serde(rename = "provider_error")]
    ProviderError,
    #[serde(rename = "transport_error")]
    TransportError,
    #[serde(rename = "permission_error")]
    PermissionError,
    #[serde(rename = "validation_error")]
    ValidationError,
    #[serde(rename = "unknown")]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ProviderRef {
    #[serde(rename = "driver")]
    pub driver: ProviderDriverKind,
    #[serde(rename = "nativeId", deserialize_with = "required_nullable")]
    pub native_id: Option<TrimmedNonEmptyString>,
    #[serde(rename = "strength")]
    pub strength: OrchestrationV2NativeRefStrength,
    #[serde(
        rename = "fingerprint",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub fingerprint: Optional<TrimmedNonEmptyString>,
    #[serde(
        rename = "ordinal",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub ordinal: Optional<NonNegativeInt>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ProviderRetry {
    #[serde(rename = "attempt")]
    pub attempt: PositiveInt,
    #[serde(rename = "maxAttempts", deserialize_with = "required_nullable")]
    pub max_attempts: Option<PositiveInt>,
    #[serde(rename = "retryDelayMs", deserialize_with = "required_nullable")]
    pub retry_delay_ms: Option<NonNegativeInt>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2ProviderSessionStatus {
    #[serde(rename = "starting")]
    Starting,
    #[serde(rename = "ready")]
    Ready,
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "waiting")]
    Waiting,
    #[serde(rename = "stopped")]
    Stopped,
    #[serde(rename = "error")]
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ProviderSession {
    #[serde(rename = "id")]
    pub id: ProviderSessionId,
    #[serde(rename = "driver")]
    pub driver: ProviderDriverKind,
    #[serde(rename = "providerInstanceId")]
    pub provider_instance_id: ProviderInstanceId,
    #[serde(rename = "status")]
    pub status: OrchestrationV2ProviderSessionStatus,
    #[serde(rename = "cwd")]
    pub cwd: TrimmedNonEmptyString,
    #[serde(rename = "model", deserialize_with = "required_nullable")]
    pub model: Option<TrimmedNonEmptyString>,
    #[serde(rename = "capabilities")]
    pub capabilities: OrchestrationV2ProviderCapabilities,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "lastError", deserialize_with = "required_nullable")]
    pub last_error: Option<IsoDateTime>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ProviderSessionDetached {
    #[serde(rename = "providerSessionId")]
    pub provider_session_id: ProviderSessionId,
    #[serde(rename = "detachedAt")]
    pub detached_at: String,
    #[serde(
        rename = "reason",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub reason: Optional<IsoDateTime>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ProviderSessionDetachedJson {
    #[serde(rename = "providerSessionId")]
    pub provider_session_id: ProviderSessionId,
    #[serde(rename = "detachedAt")]
    pub detached_at: String,
    #[serde(
        rename = "reason",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub reason: Optional<IsoDateTime>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2ProviderSessionJsonStatus {
    #[serde(rename = "starting")]
    Starting,
    #[serde(rename = "ready")]
    Ready,
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "waiting")]
    Waiting,
    #[serde(rename = "stopped")]
    Stopped,
    #[serde(rename = "error")]
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ProviderSessionJson {
    #[serde(rename = "id")]
    pub id: ProviderSessionId,
    #[serde(rename = "driver")]
    pub driver: ProviderDriverKind,
    #[serde(rename = "providerInstanceId")]
    pub provider_instance_id: ProviderInstanceId,
    #[serde(rename = "status")]
    pub status: OrchestrationV2ProviderSessionJsonStatus,
    #[serde(rename = "cwd")]
    pub cwd: TrimmedNonEmptyString,
    #[serde(rename = "model", deserialize_with = "required_nullable")]
    pub model: Option<TrimmedNonEmptyString>,
    #[serde(rename = "capabilities")]
    pub capabilities: OrchestrationV2ProviderCapabilities,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "lastError", deserialize_with = "required_nullable")]
    pub last_error: Option<IsoDateTime>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2ProviderThreadStatus {
    #[serde(rename = "not_loaded")]
    NotLoaded,
    #[serde(rename = "idle")]
    Idle,
    #[serde(rename = "active")]
    Active,
    #[serde(rename = "archived")]
    Archived,
    #[serde(rename = "closed")]
    Closed,
    #[serde(rename = "error")]
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ProviderThreadForkedFrom {
    #[serde(rename = "providerThreadId")]
    pub provider_thread_id: ProviderThreadId,
    #[serde(
        rename = "providerTurnId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_turn_id: Optional<ProviderTurnId>,
    #[serde(
        rename = "checkpointId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub checkpoint_id: Optional<CheckpointId>,
}

fn orchestration_v2_provider_thread_pending_background_tasks_default()
-> Vec<OrchestrationV2PendingBackgroundTask> {
    serde_json::from_str("[]").expect("upstream default")
}

fn orchestration_v2_provider_thread_context_usage_default() -> Option<ThreadTokenUsageSnapshot> {
    serde_json::from_str("null").expect("upstream default")
}

fn orchestration_v2_provider_thread_native_metadata_default()
-> Option<OrchestrationV2ProviderThreadNativeMetadata> {
    serde_json::from_str("null").expect("upstream default")
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ProviderThread {
    #[serde(rename = "id")]
    pub id: ProviderThreadId,
    #[serde(rename = "driver")]
    pub driver: ProviderDriverKind,
    #[serde(rename = "providerInstanceId")]
    pub provider_instance_id: ProviderInstanceId,
    #[serde(rename = "providerSessionId", deserialize_with = "required_nullable")]
    pub provider_session_id: Option<ProviderSessionId>,
    #[serde(rename = "appThreadId", deserialize_with = "required_nullable")]
    pub app_thread_id: Option<ThreadId>,
    #[serde(rename = "ownerNodeId", deserialize_with = "required_nullable")]
    pub owner_node_id: Option<NodeId>,
    #[serde(rename = "nativeThreadRef", deserialize_with = "required_nullable")]
    pub native_thread_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(
        rename = "nativeConversationHeadRef",
        deserialize_with = "required_nullable"
    )]
    pub native_conversation_head_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(rename = "status")]
    pub status: OrchestrationV2ProviderThreadStatus,
    #[serde(rename = "firstRunOrdinal", deserialize_with = "required_nullable")]
    pub first_run_ordinal: Option<PositiveInt>,
    #[serde(rename = "lastRunOrdinal", deserialize_with = "required_nullable")]
    pub last_run_ordinal: Option<PositiveInt>,
    #[serde(rename = "handoffIds")]
    pub handoff_ids: Vec<ContextHandoffId>,
    #[serde(rename = "forkedFrom", deserialize_with = "required_nullable")]
    pub forked_from: Option<OrchestrationV2ProviderThreadForkedFrom>,
    #[serde(
        rename = "pendingBackgroundTasks",
        default = "orchestration_v2_provider_thread_pending_background_tasks_default"
    )]
    pub pending_background_tasks: Vec<OrchestrationV2PendingBackgroundTask>,
    #[serde(
        rename = "contextUsage",
        default = "orchestration_v2_provider_thread_context_usage_default"
    )]
    pub context_usage: Option<ThreadTokenUsageSnapshot>,
    #[serde(
        rename = "nativeMetadata",
        default = "orchestration_v2_provider_thread_native_metadata_default"
    )]
    pub native_metadata: Option<OrchestrationV2ProviderThreadNativeMetadata>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2ProviderThreadDisposition {
    #[serde(rename = "reusable")]
    Reusable,
    #[serde(rename = "broken")]
    Broken,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2ProviderThreadJsonStatus {
    #[serde(rename = "not_loaded")]
    NotLoaded,
    #[serde(rename = "idle")]
    Idle,
    #[serde(rename = "active")]
    Active,
    #[serde(rename = "archived")]
    Archived,
    #[serde(rename = "closed")]
    Closed,
    #[serde(rename = "error")]
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ProviderThreadJsonForkedFrom {
    #[serde(rename = "providerThreadId")]
    pub provider_thread_id: ProviderThreadId,
    #[serde(
        rename = "providerTurnId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_turn_id: Optional<ProviderTurnId>,
    #[serde(
        rename = "checkpointId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub checkpoint_id: Optional<CheckpointId>,
}

fn orchestration_v2_provider_thread_json_pending_background_tasks_default()
-> Vec<OrchestrationV2PendingBackgroundTask> {
    serde_json::from_str("[]").expect("upstream default")
}

fn orchestration_v2_provider_thread_json_context_usage_default() -> Option<ThreadTokenUsageSnapshot>
{
    serde_json::from_str("null").expect("upstream default")
}

fn orchestration_v2_provider_thread_json_native_metadata_default()
-> Option<OrchestrationV2ProviderThreadNativeMetadata> {
    serde_json::from_str("null").expect("upstream default")
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ProviderThreadJson {
    #[serde(rename = "id")]
    pub id: ProviderThreadId,
    #[serde(rename = "driver")]
    pub driver: ProviderDriverKind,
    #[serde(rename = "providerInstanceId")]
    pub provider_instance_id: ProviderInstanceId,
    #[serde(rename = "providerSessionId", deserialize_with = "required_nullable")]
    pub provider_session_id: Option<ProviderSessionId>,
    #[serde(rename = "appThreadId", deserialize_with = "required_nullable")]
    pub app_thread_id: Option<ThreadId>,
    #[serde(rename = "ownerNodeId", deserialize_with = "required_nullable")]
    pub owner_node_id: Option<NodeId>,
    #[serde(rename = "nativeThreadRef", deserialize_with = "required_nullable")]
    pub native_thread_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(
        rename = "nativeConversationHeadRef",
        deserialize_with = "required_nullable"
    )]
    pub native_conversation_head_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(rename = "status")]
    pub status: OrchestrationV2ProviderThreadJsonStatus,
    #[serde(rename = "firstRunOrdinal", deserialize_with = "required_nullable")]
    pub first_run_ordinal: Option<PositiveInt>,
    #[serde(rename = "lastRunOrdinal", deserialize_with = "required_nullable")]
    pub last_run_ordinal: Option<PositiveInt>,
    #[serde(rename = "handoffIds")]
    pub handoff_ids: Vec<ContextHandoffId>,
    #[serde(rename = "forkedFrom", deserialize_with = "required_nullable")]
    pub forked_from: Option<OrchestrationV2ProviderThreadJsonForkedFrom>,
    #[serde(
        rename = "pendingBackgroundTasks",
        default = "orchestration_v2_provider_thread_json_pending_background_tasks_default"
    )]
    pub pending_background_tasks: Vec<OrchestrationV2PendingBackgroundTask>,
    #[serde(
        rename = "contextUsage",
        default = "orchestration_v2_provider_thread_json_context_usage_default"
    )]
    pub context_usage: Option<ThreadTokenUsageSnapshot>,
    #[serde(
        rename = "nativeMetadata",
        default = "orchestration_v2_provider_thread_json_native_metadata_default"
    )]
    pub native_metadata: Option<OrchestrationV2ProviderThreadNativeMetadata>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OrchestrationV2ProviderThreadNativeMetadataItemIdentityVersion;
impl Serialize for OrchestrationV2ProviderThreadNativeMetadataItemIdentityVersion {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serde_json::json!(2).serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for OrchestrationV2ProviderThreadNativeMetadataItemIdentityVersion {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        if value == serde_json::json!(2) {
            Ok(Self)
        } else {
            Err(serde::de::Error::custom("expected literal 2"))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ProviderThreadNativeMetadata {
    #[serde(
        rename = "modelSelection",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub model_selection: Optional<ModelSelection>,
    #[serde(rename = "title", default, skip_serializing_if = "Optional::is_absent")]
    pub title: Optional<Option<TrimmedNonEmptyString>>,
    #[serde(
        rename = "updatedAt",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub updated_at: Optional<Option<TrimmedNonEmptyString>>,
    #[serde(
        rename = "itemIdentityVersion",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub item_identity_version:
        Optional<OrchestrationV2ProviderThreadNativeMetadataItemIdentityVersion>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2ProviderTurnStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "interrupted")]
    Interrupted,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "cancelled")]
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ProviderTurn {
    #[serde(rename = "id")]
    pub id: ProviderTurnId,
    #[serde(rename = "providerThreadId")]
    pub provider_thread_id: ProviderThreadId,
    #[serde(rename = "nodeId")]
    pub node_id: NodeId,
    #[serde(rename = "runAttemptId", deserialize_with = "required_nullable")]
    pub run_attempt_id: Option<RunAttemptId>,
    #[serde(rename = "nativeTurnRef", deserialize_with = "required_nullable")]
    pub native_turn_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(rename = "ordinal")]
    pub ordinal: PositiveInt,
    #[serde(rename = "status")]
    pub status: OrchestrationV2ProviderTurnStatus,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<String>,
    #[serde(rename = "completedAt", deserialize_with = "required_nullable")]
    pub completed_at: Option<String>,
    #[serde(
        rename = "tokenUsage",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub token_usage: Optional<OrchestrationV2ProviderTurnTokenUsage>,
    #[serde(
        rename = "turnTokenUsage",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub turn_token_usage: Optional<TurnTokenUsage>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2ProviderTurnJsonStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "interrupted")]
    Interrupted,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "cancelled")]
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ProviderTurnJson {
    #[serde(rename = "id")]
    pub id: ProviderTurnId,
    #[serde(rename = "providerThreadId")]
    pub provider_thread_id: ProviderThreadId,
    #[serde(rename = "nodeId")]
    pub node_id: NodeId,
    #[serde(rename = "runAttemptId", deserialize_with = "required_nullable")]
    pub run_attempt_id: Option<RunAttemptId>,
    #[serde(rename = "nativeTurnRef", deserialize_with = "required_nullable")]
    pub native_turn_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(rename = "ordinal")]
    pub ordinal: PositiveInt,
    #[serde(rename = "status")]
    pub status: OrchestrationV2ProviderTurnJsonStatus,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<String>,
    #[serde(rename = "completedAt", deserialize_with = "required_nullable")]
    pub completed_at: Option<String>,
    #[serde(
        rename = "tokenUsage",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub token_usage: Optional<OrchestrationV2ProviderTurnTokenUsage>,
    #[serde(
        rename = "turnTokenUsage",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub turn_token_usage: Optional<TurnTokenUsage>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ProviderTurnTokenUsage {
    #[serde(rename = "usedTokens")]
    pub used_tokens: NonNegativeInt,
    #[serde(
        rename = "maxTokens",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub max_tokens: Optional<Option<NonNegativeInt>>,
    #[serde(
        rename = "inputTokens",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub input_tokens: Optional<NonNegativeInt>,
    #[serde(
        rename = "cachedInputTokens",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub cached_input_tokens: Optional<NonNegativeInt>,
    #[serde(
        rename = "outputTokens",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub output_tokens: Optional<NonNegativeInt>,
    #[serde(
        rename = "reasoningOutputTokens",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub reasoning_output_tokens: Optional<NonNegativeInt>,
    #[serde(rename = "updatedAt")]
    pub updated_at: IsoDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2RawProviderEventDirection {
    #[serde(rename = "incoming")]
    Incoming,
    #[serde(rename = "outgoing")]
    Outgoing,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2RawProviderEventMessageKind {
    #[serde(rename = "request")]
    Request,
    #[serde(rename = "response")]
    Response,
    #[serde(rename = "notification")]
    Notification,
    #[serde(rename = "error")]
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OrchestrationV2RawProviderEventJsonRpcId {
    Variant1(IsoDateTime),
    Variant2(JsonNumber),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2RawProviderEvent {
    #[serde(rename = "id")]
    pub id: RawEventId,
    #[serde(rename = "driver")]
    pub driver: ProviderDriverKind,
    #[serde(rename = "providerInstanceId")]
    pub provider_instance_id: ProviderInstanceId,
    #[serde(rename = "providerSessionId")]
    pub provider_session_id: ProviderSessionId,
    #[serde(rename = "sequence")]
    pub sequence: PositiveInt,
    #[serde(rename = "direction")]
    pub direction: OrchestrationV2RawProviderEventDirection,
    #[serde(rename = "messageKind")]
    pub message_kind: OrchestrationV2RawProviderEventMessageKind,
    #[serde(rename = "method", deserialize_with = "required_nullable")]
    pub method: Option<TrimmedNonEmptyString>,
    #[serde(rename = "jsonRpcId", deserialize_with = "required_nullable")]
    pub json_rpc_id: Option<OrchestrationV2RawProviderEventJsonRpcId>,
    #[serde(rename = "payload")]
    pub payload: serde_json::Value,
    #[serde(rename = "observedAt")]
    pub observed_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2RawProviderEventJsonDirection {
    #[serde(rename = "incoming")]
    Incoming,
    #[serde(rename = "outgoing")]
    Outgoing,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2RawProviderEventJsonMessageKind {
    #[serde(rename = "request")]
    Request,
    #[serde(rename = "response")]
    Response,
    #[serde(rename = "notification")]
    Notification,
    #[serde(rename = "error")]
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OrchestrationV2RawProviderEventJsonJsonRpcId {
    Variant1(IsoDateTime),
    Variant2(JsonNumber),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2RawProviderEventJson {
    #[serde(rename = "id")]
    pub id: RawEventId,
    #[serde(rename = "driver")]
    pub driver: ProviderDriverKind,
    #[serde(rename = "providerInstanceId")]
    pub provider_instance_id: ProviderInstanceId,
    #[serde(rename = "providerSessionId")]
    pub provider_session_id: ProviderSessionId,
    #[serde(rename = "sequence")]
    pub sequence: PositiveInt,
    #[serde(rename = "direction")]
    pub direction: OrchestrationV2RawProviderEventJsonDirection,
    #[serde(rename = "messageKind")]
    pub message_kind: OrchestrationV2RawProviderEventJsonMessageKind,
    #[serde(rename = "method", deserialize_with = "required_nullable")]
    pub method: Option<TrimmedNonEmptyString>,
    #[serde(rename = "jsonRpcId", deserialize_with = "required_nullable")]
    pub json_rpc_id: Option<OrchestrationV2RawProviderEventJsonJsonRpcId>,
    #[serde(rename = "payload")]
    pub payload: serde_json::Value,
    #[serde(rename = "observedAt")]
    pub observed_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2RestartCancelledBackgroundWorkKind {
    #[serde(rename = "subagent")]
    Subagent,
    #[serde(rename = "shell")]
    Shell,
    #[serde(rename = "monitor")]
    Monitor,
    #[serde(rename = "task")]
    Task,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2RestartCancelledBackgroundWork {
    #[serde(rename = "kind")]
    pub kind: OrchestrationV2RestartCancelledBackgroundWorkKind,
    #[serde(rename = "label")]
    pub label: TrimmedNonEmptyString,
    #[serde(rename = "id", default, skip_serializing_if = "Optional::is_absent")]
    pub id: Optional<TrimmedNonEmptyString>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2RpcErrorOrchestrationV2DispatchCommandError {
    #[serde(rename = "commandId")]
    pub command_id: CommandId,
    #[serde(rename = "commandType")]
    pub command_type: IsoDateTime,
    #[serde(rename = "message")]
    pub message: IsoDateTime,
    #[serde(
        rename = "detail",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub detail: Optional<IsoDateTime>,
    #[serde(rename = "cause", default, skip_serializing_if = "Optional::is_absent")]
    pub cause: Optional<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2RpcErrorOrchestrationV2GetThreadProjectionError {
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "message")]
    pub message: IsoDateTime,
    #[serde(rename = "cause", default, skip_serializing_if = "Optional::is_absent")]
    pub cause: Optional<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2RpcErrorOrchestrationV2GetShellSnapshotError {
    #[serde(rename = "message")]
    pub message: IsoDateTime,
    #[serde(rename = "cause", default, skip_serializing_if = "Optional::is_absent")]
    pub cause: Optional<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2RpcErrorOrchestrationV2ThreadLaunchError {
    #[serde(rename = "commandId")]
    pub command_id: CommandId,
    #[serde(rename = "projectId")]
    pub project_id: ProjectId,
    #[serde(rename = "message")]
    pub message: IsoDateTime,
    #[serde(rename = "cause", default, skip_serializing_if = "Optional::is_absent")]
    pub cause: Optional<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "_tag")]
pub enum OrchestrationV2RpcError {
    #[serde(rename = "OrchestrationV2DispatchCommandError")]
    OrchestrationV2DispatchCommandError(
        Box<OrchestrationV2RpcErrorOrchestrationV2DispatchCommandError>,
    ),
    #[serde(rename = "OrchestrationV2GetThreadProjectionError")]
    OrchestrationV2GetThreadProjectionError(
        Box<OrchestrationV2RpcErrorOrchestrationV2GetThreadProjectionError>,
    ),
    #[serde(rename = "OrchestrationV2GetShellSnapshotError")]
    OrchestrationV2GetShellSnapshotError(
        Box<OrchestrationV2RpcErrorOrchestrationV2GetShellSnapshotError>,
    ),
    #[serde(rename = "OrchestrationV2ThreadLaunchError")]
    OrchestrationV2ThreadLaunchError(Box<OrchestrationV2RpcErrorOrchestrationV2ThreadLaunchError>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2RunSourcePlanRef {
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "planId")]
    pub plan_id: PlanId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2Run {
    #[serde(rename = "id")]
    pub id: RunId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "ordinal")]
    pub ordinal: PositiveInt,
    #[serde(rename = "providerInstanceId")]
    pub provider_instance_id: ProviderInstanceId,
    #[serde(rename = "modelSelection")]
    pub model_selection: ModelSelection,
    #[serde(rename = "providerThreadId", deserialize_with = "required_nullable")]
    pub provider_thread_id: Option<ProviderThreadId>,
    #[serde(rename = "userMessageId")]
    pub user_message_id: MessageId,
    #[serde(rename = "rootNodeId", deserialize_with = "required_nullable")]
    pub root_node_id: Option<NodeId>,
    #[serde(rename = "activeAttemptId", deserialize_with = "required_nullable")]
    pub active_attempt_id: Option<RunAttemptId>,
    #[serde(rename = "status")]
    pub status: OrchestrationV2RunStatus,
    #[serde(
        rename = "queuePosition",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub queue_position: Optional<Option<PositiveInt>>,
    #[serde(
        rename = "queueHeld",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub queue_held: Optional<bool>,
    #[serde(rename = "requestedAt")]
    pub requested_at: String,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<String>,
    #[serde(rename = "completedAt", deserialize_with = "required_nullable")]
    pub completed_at: Option<String>,
    #[serde(rename = "checkpointId", deserialize_with = "required_nullable")]
    pub checkpoint_id: Option<CheckpointId>,
    #[serde(rename = "contextHandoffId", deserialize_with = "required_nullable")]
    pub context_handoff_id: Option<ContextHandoffId>,
    #[serde(
        rename = "restartContinuationOfRunId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub restart_continuation_of_run_id: Optional<RunId>,
    #[serde(
        rename = "workStartedAt",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub work_started_at: Optional<String>,
    #[serde(
        rename = "restartCancelledBackgroundWork",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub restart_cancelled_background_work:
        Optional<Vec<OrchestrationV2RestartCancelledBackgroundWork>>,
    #[serde(
        rename = "sourcePlanRef",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub source_plan_ref: Optional<OrchestrationV2RunSourcePlanRef>,
    #[serde(
        rename = "delegatedCompletion",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub delegated_completion: Optional<OrchestrationV2DelegatedCompletionCohort>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2RunAttemptReason {
    #[serde(rename = "initial")]
    Initial,
    #[serde(rename = "steering_restart")]
    SteeringRestart,
    #[serde(rename = "retry")]
    Retry,
    #[serde(rename = "provider_recovery")]
    ProviderRecovery,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2RunAttemptStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "interrupted")]
    Interrupted,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "cancelled")]
    Cancelled,
    #[serde(rename = "superseded")]
    Superseded,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2RunAttempt {
    #[serde(rename = "id")]
    pub id: RunAttemptId,
    #[serde(
        rename = "nativeThreadId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub native_thread_id: Optional<IsoDateTime>,
    #[serde(rename = "runId")]
    pub run_id: RunId,
    #[serde(rename = "attemptOrdinal")]
    pub attempt_ordinal: PositiveInt,
    #[serde(rename = "rootNodeId")]
    pub root_node_id: NodeId,
    #[serde(rename = "providerInstanceId")]
    pub provider_instance_id: ProviderInstanceId,
    #[serde(rename = "providerThreadId")]
    pub provider_thread_id: ProviderThreadId,
    #[serde(rename = "providerTurnId", deserialize_with = "required_nullable")]
    pub provider_turn_id: Option<ProviderTurnId>,
    #[serde(rename = "reason")]
    pub reason: OrchestrationV2RunAttemptReason,
    #[serde(rename = "status")]
    pub status: OrchestrationV2RunAttemptStatus,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<String>,
    #[serde(rename = "completedAt", deserialize_with = "required_nullable")]
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2RunAttemptJsonReason {
    #[serde(rename = "initial")]
    Initial,
    #[serde(rename = "steering_restart")]
    SteeringRestart,
    #[serde(rename = "retry")]
    Retry,
    #[serde(rename = "provider_recovery")]
    ProviderRecovery,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2RunAttemptJsonStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "interrupted")]
    Interrupted,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "cancelled")]
    Cancelled,
    #[serde(rename = "superseded")]
    Superseded,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2RunAttemptJson {
    #[serde(rename = "id")]
    pub id: RunAttemptId,
    #[serde(
        rename = "nativeThreadId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub native_thread_id: Optional<IsoDateTime>,
    #[serde(rename = "runId")]
    pub run_id: RunId,
    #[serde(rename = "attemptOrdinal")]
    pub attempt_ordinal: PositiveInt,
    #[serde(rename = "rootNodeId")]
    pub root_node_id: NodeId,
    #[serde(rename = "providerInstanceId")]
    pub provider_instance_id: ProviderInstanceId,
    #[serde(rename = "providerThreadId")]
    pub provider_thread_id: ProviderThreadId,
    #[serde(rename = "providerTurnId", deserialize_with = "required_nullable")]
    pub provider_turn_id: Option<ProviderTurnId>,
    #[serde(rename = "reason")]
    pub reason: OrchestrationV2RunAttemptJsonReason,
    #[serde(rename = "status")]
    pub status: OrchestrationV2RunAttemptJsonStatus,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<String>,
    #[serde(rename = "completedAt", deserialize_with = "required_nullable")]
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2RunBackgroundWorkCancelled {
    #[serde(rename = "runId")]
    pub run_id: RunId,
    #[serde(rename = "restartCancelledBackgroundWork")]
    pub restart_cancelled_background_work: Vec<OrchestrationV2RestartCancelledBackgroundWork>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2RunJsonSourcePlanRef {
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "planId")]
    pub plan_id: PlanId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2RunJson {
    #[serde(rename = "id")]
    pub id: RunId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "ordinal")]
    pub ordinal: PositiveInt,
    #[serde(rename = "providerInstanceId")]
    pub provider_instance_id: ProviderInstanceId,
    #[serde(rename = "modelSelection")]
    pub model_selection: ModelSelection,
    #[serde(rename = "providerThreadId", deserialize_with = "required_nullable")]
    pub provider_thread_id: Option<ProviderThreadId>,
    #[serde(rename = "userMessageId")]
    pub user_message_id: MessageId,
    #[serde(rename = "rootNodeId", deserialize_with = "required_nullable")]
    pub root_node_id: Option<NodeId>,
    #[serde(rename = "activeAttemptId", deserialize_with = "required_nullable")]
    pub active_attempt_id: Option<RunAttemptId>,
    #[serde(rename = "status")]
    pub status: OrchestrationV2RunStatus,
    #[serde(
        rename = "queuePosition",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub queue_position: Optional<Option<PositiveInt>>,
    #[serde(
        rename = "queueHeld",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub queue_held: Optional<bool>,
    #[serde(rename = "requestedAt")]
    pub requested_at: String,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<String>,
    #[serde(rename = "completedAt", deserialize_with = "required_nullable")]
    pub completed_at: Option<String>,
    #[serde(rename = "checkpointId", deserialize_with = "required_nullable")]
    pub checkpoint_id: Option<CheckpointId>,
    #[serde(rename = "contextHandoffId", deserialize_with = "required_nullable")]
    pub context_handoff_id: Option<ContextHandoffId>,
    #[serde(
        rename = "restartContinuationOfRunId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub restart_continuation_of_run_id: Optional<RunId>,
    #[serde(
        rename = "workStartedAt",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub work_started_at: Optional<String>,
    #[serde(
        rename = "restartCancelledBackgroundWork",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub restart_cancelled_background_work:
        Optional<Vec<OrchestrationV2RestartCancelledBackgroundWork>>,
    #[serde(
        rename = "sourcePlanRef",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub source_plan_ref: Optional<OrchestrationV2RunJsonSourcePlanRef>,
    #[serde(
        rename = "delegatedCompletion",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub delegated_completion: Optional<OrchestrationV2DelegatedCompletionCohort>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2RunStatus {
    #[serde(rename = "preparing")]
    Preparing,
    #[serde(rename = "queued")]
    Queued,
    #[serde(rename = "starting")]
    Starting,
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "waiting")]
    Waiting,
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "interrupted")]
    Interrupted,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "cancelled")]
    Cancelled,
    #[serde(rename = "rolled_back")]
    RolledBack,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2RuntimePolicyCapabilitiesEnforcement {
    #[serde(rename = "native")]
    Native,
    #[serde(rename = "client-boundary")]
    ClientBoundary,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2RuntimePolicyCapabilities {
    #[serde(rename = "enforcement")]
    pub enforcement: OrchestrationV2RuntimePolicyCapabilitiesEnforcement,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2RuntimeRequestKind {
    #[serde(rename = "command")]
    Command,
    #[serde(rename = "file-read")]
    FileRead,
    #[serde(rename = "file-change")]
    FileChange,
    #[serde(rename = "mcp-elicitation")]
    McpElicitation,
    #[serde(rename = "permission")]
    Permission,
    #[serde(rename = "dynamic_tool_call")]
    DynamicToolCall,
    #[serde(rename = "user_input")]
    UserInput,
    #[serde(rename = "auth_refresh")]
    AuthRefresh,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2RuntimeRequestStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "resolved")]
    Resolved,
    #[serde(rename = "expired")]
    Expired,
    #[serde(rename = "cancelled")]
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2RuntimeRequestResponseCapabilityLive {
    #[serde(rename = "providerSessionId")]
    pub provider_session_id: ProviderSessionId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2RuntimeRequestResponseCapabilityMessage {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2RuntimeRequestResponseCapabilityNotResumable {
    #[serde(rename = "reason")]
    pub reason: IsoDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum OrchestrationV2RuntimeRequestResponseCapability {
    #[serde(rename = "live")]
    Live(Box<OrchestrationV2RuntimeRequestResponseCapabilityLive>),
    #[serde(rename = "message")]
    Message(Box<OrchestrationV2RuntimeRequestResponseCapabilityMessage>),
    #[serde(rename = "not_resumable")]
    NotResumable(Box<OrchestrationV2RuntimeRequestResponseCapabilityNotResumable>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2RuntimeRequest {
    #[serde(rename = "id")]
    pub id: RuntimeRequestId,
    #[serde(rename = "nodeId")]
    pub node_id: NodeId,
    #[serde(rename = "providerTurnId", deserialize_with = "required_nullable")]
    pub provider_turn_id: Option<ProviderTurnId>,
    #[serde(rename = "nativeRequestRef", deserialize_with = "required_nullable")]
    pub native_request_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(rename = "kind")]
    pub kind: OrchestrationV2RuntimeRequestKind,
    #[serde(rename = "status")]
    pub status: OrchestrationV2RuntimeRequestStatus,
    #[serde(rename = "responseCapability")]
    pub response_capability: OrchestrationV2RuntimeRequestResponseCapability,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "resolvedAt", deserialize_with = "required_nullable")]
    pub resolved_at: Option<String>,
    #[serde(
        rename = "decision",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub decision: Optional<ProviderApprovalDecision>,
    #[serde(
        rename = "answers",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub answers: Optional<ProviderUserInputAnswers>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2RuntimeRequestJsonKind {
    #[serde(rename = "command")]
    Command,
    #[serde(rename = "file-read")]
    FileRead,
    #[serde(rename = "file-change")]
    FileChange,
    #[serde(rename = "mcp-elicitation")]
    McpElicitation,
    #[serde(rename = "permission")]
    Permission,
    #[serde(rename = "dynamic_tool_call")]
    DynamicToolCall,
    #[serde(rename = "user_input")]
    UserInput,
    #[serde(rename = "auth_refresh")]
    AuthRefresh,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2RuntimeRequestJsonStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "resolved")]
    Resolved,
    #[serde(rename = "expired")]
    Expired,
    #[serde(rename = "cancelled")]
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2RuntimeRequestJsonResponseCapabilityLive {
    #[serde(rename = "providerSessionId")]
    pub provider_session_id: ProviderSessionId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2RuntimeRequestJsonResponseCapabilityMessage {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2RuntimeRequestJsonResponseCapabilityNotResumable {
    #[serde(rename = "reason")]
    pub reason: IsoDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum OrchestrationV2RuntimeRequestJsonResponseCapability {
    #[serde(rename = "live")]
    Live(Box<OrchestrationV2RuntimeRequestJsonResponseCapabilityLive>),
    #[serde(rename = "message")]
    Message(Box<OrchestrationV2RuntimeRequestJsonResponseCapabilityMessage>),
    #[serde(rename = "not_resumable")]
    NotResumable(Box<OrchestrationV2RuntimeRequestJsonResponseCapabilityNotResumable>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2RuntimeRequestJson {
    #[serde(rename = "id")]
    pub id: RuntimeRequestId,
    #[serde(rename = "nodeId")]
    pub node_id: NodeId,
    #[serde(rename = "providerTurnId", deserialize_with = "required_nullable")]
    pub provider_turn_id: Option<ProviderTurnId>,
    #[serde(rename = "nativeRequestRef", deserialize_with = "required_nullable")]
    pub native_request_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(rename = "kind")]
    pub kind: OrchestrationV2RuntimeRequestJsonKind,
    #[serde(rename = "status")]
    pub status: OrchestrationV2RuntimeRequestJsonStatus,
    #[serde(rename = "responseCapability")]
    pub response_capability: OrchestrationV2RuntimeRequestJsonResponseCapability,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "resolvedAt", deserialize_with = "required_nullable")]
    pub resolved_at: Option<String>,
    #[serde(
        rename = "decision",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub decision: Optional<ProviderApprovalDecision>,
    #[serde(
        rename = "answers",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub answers: Optional<ProviderUserInputAnswers>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2SessionCapabilities {
    #[serde(rename = "supportsMultipleProviderThreadsPerSession")]
    pub supports_multiple_provider_threads_per_session: bool,
    #[serde(rename = "supportsModelSwitchInSession")]
    pub supports_model_switch_in_session: bool,
    #[serde(rename = "supportsProviderSwitchingViaHandoff")]
    pub supports_provider_switching_via_handoff: bool,
    #[serde(rename = "supportsRuntimeModeSwitchInSession")]
    pub supports_runtime_mode_switch_in_session: bool,
    #[serde(rename = "pendingRequestsSurviveRestart")]
    pub pending_requests_survive_restart: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ShellSnapshot {
    #[serde(rename = "schemaVersion")]
    pub schema_version: PositiveInt,
    #[serde(rename = "snapshotSequence")]
    pub snapshot_sequence: NonNegativeInt,
    #[serde(rename = "threads")]
    pub threads: Vec<OrchestrationV2ThreadShell>,
    #[serde(rename = "archivedThreads")]
    pub archived_threads: Vec<OrchestrationV2ThreadShell>,
    #[serde(rename = "projects")]
    pub projects: Vec<OrchestrationProjectShell>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ShellSnapshotJson {
    #[serde(rename = "schemaVersion")]
    pub schema_version: PositiveInt,
    #[serde(rename = "snapshotSequence")]
    pub snapshot_sequence: NonNegativeInt,
    #[serde(rename = "threads")]
    pub threads: Vec<OrchestrationV2ThreadShellJson>,
    #[serde(rename = "archivedThreads")]
    pub archived_threads: Vec<OrchestrationV2ThreadShellJson>,
    #[serde(rename = "projects")]
    pub projects: Vec<OrchestrationProjectShell>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ShellStreamItemSynchronized {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ShellStreamItemSnapshot {
    #[serde(rename = "snapshot")]
    pub snapshot: OrchestrationV2ShellSnapshot,
    #[serde(
        rename = "resolvedRepositoryIdentityRoots",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub resolved_repository_identity_roots: Optional<Vec<IsoDateTime>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ShellStreamItemProjectUpdated {
    #[serde(rename = "sequence")]
    pub sequence: NonNegativeInt,
    #[serde(rename = "project")]
    pub project: OrchestrationProjectShell,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ShellStreamItemProjectRemoved {
    #[serde(rename = "sequence")]
    pub sequence: NonNegativeInt,
    #[serde(rename = "projectId")]
    pub project_id: ProjectId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2ShellStreamItemThreadUpdatedLocation {
    #[serde(rename = "active")]
    Active,
    #[serde(rename = "archive")]
    Archive,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ShellStreamItemThreadUpdated {
    #[serde(rename = "sequence")]
    pub sequence: NonNegativeInt,
    #[serde(rename = "location")]
    pub location: OrchestrationV2ShellStreamItemThreadUpdatedLocation,
    #[serde(rename = "thread")]
    pub thread: OrchestrationV2ThreadShell,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2ShellStreamItemThreadRemovedLocation {
    #[serde(rename = "active")]
    Active,
    #[serde(rename = "archive")]
    Archive,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ShellStreamItemThreadRemoved {
    #[serde(rename = "sequence")]
    pub sequence: NonNegativeInt,
    #[serde(rename = "location")]
    pub location: OrchestrationV2ShellStreamItemThreadRemovedLocation,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum OrchestrationV2ShellStreamItem {
    #[serde(rename = "synchronized")]
    Synchronized(Box<OrchestrationV2ShellStreamItemSynchronized>),
    #[serde(rename = "snapshot")]
    Snapshot(Box<OrchestrationV2ShellStreamItemSnapshot>),
    #[serde(rename = "project.updated")]
    ProjectUpdated(Box<OrchestrationV2ShellStreamItemProjectUpdated>),
    #[serde(rename = "project.removed")]
    ProjectRemoved(Box<OrchestrationV2ShellStreamItemProjectRemoved>),
    #[serde(rename = "thread.updated")]
    ThreadUpdated(Box<OrchestrationV2ShellStreamItemThreadUpdated>),
    #[serde(rename = "thread.removed")]
    ThreadRemoved(Box<OrchestrationV2ShellStreamItemThreadRemoved>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2ShellThreadStatus {
    #[serde(rename = "idle")]
    Idle,
    #[serde(rename = "preparing")]
    Preparing,
    #[serde(rename = "queued")]
    Queued,
    #[serde(rename = "starting")]
    Starting,
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "waiting")]
    Waiting,
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "interrupted")]
    Interrupted,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "cancelled")]
    Cancelled,
    #[serde(rename = "rolled_back")]
    RolledBack,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2StoredEvent {
    #[serde(rename = "sequence")]
    pub sequence: NonNegativeInt,
    #[serde(rename = "commandId", deserialize_with = "required_nullable")]
    pub command_id: Option<CommandId>,
    #[serde(rename = "event")]
    pub event: OrchestrationV2DomainEvent,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2StoredEventJson {
    #[serde(rename = "sequence")]
    pub sequence: NonNegativeInt,
    #[serde(rename = "commandId", deserialize_with = "required_nullable")]
    pub command_id: Option<CommandId>,
    #[serde(rename = "event")]
    pub event: OrchestrationV2DomainEventJson,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2StreamingCapabilities {
    #[serde(rename = "streamsAssistantText")]
    pub streams_assistant_text: bool,
    #[serde(rename = "streamsReasoning")]
    pub streams_reasoning: bool,
    #[serde(rename = "streamsToolOutput")]
    pub streams_tool_output: bool,
    #[serde(rename = "streamsPlanText")]
    pub streams_plan_text: bool,
    #[serde(rename = "emitsMessageCompleted")]
    pub emits_message_completed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2SubagentOrigin {
    #[serde(rename = "provider_native")]
    ProviderNative,
    #[serde(rename = "app_owned")]
    AppOwned,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2SubagentCompletionWake {
    #[serde(rename = "always")]
    Always,
    #[serde(rename = "settled_only")]
    SettledOnly,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2SubagentStatus {
    #[serde(rename = "idle")]
    Idle,
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "waiting")]
    Waiting,
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "cancelled")]
    Cancelled,
    #[serde(rename = "interrupted")]
    Interrupted,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2Subagent {
    #[serde(rename = "id")]
    pub id: NodeId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "parentNodeId")]
    pub parent_node_id: NodeId,
    #[serde(rename = "origin")]
    pub origin: OrchestrationV2SubagentOrigin,
    #[serde(rename = "createdBy")]
    pub created_by: OrchestrationV2Actor,
    #[serde(rename = "driver")]
    pub driver: ProviderDriverKind,
    #[serde(rename = "providerInstanceId")]
    pub provider_instance_id: ProviderInstanceId,
    #[serde(rename = "providerThreadId", deserialize_with = "required_nullable")]
    pub provider_thread_id: Option<ProviderThreadId>,
    #[serde(rename = "childThreadId", deserialize_with = "required_nullable")]
    pub child_thread_id: Option<ThreadId>,
    #[serde(rename = "nativeTaskRef", deserialize_with = "required_nullable")]
    pub native_task_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(rename = "prompt")]
    pub prompt: IsoDateTime,
    #[serde(rename = "title", deserialize_with = "required_nullable")]
    pub title: Option<IsoDateTime>,
    #[serde(rename = "model", deserialize_with = "required_nullable")]
    pub model: Option<IsoDateTime>,
    #[serde(
        rename = "completionWake",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub completion_wake: Optional<OrchestrationV2SubagentCompletionWake>,
    #[serde(
        rename = "completionDelivery",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub completion_delivery: Optional<OrchestrationV2DelegatedCompletionTaskDelivery>,
    #[serde(rename = "status")]
    pub status: OrchestrationV2SubagentStatus,
    #[serde(
        rename = "progress",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub progress: Optional<IsoDateTime>,
    #[serde(rename = "result", deserialize_with = "required_nullable")]
    pub result: Option<IsoDateTime>,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<String>,
    #[serde(rename = "completedAt", deserialize_with = "required_nullable")]
    pub completed_at: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2SubagentCapabilities {
    #[serde(rename = "supportsSubagents")]
    pub supports_subagents: bool,
    #[serde(rename = "exposesSubagentThreadIds")]
    pub exposes_subagent_thread_ids: bool,
    #[serde(rename = "emitsSubagentLifecycle")]
    pub emits_subagent_lifecycle: bool,
    #[serde(rename = "canWaitForSubagents")]
    pub can_wait_for_subagents: bool,
    #[serde(rename = "canCloseSubagents")]
    pub can_close_subagents: bool,
    #[serde(rename = "canForkSubagentThread")]
    pub can_fork_subagent_thread: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2SubagentJsonOrigin {
    #[serde(rename = "provider_native")]
    ProviderNative,
    #[serde(rename = "app_owned")]
    AppOwned,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2SubagentJsonCompletionWake {
    #[serde(rename = "always")]
    Always,
    #[serde(rename = "settled_only")]
    SettledOnly,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2SubagentJsonStatus {
    #[serde(rename = "idle")]
    Idle,
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "waiting")]
    Waiting,
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "cancelled")]
    Cancelled,
    #[serde(rename = "interrupted")]
    Interrupted,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2SubagentJson {
    #[serde(rename = "id")]
    pub id: NodeId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "parentNodeId")]
    pub parent_node_id: NodeId,
    #[serde(rename = "origin")]
    pub origin: OrchestrationV2SubagentJsonOrigin,
    #[serde(rename = "createdBy")]
    pub created_by: OrchestrationV2Actor,
    #[serde(rename = "driver")]
    pub driver: ProviderDriverKind,
    #[serde(rename = "providerInstanceId")]
    pub provider_instance_id: ProviderInstanceId,
    #[serde(rename = "providerThreadId", deserialize_with = "required_nullable")]
    pub provider_thread_id: Option<ProviderThreadId>,
    #[serde(rename = "childThreadId", deserialize_with = "required_nullable")]
    pub child_thread_id: Option<ThreadId>,
    #[serde(rename = "nativeTaskRef", deserialize_with = "required_nullable")]
    pub native_task_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(rename = "prompt")]
    pub prompt: IsoDateTime,
    #[serde(rename = "title", deserialize_with = "required_nullable")]
    pub title: Option<IsoDateTime>,
    #[serde(rename = "model", deserialize_with = "required_nullable")]
    pub model: Option<IsoDateTime>,
    #[serde(
        rename = "completionWake",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub completion_wake: Optional<OrchestrationV2SubagentJsonCompletionWake>,
    #[serde(
        rename = "completionDelivery",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub completion_delivery: Optional<OrchestrationV2DelegatedCompletionTaskDelivery>,
    #[serde(rename = "status")]
    pub status: OrchestrationV2SubagentJsonStatus,
    #[serde(
        rename = "progress",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub progress: Optional<IsoDateTime>,
    #[serde(rename = "result", deserialize_with = "required_nullable")]
    pub result: Option<IsoDateTime>,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<String>,
    #[serde(rename = "completedAt", deserialize_with = "required_nullable")]
    pub completed_at: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2SubscribeShellInput {
    #[serde(
        rename = "afterSequence",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub after_sequence: Optional<i64>,
    #[serde(
        rename = "requestCompletionMarker",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub request_completion_marker: Optional<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2SubscribeThreadInput {
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(
        rename = "afterSequence",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub after_sequence: Optional<i64>,
    #[serde(
        rename = "requestCompletionMarker",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub request_completion_marker: Optional<bool>,
    #[serde(
        rename = "acceptBoundedSnapshot",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub accept_bounded_snapshot: Optional<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ThreadBoundedSnapshot {
    #[serde(rename = "snapshotSequence")]
    pub snapshot_sequence: NonNegativeInt,
    #[serde(rename = "projection")]
    pub projection: OrchestrationV2ThreadProjection,
    #[serde(rename = "historyCursor", deserialize_with = "required_nullable")]
    pub history_cursor: Option<TrimmedNonEmptyString>,
    #[serde(rename = "hasMoreHistory")]
    pub has_more_history: bool,
    #[serde(
        rename = "latestLocalTurnOrdinal",
        deserialize_with = "required_nullable"
    )]
    pub latest_local_turn_ordinal: Option<NonNegativeInt>,
    #[serde(
        rename = "payloadBudgetExceeded",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub payload_budget_exceeded: Optional<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ThreadCapabilities {
    #[serde(rename = "canCreateEmptyThread")]
    pub can_create_empty_thread: bool,
    #[serde(rename = "canReadThreadSnapshot")]
    pub can_read_thread_snapshot: bool,
    #[serde(rename = "canRollbackThread")]
    pub can_rollback_thread: bool,
    #[serde(rename = "canForkThread")]
    pub can_fork_thread: bool,
    #[serde(rename = "canForkFromTurn")]
    pub can_fork_from_turn: bool,
    #[serde(rename = "canForkFromSubagentThread")]
    pub can_fork_from_subagent_thread: bool,
    #[serde(rename = "exposesNativeThreadId")]
    pub exposes_native_thread_id: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ThreadDetailSnapshot {
    #[serde(rename = "snapshotSequence")]
    pub snapshot_sequence: NonNegativeInt,
    #[serde(rename = "projection")]
    pub projection: OrchestrationV2ThreadProjection,
    #[serde(
        rename = "historyCursor",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub history_cursor: Optional<Option<TrimmedNonEmptyString>>,
    #[serde(
        rename = "hasMoreHistory",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub has_more_history: Optional<bool>,
    #[serde(
        rename = "latestLocalTurnOrdinal",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub latest_local_turn_ordinal: Optional<Option<NonNegativeInt>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ThreadForkSourcePointLatestStable {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ThreadForkSourcePointRun {
    #[serde(rename = "runId")]
    pub run_id: RunId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ThreadForkSourcePointCheckpoint {
    #[serde(rename = "checkpointId")]
    pub checkpoint_id: CheckpointId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum OrchestrationV2ThreadForkSourcePoint {
    #[serde(rename = "latest_stable")]
    LatestStable(Box<OrchestrationV2ThreadForkSourcePointLatestStable>),
    #[serde(rename = "run")]
    Run(Box<OrchestrationV2ThreadForkSourcePointRun>),
    #[serde(rename = "checkpoint")]
    Checkpoint(Box<OrchestrationV2ThreadForkSourcePointCheckpoint>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2ThreadHistoryOrigin {
    #[serde(rename = "native")]
    Native,
    #[serde(rename = "v1_import")]
    V1Import,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ThreadHistoryPage {
    #[serde(rename = "snapshotSequence")]
    pub snapshot_sequence: NonNegativeInt,
    #[serde(rename = "items")]
    pub items: Vec<OrchestrationV2ProjectedTurnItem>,
    #[serde(rename = "nextCursor", deserialize_with = "required_nullable")]
    pub next_cursor: Option<TrimmedNonEmptyString>,
    #[serde(rename = "hasMoreHistory")]
    pub has_more_history: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2ThreadLaunchErrorTag {
    #[serde(rename = "OrchestrationV2ThreadLaunchError")]
    OrchestrationV2ThreadLaunchError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ThreadLaunchError {
    #[serde(rename = "_tag")]
    pub _tag: OrchestrationV2ThreadLaunchErrorTag,
    #[serde(rename = "commandId")]
    pub command_id: CommandId,
    #[serde(rename = "projectId")]
    pub project_id: ProjectId,
    #[serde(rename = "message")]
    pub message: IsoDateTime,
    #[serde(rename = "cause", default, skip_serializing_if = "Optional::is_absent")]
    pub cause: Optional<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ThreadLaunchInputInitialMessage {
    #[serde(
        rename = "messageId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub message_id: Optional<MessageId>,
    #[serde(rename = "text")]
    pub text: IsoDateTime,
    #[serde(
        rename = "context",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub context: Optional<OrchestrationMessageContext>,
    #[serde(rename = "attachments")]
    pub attachments: Vec<ChatAttachment>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ThreadLaunchInput {
    #[serde(rename = "commandId")]
    pub command_id: CommandId,
    #[serde(
        rename = "creationSource",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub creation_source: Optional<OrchestrationV2CreationSource>,
    #[serde(
        rename = "threadId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub thread_id: Optional<ThreadId>,
    #[serde(
        rename = "reuseExistingThread",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub reuse_existing_thread: Optional<bool>,
    #[serde(rename = "projectId")]
    pub project_id: ProjectId,
    #[serde(rename = "title")]
    pub title: TrimmedNonEmptyString,
    #[serde(
        rename = "generateTitle",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub generate_title: Optional<bool>,
    #[serde(rename = "modelSelection")]
    pub model_selection: ModelSelection,
    #[serde(rename = "runtimeMode")]
    pub runtime_mode: RuntimeMode,
    #[serde(rename = "interactionMode")]
    pub interaction_mode: ProviderInteractionMode,
    #[serde(rename = "workspaceStrategy")]
    pub workspace_strategy: OrchestrationV2ThreadLaunchWorkspaceStrategy,
    #[serde(
        rename = "initialMessage",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub initial_message: Optional<OrchestrationV2ThreadLaunchInputInitialMessage>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ThreadLaunchResult {
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "projection")]
    pub projection: OrchestrationV2ThreadProjection,
    #[serde(rename = "resumed")]
    pub resumed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ThreadLaunchWorkspaceStrategyRoot {
    #[serde(
        rename = "branch",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub branch: Optional<TrimmedNonEmptyString>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ThreadLaunchWorkspaceStrategyExistingWorktree {
    #[serde(rename = "worktreePath")]
    pub worktree_path: TrimmedNonEmptyString,
    #[serde(
        rename = "branch",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub branch: Optional<TrimmedNonEmptyString>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ThreadLaunchWorkspaceStrategyWorktree {
    #[serde(rename = "baseRef")]
    pub base_ref: TrimmedNonEmptyString,
    #[serde(
        rename = "branch",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub branch: Optional<TrimmedNonEmptyString>,
    #[serde(
        rename = "startFromOrigin",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub start_from_origin: Optional<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum OrchestrationV2ThreadLaunchWorkspaceStrategy {
    #[serde(rename = "root")]
    Root(Box<OrchestrationV2ThreadLaunchWorkspaceStrategyRoot>),
    #[serde(rename = "existing_worktree")]
    ExistingWorktree(Box<OrchestrationV2ThreadLaunchWorkspaceStrategyExistingWorktree>),
    #[serde(rename = "worktree")]
    Worktree(Box<OrchestrationV2ThreadLaunchWorkspaceStrategyWorktree>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ThreadProjection {
    #[serde(rename = "thread")]
    pub thread: OrchestrationV2AppThread,
    #[serde(rename = "runs")]
    pub runs: Vec<OrchestrationV2Run>,
    #[serde(rename = "attempts")]
    pub attempts: Vec<OrchestrationV2RunAttempt>,
    #[serde(rename = "nodes")]
    pub nodes: Vec<OrchestrationV2ExecutionNode>,
    #[serde(rename = "subagents")]
    pub subagents: Vec<OrchestrationV2Subagent>,
    #[serde(rename = "providerSessions")]
    pub provider_sessions: Vec<OrchestrationV2ProviderSession>,
    #[serde(rename = "providerThreads")]
    pub provider_threads: Vec<OrchestrationV2ProviderThread>,
    #[serde(rename = "providerTurns")]
    pub provider_turns: Vec<OrchestrationV2ProviderTurn>,
    #[serde(rename = "runtimeRequests")]
    pub runtime_requests: Vec<OrchestrationV2RuntimeRequest>,
    #[serde(rename = "messages")]
    pub messages: Vec<OrchestrationV2ConversationMessage>,
    #[serde(rename = "plans")]
    pub plans: Vec<OrchestrationV2PlanArtifact>,
    #[serde(rename = "turnItems")]
    pub turn_items: Vec<OrchestrationV2TurnItem>,
    #[serde(rename = "checkpointScopes")]
    pub checkpoint_scopes: Vec<OrchestrationV2CheckpointScope>,
    #[serde(rename = "checkpoints")]
    pub checkpoints: Vec<OrchestrationV2Checkpoint>,
    #[serde(rename = "contextHandoffs")]
    pub context_handoffs: Vec<OrchestrationV2ContextHandoff>,
    #[serde(rename = "contextTransfers")]
    pub context_transfers: Vec<OrchestrationV2ContextTransfer>,
    #[serde(rename = "visibleTurnItems")]
    pub visible_turn_items: Vec<OrchestrationV2ProjectedTurnItem>,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ThreadProjectionJson {
    #[serde(rename = "thread")]
    pub thread: OrchestrationV2AppThreadJson,
    #[serde(rename = "runs")]
    pub runs: Vec<OrchestrationV2RunJson>,
    #[serde(rename = "attempts")]
    pub attempts: Vec<OrchestrationV2RunAttemptJson>,
    #[serde(rename = "nodes")]
    pub nodes: Vec<OrchestrationV2ExecutionNodeJson>,
    #[serde(rename = "subagents")]
    pub subagents: Vec<OrchestrationV2SubagentJson>,
    #[serde(rename = "providerSessions")]
    pub provider_sessions: Vec<OrchestrationV2ProviderSessionJson>,
    #[serde(rename = "providerThreads")]
    pub provider_threads: Vec<OrchestrationV2ProviderThreadJson>,
    #[serde(rename = "providerTurns")]
    pub provider_turns: Vec<OrchestrationV2ProviderTurnJson>,
    #[serde(rename = "runtimeRequests")]
    pub runtime_requests: Vec<OrchestrationV2RuntimeRequestJson>,
    #[serde(rename = "messages")]
    pub messages: Vec<OrchestrationV2ConversationMessageJson>,
    #[serde(rename = "plans")]
    pub plans: Vec<OrchestrationV2PlanArtifact>,
    #[serde(rename = "turnItems")]
    pub turn_items: Vec<OrchestrationV2TurnItemJson>,
    #[serde(rename = "checkpointScopes")]
    pub checkpoint_scopes: Vec<OrchestrationV2CheckpointScopeJson>,
    #[serde(rename = "checkpoints")]
    pub checkpoints: Vec<OrchestrationV2CheckpointJson>,
    #[serde(rename = "contextHandoffs")]
    pub context_handoffs: Vec<OrchestrationV2ContextHandoffJson>,
    #[serde(rename = "contextTransfers")]
    pub context_transfers: Vec<OrchestrationV2ContextTransferJson>,
    #[serde(rename = "visibleTurnItems")]
    pub visible_turn_items: Vec<OrchestrationV2ProjectedTurnItemJson>,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ThreadShellForkedFromRun {
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId")]
    pub run_id: RunId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ThreadShellForkedFromNode {
    #[serde(rename = "nodeId")]
    pub node_id: NodeId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ThreadShellForkedFromProviderThread {
    #[serde(rename = "providerThreadId")]
    pub provider_thread_id: ProviderThreadId,
    #[serde(
        rename = "providerTurnId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_turn_id: Optional<ProviderTurnId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum OrchestrationV2ThreadShellForkedFrom {
    #[serde(rename = "run")]
    Run(Box<OrchestrationV2ThreadShellForkedFromRun>),
    #[serde(rename = "node")]
    Node(Box<OrchestrationV2ThreadShellForkedFromNode>),
    #[serde(rename = "provider_thread")]
    ProviderThread(Box<OrchestrationV2ThreadShellForkedFromProviderThread>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2ThreadShellActivityRunStatus {
    #[serde(rename = "preparing")]
    Preparing,
    #[serde(rename = "starting")]
    Starting,
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "waiting")]
    Waiting,
}

fn orchestration_v2_thread_shell_pending_background_tasks_default()
-> Vec<OrchestrationV2PendingBackgroundTask> {
    serde_json::from_str("[]").expect("upstream default")
}

fn orchestration_v2_thread_shell_provider_instance_history_default() -> Vec<ProviderInstanceId> {
    serde_json::from_str("[]").expect("upstream default")
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2ThreadShellSettledOverride {
    #[serde(rename = "settled")]
    Settled,
    #[serde(rename = "active")]
    Active,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ThreadShellTitleRegeneration {
    #[serde(rename = "requestId")]
    pub request_id: CommandId,
    #[serde(rename = "startedAt")]
    pub started_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ThreadShell {
    #[serde(rename = "createdBy")]
    pub created_by: OrchestrationV2Actor,
    #[serde(rename = "creationSource")]
    pub creation_source: OrchestrationV2CreationSource,
    #[serde(rename = "id")]
    pub id: ThreadId,
    #[serde(rename = "projectId")]
    pub project_id: ProjectId,
    #[serde(rename = "title")]
    pub title: IsoDateTime,
    #[serde(rename = "providerInstanceId")]
    pub provider_instance_id: ProviderInstanceId,
    #[serde(rename = "modelSelection")]
    pub model_selection: ModelSelection,
    #[serde(rename = "runtimeMode")]
    pub runtime_mode: RuntimeMode,
    #[serde(rename = "interactionMode")]
    pub interaction_mode: ProviderInteractionMode,
    #[serde(rename = "branch", deserialize_with = "required_nullable")]
    pub branch: Option<TrimmedNonEmptyString>,
    #[serde(rename = "worktreePath", deserialize_with = "required_nullable")]
    pub worktree_path: Option<TrimmedNonEmptyString>,
    #[serde(
        rename = "linkedPullRequest",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub linked_pull_request: Optional<Option<ThreadLinkedPullRequest>>,
    #[serde(
        rename = "pullRequests",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub pull_requests: Optional<Vec<ThreadPullRequestLink>>,
    #[serde(
        rename = "branchPullRequest",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub branch_pull_request: Optional<Option<ThreadLinkedPullRequest>>,
    #[serde(rename = "lineage")]
    pub lineage: OrchestrationV2AppThreadLineage,
    #[serde(rename = "forkedFrom", deserialize_with = "required_nullable")]
    pub forked_from: Option<OrchestrationV2ThreadShellForkedFrom>,
    #[serde(
        rename = "activeProviderThreadId",
        deserialize_with = "required_nullable"
    )]
    pub active_provider_thread_id: Option<ProviderThreadId>,
    #[serde(
        rename = "historyOrigin",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub history_origin: Optional<OrchestrationV2ThreadHistoryOrigin>,
    #[serde(rename = "latestRunId", deserialize_with = "required_nullable")]
    pub latest_run_id: Option<RunId>,
    #[serde(
        rename = "latestRunRequestedAt",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub latest_run_requested_at: Optional<Option<String>>,
    #[serde(
        rename = "latestRunStartedAt",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub latest_run_started_at: Optional<Option<String>>,
    #[serde(
        rename = "latestRunCompletedAt",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub latest_run_completed_at: Optional<Option<String>>,
    #[serde(rename = "activeRunId", deserialize_with = "required_nullable")]
    pub active_run_id: Option<RunId>,
    #[serde(
        rename = "activityRunStartedAt",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub activity_run_started_at: Optional<Option<String>>,
    #[serde(
        rename = "activityRunStatus",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub activity_run_status: Optional<Option<OrchestrationV2ThreadShellActivityRunStatus>>,
    #[serde(rename = "status")]
    pub status: OrchestrationV2ShellThreadStatus,
    #[serde(
        rename = "lastError",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub last_error: Optional<Option<IsoDateTime>>,
    #[serde(
        rename = "lastErrorClass",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub last_error_class: Optional<Option<OrchestrationV2ProviderFailureClass>>,
    #[serde(
        rename = "usageLimitResetAt",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub usage_limit_reset_at: Optional<Option<IsoDateTime>>,
    #[serde(
        rename = "pendingRuntimeRequest",
        deserialize_with = "required_nullable"
    )]
    pub pending_runtime_request: Option<OrchestrationV2PendingRuntimeRequestSummary>,
    #[serde(
        rename = "latestVisibleMessage",
        deserialize_with = "required_nullable"
    )]
    pub latest_visible_message: Option<OrchestrationV2LatestVisibleMessageSummary>,
    #[serde(rename = "latestUserMessageAt", deserialize_with = "required_nullable")]
    pub latest_user_message_at: Option<String>,
    #[serde(rename = "hasActionableProposedPlan")]
    pub has_actionable_proposed_plan: bool,
    #[serde(
        rename = "pendingBackgroundTasks",
        default = "orchestration_v2_thread_shell_pending_background_tasks_default"
    )]
    pub pending_background_tasks: Vec<OrchestrationV2PendingBackgroundTask>,
    #[serde(
        rename = "providerInstanceHistory",
        default = "orchestration_v2_thread_shell_provider_instance_history_default"
    )]
    pub provider_instance_history: Vec<ProviderInstanceId>,
    #[serde(rename = "itemCount")]
    pub item_count: NonNegativeInt,
    #[serde(rename = "visibleItemCount")]
    pub visible_item_count: NonNegativeInt,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "archivedAt", deserialize_with = "required_nullable")]
    pub archived_at: Option<String>,
    #[serde(rename = "settledOverride", deserialize_with = "required_nullable")]
    pub settled_override: Option<OrchestrationV2ThreadShellSettledOverride>,
    #[serde(rename = "settledAt", deserialize_with = "required_nullable")]
    pub settled_at: Option<String>,
    #[serde(
        rename = "unsettledAt",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub unsettled_at: Optional<Option<String>>,
    #[serde(
        rename = "snoozedUntil",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub snoozed_until: Optional<Option<String>>,
    #[serde(
        rename = "snoozedAt",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub snoozed_at: Optional<Option<String>>,
    #[serde(
        rename = "limitRecovery",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub limit_recovery: Optional<Option<OrchestrationV2LimitRecovery>>,
    #[serde(
        rename = "pinnedAt",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub pinned_at: Optional<Option<String>>,
    #[serde(
        rename = "autoSettleDisabledAt",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub auto_settle_disabled_at: Optional<Option<String>>,
    #[serde(
        rename = "pinOrderKey",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub pin_order_key: Optional<Option<TrimmedNonEmptyString>>,
    #[serde(
        rename = "activeOrderKey",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub active_order_key: Optional<Option<TrimmedNonEmptyString>>,
    #[serde(
        rename = "lastVisitedAt",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub last_visited_at: Optional<Option<String>>,
    #[serde(
        rename = "titleRegeneration",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub title_regeneration: Optional<Option<OrchestrationV2ThreadShellTitleRegeneration>>,
    #[serde(rename = "deletedAt", deserialize_with = "required_nullable")]
    pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ThreadShellJsonForkedFromRun {
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId")]
    pub run_id: RunId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ThreadShellJsonForkedFromNode {
    #[serde(rename = "nodeId")]
    pub node_id: NodeId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ThreadShellJsonForkedFromProviderThread {
    #[serde(rename = "providerThreadId")]
    pub provider_thread_id: ProviderThreadId,
    #[serde(
        rename = "providerTurnId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_turn_id: Optional<ProviderTurnId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum OrchestrationV2ThreadShellJsonForkedFrom {
    #[serde(rename = "run")]
    Run(Box<OrchestrationV2ThreadShellJsonForkedFromRun>),
    #[serde(rename = "node")]
    Node(Box<OrchestrationV2ThreadShellJsonForkedFromNode>),
    #[serde(rename = "provider_thread")]
    ProviderThread(Box<OrchestrationV2ThreadShellJsonForkedFromProviderThread>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2ThreadShellJsonActivityRunStatus {
    #[serde(rename = "preparing")]
    Preparing,
    #[serde(rename = "starting")]
    Starting,
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "waiting")]
    Waiting,
}

fn orchestration_v2_thread_shell_json_pending_background_tasks_default()
-> Vec<OrchestrationV2PendingBackgroundTask> {
    serde_json::from_str("[]").expect("upstream default")
}

fn orchestration_v2_thread_shell_json_provider_instance_history_default() -> Vec<ProviderInstanceId>
{
    serde_json::from_str("[]").expect("upstream default")
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2ThreadShellJsonSettledOverride {
    #[serde(rename = "settled")]
    Settled,
    #[serde(rename = "active")]
    Active,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ThreadShellJsonTitleRegeneration {
    #[serde(rename = "requestId")]
    pub request_id: CommandId,
    #[serde(rename = "startedAt")]
    pub started_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ThreadShellJson {
    #[serde(rename = "createdBy")]
    pub created_by: OrchestrationV2Actor,
    #[serde(rename = "creationSource")]
    pub creation_source: OrchestrationV2CreationSource,
    #[serde(rename = "id")]
    pub id: ThreadId,
    #[serde(rename = "projectId")]
    pub project_id: ProjectId,
    #[serde(rename = "title")]
    pub title: IsoDateTime,
    #[serde(rename = "providerInstanceId")]
    pub provider_instance_id: ProviderInstanceId,
    #[serde(rename = "modelSelection")]
    pub model_selection: ModelSelection,
    #[serde(rename = "runtimeMode")]
    pub runtime_mode: RuntimeMode,
    #[serde(rename = "interactionMode")]
    pub interaction_mode: ProviderInteractionMode,
    #[serde(rename = "branch", deserialize_with = "required_nullable")]
    pub branch: Option<TrimmedNonEmptyString>,
    #[serde(rename = "worktreePath", deserialize_with = "required_nullable")]
    pub worktree_path: Option<TrimmedNonEmptyString>,
    #[serde(
        rename = "linkedPullRequest",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub linked_pull_request: Optional<Option<ThreadLinkedPullRequest>>,
    #[serde(
        rename = "pullRequests",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub pull_requests: Optional<Vec<ThreadPullRequestLink>>,
    #[serde(
        rename = "branchPullRequest",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub branch_pull_request: Optional<Option<ThreadLinkedPullRequest>>,
    #[serde(rename = "lineage")]
    pub lineage: OrchestrationV2AppThreadLineage,
    #[serde(rename = "forkedFrom", deserialize_with = "required_nullable")]
    pub forked_from: Option<OrchestrationV2ThreadShellJsonForkedFrom>,
    #[serde(
        rename = "activeProviderThreadId",
        deserialize_with = "required_nullable"
    )]
    pub active_provider_thread_id: Option<ProviderThreadId>,
    #[serde(
        rename = "historyOrigin",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub history_origin: Optional<OrchestrationV2ThreadHistoryOrigin>,
    #[serde(rename = "latestRunId", deserialize_with = "required_nullable")]
    pub latest_run_id: Option<RunId>,
    #[serde(
        rename = "latestRunRequestedAt",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub latest_run_requested_at: Optional<Option<String>>,
    #[serde(
        rename = "latestRunStartedAt",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub latest_run_started_at: Optional<Option<String>>,
    #[serde(
        rename = "latestRunCompletedAt",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub latest_run_completed_at: Optional<Option<String>>,
    #[serde(rename = "activeRunId", deserialize_with = "required_nullable")]
    pub active_run_id: Option<RunId>,
    #[serde(
        rename = "activityRunStartedAt",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub activity_run_started_at: Optional<Option<String>>,
    #[serde(
        rename = "activityRunStatus",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub activity_run_status: Optional<Option<OrchestrationV2ThreadShellJsonActivityRunStatus>>,
    #[serde(rename = "status")]
    pub status: OrchestrationV2ShellThreadStatus,
    #[serde(
        rename = "lastError",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub last_error: Optional<Option<IsoDateTime>>,
    #[serde(
        rename = "lastErrorClass",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub last_error_class: Optional<Option<OrchestrationV2ProviderFailureClass>>,
    #[serde(
        rename = "usageLimitResetAt",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub usage_limit_reset_at: Optional<Option<IsoDateTime>>,
    #[serde(
        rename = "pendingRuntimeRequest",
        deserialize_with = "required_nullable"
    )]
    pub pending_runtime_request: Option<OrchestrationV2PendingRuntimeRequestSummaryJson>,
    #[serde(
        rename = "latestVisibleMessage",
        deserialize_with = "required_nullable"
    )]
    pub latest_visible_message: Option<OrchestrationV2LatestVisibleMessageSummaryJson>,
    #[serde(rename = "latestUserMessageAt", deserialize_with = "required_nullable")]
    pub latest_user_message_at: Option<String>,
    #[serde(rename = "hasActionableProposedPlan")]
    pub has_actionable_proposed_plan: bool,
    #[serde(
        rename = "pendingBackgroundTasks",
        default = "orchestration_v2_thread_shell_json_pending_background_tasks_default"
    )]
    pub pending_background_tasks: Vec<OrchestrationV2PendingBackgroundTask>,
    #[serde(
        rename = "providerInstanceHistory",
        default = "orchestration_v2_thread_shell_json_provider_instance_history_default"
    )]
    pub provider_instance_history: Vec<ProviderInstanceId>,
    #[serde(rename = "itemCount")]
    pub item_count: NonNegativeInt,
    #[serde(rename = "visibleItemCount")]
    pub visible_item_count: NonNegativeInt,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "archivedAt", deserialize_with = "required_nullable")]
    pub archived_at: Option<String>,
    #[serde(rename = "settledOverride", deserialize_with = "required_nullable")]
    pub settled_override: Option<OrchestrationV2ThreadShellJsonSettledOverride>,
    #[serde(rename = "settledAt", deserialize_with = "required_nullable")]
    pub settled_at: Option<String>,
    #[serde(
        rename = "unsettledAt",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub unsettled_at: Optional<Option<String>>,
    #[serde(
        rename = "snoozedUntil",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub snoozed_until: Optional<Option<String>>,
    #[serde(
        rename = "snoozedAt",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub snoozed_at: Optional<Option<String>>,
    #[serde(
        rename = "limitRecovery",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub limit_recovery: Optional<Option<OrchestrationV2LimitRecovery>>,
    #[serde(
        rename = "pinnedAt",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub pinned_at: Optional<Option<String>>,
    #[serde(
        rename = "autoSettleDisabledAt",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub auto_settle_disabled_at: Optional<Option<String>>,
    #[serde(
        rename = "pinOrderKey",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub pin_order_key: Optional<Option<TrimmedNonEmptyString>>,
    #[serde(
        rename = "activeOrderKey",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub active_order_key: Optional<Option<TrimmedNonEmptyString>>,
    #[serde(
        rename = "lastVisitedAt",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub last_visited_at: Optional<Option<String>>,
    #[serde(
        rename = "titleRegeneration",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub title_regeneration: Optional<Option<OrchestrationV2ThreadShellJsonTitleRegeneration>>,
    #[serde(rename = "deletedAt", deserialize_with = "required_nullable")]
    pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ThreadShellSnapshot {
    #[serde(rename = "schemaVersion")]
    pub schema_version: PositiveInt,
    #[serde(rename = "snapshotSequence")]
    pub snapshot_sequence: NonNegativeInt,
    #[serde(rename = "threads")]
    pub threads: Vec<OrchestrationV2ThreadShell>,
    #[serde(rename = "archivedThreads")]
    pub archived_threads: Vec<OrchestrationV2ThreadShell>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ThreadStreamItemSynchronized {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ThreadStreamItemSnapshot {
    #[serde(rename = "snapshotSequence")]
    pub snapshot_sequence: NonNegativeInt,
    #[serde(rename = "projection")]
    pub projection: OrchestrationV2ThreadProjection,
    #[serde(
        rename = "historyCursor",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub history_cursor: Optional<Option<TrimmedNonEmptyString>>,
    #[serde(
        rename = "hasMoreHistory",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub has_more_history: Optional<bool>,
    #[serde(
        rename = "latestLocalTurnOrdinal",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub latest_local_turn_ordinal: Optional<Option<NonNegativeInt>>,
    #[serde(
        rename = "payloadBudgetExceeded",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub payload_budget_exceeded: Optional<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ThreadStreamItemEvent {
    #[serde(rename = "sequence")]
    pub sequence: NonNegativeInt,
    #[serde(rename = "event")]
    pub event: OrchestrationV2DomainEvent,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ThreadStreamItemUnknownEvent {
    #[serde(rename = "sequence")]
    pub sequence: NonNegativeInt,
    #[serde(rename = "eventType")]
    pub event_type: IsoDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum OrchestrationV2ThreadStreamItem {
    #[serde(rename = "synchronized")]
    Synchronized(Box<OrchestrationV2ThreadStreamItemSynchronized>),
    #[serde(rename = "snapshot")]
    Snapshot(Box<OrchestrationV2ThreadStreamItemSnapshot>),
    #[serde(rename = "event")]
    Event(Box<OrchestrationV2ThreadStreamItemEvent>),
    #[serde(rename = "unknown-event")]
    UnknownEvent(Box<OrchestrationV2ThreadStreamItemUnknownEvent>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2ToolCapabilities {
    #[serde(rename = "exposesToolItemIds")]
    pub exposes_tool_item_ids: bool,
    #[serde(rename = "emitsToolStarted")]
    pub emits_tool_started: bool,
    #[serde(rename = "emitsToolCompleted")]
    pub emits_tool_completed: bool,
    #[serde(rename = "emitsToolOutput")]
    pub emits_tool_output: bool,
    #[serde(rename = "supportsMcpTools")]
    pub supports_mcp_tools: bool,
    #[serde(rename = "supportsDynamicToolCallbacks")]
    pub supports_dynamic_tool_callbacks: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2TurnCapabilitiesTerminalStatusQuality {
    #[serde(rename = "strong")]
    Strong,
    #[serde(rename = "weak")]
    Weak,
    #[serde(rename = "none")]
    None,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2TurnCapabilities {
    #[serde(rename = "exposesNativeTurnId")]
    pub exposes_native_turn_id: bool,
    #[serde(rename = "emitsTurnStarted")]
    pub emits_turn_started: bool,
    #[serde(rename = "emitsTurnCompleted")]
    pub emits_turn_completed: bool,
    #[serde(rename = "supportsInterrupt")]
    pub supports_interrupt: bool,
    #[serde(rename = "supportsActiveSteering")]
    pub supports_active_steering: bool,
    #[serde(rename = "supportsSteeringByInterruptRestart")]
    pub supports_steering_by_interrupt_restart: bool,
    #[serde(rename = "supportsQueuedMessages")]
    pub supports_queued_messages: bool,
    #[serde(rename = "terminalStatusQuality")]
    pub terminal_status_quality: OrchestrationV2TurnCapabilitiesTerminalStatusQuality,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2TurnItemNotificationOutcome {
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "cancelled")]
    Cancelled,
    #[serde(rename = "updated")]
    Updated,
    #[serde(rename = "unknown")]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2TurnItemNotification {
    #[serde(
        rename = "toolSurface",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_surface: Optional<ToolActivitySurface>,
    #[serde(
        rename = "toolIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_icon: Optional<ToolActivityIcon>,
    #[serde(
        rename = "toolSource",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_source: Optional<ToolActivitySource>,
    #[serde(rename = "id")]
    pub id: TurnItemId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "nodeId", deserialize_with = "required_nullable")]
    pub node_id: Option<NodeId>,
    #[serde(rename = "providerThreadId", deserialize_with = "required_nullable")]
    pub provider_thread_id: Option<ProviderThreadId>,
    #[serde(rename = "providerTurnId", deserialize_with = "required_nullable")]
    pub provider_turn_id: Option<ProviderTurnId>,
    #[serde(rename = "nativeItemRef", deserialize_with = "required_nullable")]
    pub native_item_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(rename = "parentItemId", deserialize_with = "required_nullable")]
    pub parent_item_id: Option<TurnItemId>,
    #[serde(rename = "ordinal")]
    pub ordinal: NonNegativeInt,
    #[serde(rename = "status")]
    pub status: OrchestrationV2TurnItemStatus,
    #[serde(rename = "title", deserialize_with = "required_nullable")]
    pub title: Option<IsoDateTime>,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<String>,
    #[serde(rename = "completedAt", deserialize_with = "required_nullable")]
    pub completed_at: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "source")]
    pub source: OrchestrationV2NotificationSource,
    #[serde(rename = "outcome")]
    pub outcome: OrchestrationV2TurnItemNotificationOutcome,
    #[serde(rename = "summary")]
    pub summary: TrimmedNonEmptyString,
    #[serde(
        rename = "detail",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub detail: Optional<IsoDateTime>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2TurnItemUserMessage {
    #[serde(
        rename = "toolSurface",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_surface: Optional<ToolActivitySurface>,
    #[serde(
        rename = "toolIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_icon: Optional<ToolActivityIcon>,
    #[serde(
        rename = "toolSource",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_source: Optional<ToolActivitySource>,
    #[serde(rename = "id")]
    pub id: TurnItemId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "nodeId", deserialize_with = "required_nullable")]
    pub node_id: Option<NodeId>,
    #[serde(rename = "providerThreadId", deserialize_with = "required_nullable")]
    pub provider_thread_id: Option<ProviderThreadId>,
    #[serde(rename = "providerTurnId", deserialize_with = "required_nullable")]
    pub provider_turn_id: Option<ProviderTurnId>,
    #[serde(rename = "nativeItemRef", deserialize_with = "required_nullable")]
    pub native_item_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(rename = "parentItemId", deserialize_with = "required_nullable")]
    pub parent_item_id: Option<TurnItemId>,
    #[serde(rename = "ordinal")]
    pub ordinal: NonNegativeInt,
    #[serde(rename = "status")]
    pub status: OrchestrationV2TurnItemStatus,
    #[serde(rename = "title", deserialize_with = "required_nullable")]
    pub title: Option<IsoDateTime>,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<String>,
    #[serde(rename = "completedAt", deserialize_with = "required_nullable")]
    pub completed_at: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "createdBy")]
    pub created_by: OrchestrationV2Actor,
    #[serde(rename = "creationSource")]
    pub creation_source: OrchestrationV2CreationSource,
    #[serde(rename = "messageId")]
    pub message_id: MessageId,
    #[serde(
        rename = "scheduledTaskId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub scheduled_task_id: Optional<ScheduledTaskId>,
    #[serde(
        rename = "senderThreadId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub sender_thread_id: Optional<ThreadId>,
    #[serde(rename = "inputIntent")]
    pub input_intent: OrchestrationV2UserMessageInputIntent,
    #[serde(rename = "text")]
    pub text: IsoDateTime,
    #[serde(
        rename = "context",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub context: Optional<OrchestrationMessageContext>,
    #[serde(rename = "attachments")]
    pub attachments: Vec<ChatAttachment>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2TurnItemAssistantMessage {
    #[serde(
        rename = "toolSurface",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_surface: Optional<ToolActivitySurface>,
    #[serde(
        rename = "toolIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_icon: Optional<ToolActivityIcon>,
    #[serde(
        rename = "toolSource",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_source: Optional<ToolActivitySource>,
    #[serde(rename = "id")]
    pub id: TurnItemId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "nodeId", deserialize_with = "required_nullable")]
    pub node_id: Option<NodeId>,
    #[serde(rename = "providerThreadId", deserialize_with = "required_nullable")]
    pub provider_thread_id: Option<ProviderThreadId>,
    #[serde(rename = "providerTurnId", deserialize_with = "required_nullable")]
    pub provider_turn_id: Option<ProviderTurnId>,
    #[serde(rename = "nativeItemRef", deserialize_with = "required_nullable")]
    pub native_item_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(rename = "parentItemId", deserialize_with = "required_nullable")]
    pub parent_item_id: Option<TurnItemId>,
    #[serde(rename = "ordinal")]
    pub ordinal: NonNegativeInt,
    #[serde(rename = "status")]
    pub status: OrchestrationV2TurnItemStatus,
    #[serde(rename = "title", deserialize_with = "required_nullable")]
    pub title: Option<IsoDateTime>,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<String>,
    #[serde(rename = "completedAt", deserialize_with = "required_nullable")]
    pub completed_at: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "messageId")]
    pub message_id: MessageId,
    #[serde(rename = "text")]
    pub text: IsoDateTime,
    #[serde(
        rename = "attachments",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub attachments: Optional<Vec<ChatAttachment>>,
    #[serde(rename = "streaming")]
    pub streaming: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2TurnItemReasoning {
    #[serde(
        rename = "toolSurface",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_surface: Optional<ToolActivitySurface>,
    #[serde(
        rename = "toolIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_icon: Optional<ToolActivityIcon>,
    #[serde(
        rename = "toolSource",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_source: Optional<ToolActivitySource>,
    #[serde(rename = "id")]
    pub id: TurnItemId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "nodeId", deserialize_with = "required_nullable")]
    pub node_id: Option<NodeId>,
    #[serde(rename = "providerThreadId", deserialize_with = "required_nullable")]
    pub provider_thread_id: Option<ProviderThreadId>,
    #[serde(rename = "providerTurnId", deserialize_with = "required_nullable")]
    pub provider_turn_id: Option<ProviderTurnId>,
    #[serde(rename = "nativeItemRef", deserialize_with = "required_nullable")]
    pub native_item_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(rename = "parentItemId", deserialize_with = "required_nullable")]
    pub parent_item_id: Option<TurnItemId>,
    #[serde(rename = "ordinal")]
    pub ordinal: NonNegativeInt,
    #[serde(rename = "status")]
    pub status: OrchestrationV2TurnItemStatus,
    #[serde(rename = "title", deserialize_with = "required_nullable")]
    pub title: Option<IsoDateTime>,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<String>,
    #[serde(rename = "completedAt", deserialize_with = "required_nullable")]
    pub completed_at: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "text")]
    pub text: IsoDateTime,
    #[serde(rename = "streaming")]
    pub streaming: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2TurnItemProposedPlan {
    #[serde(
        rename = "toolSurface",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_surface: Optional<ToolActivitySurface>,
    #[serde(
        rename = "toolIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_icon: Optional<ToolActivityIcon>,
    #[serde(
        rename = "toolSource",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_source: Optional<ToolActivitySource>,
    #[serde(rename = "id")]
    pub id: TurnItemId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "nodeId", deserialize_with = "required_nullable")]
    pub node_id: Option<NodeId>,
    #[serde(rename = "providerThreadId", deserialize_with = "required_nullable")]
    pub provider_thread_id: Option<ProviderThreadId>,
    #[serde(rename = "providerTurnId", deserialize_with = "required_nullable")]
    pub provider_turn_id: Option<ProviderTurnId>,
    #[serde(rename = "nativeItemRef", deserialize_with = "required_nullable")]
    pub native_item_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(rename = "parentItemId", deserialize_with = "required_nullable")]
    pub parent_item_id: Option<TurnItemId>,
    #[serde(rename = "ordinal")]
    pub ordinal: NonNegativeInt,
    #[serde(rename = "status")]
    pub status: OrchestrationV2TurnItemStatus,
    #[serde(rename = "title", deserialize_with = "required_nullable")]
    pub title: Option<IsoDateTime>,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<String>,
    #[serde(rename = "completedAt", deserialize_with = "required_nullable")]
    pub completed_at: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "planId")]
    pub plan_id: PlanId,
    #[serde(rename = "markdown")]
    pub markdown: IsoDateTime,
    #[serde(rename = "streaming")]
    pub streaming: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2TurnItemTodoList {
    #[serde(
        rename = "toolSurface",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_surface: Optional<ToolActivitySurface>,
    #[serde(
        rename = "toolIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_icon: Optional<ToolActivityIcon>,
    #[serde(
        rename = "toolSource",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_source: Optional<ToolActivitySource>,
    #[serde(rename = "id")]
    pub id: TurnItemId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "nodeId", deserialize_with = "required_nullable")]
    pub node_id: Option<NodeId>,
    #[serde(rename = "providerThreadId", deserialize_with = "required_nullable")]
    pub provider_thread_id: Option<ProviderThreadId>,
    #[serde(rename = "providerTurnId", deserialize_with = "required_nullable")]
    pub provider_turn_id: Option<ProviderTurnId>,
    #[serde(rename = "nativeItemRef", deserialize_with = "required_nullable")]
    pub native_item_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(rename = "parentItemId", deserialize_with = "required_nullable")]
    pub parent_item_id: Option<TurnItemId>,
    #[serde(rename = "ordinal")]
    pub ordinal: NonNegativeInt,
    #[serde(rename = "status")]
    pub status: OrchestrationV2TurnItemStatus,
    #[serde(rename = "title", deserialize_with = "required_nullable")]
    pub title: Option<IsoDateTime>,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<String>,
    #[serde(rename = "completedAt", deserialize_with = "required_nullable")]
    pub completed_at: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "planId")]
    pub plan_id: PlanId,
    #[serde(rename = "steps")]
    pub steps: Vec<OrchestrationV2PlanStep>,
    #[serde(
        rename = "explanation",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub explanation: Optional<IsoDateTime>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2TurnItemUserInputRequestResponseMode {
    #[serde(rename = "message")]
    Message,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2TurnItemUserInputRequest {
    #[serde(
        rename = "toolSurface",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_surface: Optional<ToolActivitySurface>,
    #[serde(
        rename = "toolIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_icon: Optional<ToolActivityIcon>,
    #[serde(
        rename = "toolSource",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_source: Optional<ToolActivitySource>,
    #[serde(rename = "id")]
    pub id: TurnItemId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "nodeId", deserialize_with = "required_nullable")]
    pub node_id: Option<NodeId>,
    #[serde(rename = "providerThreadId", deserialize_with = "required_nullable")]
    pub provider_thread_id: Option<ProviderThreadId>,
    #[serde(rename = "providerTurnId", deserialize_with = "required_nullable")]
    pub provider_turn_id: Option<ProviderTurnId>,
    #[serde(rename = "nativeItemRef", deserialize_with = "required_nullable")]
    pub native_item_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(rename = "parentItemId", deserialize_with = "required_nullable")]
    pub parent_item_id: Option<TurnItemId>,
    #[serde(rename = "ordinal")]
    pub ordinal: NonNegativeInt,
    #[serde(rename = "status")]
    pub status: OrchestrationV2TurnItemStatus,
    #[serde(rename = "title", deserialize_with = "required_nullable")]
    pub title: Option<IsoDateTime>,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<String>,
    #[serde(rename = "completedAt", deserialize_with = "required_nullable")]
    pub completed_at: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "requestId")]
    pub request_id: RuntimeRequestId,
    #[serde(rename = "questions")]
    pub questions: Vec<OrchestrationV2UserInputQuestion>,
    #[serde(
        rename = "questionAnswer",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub question_answer: Optional<UserInputAttachmentAnswerPayload>,
    #[serde(
        rename = "responseMode",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub response_mode: Optional<OrchestrationV2TurnItemUserInputRequestResponseMode>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2TurnItemFileChange {
    #[serde(
        rename = "toolSurface",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_surface: Optional<ToolActivitySurface>,
    #[serde(
        rename = "toolIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_icon: Optional<ToolActivityIcon>,
    #[serde(
        rename = "toolSource",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_source: Optional<ToolActivitySource>,
    #[serde(rename = "id")]
    pub id: TurnItemId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "nodeId", deserialize_with = "required_nullable")]
    pub node_id: Option<NodeId>,
    #[serde(rename = "providerThreadId", deserialize_with = "required_nullable")]
    pub provider_thread_id: Option<ProviderThreadId>,
    #[serde(rename = "providerTurnId", deserialize_with = "required_nullable")]
    pub provider_turn_id: Option<ProviderTurnId>,
    #[serde(rename = "nativeItemRef", deserialize_with = "required_nullable")]
    pub native_item_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(rename = "parentItemId", deserialize_with = "required_nullable")]
    pub parent_item_id: Option<TurnItemId>,
    #[serde(rename = "ordinal")]
    pub ordinal: NonNegativeInt,
    #[serde(rename = "status")]
    pub status: OrchestrationV2TurnItemStatus,
    #[serde(rename = "title", deserialize_with = "required_nullable")]
    pub title: Option<IsoDateTime>,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<String>,
    #[serde(rename = "completedAt", deserialize_with = "required_nullable")]
    pub completed_at: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "fileName")]
    pub file_name: TrimmedNonEmptyString,
    #[serde(
        rename = "additions",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub additions: Optional<NonNegativeInt>,
    #[serde(
        rename = "deletions",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub deletions: Optional<NonNegativeInt>,
    #[serde(
        rename = "diffStr",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub diff_str: Optional<IsoDateTime>,
    #[serde(
        rename = "oldStr",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub old_str: Optional<IsoDateTime>,
    #[serde(
        rename = "newStr",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub new_str: Optional<IsoDateTime>,
    #[serde(
        rename = "changes",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub changes: Optional<Vec<OrchestrationV2FileChangeDetail>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2TurnItemCommandExecution {
    #[serde(
        rename = "toolSurface",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_surface: Optional<ToolActivitySurface>,
    #[serde(
        rename = "toolIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_icon: Optional<ToolActivityIcon>,
    #[serde(
        rename = "toolSource",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_source: Optional<ToolActivitySource>,
    #[serde(rename = "id")]
    pub id: TurnItemId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "nodeId", deserialize_with = "required_nullable")]
    pub node_id: Option<NodeId>,
    #[serde(rename = "providerThreadId", deserialize_with = "required_nullable")]
    pub provider_thread_id: Option<ProviderThreadId>,
    #[serde(rename = "providerTurnId", deserialize_with = "required_nullable")]
    pub provider_turn_id: Option<ProviderTurnId>,
    #[serde(rename = "nativeItemRef", deserialize_with = "required_nullable")]
    pub native_item_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(rename = "parentItemId", deserialize_with = "required_nullable")]
    pub parent_item_id: Option<TurnItemId>,
    #[serde(rename = "ordinal")]
    pub ordinal: NonNegativeInt,
    #[serde(rename = "status")]
    pub status: OrchestrationV2TurnItemStatus,
    #[serde(rename = "title", deserialize_with = "required_nullable")]
    pub title: Option<IsoDateTime>,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<String>,
    #[serde(rename = "completedAt", deserialize_with = "required_nullable")]
    pub completed_at: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "input")]
    pub input: IsoDateTime,
    #[serde(
        rename = "output",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub output: Optional<IsoDateTime>,
    #[serde(
        rename = "outputIndicatesFailure",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub output_indicates_failure: Optional<bool>,
    #[serde(
        rename = "exitCode",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub exit_code: Optional<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2TurnItemFileSearch {
    #[serde(
        rename = "toolSurface",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_surface: Optional<ToolActivitySurface>,
    #[serde(
        rename = "toolIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_icon: Optional<ToolActivityIcon>,
    #[serde(
        rename = "toolSource",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_source: Optional<ToolActivitySource>,
    #[serde(rename = "id")]
    pub id: TurnItemId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "nodeId", deserialize_with = "required_nullable")]
    pub node_id: Option<NodeId>,
    #[serde(rename = "providerThreadId", deserialize_with = "required_nullable")]
    pub provider_thread_id: Option<ProviderThreadId>,
    #[serde(rename = "providerTurnId", deserialize_with = "required_nullable")]
    pub provider_turn_id: Option<ProviderTurnId>,
    #[serde(rename = "nativeItemRef", deserialize_with = "required_nullable")]
    pub native_item_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(rename = "parentItemId", deserialize_with = "required_nullable")]
    pub parent_item_id: Option<TurnItemId>,
    #[serde(rename = "ordinal")]
    pub ordinal: NonNegativeInt,
    #[serde(rename = "status")]
    pub status: OrchestrationV2TurnItemStatus,
    #[serde(rename = "title", deserialize_with = "required_nullable")]
    pub title: Option<IsoDateTime>,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<String>,
    #[serde(rename = "completedAt", deserialize_with = "required_nullable")]
    pub completed_at: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(
        rename = "pattern",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub pattern: Optional<IsoDateTime>,
    #[serde(
        rename = "results",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub results: Optional<Vec<OrchestrationV2FileSearchResult>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2TurnItemWebSearch {
    #[serde(
        rename = "toolSurface",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_surface: Optional<ToolActivitySurface>,
    #[serde(
        rename = "toolIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_icon: Optional<ToolActivityIcon>,
    #[serde(
        rename = "toolSource",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_source: Optional<ToolActivitySource>,
    #[serde(rename = "id")]
    pub id: TurnItemId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "nodeId", deserialize_with = "required_nullable")]
    pub node_id: Option<NodeId>,
    #[serde(rename = "providerThreadId", deserialize_with = "required_nullable")]
    pub provider_thread_id: Option<ProviderThreadId>,
    #[serde(rename = "providerTurnId", deserialize_with = "required_nullable")]
    pub provider_turn_id: Option<ProviderTurnId>,
    #[serde(rename = "nativeItemRef", deserialize_with = "required_nullable")]
    pub native_item_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(rename = "parentItemId", deserialize_with = "required_nullable")]
    pub parent_item_id: Option<TurnItemId>,
    #[serde(rename = "ordinal")]
    pub ordinal: NonNegativeInt,
    #[serde(rename = "status")]
    pub status: OrchestrationV2TurnItemStatus,
    #[serde(rename = "title", deserialize_with = "required_nullable")]
    pub title: Option<IsoDateTime>,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<String>,
    #[serde(rename = "completedAt", deserialize_with = "required_nullable")]
    pub completed_at: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(
        rename = "patterns",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub patterns: Optional<Vec<IsoDateTime>>,
    #[serde(
        rename = "results",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub results: Optional<Vec<OrchestrationV2WebSearchResult>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2TurnItemApprovalRequest {
    #[serde(
        rename = "toolSurface",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_surface: Optional<ToolActivitySurface>,
    #[serde(
        rename = "toolIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_icon: Optional<ToolActivityIcon>,
    #[serde(
        rename = "toolSource",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_source: Optional<ToolActivitySource>,
    #[serde(rename = "id")]
    pub id: TurnItemId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "nodeId", deserialize_with = "required_nullable")]
    pub node_id: Option<NodeId>,
    #[serde(rename = "providerThreadId", deserialize_with = "required_nullable")]
    pub provider_thread_id: Option<ProviderThreadId>,
    #[serde(rename = "providerTurnId", deserialize_with = "required_nullable")]
    pub provider_turn_id: Option<ProviderTurnId>,
    #[serde(rename = "nativeItemRef", deserialize_with = "required_nullable")]
    pub native_item_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(rename = "parentItemId", deserialize_with = "required_nullable")]
    pub parent_item_id: Option<TurnItemId>,
    #[serde(rename = "ordinal")]
    pub ordinal: NonNegativeInt,
    #[serde(rename = "status")]
    pub status: OrchestrationV2TurnItemStatus,
    #[serde(rename = "title", deserialize_with = "required_nullable")]
    pub title: Option<IsoDateTime>,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<String>,
    #[serde(rename = "completedAt", deserialize_with = "required_nullable")]
    pub completed_at: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "requestId")]
    pub request_id: RuntimeRequestId,
    #[serde(rename = "requestKind")]
    pub request_kind: ProviderRequestKind,
    #[serde(
        rename = "prompt",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub prompt: Optional<IsoDateTime>,
    #[serde(
        rename = "appName",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub app_name: Optional<IsoDateTime>,
    #[serde(
        rename = "options",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub options: Optional<Vec<ProviderApprovalOption>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2TurnItemCheckpoint {
    #[serde(
        rename = "toolSurface",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_surface: Optional<ToolActivitySurface>,
    #[serde(
        rename = "toolIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_icon: Optional<ToolActivityIcon>,
    #[serde(
        rename = "toolSource",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_source: Optional<ToolActivitySource>,
    #[serde(rename = "id")]
    pub id: TurnItemId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "nodeId", deserialize_with = "required_nullable")]
    pub node_id: Option<NodeId>,
    #[serde(rename = "providerThreadId", deserialize_with = "required_nullable")]
    pub provider_thread_id: Option<ProviderThreadId>,
    #[serde(rename = "providerTurnId", deserialize_with = "required_nullable")]
    pub provider_turn_id: Option<ProviderTurnId>,
    #[serde(rename = "nativeItemRef", deserialize_with = "required_nullable")]
    pub native_item_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(rename = "parentItemId", deserialize_with = "required_nullable")]
    pub parent_item_id: Option<TurnItemId>,
    #[serde(rename = "ordinal")]
    pub ordinal: NonNegativeInt,
    #[serde(rename = "status")]
    pub status: OrchestrationV2TurnItemStatus,
    #[serde(rename = "title", deserialize_with = "required_nullable")]
    pub title: Option<IsoDateTime>,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<String>,
    #[serde(rename = "completedAt", deserialize_with = "required_nullable")]
    pub completed_at: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "checkpointId")]
    pub checkpoint_id: CheckpointId,
    #[serde(rename = "scopeId")]
    pub scope_id: CheckpointScopeId,
    #[serde(rename = "files")]
    pub files: Vec<OrchestrationV2CheckpointFileSummary>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2TurnItemRunInterruptRequest {
    #[serde(
        rename = "toolSurface",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_surface: Optional<ToolActivitySurface>,
    #[serde(
        rename = "toolIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_icon: Optional<ToolActivityIcon>,
    #[serde(
        rename = "toolSource",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_source: Optional<ToolActivitySource>,
    #[serde(rename = "id")]
    pub id: TurnItemId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "nodeId", deserialize_with = "required_nullable")]
    pub node_id: Option<NodeId>,
    #[serde(rename = "providerThreadId", deserialize_with = "required_nullable")]
    pub provider_thread_id: Option<ProviderThreadId>,
    #[serde(rename = "providerTurnId", deserialize_with = "required_nullable")]
    pub provider_turn_id: Option<ProviderTurnId>,
    #[serde(rename = "nativeItemRef", deserialize_with = "required_nullable")]
    pub native_item_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(rename = "parentItemId", deserialize_with = "required_nullable")]
    pub parent_item_id: Option<TurnItemId>,
    #[serde(rename = "ordinal")]
    pub ordinal: NonNegativeInt,
    #[serde(rename = "status")]
    pub status: OrchestrationV2TurnItemStatus,
    #[serde(rename = "title", deserialize_with = "required_nullable")]
    pub title: Option<IsoDateTime>,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<String>,
    #[serde(rename = "completedAt", deserialize_with = "required_nullable")]
    pub completed_at: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "message")]
    pub message: IsoDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2TurnItemRunInterruptResult {
    #[serde(
        rename = "toolSurface",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_surface: Optional<ToolActivitySurface>,
    #[serde(
        rename = "toolIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_icon: Optional<ToolActivityIcon>,
    #[serde(
        rename = "toolSource",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_source: Optional<ToolActivitySource>,
    #[serde(rename = "id")]
    pub id: TurnItemId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "nodeId", deserialize_with = "required_nullable")]
    pub node_id: Option<NodeId>,
    #[serde(rename = "providerThreadId", deserialize_with = "required_nullable")]
    pub provider_thread_id: Option<ProviderThreadId>,
    #[serde(rename = "providerTurnId", deserialize_with = "required_nullable")]
    pub provider_turn_id: Option<ProviderTurnId>,
    #[serde(rename = "nativeItemRef", deserialize_with = "required_nullable")]
    pub native_item_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(rename = "parentItemId", deserialize_with = "required_nullable")]
    pub parent_item_id: Option<TurnItemId>,
    #[serde(rename = "ordinal")]
    pub ordinal: NonNegativeInt,
    #[serde(rename = "status")]
    pub status: OrchestrationV2TurnItemStatus,
    #[serde(rename = "title", deserialize_with = "required_nullable")]
    pub title: Option<IsoDateTime>,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<String>,
    #[serde(rename = "completedAt", deserialize_with = "required_nullable")]
    pub completed_at: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "message")]
    pub message: IsoDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2TurnItemSystemNotice {
    #[serde(
        rename = "toolSurface",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_surface: Optional<ToolActivitySurface>,
    #[serde(
        rename = "toolIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_icon: Optional<ToolActivityIcon>,
    #[serde(
        rename = "toolSource",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_source: Optional<ToolActivitySource>,
    #[serde(rename = "id")]
    pub id: TurnItemId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "nodeId", deserialize_with = "required_nullable")]
    pub node_id: Option<NodeId>,
    #[serde(rename = "providerThreadId", deserialize_with = "required_nullable")]
    pub provider_thread_id: Option<ProviderThreadId>,
    #[serde(rename = "providerTurnId", deserialize_with = "required_nullable")]
    pub provider_turn_id: Option<ProviderTurnId>,
    #[serde(rename = "nativeItemRef", deserialize_with = "required_nullable")]
    pub native_item_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(rename = "parentItemId", deserialize_with = "required_nullable")]
    pub parent_item_id: Option<TurnItemId>,
    #[serde(rename = "ordinal")]
    pub ordinal: NonNegativeInt,
    #[serde(rename = "status")]
    pub status: OrchestrationV2TurnItemStatus,
    #[serde(rename = "title", deserialize_with = "required_nullable")]
    pub title: Option<IsoDateTime>,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<String>,
    #[serde(rename = "completedAt", deserialize_with = "required_nullable")]
    pub completed_at: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "message")]
    pub message: IsoDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2TurnItemError {
    #[serde(
        rename = "toolSurface",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_surface: Optional<ToolActivitySurface>,
    #[serde(
        rename = "toolIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_icon: Optional<ToolActivityIcon>,
    #[serde(
        rename = "toolSource",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_source: Optional<ToolActivitySource>,
    #[serde(rename = "id")]
    pub id: TurnItemId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "nodeId", deserialize_with = "required_nullable")]
    pub node_id: Option<NodeId>,
    #[serde(rename = "providerThreadId", deserialize_with = "required_nullable")]
    pub provider_thread_id: Option<ProviderThreadId>,
    #[serde(rename = "providerTurnId", deserialize_with = "required_nullable")]
    pub provider_turn_id: Option<ProviderTurnId>,
    #[serde(rename = "nativeItemRef", deserialize_with = "required_nullable")]
    pub native_item_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(rename = "parentItemId", deserialize_with = "required_nullable")]
    pub parent_item_id: Option<TurnItemId>,
    #[serde(rename = "ordinal")]
    pub ordinal: NonNegativeInt,
    #[serde(rename = "status")]
    pub status: OrchestrationV2TurnItemStatus,
    #[serde(rename = "title", deserialize_with = "required_nullable")]
    pub title: Option<IsoDateTime>,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<String>,
    #[serde(rename = "completedAt", deserialize_with = "required_nullable")]
    pub completed_at: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "failure")]
    pub failure: OrchestrationV2ProviderFailure,
    #[serde(rename = "retry", default, skip_serializing_if = "Optional::is_absent")]
    pub retry: Optional<OrchestrationV2ProviderRetry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2TurnItemCompaction {
    #[serde(
        rename = "toolSurface",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_surface: Optional<ToolActivitySurface>,
    #[serde(
        rename = "toolIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_icon: Optional<ToolActivityIcon>,
    #[serde(
        rename = "toolSource",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_source: Optional<ToolActivitySource>,
    #[serde(rename = "id")]
    pub id: TurnItemId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "nodeId", deserialize_with = "required_nullable")]
    pub node_id: Option<NodeId>,
    #[serde(rename = "providerThreadId", deserialize_with = "required_nullable")]
    pub provider_thread_id: Option<ProviderThreadId>,
    #[serde(rename = "providerTurnId", deserialize_with = "required_nullable")]
    pub provider_turn_id: Option<ProviderTurnId>,
    #[serde(rename = "nativeItemRef", deserialize_with = "required_nullable")]
    pub native_item_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(rename = "parentItemId", deserialize_with = "required_nullable")]
    pub parent_item_id: Option<TurnItemId>,
    #[serde(rename = "ordinal")]
    pub ordinal: NonNegativeInt,
    #[serde(rename = "status")]
    pub status: OrchestrationV2TurnItemStatus,
    #[serde(rename = "title", deserialize_with = "required_nullable")]
    pub title: Option<IsoDateTime>,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<String>,
    #[serde(rename = "completedAt", deserialize_with = "required_nullable")]
    pub completed_at: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "driver", deserialize_with = "required_nullable")]
    pub driver: Option<ProviderDriverKind>,
    #[serde(
        rename = "summary",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub summary: Optional<IsoDateTime>,
    #[serde(
        rename = "beforeTokenCount",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub before_token_count: Optional<NonNegativeInt>,
    #[serde(
        rename = "afterTokenCount",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub after_token_count: Optional<NonNegativeInt>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2TurnItemHandoffStrategy {
    #[serde(rename = "delta_since_target_last_seen")]
    DeltaSinceTargetLastSeen,
    #[serde(rename = "fork_delta_summary")]
    ForkDeltaSummary,
    #[serde(rename = "full_thread_summary")]
    FullThreadSummary,
    #[serde(rename = "checkpoint_summary")]
    CheckpointSummary,
    #[serde(rename = "manual_context")]
    ManualContext,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2TurnItemHandoff {
    #[serde(
        rename = "toolSurface",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_surface: Optional<ToolActivitySurface>,
    #[serde(
        rename = "toolIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_icon: Optional<ToolActivityIcon>,
    #[serde(
        rename = "toolSource",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_source: Optional<ToolActivitySource>,
    #[serde(rename = "id")]
    pub id: TurnItemId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "nodeId", deserialize_with = "required_nullable")]
    pub node_id: Option<NodeId>,
    #[serde(rename = "providerThreadId", deserialize_with = "required_nullable")]
    pub provider_thread_id: Option<ProviderThreadId>,
    #[serde(rename = "providerTurnId", deserialize_with = "required_nullable")]
    pub provider_turn_id: Option<ProviderTurnId>,
    #[serde(rename = "nativeItemRef", deserialize_with = "required_nullable")]
    pub native_item_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(rename = "parentItemId", deserialize_with = "required_nullable")]
    pub parent_item_id: Option<TurnItemId>,
    #[serde(rename = "ordinal")]
    pub ordinal: NonNegativeInt,
    #[serde(rename = "status")]
    pub status: OrchestrationV2TurnItemStatus,
    #[serde(rename = "title", deserialize_with = "required_nullable")]
    pub title: Option<IsoDateTime>,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<String>,
    #[serde(rename = "completedAt", deserialize_with = "required_nullable")]
    pub completed_at: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "contextHandoffId")]
    pub context_handoff_id: ContextHandoffId,
    #[serde(rename = "fromProviderThreadIds")]
    pub from_provider_thread_ids: Vec<ProviderThreadId>,
    #[serde(rename = "toProviderThreadId")]
    pub to_provider_thread_id: ProviderThreadId,
    #[serde(rename = "fromProviderInstanceIds")]
    pub from_provider_instance_ids: Vec<ProviderInstanceId>,
    #[serde(rename = "toProviderInstanceId")]
    pub to_provider_instance_id: ProviderInstanceId,
    #[serde(
        rename = "fromModelSelections",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub from_model_selections: Optional<Vec<ModelSelection>>,
    #[serde(
        rename = "toModel",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub to_model: Optional<IsoDateTime>,
    #[serde(rename = "strategy")]
    pub strategy: OrchestrationV2TurnItemHandoffStrategy,
    #[serde(
        rename = "summary",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub summary: Optional<IsoDateTime>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2TurnItemForkSourceRun {
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId")]
    pub run_id: RunId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2TurnItemForkSourceNode {
    #[serde(rename = "nodeId")]
    pub node_id: NodeId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2TurnItemForkSourceProviderThread {
    #[serde(rename = "providerThreadId")]
    pub provider_thread_id: ProviderThreadId,
    #[serde(
        rename = "providerTurnId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_turn_id: Optional<ProviderTurnId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum OrchestrationV2TurnItemForkSource {
    #[serde(rename = "run")]
    Run(Box<OrchestrationV2TurnItemForkSourceRun>),
    #[serde(rename = "node")]
    Node(Box<OrchestrationV2TurnItemForkSourceNode>),
    #[serde(rename = "provider_thread")]
    ProviderThread(Box<OrchestrationV2TurnItemForkSourceProviderThread>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2TurnItemFork {
    #[serde(
        rename = "toolSurface",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_surface: Optional<ToolActivitySurface>,
    #[serde(
        rename = "toolIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_icon: Optional<ToolActivityIcon>,
    #[serde(
        rename = "toolSource",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_source: Optional<ToolActivitySource>,
    #[serde(rename = "id")]
    pub id: TurnItemId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "nodeId", deserialize_with = "required_nullable")]
    pub node_id: Option<NodeId>,
    #[serde(
        rename = "providerThreadId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_thread_id: Optional<ProviderThreadId>,
    #[serde(rename = "providerTurnId", deserialize_with = "required_nullable")]
    pub provider_turn_id: Option<ProviderTurnId>,
    #[serde(rename = "nativeItemRef", deserialize_with = "required_nullable")]
    pub native_item_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(rename = "parentItemId", deserialize_with = "required_nullable")]
    pub parent_item_id: Option<TurnItemId>,
    #[serde(rename = "ordinal")]
    pub ordinal: NonNegativeInt,
    #[serde(rename = "status")]
    pub status: OrchestrationV2TurnItemStatus,
    #[serde(rename = "title", deserialize_with = "required_nullable")]
    pub title: Option<IsoDateTime>,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<String>,
    #[serde(rename = "completedAt", deserialize_with = "required_nullable")]
    pub completed_at: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "source")]
    pub source: OrchestrationV2TurnItemForkSource,
    #[serde(rename = "targetThreadId")]
    pub target_thread_id: ThreadId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2TurnItemThreadCreated {
    #[serde(
        rename = "toolSurface",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_surface: Optional<ToolActivitySurface>,
    #[serde(
        rename = "toolIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_icon: Optional<ToolActivityIcon>,
    #[serde(
        rename = "toolSource",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_source: Optional<ToolActivitySource>,
    #[serde(rename = "id")]
    pub id: TurnItemId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "nodeId", deserialize_with = "required_nullable")]
    pub node_id: Option<NodeId>,
    #[serde(rename = "providerThreadId", deserialize_with = "required_nullable")]
    pub provider_thread_id: Option<ProviderThreadId>,
    #[serde(rename = "providerTurnId", deserialize_with = "required_nullable")]
    pub provider_turn_id: Option<ProviderTurnId>,
    #[serde(rename = "nativeItemRef", deserialize_with = "required_nullable")]
    pub native_item_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(rename = "parentItemId", deserialize_with = "required_nullable")]
    pub parent_item_id: Option<TurnItemId>,
    #[serde(rename = "ordinal")]
    pub ordinal: NonNegativeInt,
    #[serde(rename = "status")]
    pub status: OrchestrationV2TurnItemStatus,
    #[serde(rename = "title", deserialize_with = "required_nullable")]
    pub title: Option<IsoDateTime>,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<String>,
    #[serde(rename = "completedAt", deserialize_with = "required_nullable")]
    pub completed_at: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "targetThreadId")]
    pub target_thread_id: ThreadId,
    #[serde(rename = "targetRunId", deserialize_with = "required_nullable")]
    pub target_run_id: Option<RunId>,
    #[serde(rename = "targetProviderInstanceId")]
    pub target_provider_instance_id: ProviderInstanceId,
    #[serde(rename = "targetModel")]
    pub target_model: TrimmedNonEmptyString,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2TurnItemSubagentOrigin {
    #[serde(rename = "provider_native")]
    ProviderNative,
    #[serde(rename = "app_owned")]
    AppOwned,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2TurnItemSubagent {
    #[serde(
        rename = "toolSurface",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_surface: Optional<ToolActivitySurface>,
    #[serde(
        rename = "toolIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_icon: Optional<ToolActivityIcon>,
    #[serde(
        rename = "toolSource",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_source: Optional<ToolActivitySource>,
    #[serde(rename = "id")]
    pub id: TurnItemId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "nodeId", deserialize_with = "required_nullable")]
    pub node_id: Option<NodeId>,
    #[serde(rename = "providerThreadId", deserialize_with = "required_nullable")]
    pub provider_thread_id: Option<ProviderThreadId>,
    #[serde(rename = "providerTurnId", deserialize_with = "required_nullable")]
    pub provider_turn_id: Option<ProviderTurnId>,
    #[serde(rename = "nativeItemRef", deserialize_with = "required_nullable")]
    pub native_item_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(rename = "parentItemId", deserialize_with = "required_nullable")]
    pub parent_item_id: Option<TurnItemId>,
    #[serde(rename = "ordinal")]
    pub ordinal: NonNegativeInt,
    #[serde(rename = "status")]
    pub status: OrchestrationV2TurnItemStatus,
    #[serde(rename = "title", deserialize_with = "required_nullable")]
    pub title: Option<IsoDateTime>,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<String>,
    #[serde(rename = "completedAt", deserialize_with = "required_nullable")]
    pub completed_at: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "subagentId")]
    pub subagent_id: NodeId,
    #[serde(rename = "origin")]
    pub origin: OrchestrationV2TurnItemSubagentOrigin,
    #[serde(rename = "driver")]
    pub driver: ProviderDriverKind,
    #[serde(rename = "providerInstanceId")]
    pub provider_instance_id: ProviderInstanceId,
    #[serde(rename = "childThreadId", deserialize_with = "required_nullable")]
    pub child_thread_id: Option<ThreadId>,
    #[serde(rename = "prompt")]
    pub prompt: IsoDateTime,
    #[serde(
        rename = "progress",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub progress: Optional<IsoDateTime>,
    #[serde(rename = "result", deserialize_with = "required_nullable")]
    pub result: Option<IsoDateTime>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2TurnItemDynamicTool {
    #[serde(
        rename = "toolSurface",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_surface: Optional<ToolActivitySurface>,
    #[serde(
        rename = "toolIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_icon: Optional<ToolActivityIcon>,
    #[serde(
        rename = "toolSource",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_source: Optional<ToolActivitySource>,
    #[serde(rename = "id")]
    pub id: TurnItemId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "nodeId", deserialize_with = "required_nullable")]
    pub node_id: Option<NodeId>,
    #[serde(rename = "providerThreadId", deserialize_with = "required_nullable")]
    pub provider_thread_id: Option<ProviderThreadId>,
    #[serde(rename = "providerTurnId", deserialize_with = "required_nullable")]
    pub provider_turn_id: Option<ProviderTurnId>,
    #[serde(rename = "nativeItemRef", deserialize_with = "required_nullable")]
    pub native_item_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(rename = "parentItemId", deserialize_with = "required_nullable")]
    pub parent_item_id: Option<TurnItemId>,
    #[serde(rename = "ordinal")]
    pub ordinal: NonNegativeInt,
    #[serde(rename = "status")]
    pub status: OrchestrationV2TurnItemStatus,
    #[serde(rename = "title", deserialize_with = "required_nullable")]
    pub title: Option<IsoDateTime>,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<String>,
    #[serde(rename = "completedAt", deserialize_with = "required_nullable")]
    pub completed_at: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "toolName", deserialize_with = "required_nullable")]
    pub tool_name: Option<TrimmedNonEmptyString>,
    #[serde(
        rename = "viewedImagePath",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub viewed_image_path: Optional<TrimmedNonEmptyString>,
    #[serde(rename = "input")]
    pub input: serde_json::Value,
    #[serde(
        rename = "output",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub output: Optional<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum OrchestrationV2TurnItem {
    #[serde(rename = "notification")]
    Notification(Box<OrchestrationV2TurnItemNotification>),
    #[serde(rename = "user_message")]
    UserMessage(Box<OrchestrationV2TurnItemUserMessage>),
    #[serde(rename = "assistant_message")]
    AssistantMessage(Box<OrchestrationV2TurnItemAssistantMessage>),
    #[serde(rename = "reasoning")]
    Reasoning(Box<OrchestrationV2TurnItemReasoning>),
    #[serde(rename = "proposed_plan")]
    ProposedPlan(Box<OrchestrationV2TurnItemProposedPlan>),
    #[serde(rename = "todo_list")]
    TodoList(Box<OrchestrationV2TurnItemTodoList>),
    #[serde(rename = "user_input_request")]
    UserInputRequest(Box<OrchestrationV2TurnItemUserInputRequest>),
    #[serde(rename = "file_change")]
    FileChange(Box<OrchestrationV2TurnItemFileChange>),
    #[serde(rename = "command_execution")]
    CommandExecution(Box<OrchestrationV2TurnItemCommandExecution>),
    #[serde(rename = "file_search")]
    FileSearch(Box<OrchestrationV2TurnItemFileSearch>),
    #[serde(rename = "web_search")]
    WebSearch(Box<OrchestrationV2TurnItemWebSearch>),
    #[serde(rename = "approval_request")]
    ApprovalRequest(Box<OrchestrationV2TurnItemApprovalRequest>),
    #[serde(rename = "checkpoint")]
    Checkpoint(Box<OrchestrationV2TurnItemCheckpoint>),
    #[serde(rename = "run_interrupt_request")]
    RunInterruptRequest(Box<OrchestrationV2TurnItemRunInterruptRequest>),
    #[serde(rename = "run_interrupt_result")]
    RunInterruptResult(Box<OrchestrationV2TurnItemRunInterruptResult>),
    #[serde(rename = "system_notice")]
    SystemNotice(Box<OrchestrationV2TurnItemSystemNotice>),
    #[serde(rename = "error")]
    Error(Box<OrchestrationV2TurnItemError>),
    #[serde(rename = "compaction")]
    Compaction(Box<OrchestrationV2TurnItemCompaction>),
    #[serde(rename = "handoff")]
    Handoff(Box<OrchestrationV2TurnItemHandoff>),
    #[serde(rename = "fork")]
    Fork(Box<OrchestrationV2TurnItemFork>),
    #[serde(rename = "thread_created")]
    ThreadCreated(Box<OrchestrationV2TurnItemThreadCreated>),
    #[serde(rename = "subagent")]
    Subagent(Box<OrchestrationV2TurnItemSubagent>),
    #[serde(rename = "dynamic_tool")]
    DynamicTool(Box<OrchestrationV2TurnItemDynamicTool>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2TurnItemJsonNotificationOutcome {
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "cancelled")]
    Cancelled,
    #[serde(rename = "updated")]
    Updated,
    #[serde(rename = "unknown")]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2TurnItemJsonNotification {
    #[serde(
        rename = "toolSurface",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_surface: Optional<ToolActivitySurface>,
    #[serde(
        rename = "toolIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_icon: Optional<ToolActivityIcon>,
    #[serde(
        rename = "toolSource",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_source: Optional<ToolActivitySource>,
    #[serde(rename = "id")]
    pub id: TurnItemId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "nodeId", deserialize_with = "required_nullable")]
    pub node_id: Option<NodeId>,
    #[serde(rename = "providerThreadId", deserialize_with = "required_nullable")]
    pub provider_thread_id: Option<ProviderThreadId>,
    #[serde(rename = "providerTurnId", deserialize_with = "required_nullable")]
    pub provider_turn_id: Option<ProviderTurnId>,
    #[serde(rename = "nativeItemRef", deserialize_with = "required_nullable")]
    pub native_item_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(rename = "parentItemId", deserialize_with = "required_nullable")]
    pub parent_item_id: Option<TurnItemId>,
    #[serde(rename = "ordinal")]
    pub ordinal: NonNegativeInt,
    #[serde(rename = "status")]
    pub status: OrchestrationV2TurnItemStatus,
    #[serde(rename = "title", deserialize_with = "required_nullable")]
    pub title: Option<IsoDateTime>,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<String>,
    #[serde(rename = "completedAt", deserialize_with = "required_nullable")]
    pub completed_at: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "source")]
    pub source: OrchestrationV2NotificationSource,
    #[serde(rename = "outcome")]
    pub outcome: OrchestrationV2TurnItemJsonNotificationOutcome,
    #[serde(rename = "summary")]
    pub summary: TrimmedNonEmptyString,
    #[serde(
        rename = "detail",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub detail: Optional<IsoDateTime>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2TurnItemJsonUserMessage {
    #[serde(
        rename = "toolSurface",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_surface: Optional<ToolActivitySurface>,
    #[serde(
        rename = "toolIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_icon: Optional<ToolActivityIcon>,
    #[serde(
        rename = "toolSource",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_source: Optional<ToolActivitySource>,
    #[serde(rename = "id")]
    pub id: TurnItemId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "nodeId", deserialize_with = "required_nullable")]
    pub node_id: Option<NodeId>,
    #[serde(rename = "providerThreadId", deserialize_with = "required_nullable")]
    pub provider_thread_id: Option<ProviderThreadId>,
    #[serde(rename = "providerTurnId", deserialize_with = "required_nullable")]
    pub provider_turn_id: Option<ProviderTurnId>,
    #[serde(rename = "nativeItemRef", deserialize_with = "required_nullable")]
    pub native_item_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(rename = "parentItemId", deserialize_with = "required_nullable")]
    pub parent_item_id: Option<TurnItemId>,
    #[serde(rename = "ordinal")]
    pub ordinal: NonNegativeInt,
    #[serde(rename = "status")]
    pub status: OrchestrationV2TurnItemStatus,
    #[serde(rename = "title", deserialize_with = "required_nullable")]
    pub title: Option<IsoDateTime>,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<String>,
    #[serde(rename = "completedAt", deserialize_with = "required_nullable")]
    pub completed_at: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "createdBy")]
    pub created_by: OrchestrationV2Actor,
    #[serde(rename = "creationSource")]
    pub creation_source: OrchestrationV2CreationSource,
    #[serde(rename = "messageId")]
    pub message_id: MessageId,
    #[serde(
        rename = "scheduledTaskId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub scheduled_task_id: Optional<ScheduledTaskId>,
    #[serde(
        rename = "senderThreadId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub sender_thread_id: Optional<ThreadId>,
    #[serde(rename = "inputIntent")]
    pub input_intent: OrchestrationV2UserMessageInputIntent,
    #[serde(rename = "text")]
    pub text: IsoDateTime,
    #[serde(
        rename = "context",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub context: Optional<OrchestrationMessageContext>,
    #[serde(rename = "attachments")]
    pub attachments: Vec<ChatAttachment>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2TurnItemJsonAssistantMessage {
    #[serde(
        rename = "toolSurface",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_surface: Optional<ToolActivitySurface>,
    #[serde(
        rename = "toolIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_icon: Optional<ToolActivityIcon>,
    #[serde(
        rename = "toolSource",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_source: Optional<ToolActivitySource>,
    #[serde(rename = "id")]
    pub id: TurnItemId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "nodeId", deserialize_with = "required_nullable")]
    pub node_id: Option<NodeId>,
    #[serde(rename = "providerThreadId", deserialize_with = "required_nullable")]
    pub provider_thread_id: Option<ProviderThreadId>,
    #[serde(rename = "providerTurnId", deserialize_with = "required_nullable")]
    pub provider_turn_id: Option<ProviderTurnId>,
    #[serde(rename = "nativeItemRef", deserialize_with = "required_nullable")]
    pub native_item_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(rename = "parentItemId", deserialize_with = "required_nullable")]
    pub parent_item_id: Option<TurnItemId>,
    #[serde(rename = "ordinal")]
    pub ordinal: NonNegativeInt,
    #[serde(rename = "status")]
    pub status: OrchestrationV2TurnItemStatus,
    #[serde(rename = "title", deserialize_with = "required_nullable")]
    pub title: Option<IsoDateTime>,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<String>,
    #[serde(rename = "completedAt", deserialize_with = "required_nullable")]
    pub completed_at: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "messageId")]
    pub message_id: MessageId,
    #[serde(rename = "text")]
    pub text: IsoDateTime,
    #[serde(
        rename = "attachments",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub attachments: Optional<Vec<ChatAttachment>>,
    #[serde(rename = "streaming")]
    pub streaming: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2TurnItemJsonReasoning {
    #[serde(
        rename = "toolSurface",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_surface: Optional<ToolActivitySurface>,
    #[serde(
        rename = "toolIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_icon: Optional<ToolActivityIcon>,
    #[serde(
        rename = "toolSource",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_source: Optional<ToolActivitySource>,
    #[serde(rename = "id")]
    pub id: TurnItemId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "nodeId", deserialize_with = "required_nullable")]
    pub node_id: Option<NodeId>,
    #[serde(rename = "providerThreadId", deserialize_with = "required_nullable")]
    pub provider_thread_id: Option<ProviderThreadId>,
    #[serde(rename = "providerTurnId", deserialize_with = "required_nullable")]
    pub provider_turn_id: Option<ProviderTurnId>,
    #[serde(rename = "nativeItemRef", deserialize_with = "required_nullable")]
    pub native_item_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(rename = "parentItemId", deserialize_with = "required_nullable")]
    pub parent_item_id: Option<TurnItemId>,
    #[serde(rename = "ordinal")]
    pub ordinal: NonNegativeInt,
    #[serde(rename = "status")]
    pub status: OrchestrationV2TurnItemStatus,
    #[serde(rename = "title", deserialize_with = "required_nullable")]
    pub title: Option<IsoDateTime>,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<String>,
    #[serde(rename = "completedAt", deserialize_with = "required_nullable")]
    pub completed_at: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "text")]
    pub text: IsoDateTime,
    #[serde(rename = "streaming")]
    pub streaming: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2TurnItemJsonProposedPlan {
    #[serde(
        rename = "toolSurface",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_surface: Optional<ToolActivitySurface>,
    #[serde(
        rename = "toolIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_icon: Optional<ToolActivityIcon>,
    #[serde(
        rename = "toolSource",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_source: Optional<ToolActivitySource>,
    #[serde(rename = "id")]
    pub id: TurnItemId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "nodeId", deserialize_with = "required_nullable")]
    pub node_id: Option<NodeId>,
    #[serde(rename = "providerThreadId", deserialize_with = "required_nullable")]
    pub provider_thread_id: Option<ProviderThreadId>,
    #[serde(rename = "providerTurnId", deserialize_with = "required_nullable")]
    pub provider_turn_id: Option<ProviderTurnId>,
    #[serde(rename = "nativeItemRef", deserialize_with = "required_nullable")]
    pub native_item_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(rename = "parentItemId", deserialize_with = "required_nullable")]
    pub parent_item_id: Option<TurnItemId>,
    #[serde(rename = "ordinal")]
    pub ordinal: NonNegativeInt,
    #[serde(rename = "status")]
    pub status: OrchestrationV2TurnItemStatus,
    #[serde(rename = "title", deserialize_with = "required_nullable")]
    pub title: Option<IsoDateTime>,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<String>,
    #[serde(rename = "completedAt", deserialize_with = "required_nullable")]
    pub completed_at: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "planId")]
    pub plan_id: PlanId,
    #[serde(rename = "markdown")]
    pub markdown: IsoDateTime,
    #[serde(rename = "streaming")]
    pub streaming: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2TurnItemJsonTodoList {
    #[serde(
        rename = "toolSurface",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_surface: Optional<ToolActivitySurface>,
    #[serde(
        rename = "toolIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_icon: Optional<ToolActivityIcon>,
    #[serde(
        rename = "toolSource",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_source: Optional<ToolActivitySource>,
    #[serde(rename = "id")]
    pub id: TurnItemId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "nodeId", deserialize_with = "required_nullable")]
    pub node_id: Option<NodeId>,
    #[serde(rename = "providerThreadId", deserialize_with = "required_nullable")]
    pub provider_thread_id: Option<ProviderThreadId>,
    #[serde(rename = "providerTurnId", deserialize_with = "required_nullable")]
    pub provider_turn_id: Option<ProviderTurnId>,
    #[serde(rename = "nativeItemRef", deserialize_with = "required_nullable")]
    pub native_item_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(rename = "parentItemId", deserialize_with = "required_nullable")]
    pub parent_item_id: Option<TurnItemId>,
    #[serde(rename = "ordinal")]
    pub ordinal: NonNegativeInt,
    #[serde(rename = "status")]
    pub status: OrchestrationV2TurnItemStatus,
    #[serde(rename = "title", deserialize_with = "required_nullable")]
    pub title: Option<IsoDateTime>,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<String>,
    #[serde(rename = "completedAt", deserialize_with = "required_nullable")]
    pub completed_at: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "planId")]
    pub plan_id: PlanId,
    #[serde(rename = "steps")]
    pub steps: Vec<OrchestrationV2PlanStep>,
    #[serde(
        rename = "explanation",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub explanation: Optional<IsoDateTime>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2TurnItemJsonUserInputRequestResponseMode {
    #[serde(rename = "message")]
    Message,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2TurnItemJsonUserInputRequest {
    #[serde(
        rename = "toolSurface",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_surface: Optional<ToolActivitySurface>,
    #[serde(
        rename = "toolIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_icon: Optional<ToolActivityIcon>,
    #[serde(
        rename = "toolSource",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_source: Optional<ToolActivitySource>,
    #[serde(rename = "id")]
    pub id: TurnItemId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "nodeId", deserialize_with = "required_nullable")]
    pub node_id: Option<NodeId>,
    #[serde(rename = "providerThreadId", deserialize_with = "required_nullable")]
    pub provider_thread_id: Option<ProviderThreadId>,
    #[serde(rename = "providerTurnId", deserialize_with = "required_nullable")]
    pub provider_turn_id: Option<ProviderTurnId>,
    #[serde(rename = "nativeItemRef", deserialize_with = "required_nullable")]
    pub native_item_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(rename = "parentItemId", deserialize_with = "required_nullable")]
    pub parent_item_id: Option<TurnItemId>,
    #[serde(rename = "ordinal")]
    pub ordinal: NonNegativeInt,
    #[serde(rename = "status")]
    pub status: OrchestrationV2TurnItemStatus,
    #[serde(rename = "title", deserialize_with = "required_nullable")]
    pub title: Option<IsoDateTime>,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<String>,
    #[serde(rename = "completedAt", deserialize_with = "required_nullable")]
    pub completed_at: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "requestId")]
    pub request_id: RuntimeRequestId,
    #[serde(rename = "questions")]
    pub questions: Vec<OrchestrationV2UserInputQuestion>,
    #[serde(
        rename = "questionAnswer",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub question_answer: Optional<UserInputAttachmentAnswerPayload>,
    #[serde(
        rename = "responseMode",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub response_mode: Optional<OrchestrationV2TurnItemJsonUserInputRequestResponseMode>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2TurnItemJsonFileChange {
    #[serde(
        rename = "toolSurface",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_surface: Optional<ToolActivitySurface>,
    #[serde(
        rename = "toolIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_icon: Optional<ToolActivityIcon>,
    #[serde(
        rename = "toolSource",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_source: Optional<ToolActivitySource>,
    #[serde(rename = "id")]
    pub id: TurnItemId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "nodeId", deserialize_with = "required_nullable")]
    pub node_id: Option<NodeId>,
    #[serde(rename = "providerThreadId", deserialize_with = "required_nullable")]
    pub provider_thread_id: Option<ProviderThreadId>,
    #[serde(rename = "providerTurnId", deserialize_with = "required_nullable")]
    pub provider_turn_id: Option<ProviderTurnId>,
    #[serde(rename = "nativeItemRef", deserialize_with = "required_nullable")]
    pub native_item_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(rename = "parentItemId", deserialize_with = "required_nullable")]
    pub parent_item_id: Option<TurnItemId>,
    #[serde(rename = "ordinal")]
    pub ordinal: NonNegativeInt,
    #[serde(rename = "status")]
    pub status: OrchestrationV2TurnItemStatus,
    #[serde(rename = "title", deserialize_with = "required_nullable")]
    pub title: Option<IsoDateTime>,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<String>,
    #[serde(rename = "completedAt", deserialize_with = "required_nullable")]
    pub completed_at: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "fileName")]
    pub file_name: TrimmedNonEmptyString,
    #[serde(
        rename = "additions",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub additions: Optional<NonNegativeInt>,
    #[serde(
        rename = "deletions",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub deletions: Optional<NonNegativeInt>,
    #[serde(
        rename = "diffStr",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub diff_str: Optional<IsoDateTime>,
    #[serde(
        rename = "oldStr",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub old_str: Optional<IsoDateTime>,
    #[serde(
        rename = "newStr",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub new_str: Optional<IsoDateTime>,
    #[serde(
        rename = "changes",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub changes: Optional<Vec<OrchestrationV2FileChangeDetail>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2TurnItemJsonCommandExecution {
    #[serde(
        rename = "toolSurface",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_surface: Optional<ToolActivitySurface>,
    #[serde(
        rename = "toolIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_icon: Optional<ToolActivityIcon>,
    #[serde(
        rename = "toolSource",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_source: Optional<ToolActivitySource>,
    #[serde(rename = "id")]
    pub id: TurnItemId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "nodeId", deserialize_with = "required_nullable")]
    pub node_id: Option<NodeId>,
    #[serde(rename = "providerThreadId", deserialize_with = "required_nullable")]
    pub provider_thread_id: Option<ProviderThreadId>,
    #[serde(rename = "providerTurnId", deserialize_with = "required_nullable")]
    pub provider_turn_id: Option<ProviderTurnId>,
    #[serde(rename = "nativeItemRef", deserialize_with = "required_nullable")]
    pub native_item_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(rename = "parentItemId", deserialize_with = "required_nullable")]
    pub parent_item_id: Option<TurnItemId>,
    #[serde(rename = "ordinal")]
    pub ordinal: NonNegativeInt,
    #[serde(rename = "status")]
    pub status: OrchestrationV2TurnItemStatus,
    #[serde(rename = "title", deserialize_with = "required_nullable")]
    pub title: Option<IsoDateTime>,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<String>,
    #[serde(rename = "completedAt", deserialize_with = "required_nullable")]
    pub completed_at: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "input")]
    pub input: IsoDateTime,
    #[serde(
        rename = "output",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub output: Optional<IsoDateTime>,
    #[serde(
        rename = "outputIndicatesFailure",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub output_indicates_failure: Optional<bool>,
    #[serde(
        rename = "exitCode",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub exit_code: Optional<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2TurnItemJsonFileSearch {
    #[serde(
        rename = "toolSurface",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_surface: Optional<ToolActivitySurface>,
    #[serde(
        rename = "toolIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_icon: Optional<ToolActivityIcon>,
    #[serde(
        rename = "toolSource",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_source: Optional<ToolActivitySource>,
    #[serde(rename = "id")]
    pub id: TurnItemId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "nodeId", deserialize_with = "required_nullable")]
    pub node_id: Option<NodeId>,
    #[serde(rename = "providerThreadId", deserialize_with = "required_nullable")]
    pub provider_thread_id: Option<ProviderThreadId>,
    #[serde(rename = "providerTurnId", deserialize_with = "required_nullable")]
    pub provider_turn_id: Option<ProviderTurnId>,
    #[serde(rename = "nativeItemRef", deserialize_with = "required_nullable")]
    pub native_item_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(rename = "parentItemId", deserialize_with = "required_nullable")]
    pub parent_item_id: Option<TurnItemId>,
    #[serde(rename = "ordinal")]
    pub ordinal: NonNegativeInt,
    #[serde(rename = "status")]
    pub status: OrchestrationV2TurnItemStatus,
    #[serde(rename = "title", deserialize_with = "required_nullable")]
    pub title: Option<IsoDateTime>,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<String>,
    #[serde(rename = "completedAt", deserialize_with = "required_nullable")]
    pub completed_at: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(
        rename = "pattern",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub pattern: Optional<IsoDateTime>,
    #[serde(
        rename = "results",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub results: Optional<Vec<OrchestrationV2FileSearchResult>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2TurnItemJsonWebSearch {
    #[serde(
        rename = "toolSurface",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_surface: Optional<ToolActivitySurface>,
    #[serde(
        rename = "toolIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_icon: Optional<ToolActivityIcon>,
    #[serde(
        rename = "toolSource",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_source: Optional<ToolActivitySource>,
    #[serde(rename = "id")]
    pub id: TurnItemId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "nodeId", deserialize_with = "required_nullable")]
    pub node_id: Option<NodeId>,
    #[serde(rename = "providerThreadId", deserialize_with = "required_nullable")]
    pub provider_thread_id: Option<ProviderThreadId>,
    #[serde(rename = "providerTurnId", deserialize_with = "required_nullable")]
    pub provider_turn_id: Option<ProviderTurnId>,
    #[serde(rename = "nativeItemRef", deserialize_with = "required_nullable")]
    pub native_item_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(rename = "parentItemId", deserialize_with = "required_nullable")]
    pub parent_item_id: Option<TurnItemId>,
    #[serde(rename = "ordinal")]
    pub ordinal: NonNegativeInt,
    #[serde(rename = "status")]
    pub status: OrchestrationV2TurnItemStatus,
    #[serde(rename = "title", deserialize_with = "required_nullable")]
    pub title: Option<IsoDateTime>,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<String>,
    #[serde(rename = "completedAt", deserialize_with = "required_nullable")]
    pub completed_at: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(
        rename = "patterns",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub patterns: Optional<Vec<IsoDateTime>>,
    #[serde(
        rename = "results",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub results: Optional<Vec<OrchestrationV2WebSearchResult>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2TurnItemJsonApprovalRequest {
    #[serde(
        rename = "toolSurface",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_surface: Optional<ToolActivitySurface>,
    #[serde(
        rename = "toolIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_icon: Optional<ToolActivityIcon>,
    #[serde(
        rename = "toolSource",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_source: Optional<ToolActivitySource>,
    #[serde(rename = "id")]
    pub id: TurnItemId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "nodeId", deserialize_with = "required_nullable")]
    pub node_id: Option<NodeId>,
    #[serde(rename = "providerThreadId", deserialize_with = "required_nullable")]
    pub provider_thread_id: Option<ProviderThreadId>,
    #[serde(rename = "providerTurnId", deserialize_with = "required_nullable")]
    pub provider_turn_id: Option<ProviderTurnId>,
    #[serde(rename = "nativeItemRef", deserialize_with = "required_nullable")]
    pub native_item_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(rename = "parentItemId", deserialize_with = "required_nullable")]
    pub parent_item_id: Option<TurnItemId>,
    #[serde(rename = "ordinal")]
    pub ordinal: NonNegativeInt,
    #[serde(rename = "status")]
    pub status: OrchestrationV2TurnItemStatus,
    #[serde(rename = "title", deserialize_with = "required_nullable")]
    pub title: Option<IsoDateTime>,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<String>,
    #[serde(rename = "completedAt", deserialize_with = "required_nullable")]
    pub completed_at: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "requestId")]
    pub request_id: RuntimeRequestId,
    #[serde(rename = "requestKind")]
    pub request_kind: ProviderRequestKind,
    #[serde(
        rename = "prompt",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub prompt: Optional<IsoDateTime>,
    #[serde(
        rename = "appName",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub app_name: Optional<IsoDateTime>,
    #[serde(
        rename = "options",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub options: Optional<Vec<ProviderApprovalOption>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2TurnItemJsonCheckpoint {
    #[serde(
        rename = "toolSurface",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_surface: Optional<ToolActivitySurface>,
    #[serde(
        rename = "toolIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_icon: Optional<ToolActivityIcon>,
    #[serde(
        rename = "toolSource",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_source: Optional<ToolActivitySource>,
    #[serde(rename = "id")]
    pub id: TurnItemId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "nodeId", deserialize_with = "required_nullable")]
    pub node_id: Option<NodeId>,
    #[serde(rename = "providerThreadId", deserialize_with = "required_nullable")]
    pub provider_thread_id: Option<ProviderThreadId>,
    #[serde(rename = "providerTurnId", deserialize_with = "required_nullable")]
    pub provider_turn_id: Option<ProviderTurnId>,
    #[serde(rename = "nativeItemRef", deserialize_with = "required_nullable")]
    pub native_item_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(rename = "parentItemId", deserialize_with = "required_nullable")]
    pub parent_item_id: Option<TurnItemId>,
    #[serde(rename = "ordinal")]
    pub ordinal: NonNegativeInt,
    #[serde(rename = "status")]
    pub status: OrchestrationV2TurnItemStatus,
    #[serde(rename = "title", deserialize_with = "required_nullable")]
    pub title: Option<IsoDateTime>,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<String>,
    #[serde(rename = "completedAt", deserialize_with = "required_nullable")]
    pub completed_at: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "checkpointId")]
    pub checkpoint_id: CheckpointId,
    #[serde(rename = "scopeId")]
    pub scope_id: CheckpointScopeId,
    #[serde(rename = "files")]
    pub files: Vec<OrchestrationV2CheckpointFileSummary>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2TurnItemJsonRunInterruptRequest {
    #[serde(
        rename = "toolSurface",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_surface: Optional<ToolActivitySurface>,
    #[serde(
        rename = "toolIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_icon: Optional<ToolActivityIcon>,
    #[serde(
        rename = "toolSource",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_source: Optional<ToolActivitySource>,
    #[serde(rename = "id")]
    pub id: TurnItemId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "nodeId", deserialize_with = "required_nullable")]
    pub node_id: Option<NodeId>,
    #[serde(rename = "providerThreadId", deserialize_with = "required_nullable")]
    pub provider_thread_id: Option<ProviderThreadId>,
    #[serde(rename = "providerTurnId", deserialize_with = "required_nullable")]
    pub provider_turn_id: Option<ProviderTurnId>,
    #[serde(rename = "nativeItemRef", deserialize_with = "required_nullable")]
    pub native_item_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(rename = "parentItemId", deserialize_with = "required_nullable")]
    pub parent_item_id: Option<TurnItemId>,
    #[serde(rename = "ordinal")]
    pub ordinal: NonNegativeInt,
    #[serde(rename = "status")]
    pub status: OrchestrationV2TurnItemStatus,
    #[serde(rename = "title", deserialize_with = "required_nullable")]
    pub title: Option<IsoDateTime>,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<String>,
    #[serde(rename = "completedAt", deserialize_with = "required_nullable")]
    pub completed_at: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "message")]
    pub message: IsoDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2TurnItemJsonRunInterruptResult {
    #[serde(
        rename = "toolSurface",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_surface: Optional<ToolActivitySurface>,
    #[serde(
        rename = "toolIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_icon: Optional<ToolActivityIcon>,
    #[serde(
        rename = "toolSource",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_source: Optional<ToolActivitySource>,
    #[serde(rename = "id")]
    pub id: TurnItemId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "nodeId", deserialize_with = "required_nullable")]
    pub node_id: Option<NodeId>,
    #[serde(rename = "providerThreadId", deserialize_with = "required_nullable")]
    pub provider_thread_id: Option<ProviderThreadId>,
    #[serde(rename = "providerTurnId", deserialize_with = "required_nullable")]
    pub provider_turn_id: Option<ProviderTurnId>,
    #[serde(rename = "nativeItemRef", deserialize_with = "required_nullable")]
    pub native_item_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(rename = "parentItemId", deserialize_with = "required_nullable")]
    pub parent_item_id: Option<TurnItemId>,
    #[serde(rename = "ordinal")]
    pub ordinal: NonNegativeInt,
    #[serde(rename = "status")]
    pub status: OrchestrationV2TurnItemStatus,
    #[serde(rename = "title", deserialize_with = "required_nullable")]
    pub title: Option<IsoDateTime>,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<String>,
    #[serde(rename = "completedAt", deserialize_with = "required_nullable")]
    pub completed_at: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "message")]
    pub message: IsoDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2TurnItemJsonSystemNotice {
    #[serde(
        rename = "toolSurface",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_surface: Optional<ToolActivitySurface>,
    #[serde(
        rename = "toolIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_icon: Optional<ToolActivityIcon>,
    #[serde(
        rename = "toolSource",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_source: Optional<ToolActivitySource>,
    #[serde(rename = "id")]
    pub id: TurnItemId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "nodeId", deserialize_with = "required_nullable")]
    pub node_id: Option<NodeId>,
    #[serde(rename = "providerThreadId", deserialize_with = "required_nullable")]
    pub provider_thread_id: Option<ProviderThreadId>,
    #[serde(rename = "providerTurnId", deserialize_with = "required_nullable")]
    pub provider_turn_id: Option<ProviderTurnId>,
    #[serde(rename = "nativeItemRef", deserialize_with = "required_nullable")]
    pub native_item_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(rename = "parentItemId", deserialize_with = "required_nullable")]
    pub parent_item_id: Option<TurnItemId>,
    #[serde(rename = "ordinal")]
    pub ordinal: NonNegativeInt,
    #[serde(rename = "status")]
    pub status: OrchestrationV2TurnItemStatus,
    #[serde(rename = "title", deserialize_with = "required_nullable")]
    pub title: Option<IsoDateTime>,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<String>,
    #[serde(rename = "completedAt", deserialize_with = "required_nullable")]
    pub completed_at: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "message")]
    pub message: IsoDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2TurnItemJsonError {
    #[serde(
        rename = "toolSurface",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_surface: Optional<ToolActivitySurface>,
    #[serde(
        rename = "toolIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_icon: Optional<ToolActivityIcon>,
    #[serde(
        rename = "toolSource",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_source: Optional<ToolActivitySource>,
    #[serde(rename = "id")]
    pub id: TurnItemId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "nodeId", deserialize_with = "required_nullable")]
    pub node_id: Option<NodeId>,
    #[serde(rename = "providerThreadId", deserialize_with = "required_nullable")]
    pub provider_thread_id: Option<ProviderThreadId>,
    #[serde(rename = "providerTurnId", deserialize_with = "required_nullable")]
    pub provider_turn_id: Option<ProviderTurnId>,
    #[serde(rename = "nativeItemRef", deserialize_with = "required_nullable")]
    pub native_item_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(rename = "parentItemId", deserialize_with = "required_nullable")]
    pub parent_item_id: Option<TurnItemId>,
    #[serde(rename = "ordinal")]
    pub ordinal: NonNegativeInt,
    #[serde(rename = "status")]
    pub status: OrchestrationV2TurnItemStatus,
    #[serde(rename = "title", deserialize_with = "required_nullable")]
    pub title: Option<IsoDateTime>,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<String>,
    #[serde(rename = "completedAt", deserialize_with = "required_nullable")]
    pub completed_at: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "failure")]
    pub failure: OrchestrationV2ProviderFailure,
    #[serde(rename = "retry", default, skip_serializing_if = "Optional::is_absent")]
    pub retry: Optional<OrchestrationV2ProviderRetry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2TurnItemJsonCompaction {
    #[serde(
        rename = "toolSurface",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_surface: Optional<ToolActivitySurface>,
    #[serde(
        rename = "toolIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_icon: Optional<ToolActivityIcon>,
    #[serde(
        rename = "toolSource",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_source: Optional<ToolActivitySource>,
    #[serde(rename = "id")]
    pub id: TurnItemId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "nodeId", deserialize_with = "required_nullable")]
    pub node_id: Option<NodeId>,
    #[serde(rename = "providerThreadId", deserialize_with = "required_nullable")]
    pub provider_thread_id: Option<ProviderThreadId>,
    #[serde(rename = "providerTurnId", deserialize_with = "required_nullable")]
    pub provider_turn_id: Option<ProviderTurnId>,
    #[serde(rename = "nativeItemRef", deserialize_with = "required_nullable")]
    pub native_item_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(rename = "parentItemId", deserialize_with = "required_nullable")]
    pub parent_item_id: Option<TurnItemId>,
    #[serde(rename = "ordinal")]
    pub ordinal: NonNegativeInt,
    #[serde(rename = "status")]
    pub status: OrchestrationV2TurnItemStatus,
    #[serde(rename = "title", deserialize_with = "required_nullable")]
    pub title: Option<IsoDateTime>,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<String>,
    #[serde(rename = "completedAt", deserialize_with = "required_nullable")]
    pub completed_at: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "driver", deserialize_with = "required_nullable")]
    pub driver: Option<ProviderDriverKind>,
    #[serde(
        rename = "summary",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub summary: Optional<IsoDateTime>,
    #[serde(
        rename = "beforeTokenCount",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub before_token_count: Optional<NonNegativeInt>,
    #[serde(
        rename = "afterTokenCount",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub after_token_count: Optional<NonNegativeInt>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2TurnItemJsonHandoffStrategy {
    #[serde(rename = "delta_since_target_last_seen")]
    DeltaSinceTargetLastSeen,
    #[serde(rename = "fork_delta_summary")]
    ForkDeltaSummary,
    #[serde(rename = "full_thread_summary")]
    FullThreadSummary,
    #[serde(rename = "checkpoint_summary")]
    CheckpointSummary,
    #[serde(rename = "manual_context")]
    ManualContext,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2TurnItemJsonHandoff {
    #[serde(
        rename = "toolSurface",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_surface: Optional<ToolActivitySurface>,
    #[serde(
        rename = "toolIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_icon: Optional<ToolActivityIcon>,
    #[serde(
        rename = "toolSource",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_source: Optional<ToolActivitySource>,
    #[serde(rename = "id")]
    pub id: TurnItemId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "nodeId", deserialize_with = "required_nullable")]
    pub node_id: Option<NodeId>,
    #[serde(rename = "providerThreadId", deserialize_with = "required_nullable")]
    pub provider_thread_id: Option<ProviderThreadId>,
    #[serde(rename = "providerTurnId", deserialize_with = "required_nullable")]
    pub provider_turn_id: Option<ProviderTurnId>,
    #[serde(rename = "nativeItemRef", deserialize_with = "required_nullable")]
    pub native_item_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(rename = "parentItemId", deserialize_with = "required_nullable")]
    pub parent_item_id: Option<TurnItemId>,
    #[serde(rename = "ordinal")]
    pub ordinal: NonNegativeInt,
    #[serde(rename = "status")]
    pub status: OrchestrationV2TurnItemStatus,
    #[serde(rename = "title", deserialize_with = "required_nullable")]
    pub title: Option<IsoDateTime>,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<String>,
    #[serde(rename = "completedAt", deserialize_with = "required_nullable")]
    pub completed_at: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "contextHandoffId")]
    pub context_handoff_id: ContextHandoffId,
    #[serde(rename = "fromProviderThreadIds")]
    pub from_provider_thread_ids: Vec<ProviderThreadId>,
    #[serde(rename = "toProviderThreadId")]
    pub to_provider_thread_id: ProviderThreadId,
    #[serde(rename = "fromProviderInstanceIds")]
    pub from_provider_instance_ids: Vec<ProviderInstanceId>,
    #[serde(rename = "toProviderInstanceId")]
    pub to_provider_instance_id: ProviderInstanceId,
    #[serde(
        rename = "fromModelSelections",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub from_model_selections: Optional<Vec<ModelSelection>>,
    #[serde(
        rename = "toModel",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub to_model: Optional<IsoDateTime>,
    #[serde(rename = "strategy")]
    pub strategy: OrchestrationV2TurnItemJsonHandoffStrategy,
    #[serde(
        rename = "summary",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub summary: Optional<IsoDateTime>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2TurnItemJsonForkSourceRun {
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId")]
    pub run_id: RunId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2TurnItemJsonForkSourceNode {
    #[serde(rename = "nodeId")]
    pub node_id: NodeId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2TurnItemJsonForkSourceProviderThread {
    #[serde(rename = "providerThreadId")]
    pub provider_thread_id: ProviderThreadId,
    #[serde(
        rename = "providerTurnId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_turn_id: Optional<ProviderTurnId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum OrchestrationV2TurnItemJsonForkSource {
    #[serde(rename = "run")]
    Run(Box<OrchestrationV2TurnItemJsonForkSourceRun>),
    #[serde(rename = "node")]
    Node(Box<OrchestrationV2TurnItemJsonForkSourceNode>),
    #[serde(rename = "provider_thread")]
    ProviderThread(Box<OrchestrationV2TurnItemJsonForkSourceProviderThread>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2TurnItemJsonFork {
    #[serde(
        rename = "toolSurface",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_surface: Optional<ToolActivitySurface>,
    #[serde(
        rename = "toolIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_icon: Optional<ToolActivityIcon>,
    #[serde(
        rename = "toolSource",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_source: Optional<ToolActivitySource>,
    #[serde(rename = "id")]
    pub id: TurnItemId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "nodeId", deserialize_with = "required_nullable")]
    pub node_id: Option<NodeId>,
    #[serde(
        rename = "providerThreadId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_thread_id: Optional<ProviderThreadId>,
    #[serde(rename = "providerTurnId", deserialize_with = "required_nullable")]
    pub provider_turn_id: Option<ProviderTurnId>,
    #[serde(rename = "nativeItemRef", deserialize_with = "required_nullable")]
    pub native_item_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(rename = "parentItemId", deserialize_with = "required_nullable")]
    pub parent_item_id: Option<TurnItemId>,
    #[serde(rename = "ordinal")]
    pub ordinal: NonNegativeInt,
    #[serde(rename = "status")]
    pub status: OrchestrationV2TurnItemStatus,
    #[serde(rename = "title", deserialize_with = "required_nullable")]
    pub title: Option<IsoDateTime>,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<String>,
    #[serde(rename = "completedAt", deserialize_with = "required_nullable")]
    pub completed_at: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "source")]
    pub source: OrchestrationV2TurnItemJsonForkSource,
    #[serde(rename = "targetThreadId")]
    pub target_thread_id: ThreadId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2TurnItemJsonThreadCreated {
    #[serde(
        rename = "toolSurface",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_surface: Optional<ToolActivitySurface>,
    #[serde(
        rename = "toolIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_icon: Optional<ToolActivityIcon>,
    #[serde(
        rename = "toolSource",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_source: Optional<ToolActivitySource>,
    #[serde(rename = "id")]
    pub id: TurnItemId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "nodeId", deserialize_with = "required_nullable")]
    pub node_id: Option<NodeId>,
    #[serde(rename = "providerThreadId", deserialize_with = "required_nullable")]
    pub provider_thread_id: Option<ProviderThreadId>,
    #[serde(rename = "providerTurnId", deserialize_with = "required_nullable")]
    pub provider_turn_id: Option<ProviderTurnId>,
    #[serde(rename = "nativeItemRef", deserialize_with = "required_nullable")]
    pub native_item_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(rename = "parentItemId", deserialize_with = "required_nullable")]
    pub parent_item_id: Option<TurnItemId>,
    #[serde(rename = "ordinal")]
    pub ordinal: NonNegativeInt,
    #[serde(rename = "status")]
    pub status: OrchestrationV2TurnItemStatus,
    #[serde(rename = "title", deserialize_with = "required_nullable")]
    pub title: Option<IsoDateTime>,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<String>,
    #[serde(rename = "completedAt", deserialize_with = "required_nullable")]
    pub completed_at: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "targetThreadId")]
    pub target_thread_id: ThreadId,
    #[serde(rename = "targetRunId", deserialize_with = "required_nullable")]
    pub target_run_id: Option<RunId>,
    #[serde(rename = "targetProviderInstanceId")]
    pub target_provider_instance_id: ProviderInstanceId,
    #[serde(rename = "targetModel")]
    pub target_model: TrimmedNonEmptyString,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2TurnItemJsonSubagentOrigin {
    #[serde(rename = "provider_native")]
    ProviderNative,
    #[serde(rename = "app_owned")]
    AppOwned,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2TurnItemJsonSubagent {
    #[serde(
        rename = "toolSurface",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_surface: Optional<ToolActivitySurface>,
    #[serde(
        rename = "toolIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_icon: Optional<ToolActivityIcon>,
    #[serde(
        rename = "toolSource",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_source: Optional<ToolActivitySource>,
    #[serde(rename = "id")]
    pub id: TurnItemId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "nodeId", deserialize_with = "required_nullable")]
    pub node_id: Option<NodeId>,
    #[serde(rename = "providerThreadId", deserialize_with = "required_nullable")]
    pub provider_thread_id: Option<ProviderThreadId>,
    #[serde(rename = "providerTurnId", deserialize_with = "required_nullable")]
    pub provider_turn_id: Option<ProviderTurnId>,
    #[serde(rename = "nativeItemRef", deserialize_with = "required_nullable")]
    pub native_item_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(rename = "parentItemId", deserialize_with = "required_nullable")]
    pub parent_item_id: Option<TurnItemId>,
    #[serde(rename = "ordinal")]
    pub ordinal: NonNegativeInt,
    #[serde(rename = "status")]
    pub status: OrchestrationV2TurnItemStatus,
    #[serde(rename = "title", deserialize_with = "required_nullable")]
    pub title: Option<IsoDateTime>,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<String>,
    #[serde(rename = "completedAt", deserialize_with = "required_nullable")]
    pub completed_at: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "subagentId")]
    pub subagent_id: NodeId,
    #[serde(rename = "origin")]
    pub origin: OrchestrationV2TurnItemJsonSubagentOrigin,
    #[serde(rename = "driver")]
    pub driver: ProviderDriverKind,
    #[serde(rename = "providerInstanceId")]
    pub provider_instance_id: ProviderInstanceId,
    #[serde(rename = "childThreadId", deserialize_with = "required_nullable")]
    pub child_thread_id: Option<ThreadId>,
    #[serde(rename = "prompt")]
    pub prompt: IsoDateTime,
    #[serde(
        rename = "progress",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub progress: Optional<IsoDateTime>,
    #[serde(rename = "result", deserialize_with = "required_nullable")]
    pub result: Option<IsoDateTime>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2TurnItemJsonDynamicTool {
    #[serde(
        rename = "toolSurface",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_surface: Optional<ToolActivitySurface>,
    #[serde(
        rename = "toolIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_icon: Optional<ToolActivityIcon>,
    #[serde(
        rename = "toolSource",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_source: Optional<ToolActivitySource>,
    #[serde(rename = "id")]
    pub id: TurnItemId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "nodeId", deserialize_with = "required_nullable")]
    pub node_id: Option<NodeId>,
    #[serde(rename = "providerThreadId", deserialize_with = "required_nullable")]
    pub provider_thread_id: Option<ProviderThreadId>,
    #[serde(rename = "providerTurnId", deserialize_with = "required_nullable")]
    pub provider_turn_id: Option<ProviderTurnId>,
    #[serde(rename = "nativeItemRef", deserialize_with = "required_nullable")]
    pub native_item_ref: Option<OrchestrationV2ProviderRef>,
    #[serde(rename = "parentItemId", deserialize_with = "required_nullable")]
    pub parent_item_id: Option<TurnItemId>,
    #[serde(rename = "ordinal")]
    pub ordinal: NonNegativeInt,
    #[serde(rename = "status")]
    pub status: OrchestrationV2TurnItemStatus,
    #[serde(rename = "title", deserialize_with = "required_nullable")]
    pub title: Option<IsoDateTime>,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<String>,
    #[serde(rename = "completedAt", deserialize_with = "required_nullable")]
    pub completed_at: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "toolName", deserialize_with = "required_nullable")]
    pub tool_name: Option<TrimmedNonEmptyString>,
    #[serde(
        rename = "viewedImagePath",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub viewed_image_path: Optional<TrimmedNonEmptyString>,
    #[serde(rename = "input")]
    pub input: serde_json::Value,
    #[serde(
        rename = "output",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub output: Optional<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum OrchestrationV2TurnItemJson {
    #[serde(rename = "notification")]
    Notification(Box<OrchestrationV2TurnItemJsonNotification>),
    #[serde(rename = "user_message")]
    UserMessage(Box<OrchestrationV2TurnItemJsonUserMessage>),
    #[serde(rename = "assistant_message")]
    AssistantMessage(Box<OrchestrationV2TurnItemJsonAssistantMessage>),
    #[serde(rename = "reasoning")]
    Reasoning(Box<OrchestrationV2TurnItemJsonReasoning>),
    #[serde(rename = "proposed_plan")]
    ProposedPlan(Box<OrchestrationV2TurnItemJsonProposedPlan>),
    #[serde(rename = "todo_list")]
    TodoList(Box<OrchestrationV2TurnItemJsonTodoList>),
    #[serde(rename = "user_input_request")]
    UserInputRequest(Box<OrchestrationV2TurnItemJsonUserInputRequest>),
    #[serde(rename = "file_change")]
    FileChange(Box<OrchestrationV2TurnItemJsonFileChange>),
    #[serde(rename = "command_execution")]
    CommandExecution(Box<OrchestrationV2TurnItemJsonCommandExecution>),
    #[serde(rename = "file_search")]
    FileSearch(Box<OrchestrationV2TurnItemJsonFileSearch>),
    #[serde(rename = "web_search")]
    WebSearch(Box<OrchestrationV2TurnItemJsonWebSearch>),
    #[serde(rename = "approval_request")]
    ApprovalRequest(Box<OrchestrationV2TurnItemJsonApprovalRequest>),
    #[serde(rename = "checkpoint")]
    Checkpoint(Box<OrchestrationV2TurnItemJsonCheckpoint>),
    #[serde(rename = "run_interrupt_request")]
    RunInterruptRequest(Box<OrchestrationV2TurnItemJsonRunInterruptRequest>),
    #[serde(rename = "run_interrupt_result")]
    RunInterruptResult(Box<OrchestrationV2TurnItemJsonRunInterruptResult>),
    #[serde(rename = "system_notice")]
    SystemNotice(Box<OrchestrationV2TurnItemJsonSystemNotice>),
    #[serde(rename = "error")]
    Error(Box<OrchestrationV2TurnItemJsonError>),
    #[serde(rename = "compaction")]
    Compaction(Box<OrchestrationV2TurnItemJsonCompaction>),
    #[serde(rename = "handoff")]
    Handoff(Box<OrchestrationV2TurnItemJsonHandoff>),
    #[serde(rename = "fork")]
    Fork(Box<OrchestrationV2TurnItemJsonFork>),
    #[serde(rename = "thread_created")]
    ThreadCreated(Box<OrchestrationV2TurnItemJsonThreadCreated>),
    #[serde(rename = "subagent")]
    Subagent(Box<OrchestrationV2TurnItemJsonSubagent>),
    #[serde(rename = "dynamic_tool")]
    DynamicTool(Box<OrchestrationV2TurnItemJsonDynamicTool>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2TurnItemStatus {
    #[serde(rename = "idle")]
    Idle,
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "waiting")]
    Waiting,
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "cancelled")]
    Cancelled,
    #[serde(rename = "interrupted")]
    Interrupted,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2UserInputQuestionOptionsItem {
    #[serde(rename = "label")]
    pub label: TrimmedNonEmptyString,
    #[serde(rename = "description")]
    pub description: TrimmedNonEmptyString,
    #[serde(rename = "value", default, skip_serializing_if = "Optional::is_absent")]
    pub value: Optional<IsoDateTime>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2UserInputQuestion {
    #[serde(rename = "id")]
    pub id: TrimmedNonEmptyString,
    #[serde(rename = "header")]
    pub header: TrimmedNonEmptyString,
    #[serde(rename = "question")]
    pub question: TrimmedNonEmptyString,
    #[serde(rename = "options")]
    pub options: Vec<OrchestrationV2UserInputQuestionOptionsItem>,
    #[serde(
        rename = "multiSelect",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub multi_select: Optional<bool>,
    #[serde(
        rename = "allowCustomAnswer",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub allow_custom_answer: Optional<bool>,
    #[serde(
        rename = "required",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub required: Optional<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestrationV2UserMessageInputIntent {
    #[serde(rename = "turn_start")]
    TurnStart,
    #[serde(rename = "queued_turn")]
    QueuedTurn,
    #[serde(rename = "steer")]
    Steer,
    #[serde(rename = "promoted_queued_to_steer")]
    PromotedQueuedToSteer,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationV2WebSearchResult {
    #[serde(rename = "title", default, skip_serializing_if = "Optional::is_absent")]
    pub title: Optional<IsoDateTime>,
    #[serde(rename = "url", default, skip_serializing_if = "Optional::is_absent")]
    pub url: Optional<TrimmedNonEmptyString>,
    #[serde(
        rename = "snippet",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub snippet: Optional<IsoDateTime>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PastedTextAttachmentSourceTag {
    #[serde(rename = "pasted-text")]
    PastedText,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PastedTextAttachmentSource {
    #[serde(rename = "_tag")]
    pub _tag: PastedTextAttachmentSourceTag,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PlanId(pub String);
impl From<String> for PlanId {
    fn from(value: String) -> Self {
        Self(value)
    }
}
impl From<&str> for PlanId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}
impl AsRef<str> for PlanId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
impl std::fmt::Display for PlanId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

pub type PositiveInt = i64;

#[derive(Debug, Clone, PartialEq)]
pub struct PreviewAnnotationContextRecordVersion;
impl Serialize for PreviewAnnotationContextRecordVersion {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serde_json::json!(1).serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for PreviewAnnotationContextRecordVersion {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        if value == serde_json::json!(1) {
            Ok(Self)
        } else {
            Err(serde::de::Error::custom("expected literal 1"))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewAnnotationContextRecordKind {
    #[serde(rename = "preview-annotation")]
    PreviewAnnotation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewAnnotationContextRecordStyleChangeDetailsItem {
    #[serde(rename = "targetId")]
    pub target_id: String,
    #[serde(rename = "selector", deserialize_with = "required_nullable")]
    pub selector: Option<String>,
    #[serde(rename = "property")]
    pub property: String,
    #[serde(rename = "previousValue")]
    pub previous_value: String,
    #[serde(rename = "value")]
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewAnnotationContextRecord {
    #[serde(rename = "version")]
    pub version: PreviewAnnotationContextRecordVersion,
    #[serde(rename = "contextId")]
    pub context_id: ComposerContextId,
    #[serde(rename = "label")]
    pub label: String,
    #[serde(rename = "kind")]
    pub kind: PreviewAnnotationContextRecordKind,
    #[serde(rename = "annotationId")]
    pub annotation_id: String,
    #[serde(rename = "pageUrl")]
    pub page_url: String,
    #[serde(rename = "pageTitle", deserialize_with = "required_nullable")]
    pub page_title: Option<String>,
    #[serde(rename = "comment")]
    pub comment: String,
    #[serde(rename = "targetSummary")]
    pub target_summary: String,
    #[serde(rename = "styleChanges")]
    pub style_changes: Vec<String>,
    #[serde(
        rename = "elements",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub elements: Optional<Vec<ElementContextDetails>>,
    #[serde(
        rename = "elementIds",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub element_ids: Optional<Vec<String>>,
    #[serde(
        rename = "regionCount",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub region_count: Optional<NonNegativeInt>,
    #[serde(
        rename = "strokeCount",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub stroke_count: Optional<NonNegativeInt>,
    #[serde(
        rename = "styleChangeDetails",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub style_change_details: Optional<Vec<PreviewAnnotationContextRecordStyleChangeDetailsItem>>,
    #[serde(
        rename = "screenshotContextId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub screenshot_context_id: Optional<ComposerContextId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewNavStatusIdle {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewNavStatusLoading {
    #[serde(rename = "url")]
    pub url: String,
    #[serde(rename = "title")]
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewNavStatusSuccess {
    #[serde(rename = "url")]
    pub url: String,
    #[serde(rename = "title")]
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewNavStatusLoadFailed {
    #[serde(rename = "url")]
    pub url: String,
    #[serde(rename = "title")]
    pub title: String,
    #[serde(rename = "code")]
    pub code: i64,
    #[serde(rename = "description")]
    pub description: IsoDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "_tag")]
pub enum PreviewNavStatus {
    #[serde(rename = "Idle")]
    Idle(Box<PreviewNavStatusIdle>),
    #[serde(rename = "Loading")]
    Loading(Box<PreviewNavStatusLoading>),
    #[serde(rename = "Success")]
    Success(Box<PreviewNavStatusSuccess>),
    #[serde(rename = "LoadFailed")]
    LoadFailed(Box<PreviewNavStatusLoadFailed>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewRenderedViewportSize {
    #[serde(rename = "width")]
    pub width: i64,
    #[serde(rename = "height")]
    pub height: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewSessionSnapshot {
    #[serde(rename = "threadId")]
    pub thread_id: TrimmedNonEmptyString,
    #[serde(rename = "tabId")]
    pub tab_id: PreviewTabId,
    #[serde(rename = "navStatus")]
    pub nav_status: PreviewNavStatus,
    #[serde(rename = "canGoBack")]
    pub can_go_back: bool,
    #[serde(rename = "canGoForward")]
    pub can_go_forward: bool,
    #[serde(
        rename = "viewport",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub viewport: Optional<PreviewViewportSetting>,
    #[serde(
        rename = "profileId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub profile_id: Optional<BrowserProfileId>,
    #[serde(rename = "updatedAt")]
    pub updated_at: IsoDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PreviewTabId(pub String);
impl From<String> for PreviewTabId {
    fn from(value: String) -> Self {
        Self(value)
    }
}
impl From<&str> for PreviewTabId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}
impl AsRef<str> for PreviewTabId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
impl std::fmt::Display for PreviewTabId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewViewportSettingFill {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewViewportSettingFreeform {
    #[serde(rename = "width")]
    pub width: i64,
    #[serde(rename = "height")]
    pub height: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewViewportSettingPresetPresetId {
    #[serde(rename = "iphone-se")]
    IphoneSe,
    #[serde(rename = "iphone-xr")]
    IphoneXr,
    #[serde(rename = "iphone-12-pro")]
    Iphone12Pro,
    #[serde(rename = "iphone-14-pro-max")]
    Iphone14ProMax,
    #[serde(rename = "pixel-7")]
    Pixel7,
    #[serde(rename = "samsung-galaxy-s8-plus")]
    SamsungGalaxyS8Plus,
    #[serde(rename = "samsung-galaxy-s20-ultra")]
    SamsungGalaxyS20Ultra,
    #[serde(rename = "ipad-mini")]
    IpadMini,
    #[serde(rename = "ipad-air")]
    IpadAir,
    #[serde(rename = "ipad-pro")]
    IpadPro,
    #[serde(rename = "surface-pro-7")]
    SurfacePro7,
    #[serde(rename = "surface-duo")]
    SurfaceDuo,
    #[serde(rename = "galaxy-z-fold-5")]
    GalaxyZFold5,
    #[serde(rename = "asus-zenbook-fold")]
    AsusZenbookFold,
    #[serde(rename = "samsung-galaxy-a51-71")]
    SamsungGalaxyA5171,
    #[serde(rename = "nest-hub")]
    NestHub,
    #[serde(rename = "nest-hub-max")]
    NestHubMax,
    #[serde(rename = "desktop-1920x1080")]
    Desktop1920x1080,
    #[serde(rename = "desktop-1440x900")]
    Desktop1440x900,
    #[serde(rename = "laptop-1366x768")]
    Laptop1366x768,
    #[serde(rename = "laptop-1280x800")]
    Laptop1280x800,
    #[serde(rename = "ipad-pro-11")]
    IpadPro11,
    #[serde(rename = "iphone-15-pro")]
    Iphone15Pro,
    #[serde(rename = "pixel-8")]
    Pixel8,
    #[serde(rename = "galaxy-s24")]
    GalaxyS24,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewViewportSettingPreset {
    #[serde(rename = "width")]
    pub width: i64,
    #[serde(rename = "height")]
    pub height: i64,
    #[serde(rename = "presetId")]
    pub preset_id: PreviewViewportSettingPresetPresetId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "_tag")]
pub enum PreviewViewportSetting {
    #[serde(rename = "fill")]
    Fill(Box<PreviewViewportSettingFill>),
    #[serde(rename = "freeform")]
    Freeform(Box<PreviewViewportSettingFreeform>),
    #[serde(rename = "preset")]
    Preset(Box<PreviewViewportSettingPreset>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    #[serde(rename = "id")]
    pub id: ProjectId,
    #[serde(rename = "title")]
    pub title: TrimmedNonEmptyString,
    #[serde(rename = "workspaceRoot")]
    pub workspace_root: TrimmedNonEmptyString,
    #[serde(
        rename = "repositoryIdentity",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub repository_identity: Optional<Option<RepositoryIdentity>>,
    #[serde(
        rename = "faviconPath",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub favicon_path: Optional<Option<TrimmedNonEmptyString>>,
    #[serde(
        rename = "projectIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub project_icon: Optional<Option<ProjectIconOverride>>,
    #[serde(
        rename = "defaultModelSelection",
        deserialize_with = "required_nullable"
    )]
    pub default_model_selection: Option<ModelSelection>,
    #[serde(
        rename = "defaultThreadEnvMode",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub default_thread_env_mode: Optional<Option<ThreadEnvMode>>,
    #[serde(
        rename = "autoPull",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub auto_pull: Optional<bool>,
    #[serde(rename = "scripts")]
    pub scripts: Vec<ProjectScript>,
    #[serde(rename = "createdAt")]
    pub created_at: IsoDateTime,
    #[serde(rename = "updatedAt")]
    pub updated_at: IsoDateTime,
    #[serde(rename = "deletedAt", deserialize_with = "required_nullable")]
    pub deleted_at: Option<IsoDateTime>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProjectIconColor {
    #[serde(rename = "gray")]
    Gray,
    #[serde(rename = "red")]
    Red,
    #[serde(rename = "orange")]
    Orange,
    #[serde(rename = "amber")]
    Amber,
    #[serde(rename = "yellow")]
    Yellow,
    #[serde(rename = "lime")]
    Lime,
    #[serde(rename = "green")]
    Green,
    #[serde(rename = "emerald")]
    Emerald,
    #[serde(rename = "teal")]
    Teal,
    #[serde(rename = "cyan")]
    Cyan,
    #[serde(rename = "sky")]
    Sky,
    #[serde(rename = "blue")]
    Blue,
    #[serde(rename = "indigo")]
    Indigo,
    #[serde(rename = "violet")]
    Violet,
    #[serde(rename = "purple")]
    Purple,
    #[serde(rename = "fuchsia")]
    Fuchsia,
    #[serde(rename = "pink")]
    Pink,
    #[serde(rename = "rose")]
    Rose,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectIconOverrideLucide {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "color")]
    pub color: ProjectIconColor,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectIconOverrideEmoji {
    #[serde(rename = "emoji")]
    pub emoji: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectIconOverrideMonogram {
    #[serde(rename = "text")]
    pub text: ProjectMonogramText,
    #[serde(rename = "color")]
    pub color: ProjectIconColor,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum ProjectIconOverride {
    #[serde(rename = "lucide")]
    Lucide(Box<ProjectIconOverrideLucide>),
    #[serde(rename = "emoji")]
    Emoji(Box<ProjectIconOverrideEmoji>),
    #[serde(rename = "monogram")]
    Monogram(Box<ProjectIconOverrideMonogram>),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProjectId(pub String);
impl From<String> for ProjectId {
    fn from(value: String) -> Self {
        Self(value)
    }
}
impl From<&str> for ProjectId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}
impl AsRef<str> for ProjectId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
impl std::fmt::Display for ProjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

pub type ProjectMonogramText = String;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectScript {
    #[serde(rename = "id")]
    pub id: TrimmedNonEmptyString,
    #[serde(rename = "name")]
    pub name: TrimmedNonEmptyString,
    #[serde(rename = "command")]
    pub command: TrimmedNonEmptyString,
    #[serde(rename = "icon")]
    pub icon: ProjectScriptIcon,
    #[serde(rename = "runOnWorktreeCreate")]
    pub run_on_worktree_create: bool,
    #[serde(rename = "async", default, skip_serializing_if = "Optional::is_absent")]
    pub r#async: Optional<bool>,
    #[serde(
        rename = "previewUrl",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub preview_url: Optional<TrimmedNonEmptyString>,
    #[serde(
        rename = "autoOpenPreview",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub auto_open_preview: Optional<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProjectScriptIcon {
    #[serde(rename = "play")]
    Play,
    #[serde(rename = "test")]
    Test,
    #[serde(rename = "lint")]
    Lint,
    #[serde(rename = "configure")]
    Configure,
    #[serde(rename = "build")]
    Build,
    #[serde(rename = "debug")]
    Debug,
}

pub use crate::runtime_policy::PermissionDecision as ProviderApprovalDecision;

/// T3 wire approval option. Host-local callback IDs and scopes live in
/// runtime_policy::PermissionOption, not this generated contract.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderApprovalOption {
    #[serde(rename = "decision")]
    pub decision: ProviderApprovalDecision,
    #[serde(rename = "label")]
    pub label: TrimmedNonEmptyString,
    #[serde(
        rename = "warning",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub warning: Optional<TrimmedNonEmptyString>,
}

pub use crate::runtime_policy::InteractionMode as ProviderInteractionMode;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderReplayEntryExpectOutbound {
    #[serde(rename = "label", default, skip_serializing_if = "Optional::is_absent")]
    pub label: Optional<TrimmedNonEmptyString>,
    #[serde(rename = "frame")]
    pub frame: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderReplayEntryEmitInbound {
    #[serde(rename = "label", default, skip_serializing_if = "Optional::is_absent")]
    pub label: Optional<TrimmedNonEmptyString>,
    #[serde(rename = "frame")]
    pub frame: serde_json::Value,
    #[serde(
        rename = "afterMs",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub after_ms: Optional<NonNegativeInt>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProviderReplayEntryRuntimeExitStatus {
    #[serde(rename = "success")]
    Success,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "cancelled")]
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderReplayEntryRuntimeExit {
    #[serde(rename = "status")]
    pub status: ProviderReplayEntryRuntimeExitStatus,
    #[serde(rename = "error", default, skip_serializing_if = "Optional::is_absent")]
    pub error: Optional<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ProviderReplayEntry {
    #[serde(rename = "expect_outbound")]
    ExpectOutbound(Box<ProviderReplayEntryExpectOutbound>),
    #[serde(rename = "emit_inbound")]
    EmitInbound(Box<ProviderReplayEntryEmitInbound>),
    #[serde(rename = "runtime_exit")]
    RuntimeExit(Box<ProviderReplayEntryRuntimeExit>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderReplayNdjsonRecordTranscriptStart {
    #[serde(rename = "provider")]
    pub provider: TrimmedNonEmptyString,
    #[serde(rename = "protocol")]
    pub protocol: TrimmedNonEmptyString,
    #[serde(rename = "version")]
    pub version: TrimmedNonEmptyString,
    #[serde(rename = "scenario")]
    pub scenario: TrimmedNonEmptyString,
    #[serde(
        rename = "metadata",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub metadata: Optional<BTreeMap<IsoDateTime, serde_json::Value>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderReplayNdjsonRecordExpectOutbound {
    #[serde(rename = "label", default, skip_serializing_if = "Optional::is_absent")]
    pub label: Optional<TrimmedNonEmptyString>,
    #[serde(rename = "frame")]
    pub frame: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderReplayNdjsonRecordEmitInbound {
    #[serde(rename = "label", default, skip_serializing_if = "Optional::is_absent")]
    pub label: Optional<TrimmedNonEmptyString>,
    #[serde(rename = "frame")]
    pub frame: serde_json::Value,
    #[serde(
        rename = "afterMs",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub after_ms: Optional<NonNegativeInt>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProviderReplayNdjsonRecordRuntimeExitStatus {
    #[serde(rename = "success")]
    Success,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "cancelled")]
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderReplayNdjsonRecordRuntimeExit {
    #[serde(rename = "status")]
    pub status: ProviderReplayNdjsonRecordRuntimeExitStatus,
    #[serde(rename = "error", default, skip_serializing_if = "Optional::is_absent")]
    pub error: Optional<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ProviderReplayNdjsonRecord {
    #[serde(rename = "transcript_start")]
    TranscriptStart(Box<ProviderReplayNdjsonRecordTranscriptStart>),
    #[serde(rename = "expect_outbound")]
    ExpectOutbound(Box<ProviderReplayNdjsonRecordExpectOutbound>),
    #[serde(rename = "emit_inbound")]
    EmitInbound(Box<ProviderReplayNdjsonRecordEmitInbound>),
    #[serde(rename = "runtime_exit")]
    RuntimeExit(Box<ProviderReplayNdjsonRecordRuntimeExit>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderReplayTranscript {
    #[serde(rename = "provider")]
    pub provider: TrimmedNonEmptyString,
    #[serde(rename = "protocol")]
    pub protocol: TrimmedNonEmptyString,
    #[serde(rename = "version")]
    pub version: TrimmedNonEmptyString,
    #[serde(rename = "scenario")]
    pub scenario: TrimmedNonEmptyString,
    #[serde(
        rename = "metadata",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub metadata: Optional<BTreeMap<IsoDateTime, serde_json::Value>>,
    #[serde(rename = "entries")]
    pub entries: Vec<ProviderReplayEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProviderReplayTranscriptHeaderType {
    #[serde(rename = "transcript_start")]
    TranscriptStart,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderReplayTranscriptHeader {
    #[serde(rename = "type")]
    pub r#type: ProviderReplayTranscriptHeaderType,
    #[serde(rename = "provider")]
    pub provider: TrimmedNonEmptyString,
    #[serde(rename = "protocol")]
    pub protocol: TrimmedNonEmptyString,
    #[serde(rename = "version")]
    pub version: TrimmedNonEmptyString,
    #[serde(rename = "scenario")]
    pub scenario: TrimmedNonEmptyString,
    #[serde(
        rename = "metadata",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub metadata: Optional<BTreeMap<IsoDateTime, serde_json::Value>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProviderRequestKind {
    #[serde(rename = "command")]
    Command,
    #[serde(rename = "file-read")]
    FileRead,
    #[serde(rename = "file-change")]
    FileChange,
    #[serde(rename = "mcp-elicitation")]
    McpElicitation,
    #[serde(rename = "permission")]
    Permission,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProviderSessionId(pub String);
impl From<String> for ProviderSessionId {
    fn from(value: String) -> Self {
        Self(value)
    }
}
impl From<&str> for ProviderSessionId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}
impl AsRef<str> for ProviderSessionId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
impl std::fmt::Display for ProviderSessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProviderThreadId(pub String);
impl From<String> for ProviderThreadId {
    fn from(value: String) -> Self {
        Self(value)
    }
}
impl From<&str> for ProviderThreadId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}
impl AsRef<str> for ProviderThreadId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
impl std::fmt::Display for ProviderThreadId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProviderTurnId(pub String);
impl From<String> for ProviderTurnId {
    fn from(value: String) -> Self {
        Self(value)
    }
}
impl From<&str> for ProviderTurnId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}
impl AsRef<str> for ProviderTurnId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
impl std::fmt::Display for ProviderTurnId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

pub type ProviderUserInputAnswers = BTreeMap<IsoDateTime, serde_json::Value>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PullRequestActor {
    #[serde(rename = "isBot", default, skip_serializing_if = "Optional::is_absent")]
    pub is_bot: Optional<bool>,
    #[serde(rename = "login")]
    pub login: TrimmedNonEmptyString,
    #[serde(rename = "name", deserialize_with = "required_nullable")]
    pub name: Option<IsoDateTime>,
    #[serde(rename = "avatarUrl", deserialize_with = "required_nullable")]
    pub avatar_url: Option<IsoDateTime>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PullRequestChecksState {
    #[serde(rename = "passing")]
    Passing,
    #[serde(rename = "failing")]
    Failing,
    #[serde(rename = "pending")]
    Pending,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PullRequestContextMetadataState {
    #[serde(rename = "open")]
    Open,
    #[serde(rename = "closed")]
    Closed,
    #[serde(rename = "merged")]
    Merged,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PullRequestContextMetadata {
    #[serde(rename = "number")]
    pub number: PositiveInt,
    #[serde(rename = "title")]
    pub title: String,
    #[serde(rename = "url")]
    pub url: String,
    #[serde(rename = "headBranch")]
    pub head_branch: String,
    #[serde(rename = "baseBranch")]
    pub base_branch: String,
    #[serde(rename = "state")]
    pub state: PullRequestContextMetadataState,
    #[serde(rename = "isDraft")]
    pub is_draft: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PullRequestMergeability {
    #[serde(rename = "mergeable")]
    Mergeable,
    #[serde(rename = "conflicting")]
    Conflicting,
    #[serde(rename = "unknown")]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PullRequestReviewDecision {
    #[serde(rename = "approved")]
    Approved,
    #[serde(rename = "changes-requested")]
    ChangesRequested,
    #[serde(rename = "review-required")]
    ReviewRequired,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PullRequestState {
    #[serde(rename = "open")]
    Open,
    #[serde(rename = "closed")]
    Closed,
    #[serde(rename = "merged")]
    Merged,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RawEventId(pub String);
impl From<String> for RawEventId {
    fn from(value: String) -> Self {
        Self(value)
    }
}
impl From<&str> for RawEventId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}
impl AsRef<str> for RawEventId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
impl std::fmt::Display for RawEventId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RepositoryIdentity {
    #[serde(rename = "canonicalKey")]
    pub canonical_key: TrimmedNonEmptyString,
    #[serde(rename = "locator")]
    pub locator: RepositoryIdentityLocator,
    #[serde(
        rename = "webUrl",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub web_url: Optional<String>,
    #[serde(
        rename = "rootPath",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub root_path: Optional<String>,
    #[serde(
        rename = "displayName",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub display_name: Optional<String>,
    #[serde(
        rename = "provider",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider: Optional<String>,
    #[serde(rename = "owner", default, skip_serializing_if = "Optional::is_absent")]
    pub owner: Optional<String>,
    #[serde(rename = "name", default, skip_serializing_if = "Optional::is_absent")]
    pub name: Optional<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RepositoryIdentityLocatorSource {
    #[serde(rename = "git-remote")]
    GitRemote,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RepositoryIdentityLocator {
    #[serde(rename = "source")]
    pub source: RepositoryIdentityLocatorSource,
    #[serde(rename = "remoteName")]
    pub remote_name: TrimmedNonEmptyString,
    #[serde(rename = "remoteUrl")]
    pub remote_url: TrimmedNonEmptyString,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReviewCommentContextRecordVersion;
impl Serialize for ReviewCommentContextRecordVersion {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serde_json::json!(1).serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for ReviewCommentContextRecordVersion {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        if value == serde_json::json!(1) {
            Ok(Self)
        } else {
            Err(serde::de::Error::custom("expected literal 1"))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ReviewCommentContextRecordKind {
    #[serde(rename = "review-comment")]
    ReviewComment,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReviewCommentContextRecord {
    #[serde(rename = "version")]
    pub version: ReviewCommentContextRecordVersion,
    #[serde(rename = "contextId")]
    pub context_id: ComposerContextId,
    #[serde(rename = "label")]
    pub label: String,
    #[serde(rename = "kind")]
    pub kind: ReviewCommentContextRecordKind,
    #[serde(rename = "sectionId")]
    pub section_id: String,
    #[serde(rename = "sectionTitle")]
    pub section_title: String,
    #[serde(rename = "filePath")]
    pub file_path: String,
    #[serde(rename = "startIndex")]
    pub start_index: NonNegativeInt,
    #[serde(rename = "endIndex")]
    pub end_index: NonNegativeInt,
    #[serde(rename = "rangeLabel")]
    pub range_label: String,
    #[serde(rename = "text")]
    pub text: String,
    #[serde(rename = "diff")]
    pub diff: String,
    #[serde(
        rename = "fenceLanguage",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub fence_language: Optional<String>,
    #[serde(
        rename = "pullRequest",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub pull_request: Optional<PullRequestContextMetadata>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RunAttemptId(pub String);
impl From<String> for RunAttemptId {
    fn from(value: String) -> Self {
        Self(value)
    }
}
impl From<&str> for RunAttemptId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}
impl AsRef<str> for RunAttemptId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
impl std::fmt::Display for RunAttemptId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RunId(pub String);
impl From<String> for RunId {
    fn from(value: String) -> Self {
        Self(value)
    }
}
impl From<&str> for RunId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}
impl AsRef<str> for RunId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
impl std::fmt::Display for RunId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

pub use crate::runtime_policy::RuntimeMode;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RuntimeRequestId(pub String);
impl From<String> for RuntimeRequestId {
    fn from(value: String) -> Self {
        Self(value)
    }
}
impl From<&str> for RuntimeRequestId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}
impl AsRef<str> for RuntimeRequestId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
impl std::fmt::Display for RuntimeRequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduledTask {
    #[serde(rename = "id")]
    pub id: ScheduledTaskId,
    #[serde(rename = "title")]
    pub title: TrimmedNonEmptyString,
    #[serde(rename = "prompt")]
    pub prompt: TrimmedNonEmptyString,
    #[serde(rename = "enabled")]
    pub enabled: bool,
    #[serde(rename = "schedule")]
    pub schedule: ScheduledTaskSchedule,
    #[serde(rename = "projectId")]
    pub project_id: ProjectId,
    #[serde(rename = "threadId", deserialize_with = "required_nullable")]
    pub thread_id: Option<ThreadId>,
    #[serde(rename = "workspaceStrategy")]
    pub workspace_strategy: OrchestrationV2ThreadLaunchWorkspaceStrategy,
    #[serde(rename = "modelSelection")]
    pub model_selection: ModelSelection,
    #[serde(rename = "runtimeMode")]
    pub runtime_mode: RuntimeMode,
    #[serde(rename = "interactionMode")]
    pub interaction_mode: ProviderInteractionMode,
    #[serde(rename = "createdBy")]
    pub created_by: OrchestrationV2Actor,
    #[serde(rename = "creationSource")]
    pub creation_source: OrchestrationV2CreationSource,
    #[serde(rename = "createdAt")]
    pub created_at: IsoDateTime,
    #[serde(rename = "updatedAt")]
    pub updated_at: IsoDateTime,
    #[serde(rename = "nextRunAt", deserialize_with = "required_nullable")]
    pub next_run_at: Option<IsoDateTime>,
    #[serde(rename = "lastRunAt", deserialize_with = "required_nullable")]
    pub last_run_at: Option<IsoDateTime>,
    #[serde(rename = "lastRunStatus")]
    pub last_run_status: ScheduledTaskRunStatus,
    #[serde(rename = "lastRunError", deserialize_with = "required_nullable")]
    pub last_run_error: Option<IsoDateTime>,
    #[serde(rename = "runCount")]
    pub run_count: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduledTaskDeleteInput {
    #[serde(rename = "id")]
    pub id: ScheduledTaskId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduledTaskDeleteResult {
    #[serde(rename = "id")]
    pub id: ScheduledTaskId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ScheduledTaskErrorTag {
    #[serde(rename = "ScheduledTaskError")]
    ScheduledTaskError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduledTaskError {
    #[serde(rename = "_tag")]
    pub _tag: ScheduledTaskErrorTag,
    #[serde(rename = "message")]
    pub message: IsoDateTime,
    #[serde(
        rename = "taskId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub task_id: Optional<ScheduledTaskId>,
    #[serde(rename = "cause", default, skip_serializing_if = "Optional::is_absent")]
    pub cause: Optional<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ScheduledTaskId(pub String);
impl From<String> for ScheduledTaskId {
    fn from(value: String) -> Self {
        Self(value)
    }
}
impl From<&str> for ScheduledTaskId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}
impl AsRef<str> for ScheduledTaskId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
impl std::fmt::Display for ScheduledTaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduledTaskListInput {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduledTaskListResult {
    #[serde(rename = "tasks")]
    pub tasks: Vec<ScheduledTask>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduledTaskMutationResult {
    #[serde(rename = "task")]
    pub task: ScheduledTask,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduledTaskRunNowInput {
    #[serde(rename = "id")]
    pub id: ScheduledTaskId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduledTaskRunNowResult {
    #[serde(rename = "task")]
    pub task: ScheduledTask,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ScheduledTaskRunStatus {
    #[serde(rename = "never")]
    Never,
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "succeeded")]
    Succeeded,
    #[serde(rename = "failed")]
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduledTaskScheduleInterval {
    #[serde(rename = "everyMs")]
    pub every_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduledTaskScheduleFixedTime {
    #[serde(rename = "timeOfDay")]
    pub time_of_day: String,
    #[serde(
        rename = "weekdays",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub weekdays: Optional<Vec<i64>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
enum ScheduledTaskScheduleCanonical {
    #[serde(rename = "interval")]
    Interval(Box<ScheduledTaskScheduleInterval>),
    #[serde(rename = "fixed_time")]
    FixedTime(Box<ScheduledTaskScheduleFixedTime>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "serde_json::Value")]
#[serde(tag = "type")]
pub enum ScheduledTaskSchedule {
    #[serde(rename = "interval")]
    Interval(Box<ScheduledTaskScheduleInterval>),
    #[serde(rename = "fixed_time")]
    FixedTime(Box<ScheduledTaskScheduleFixedTime>),
}
impl TryFrom<serde_json::Value> for ScheduledTaskSchedule {
    type Error = String;
    fn try_from(value: serde_json::Value) -> Result<Self, String> {
        let value: ScheduledTaskScheduleCanonical =
            serde_json::from_value(normalize_contract("ScheduledTaskSchedule", value)?)
                .map_err(|e| e.to_string())?;
        Ok(match value {
            ScheduledTaskScheduleCanonical::Interval(value) => Self::Interval(value),
            ScheduledTaskScheduleCanonical::FixedTime(value) => Self::FixedTime(value),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduledTaskSetEnabledInput {
    #[serde(rename = "id")]
    pub id: ScheduledTaskId,
    #[serde(rename = "enabled")]
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduledTaskUpsertInput {
    #[serde(rename = "id", default, skip_serializing_if = "Optional::is_absent")]
    pub id: Optional<ScheduledTaskId>,
    #[serde(
        rename = "requireExisting",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub require_existing: Optional<bool>,
    #[serde(
        rename = "commandId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub command_id: Optional<CommandId>,
    #[serde(rename = "title")]
    pub title: TrimmedNonEmptyString,
    #[serde(rename = "prompt")]
    pub prompt: TrimmedNonEmptyString,
    #[serde(rename = "enabled")]
    pub enabled: bool,
    #[serde(rename = "schedule")]
    pub schedule: ScheduledTaskUpsertSchedule,
    #[serde(rename = "projectId")]
    pub project_id: ProjectId,
    #[serde(
        rename = "threadId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub thread_id: Optional<Option<ThreadId>>,
    #[serde(rename = "workspaceStrategy")]
    pub workspace_strategy: OrchestrationV2ThreadLaunchWorkspaceStrategy,
    #[serde(rename = "modelSelection")]
    pub model_selection: ModelSelection,
    #[serde(rename = "runtimeMode")]
    pub runtime_mode: RuntimeMode,
    #[serde(rename = "interactionMode")]
    pub interaction_mode: ProviderInteractionMode,
    #[serde(
        rename = "createdBy",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub created_by: Optional<OrchestrationV2Actor>,
    #[serde(
        rename = "creationSource",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub creation_source: Optional<OrchestrationV2CreationSource>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduledTaskUpsertScheduleInterval {
    #[serde(rename = "everyMs")]
    pub every_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduledTaskUpsertScheduleFixedTime {
    #[serde(rename = "timeOfDay")]
    pub time_of_day: String,
    #[serde(
        rename = "weekdays",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub weekdays: Optional<Vec<i64>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
enum ScheduledTaskUpsertScheduleCanonical {
    #[serde(rename = "interval")]
    Interval(Box<ScheduledTaskUpsertScheduleInterval>),
    #[serde(rename = "fixed_time")]
    FixedTime(Box<ScheduledTaskUpsertScheduleFixedTime>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "serde_json::Value")]
#[serde(tag = "type")]
pub enum ScheduledTaskUpsertSchedule {
    #[serde(rename = "interval")]
    Interval(Box<ScheduledTaskUpsertScheduleInterval>),
    #[serde(rename = "fixed_time")]
    FixedTime(Box<ScheduledTaskUpsertScheduleFixedTime>),
}
impl TryFrom<serde_json::Value> for ScheduledTaskUpsertSchedule {
    type Error = String;
    fn try_from(value: serde_json::Value) -> Result<Self, String> {
        let value: ScheduledTaskUpsertScheduleCanonical =
            serde_json::from_value(normalize_contract("ScheduledTaskUpsertSchedule", value)?)
                .map_err(|e| e.to_string())?;
        Ok(match value {
            ScheduledTaskUpsertScheduleCanonical::Interval(value) => Self::Interval(value),
            ScheduledTaskUpsertScheduleCanonical::FixedTime(value) => Self::FixedTime(value),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SkillContextRecordVersion;
impl Serialize for SkillContextRecordVersion {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serde_json::json!(1).serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for SkillContextRecordVersion {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        if value == serde_json::json!(1) {
            Ok(Self)
        } else {
            Err(serde::de::Error::custom("expected literal 1"))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SkillContextRecordKind {
    #[serde(rename = "skill")]
    Skill,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillContextRecord {
    #[serde(rename = "version")]
    pub version: SkillContextRecordVersion,
    #[serde(rename = "contextId")]
    pub context_id: ComposerContextId,
    #[serde(rename = "label")]
    pub label: String,
    #[serde(rename = "kind")]
    pub kind: SkillContextRecordKind,
    #[serde(rename = "name")]
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SnapShotAccessibilityVariant1Format {
    #[serde(rename = "flat-text")]
    FlatText,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapShotAccessibilityVariant1 {
    #[serde(rename = "format")]
    pub format: SnapShotAccessibilityVariant1Format,
    #[serde(rename = "text")]
    pub text: String,
    #[serde(rename = "truncated")]
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SnapShotAccessibilityVariant2Format {
    #[serde(rename = "element-tree")]
    ElementTree,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SnapShotAccessibilityVariant2CoordinateSpace {
    #[serde(rename = "captured-image")]
    CapturedImage,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapShotAccessibilityVariant2ImageSize {
    #[serde(rename = "width")]
    pub width: PositiveInt,
    #[serde(rename = "height")]
    pub height: PositiveInt,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapShotAccessibilityVariant2 {
    #[serde(rename = "format")]
    pub format: SnapShotAccessibilityVariant2Format,
    #[serde(rename = "coordinateSpace")]
    pub coordinate_space: SnapShotAccessibilityVariant2CoordinateSpace,
    #[serde(rename = "imageSize")]
    pub image_size: SnapShotAccessibilityVariant2ImageSize,
    #[serde(rename = "truncated")]
    pub truncated: bool,
    #[serde(rename = "root")]
    pub root: SnapShotAccessibilityNode,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SnapShotAccessibility {
    Variant1(SnapShotAccessibilityVariant1),
    Variant2(SnapShotAccessibilityVariant2),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapShotAccessibilityNodeBounds {
    #[serde(rename = "x")]
    pub x: NonNegativeInt,
    #[serde(rename = "y")]
    pub y: NonNegativeInt,
    #[serde(rename = "width")]
    pub width: PositiveInt,
    #[serde(rename = "height")]
    pub height: PositiveInt,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SnapShotAccessibilityNodeStateChecked {
    #[serde(rename = "on")]
    On,
    #[serde(rename = "off")]
    Off,
    #[serde(rename = "mixed")]
    Mixed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapShotAccessibilityNodeState {
    #[serde(
        rename = "active",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub active: Optional<bool>,
    #[serde(rename = "busy", default, skip_serializing_if = "Optional::is_absent")]
    pub busy: Optional<bool>,
    #[serde(
        rename = "checked",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub checked: Optional<SnapShotAccessibilityNodeStateChecked>,
    #[serde(
        rename = "editable",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub editable: Optional<bool>,
    #[serde(
        rename = "enabled",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub enabled: Optional<bool>,
    #[serde(
        rename = "expanded",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub expanded: Optional<bool>,
    #[serde(
        rename = "focused",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub focused: Optional<bool>,
    #[serde(
        rename = "selected",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub selected: Optional<bool>,
    #[serde(
        rename = "visible",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub visible: Optional<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapShotAccessibilityNode {
    #[serde(rename = "role")]
    pub role: String,
    #[serde(rename = "name", default, skip_serializing_if = "Optional::is_absent")]
    pub name: Optional<String>,
    #[serde(rename = "value", default, skip_serializing_if = "Optional::is_absent")]
    pub value: Optional<String>,
    #[serde(
        rename = "description",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub description: Optional<String>,
    #[serde(rename = "bounds", deserialize_with = "required_nullable")]
    pub bounds: Option<SnapShotAccessibilityNodeBounds>,
    #[serde(rename = "state", default, skip_serializing_if = "Optional::is_absent")]
    pub state: Optional<SnapShotAccessibilityNodeState>,
    #[serde(
        rename = "actions",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub actions: Optional<Vec<String>>,
    #[serde(rename = "children")]
    pub children: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SnapShotSourceKind {
    #[serde(rename = "snap-shot")]
    SnapShot,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapShotSource {
    #[serde(rename = "kind")]
    pub kind: SnapShotSourceKind,
    #[serde(rename = "capturedAt")]
    pub captured_at: IsoDateTime,
    #[serde(rename = "appName")]
    pub app_name: String,
    #[serde(rename = "windowTitle")]
    pub window_title: String,
    #[serde(
        rename = "accessibleText",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub accessible_text: Optional<String>,
    #[serde(
        rename = "accessibility",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub accessibility: Optional<SnapShotAccessibility>,
    #[serde(
        rename = "appIdentifier",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub app_identifier: Optional<String>,
    #[serde(
        rename = "appIconDataUrl",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub app_icon_data_url: Optional<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SourceControlCloneProtocol {
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "ssh")]
    Ssh,
    #[serde(rename = "https")]
    Https,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SourceControlCloneRepositoryResult {
    #[serde(rename = "cwd")]
    pub cwd: TrimmedNonEmptyString,
    #[serde(rename = "remoteUrl")]
    pub remote_url: TrimmedNonEmptyString,
    #[serde(rename = "repository", deserialize_with = "required_nullable")]
    pub repository: Option<SourceControlRepositoryInfo>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SourceControlProviderKind {
    #[serde(rename = "github")]
    Github,
    #[serde(rename = "gitlab")]
    Gitlab,
    #[serde(rename = "forgejo")]
    Forgejo,
    #[serde(rename = "azure-devops")]
    AzureDevops,
    #[serde(rename = "bitbucket")]
    Bitbucket,
    #[serde(rename = "unknown")]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SourceControlRepositoryInfo {
    #[serde(rename = "provider")]
    pub provider: SourceControlProviderKind,
    #[serde(rename = "nameWithOwner")]
    pub name_with_owner: TrimmedNonEmptyString,
    #[serde(rename = "url")]
    pub url: TrimmedNonEmptyString,
    #[serde(rename = "sshUrl")]
    pub ssh_url: TrimmedNonEmptyString,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TerminalContextRecordVersion;
impl Serialize for TerminalContextRecordVersion {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serde_json::json!(1).serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for TerminalContextRecordVersion {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        if value == serde_json::json!(1) {
            Ok(Self)
        } else {
            Err(serde::de::Error::custom("expected literal 1"))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TerminalContextRecordKind {
    #[serde(rename = "terminal")]
    Terminal,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TerminalContextRecord {
    #[serde(rename = "version")]
    pub version: TerminalContextRecordVersion,
    #[serde(rename = "contextId")]
    pub context_id: ComposerContextId,
    #[serde(rename = "label")]
    pub label: String,
    #[serde(rename = "kind")]
    pub kind: TerminalContextRecordKind,
    #[serde(rename = "terminalId")]
    pub terminal_id: String,
    #[serde(rename = "terminalLabel")]
    pub terminal_label: String,
    #[serde(rename = "lineStart")]
    pub line_start: NonNegativeInt,
    #[serde(rename = "lineEnd")]
    pub line_end: NonNegativeInt,
    #[serde(rename = "text")]
    pub text: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ThreadContextRecordVersion;
impl Serialize for ThreadContextRecordVersion {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serde_json::json!(1).serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for ThreadContextRecordVersion {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        if value == serde_json::json!(1) {
            Ok(Self)
        } else {
            Err(serde::de::Error::custom("expected literal 1"))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ThreadContextRecordKind {
    #[serde(rename = "thread")]
    Thread,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThreadContextRecord {
    #[serde(rename = "version")]
    pub version: ThreadContextRecordVersion,
    #[serde(rename = "contextId")]
    pub context_id: ComposerContextId,
    #[serde(rename = "label")]
    pub label: String,
    #[serde(rename = "kind")]
    pub kind: ThreadContextRecordKind,
    #[serde(rename = "environmentId")]
    pub environment_id: EnvironmentId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "title")]
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ThreadEnvMode {
    #[serde(rename = "local")]
    Local,
    #[serde(rename = "worktree")]
    Worktree,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ThreadId(pub String);
impl From<String> for ThreadId {
    fn from(value: String) -> Self {
        Self(value)
    }
}
impl From<&str> for ThreadId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}
impl AsRef<str> for ThreadId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
impl std::fmt::Display for ThreadId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThreadLinkedPullRequest {
    #[serde(rename = "projectId")]
    pub project_id: ProjectId,
    #[serde(rename = "repository")]
    pub repository: TrimmedNonEmptyString,
    #[serde(rename = "number")]
    pub number: PositiveInt,
    #[serde(rename = "url")]
    pub url: TrimmedNonEmptyString,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThreadPullRequestKey {
    #[serde(rename = "host")]
    pub host: TrimmedNonEmptyString,
    #[serde(rename = "repository")]
    pub repository: TrimmedNonEmptyString,
    #[serde(rename = "number")]
    pub number: PositiveInt,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThreadPullRequestLink {
    #[serde(rename = "host")]
    pub host: TrimmedNonEmptyString,
    #[serde(rename = "repository")]
    pub repository: TrimmedNonEmptyString,
    #[serde(rename = "number")]
    pub number: PositiveInt,
    #[serde(rename = "url")]
    pub url: TrimmedNonEmptyString,
    #[serde(rename = "source")]
    pub source: ThreadPullRequestLinkSource,
    #[serde(rename = "linkedAt")]
    pub linked_at: IsoDateTime,
    #[serde(rename = "snapshot", deserialize_with = "required_nullable")]
    pub snapshot: Option<ThreadPullRequestSnapshot>,
    #[serde(rename = "stack", deserialize_with = "required_nullable")]
    pub stack: Option<ThreadPullRequestStack>,
    #[serde(rename = "watch", default, skip_serializing_if = "Optional::is_absent")]
    pub watch: Optional<ThreadPullRequestWatch>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ThreadPullRequestLinkSource {
    #[serde(rename = "manual")]
    Manual,
    #[serde(rename = "created")]
    Created,
    #[serde(rename = "agent")]
    Agent,
    #[serde(rename = "stack")]
    Stack,
    #[serde(rename = "stack-dismissed")]
    StackDismissed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThreadPullRequestSnapshot {
    #[serde(rename = "state")]
    pub state: PullRequestState,
    #[serde(rename = "title")]
    pub title: TrimmedNonEmptyString,
    #[serde(rename = "headBranch")]
    pub head_branch: TrimmedNonEmptyString,
    #[serde(rename = "baseBranch")]
    pub base_branch: TrimmedNonEmptyString,
    #[serde(rename = "isDraft")]
    pub is_draft: bool,
    #[serde(rename = "updatedAt", deserialize_with = "required_nullable")]
    pub updated_at: Option<IsoDateTime>,
    #[serde(rename = "syncedAt")]
    pub synced_at: IsoDateTime,
    #[serde(
        rename = "closedAt",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub closed_at: Optional<Option<IsoDateTime>>,
    #[serde(
        rename = "mergedAt",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub merged_at: Optional<Option<IsoDateTime>>,
    #[serde(
        rename = "author",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub author: Optional<Option<PullRequestActor>>,
    #[serde(
        rename = "additions",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub additions: Optional<NonNegativeInt>,
    #[serde(
        rename = "deletions",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub deletions: Optional<NonNegativeInt>,
    #[serde(
        rename = "changedFiles",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub changed_files: Optional<NonNegativeInt>,
    #[serde(
        rename = "reviewDecision",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub review_decision: Optional<Option<PullRequestReviewDecision>>,
    #[serde(
        rename = "checksState",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub checks_state: Optional<Option<PullRequestChecksState>>,
    #[serde(
        rename = "mergeability",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub mergeability: Optional<PullRequestMergeability>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ThreadPullRequestStackKind {
    #[serde(rename = "native")]
    Native,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThreadPullRequestStack {
    #[serde(rename = "kind")]
    pub kind: ThreadPullRequestStackKind,
    #[serde(rename = "id")]
    pub id: TrimmedNonEmptyString,
    #[serde(rename = "number")]
    pub number: PositiveInt,
    #[serde(rename = "url")]
    pub url: TrimmedNonEmptyString,
    #[serde(rename = "base")]
    pub base: TrimmedNonEmptyString,
    #[serde(rename = "layers")]
    pub layers: Vec<ThreadPullRequestStackLayer>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThreadPullRequestStackLayer {
    #[serde(rename = "number")]
    pub number: PositiveInt,
    #[serde(rename = "headBranch")]
    pub head_branch: TrimmedNonEmptyString,
    #[serde(rename = "state")]
    pub state: PullRequestState,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThreadPullRequestWatch {
    #[serde(rename = "startedAt")]
    pub started_at: IsoDateTime,
    #[serde(rename = "headSha", deserialize_with = "required_nullable")]
    pub head_sha: Option<TrimmedNonEmptyString>,
    #[serde(rename = "failedChecks")]
    pub failed_checks: Vec<TrimmedNonEmptyString>,
    #[serde(rename = "passed")]
    pub passed: bool,
    #[serde(rename = "remarksThrough")]
    pub remarks_through: IsoDateTime,
    #[serde(rename = "remarkIds")]
    pub remark_ids: Vec<TrimmedNonEmptyString>,
    #[serde(rename = "conflicting")]
    pub conflicting: bool,
    #[serde(rename = "wakes")]
    pub wakes: NonNegativeInt,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThreadTitleRegeneration {
    #[serde(rename = "requestId")]
    pub request_id: CommandId,
    #[serde(rename = "startedAt")]
    pub started_at: IsoDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThreadTokenUsageSnapshotCost {
    #[serde(rename = "amount")]
    pub amount: JsonNumber,
    #[serde(rename = "currency")]
    pub currency: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThreadTokenUsageSnapshot {
    #[serde(rename = "usedTokens")]
    pub used_tokens: NonNegativeInt,
    #[serde(
        rename = "totalProcessedTokens",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub total_processed_tokens: Optional<NonNegativeInt>,
    #[serde(
        rename = "maxTokens",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub max_tokens: Optional<PositiveInt>,
    #[serde(
        rename = "inputTokens",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub input_tokens: Optional<NonNegativeInt>,
    #[serde(
        rename = "cachedInputTokens",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub cached_input_tokens: Optional<NonNegativeInt>,
    #[serde(
        rename = "outputTokens",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub output_tokens: Optional<NonNegativeInt>,
    #[serde(
        rename = "reasoningOutputTokens",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub reasoning_output_tokens: Optional<NonNegativeInt>,
    #[serde(
        rename = "lastUsedTokens",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub last_used_tokens: Optional<NonNegativeInt>,
    #[serde(
        rename = "lastInputTokens",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub last_input_tokens: Optional<NonNegativeInt>,
    #[serde(
        rename = "lastCachedInputTokens",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub last_cached_input_tokens: Optional<NonNegativeInt>,
    #[serde(
        rename = "lastOutputTokens",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub last_output_tokens: Optional<NonNegativeInt>,
    #[serde(
        rename = "lastReasoningOutputTokens",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub last_reasoning_output_tokens: Optional<NonNegativeInt>,
    #[serde(
        rename = "toolUses",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_uses: Optional<NonNegativeInt>,
    #[serde(
        rename = "durationMs",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub duration_ms: Optional<NonNegativeInt>,
    #[serde(
        rename = "compactsAutomatically",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub compacts_automatically: Optional<bool>,
    #[serde(
        rename = "autoCompactThreshold",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub auto_compact_threshold: Optional<PositiveInt>,
    #[serde(rename = "cost", default, skip_serializing_if = "Optional::is_absent")]
    pub cost: Optional<ThreadTokenUsageSnapshotCost>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolActivityIconWebsite {
    #[serde(rename = "pageUrl")]
    pub page_url: String,
    #[serde(
        rename = "faviconUrl",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub favicon_url: Optional<String>,
    #[serde(
        rename = "faviconUrlDark",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub favicon_url_dark: Optional<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolActivityIconNativeApp {
    #[serde(rename = "app")]
    pub app: ToolActivityNativeAppReference,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolActivityIconThemedLogo {
    #[serde(rename = "logoUrl")]
    pub logo_url: String,
    #[serde(
        rename = "logoUrlDark",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub logo_url_dark: Optional<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "_tag")]
pub enum ToolActivityIcon {
    #[serde(rename = "website")]
    Website(Box<ToolActivityIconWebsite>),
    #[serde(rename = "native-app")]
    NativeApp(Box<ToolActivityIconNativeApp>),
    #[serde(rename = "themed-logo")]
    ThemedLogo(Box<ToolActivityIconThemedLogo>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolActivityNativeAppReferenceAppId {
    #[serde(rename = "appId")]
    pub app_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolActivityNativeAppReferenceDisplayName {
    #[serde(rename = "displayName")]
    pub display_name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "_tag")]
pub enum ToolActivityNativeAppReference {
    #[serde(rename = "app-id")]
    AppId(Box<ToolActivityNativeAppReferenceAppId>),
    #[serde(rename = "display-name")]
    DisplayName(Box<ToolActivityNativeAppReferenceDisplayName>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ToolActivitySourceKind {
    #[serde(rename = "browser")]
    Browser,
    #[serde(rename = "computer")]
    Computer,
    #[serde(rename = "integration")]
    Integration,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolActivitySource {
    #[serde(rename = "key")]
    pub key: String,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "kind")]
    pub kind: ToolActivitySourceKind,
    #[serde(rename = "icon", default, skip_serializing_if = "Optional::is_absent")]
    pub icon: Optional<ToolActivityIcon>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ToolActivitySurface {
    #[serde(rename = "browser")]
    Browser,
    #[serde(rename = "computer")]
    Computer,
}

pub type TrimmedNonEmptyString = String;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TurnItemId(pub String);
impl From<String> for TurnItemId {
    fn from(value: String) -> Self {
        Self(value)
    }
}
impl From<&str> for TurnItemId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}
impl AsRef<str> for TurnItemId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
impl std::fmt::Display for TurnItemId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TurnTokenUsageVariant1UsageScope {
    #[serde(rename = "main_agent")]
    MainAgent,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TurnTokenUsageVariant1UsageStatus {
    #[serde(rename = "complete")]
    Complete,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TurnTokenUsageVariant1 {
    #[serde(rename = "usageScope")]
    pub usage_scope: TurnTokenUsageVariant1UsageScope,
    #[serde(
        rename = "cachedInputTokens",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub cached_input_tokens: Optional<NonNegativeInt>,
    #[serde(
        rename = "cacheCreationTokens",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub cache_creation_tokens: Optional<NonNegativeInt>,
    #[serde(
        rename = "reasoningTokens",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub reasoning_tokens: Optional<NonNegativeInt>,
    #[serde(rename = "hasSubagents")]
    pub has_subagents: bool,
    #[serde(rename = "usageStatus")]
    pub usage_status: TurnTokenUsageVariant1UsageStatus,
    #[serde(rename = "inputTokens")]
    pub input_tokens: NonNegativeInt,
    #[serde(rename = "outputTokens")]
    pub output_tokens: NonNegativeInt,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TurnTokenUsageVariant2UsageScope {
    #[serde(rename = "main_agent")]
    MainAgent,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TurnTokenUsageVariant2UsageStatus {
    #[serde(rename = "partial")]
    Partial,
    #[serde(rename = "unavailable")]
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TurnTokenUsageVariant2 {
    #[serde(rename = "usageScope")]
    pub usage_scope: TurnTokenUsageVariant2UsageScope,
    #[serde(
        rename = "cachedInputTokens",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub cached_input_tokens: Optional<NonNegativeInt>,
    #[serde(
        rename = "cacheCreationTokens",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub cache_creation_tokens: Optional<NonNegativeInt>,
    #[serde(
        rename = "reasoningTokens",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub reasoning_tokens: Optional<NonNegativeInt>,
    #[serde(rename = "hasSubagents")]
    pub has_subagents: bool,
    #[serde(rename = "usageStatus")]
    pub usage_status: TurnTokenUsageVariant2UsageStatus,
    #[serde(
        rename = "inputTokens",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub input_tokens: Optional<NonNegativeInt>,
    #[serde(
        rename = "outputTokens",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub output_tokens: Optional<NonNegativeInt>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TurnTokenUsage {
    Variant1(TurnTokenUsageVariant1),
    Variant2(TurnTokenUsageVariant2),
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnknownContextRecordVersion;
impl Serialize for UnknownContextRecordVersion {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serde_json::json!(1).serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for UnknownContextRecordVersion {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        if value == serde_json::json!(1) {
            Ok(Self)
        } else {
            Err(serde::de::Error::custom("expected literal 1"))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnknownContextRecord {
    #[serde(rename = "version")]
    pub version: UnknownContextRecordVersion,
    #[serde(rename = "contextId")]
    pub context_id: ComposerContextId,
    #[serde(rename = "label")]
    pub label: String,
    #[serde(rename = "kind")]
    pub kind: String,
    #[serde(rename = "payload")]
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserInputAttachmentAnswerPayload {
    #[serde(rename = "requestId")]
    pub request_id: TrimmedNonEmptyString,
    #[serde(
        rename = "questionTextById",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub question_text_by_id: Optional<BTreeMap<IsoDateTime, IsoDateTime>>,
    #[serde(rename = "answers")]
    pub answers: ProviderUserInputAnswers,
    #[serde(rename = "attachmentsByQuestionId")]
    pub attachments_by_question_id: UserInputAttachments,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserInputAttachmentsValueValueItemImage {
    #[serde(rename = "id")]
    pub id: ChatAttachmentId,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    #[serde(rename = "sizeBytes")]
    pub size_bytes: i64,
    #[serde(
        rename = "source",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub source: Optional<SnapShotSource>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserInputAttachmentsValueValueItemFile {
    #[serde(rename = "id")]
    pub id: ChatAttachmentId,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    #[serde(rename = "sizeBytes")]
    pub size_bytes: i64,
    #[serde(
        rename = "source",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub source: Optional<PastedTextAttachmentSource>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum UserInputAttachmentsValueValueItem {
    #[serde(rename = "image")]
    Image(Box<UserInputAttachmentsValueValueItemImage>),
    #[serde(rename = "file")]
    File(Box<UserInputAttachmentsValueValueItemFile>),
}

pub type UserInputAttachments = BTreeMap<IsoDateTime, Vec<UserInputAttachmentsValueValueItem>>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VcsListRefsResult {
    #[serde(rename = "refs")]
    pub refs: Vec<VcsRef>,
    #[serde(rename = "isRepo")]
    pub is_repo: bool,
    #[serde(rename = "hasPrimaryRemote")]
    pub has_primary_remote: bool,
    #[serde(rename = "nextCursor", deserialize_with = "required_nullable")]
    pub next_cursor: Option<NonNegativeInt>,
    #[serde(rename = "totalCount")]
    pub total_count: NonNegativeInt,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VcsRef {
    #[serde(rename = "name")]
    pub name: TrimmedNonEmptyString,
    #[serde(
        rename = "isRemote",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub is_remote: Optional<bool>,
    #[serde(
        rename = "remoteName",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub remote_name: Optional<TrimmedNonEmptyString>,
    #[serde(rename = "current")]
    pub current: bool,
    #[serde(rename = "isDefault")]
    pub is_default: bool,
    #[serde(rename = "worktreePath", deserialize_with = "required_nullable")]
    pub worktree_path: Option<TrimmedNonEmptyString>,
}

/// Empty schema, used only for maps with no possible values (empty tool input).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Never {}

/// T3 delegated task identity is exactly its execution NodeId.
pub type TaskId = NodeId;
/// A queue entry is a queued Run, not a separately owned message lifecycle.
pub type QueueEntry = OrchestrationV2Run;
/// Shared runtime-policy interaction mode, also exposed by its T3 wire name.
pub use crate::runtime_policy::InteractionMode;

/// Effect's JSON number codec preserves non-finite JS numbers as named strings.
/// Plain serde f64 would serialize them as null and silently change the wire.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum JsonNumber {
    Finite(f64),
    Special(JsonNumberSpecial),
}
impl Serialize for JsonNumber {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Finite(value) if value.is_finite() => value.serialize(serializer),
            Self::Finite(value) if value.is_nan() => serializer.serialize_str("NaN"),
            Self::Finite(value) if value.is_sign_negative() => {
                serializer.serialize_str("-Infinity")
            }
            Self::Finite(_) => serializer.serialize_str("Infinity"),
            Self::Special(value) => value.serialize(serializer),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum JsonNumberSpecial {
    #[serde(rename = "Infinity")]
    Infinity,
    #[serde(rename = "-Infinity")]
    NegativeInfinity,
    #[serde(rename = "NaN")]
    NaN,
}
impl JsonNumber {
    pub fn as_f64(&self) -> f64 {
        match self {
            Self::Finite(value) => *value,
            Self::Special(JsonNumberSpecial::Infinity) => f64::INFINITY,
            Self::Special(JsonNumberSpecial::NegativeInfinity) => f64::NEG_INFINITY,
            Self::Special(JsonNumberSpecial::NaN) => f64::NAN,
        }
    }
}

/// Distinguishes an omitted key from a present value, including an explicit null
/// when T is Option<U>. `Option<Option<U>>` alone loses this distinction in serde.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Optional<T> {
    #[default]
    Absent,
    Present(T),
}
impl<T> Optional<T> {
    pub fn is_absent(&self) -> bool {
        matches!(self, Self::Absent)
    }
    pub fn as_ref(&self) -> Option<&T> {
        match self {
            Self::Absent => None,
            Self::Present(value) => Some(value),
        }
    }
}
impl<T: Serialize> Serialize for Optional<T> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Absent => serializer.serialize_unit(),
            Self::Present(value) => value.serialize(serializer),
        }
    }
}
impl<'de, T: Deserialize<'de>> Deserialize<'de> for Optional<T> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        T::deserialize(deserializer).map(Self::Present)
    }
}
/// A required nullable field must still be present. Derive's Option special
/// case otherwise accepts a missing key as null, contrary to the upstream shape.
pub fn required_nullable<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

/// Compatibility decoding only: no scheduling, permission, or engine policy.
pub fn normalize_contract(
    name: &str,
    mut value: serde_json::Value,
) -> Result<serde_json::Value, String> {
    use serde_json::{Value, json};
    match name {
        "ProviderOptionSelections" => return normalize_option_selections(value, false),
        "OrchestratorMcpTargetOptions" => return normalize_option_selections(value, true),
        "ModelSelection" => {
            let map = value
                .as_object_mut()
                .ok_or("model selection must be an object")?;
            if !map.contains_key("instanceId")
                && let Some(provider) = map.get("provider").and_then(Value::as_str)
            {
                map.insert("instanceId".into(), json!(provider));
            }
            trim_field(map, "model")?;
            if let Some(options) = map.get_mut("options") {
                *options = normalize_option_selections(options.take(), false)?;
            }
        }
        "OrchestratorMcpScheduleTaskInput" => {
            let map = value
                .as_object_mut()
                .ok_or("schedule input must be an object")?;
            trim_field(map, "prompt")?;
            if let Some(schedule) = map.get_mut("schedule") {
                if let Some(text) = schedule.as_str() {
                    *schedule = serde_json::from_str(text)
                        .map_err(|e| format!("invalid schedule JSON: {e}"))?;
                }
                *schedule = normalize_contract("ScheduledTaskUpsertSchedule", schedule.take())?;
            }
        }
        "ScheduledTaskSchedule" | "ScheduledTaskUpsertSchedule" => {
            let map = value.as_object_mut().ok_or("schedule must be an object")?;
            match map.get("type").and_then(Value::as_str) {
                Some("interval") => {
                    let interval = map
                        .get("everyMs")
                        .and_then(Value::as_i64)
                        .ok_or("everyMs must be an integer")?;
                    let min = if name == "ScheduledTaskUpsertSchedule" {
                        60000
                    } else {
                        1
                    };
                    if interval < min {
                        return Err(format!("everyMs must be >= {min}"));
                    }
                }
                Some("fixed_time") => {
                    trim_field(map, "timeOfDay")?;
                    let time = map
                        .get("timeOfDay")
                        .and_then(Value::as_str)
                        .ok_or("timeOfDay is required")?;
                    let parts: Vec<_> = time.split(':').collect();
                    if parts.len() != 2
                        || !(1..=2).contains(&parts[0].len())
                        || parts[1].len() != 2
                        || !parts.iter().all(|p| p.bytes().all(|c| c.is_ascii_digit()))
                        || parts[0].parse::<u8>().map_or(true, |h| h > 23)
                        || parts[1].parse::<u8>().map_or(true, |m| m > 59)
                    {
                        return Err("invalid timeOfDay".into());
                    }
                    if let Some(weekdays) = map.get("weekdays") {
                        let days = weekdays.as_array().ok_or("weekdays must be an array")?;
                        if days
                            .iter()
                            .any(|d| d.as_i64().is_none_or(|n| !(0..=6).contains(&n)))
                        {
                            return Err("weekday must be 0..6".into());
                        }
                    }
                }
                _ => return Err("unknown schedule type".into()),
            }
        }
        "OrchestrationV2NotificationSource" | "OrchestrationV2PendingBackgroundTask" => {
            let map = value.as_object_mut().ok_or("source must be an object")?;
            let kind = match map.get("kind") {
                None => "",
                Some(Value::String(s)) => s.as_str(),
                _ => return Err("kind must be a string or omitted".into()),
            };
            let normalized = if name == "OrchestrationV2NotificationSource" {
                match kind {
                    "background_command" => "command",
                    "background_task"
                        if map.get("work").and_then(Value::as_str) == Some("subagent") =>
                    {
                        "subagent"
                    }
                    "delegated_task" | "subagent" | "command" | "monitor" | "background_task" => {
                        kind
                    }
                    _ => "background_task",
                }
            } else {
                match kind {
                    "subagent" | "command" | "monitor" | "background_task" => kind,
                    _ => "background_task",
                }
            }
            .to_owned();
            map.insert("kind".into(), json!(normalized));
        }
        _ => {}
    }
    Ok(value)
}

fn trim_field(
    map: &mut serde_json::Map<String, serde_json::Value>,
    field: &str,
) -> Result<(), String> {
    if let Some(value) = map.get_mut(field) {
        let trimmed = value
            .as_str()
            .ok_or_else(|| format!("{field} must be a string"))?
            .trim()
            .to_owned();
        if trimmed.is_empty() {
            return Err(format!("{field} must be nonempty"));
        }
        *value = serde_json::json!(trimmed);
    }
    Ok(())
}

fn normalize_option_selections(
    value: serde_json::Value,
    strict: bool,
) -> Result<serde_json::Value, String> {
    use serde_json::{Value, json};
    let mut selections = Vec::new();
    match value {
        Value::Array(values) => {
            for mut entry in values {
                let map = entry.as_object_mut().ok_or("option must be an object")?;
                trim_field(map, "id")?;
                if map.get("value").is_some_and(Value::is_string) {
                    trim_field(map, "value")?;
                }
                selections.push(entry);
            }
        }
        Value::Object(map) => {
            for (key, value) in map {
                let id = key.trim();
                if id.is_empty() {
                    if strict {
                        return Err("empty option id".into());
                    }
                    continue;
                }
                match value {
                    Value::String(s) if !s.trim().is_empty() => {
                        selections.push(json!({"id":id,"value":s.trim()}))
                    }
                    Value::Bool(b) => selections.push(json!({"id":id,"value":b})),
                    _ if strict => {
                        return Err("option value must be a nonempty string or boolean".into());
                    }
                    _ => {}
                }
            }
        }
        _ => return Err("options must be an array or object".into()),
    }
    Ok(Value::Array(selections))
}

fn encode_notification_source(mut value: serde_json::Value) -> serde_json::Value {
    match value.get("kind").and_then(serde_json::Value::as_str) {
        Some("subagent") => {
            value["kind"] = serde_json::json!("background_task");
            value["work"] = serde_json::json!("subagent");
        }
        Some("command") => value["kind"] = serde_json::json!("background_command"),
        _ => {}
    }
    value
}

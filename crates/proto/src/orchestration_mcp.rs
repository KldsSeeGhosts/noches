//! Generated T3 V2 wire contracts. See docs/orchestration/contracts.md.
//! Regenerate: node crates/proto/tests/t3_oracle/generate.mjs
//! Source: T3 Tools Inc., MIT, pinned in tests/t3_oracle/fixtures/provenance.json.
#![allow(unused_imports)]
use crate::orchestration::*;
use crate::provider_instance::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CreateThreadsError {
    Variant1(ToolFrameworkAiError),
    Variant2(OrchestratorMcpFailure),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CreateThreadsInputThreadsItemTargetOptionsVariant1ItemValue {
    Variant1(String),
    Variant2(bool),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateThreadsInputThreadsItemTargetOptionsVariant1Item {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "value")]
    pub value: CreateThreadsInputThreadsItemTargetOptionsVariant1ItemValue,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CreateThreadsInputThreadsItemTargetOptionsVariant2Value {
    Variant1(String),
    Variant2(bool),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CreateThreadsInputThreadsItemTargetOptions {
    Variant1(Vec<CreateThreadsInputThreadsItemTargetOptionsVariant1Item>),
    Variant2(BTreeMap<IsoDateTime, CreateThreadsInputThreadsItemTargetOptionsVariant2Value>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateThreadsInputThreadsItemTarget {
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<String>,
    #[serde(
        rename = "driverKind",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver_kind: Optional<String>,
    #[serde(rename = "model", default, skip_serializing_if = "Optional::is_absent")]
    pub model: Optional<String>,
    #[serde(
        rename = "options",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub options: Optional<CreateThreadsInputThreadsItemTargetOptions>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateThreadsInputThreadsItem {
    #[serde(
        rename = "prompt",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub prompt: Optional<String>,
    #[serde(rename = "title", default, skip_serializing_if = "Optional::is_absent")]
    pub title: Optional<String>,
    #[serde(
        rename = "target",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub target: Optional<CreateThreadsInputThreadsItemTarget>,
    #[serde(
        rename = "runtimeMode",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub runtime_mode: Optional<OrchestratorMcpRuntimeMode>,
    #[serde(
        rename = "interactionMode",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub interaction_mode: Optional<OrchestratorMcpInteractionMode>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateThreadsInput {
    #[serde(rename = "threads")]
    pub threads: Vec<CreateThreadsInputThreadsItem>,
    #[serde(
        rename = "clientRequestId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub client_request_id: Optional<String>,
}

pub type CreateThreadsResult = OrchestratorMcpCreateThreadsResult;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DelegateTaskError {
    Variant1(ToolFrameworkAiError),
    Variant2(OrchestratorMcpFailure),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DelegateTaskInputTargetOptionsVariant1ItemValue {
    Variant1(String),
    Variant2(bool),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DelegateTaskInputTargetOptionsVariant1Item {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "value")]
    pub value: DelegateTaskInputTargetOptionsVariant1ItemValue,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DelegateTaskInputTargetOptionsVariant2Value {
    Variant1(String),
    Variant2(bool),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DelegateTaskInputTargetOptions {
    Variant1(Vec<DelegateTaskInputTargetOptionsVariant1Item>),
    Variant2(BTreeMap<IsoDateTime, DelegateTaskInputTargetOptionsVariant2Value>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DelegateTaskInputTarget {
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<String>,
    #[serde(
        rename = "driverKind",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver_kind: Optional<String>,
    #[serde(rename = "model", default, skip_serializing_if = "Optional::is_absent")]
    pub model: Optional<String>,
    #[serde(
        rename = "options",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub options: Optional<DelegateTaskInputTargetOptions>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DelegateTaskInputMode {
    #[serde(rename = "async")]
    Async,
    #[serde(rename = "wait")]
    Wait,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DelegateTaskInput {
    #[serde(rename = "task")]
    pub task: String,
    #[serde(
        rename = "target",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub target: Optional<DelegateTaskInputTarget>,
    #[serde(rename = "title", default, skip_serializing_if = "Optional::is_absent")]
    pub title: Optional<String>,
    #[serde(rename = "role", default, skip_serializing_if = "Optional::is_absent")]
    pub role: Optional<OrchestratorMcpTaskRole>,
    #[serde(rename = "mode", default, skip_serializing_if = "Optional::is_absent")]
    pub mode: Optional<DelegateTaskInputMode>,
    #[serde(
        rename = "timeoutMs",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub timeout_ms: Optional<JsonNumber>,
    #[serde(
        rename = "clientRequestId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub client_request_id: Optional<String>,
    #[serde(
        rename = "runtimeMode",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub runtime_mode: Optional<OrchestratorMcpRuntimeMode>,
    #[serde(
        rename = "interactionMode",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub interaction_mode: Optional<OrchestratorMcpInteractionMode>,
}

pub type DelegateTaskResult = OrchestratorMcpDelegateTaskResult;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DeleteScheduledTaskError {
    Variant1(ToolFrameworkAiError),
    Variant2(OrchestratorMcpFailure),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeleteScheduledTaskInput {
    #[serde(rename = "scheduledTaskId")]
    pub scheduled_task_id: String,
}

pub type DeleteScheduledTaskResult = OrchestratorMcpDeleteScheduledTaskResult;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DeviceCloseError {
    Variant1(ToolFrameworkAiError),
    Variant2(DeviceToolError),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceCloseInput {
    #[serde(
        rename = "deviceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub device_id: Optional<String>,
    #[serde(
        rename = "hostId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub host_id: Optional<String>,
    #[serde(
        rename = "shutdown",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub shutdown: Optional<bool>,
}

pub type DeviceCloseResult = BTreeMap<IsoDateTime, Never>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DeviceListError {
    Variant1(ToolFrameworkAiError),
    Variant2(DeviceToolError),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceListInput {
    #[serde(
        rename = "hostId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub host_id: Optional<String>,
}

pub type DeviceListResult = DeviceToolListResult;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DeviceOpenError {
    Variant1(ToolFrameworkAiError),
    Variant2(DeviceToolError),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeviceOpenInputPlatform {
    #[serde(rename = "ios")]
    Ios,
    #[serde(rename = "android")]
    Android,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceOpenInput {
    #[serde(
        rename = "deviceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub device_id: Optional<String>,
    #[serde(
        rename = "platform",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub platform: Optional<DeviceOpenInputPlatform>,
    #[serde(
        rename = "hostId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub host_id: Optional<String>,
}

pub type DeviceOpenResult = DeviceToolOpenResult;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DeviceScreenshotError {
    Variant1(ToolFrameworkAiError),
    Variant2(DeviceToolError),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceScreenshotInput {
    #[serde(
        rename = "deviceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub device_id: Optional<String>,
    #[serde(
        rename = "hostId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub host_id: Optional<String>,
}

pub type DeviceScreenshotResult = DeviceToolScreenshotResult;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LinkPullRequestErrorVariant3Tag {
    #[serde(rename = "PullRequestUrlInvalidError")]
    PullRequestUrlInvalidError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LinkPullRequestErrorVariant3 {
    #[serde(rename = "_tag")]
    pub _tag: LinkPullRequestErrorVariant3Tag,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LinkPullRequestErrorVariant4Tag {
    #[serde(rename = "PullRequestTargetIncompleteError")]
    PullRequestTargetIncompleteError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LinkPullRequestErrorVariant4 {
    #[serde(rename = "_tag")]
    pub _tag: LinkPullRequestErrorVariant4Tag,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LinkPullRequestErrorVariant5Tag {
    #[serde(rename = "PullRequestHostRequiredError")]
    PullRequestHostRequiredError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LinkPullRequestErrorVariant5 {
    #[serde(rename = "_tag")]
    pub _tag: LinkPullRequestErrorVariant5Tag,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LinkPullRequestErrorVariant6Tag {
    #[serde(rename = "PullRequestThreadNotFoundError")]
    PullRequestThreadNotFoundError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LinkPullRequestErrorVariant6 {
    #[serde(rename = "_tag")]
    pub _tag: LinkPullRequestErrorVariant6Tag,
    #[serde(rename = "threadId")]
    pub thread_id: IsoDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LinkPullRequestErrorVariant7Tag {
    #[serde(rename = "PullRequestLinkFailedError")]
    PullRequestLinkFailedError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LinkPullRequestErrorVariant7 {
    #[serde(rename = "_tag")]
    pub _tag: LinkPullRequestErrorVariant7Tag,
    #[serde(rename = "cause")]
    pub cause: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LinkPullRequestErrorVariant8Tag {
    #[serde(rename = "PullRequestUnlinkFailedError")]
    PullRequestUnlinkFailedError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LinkPullRequestErrorVariant8 {
    #[serde(rename = "_tag")]
    pub _tag: LinkPullRequestErrorVariant8Tag,
    #[serde(rename = "cause")]
    pub cause: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LinkPullRequestErrorVariant9Tag {
    #[serde(rename = "PullRequestListFailedError")]
    PullRequestListFailedError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LinkPullRequestErrorVariant9 {
    #[serde(rename = "_tag")]
    pub _tag: LinkPullRequestErrorVariant9Tag,
    #[serde(rename = "cause")]
    pub cause: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LinkPullRequestErrorVariant10Tag {
    #[serde(rename = "PullRequestWatchFailedError")]
    PullRequestWatchFailedError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LinkPullRequestErrorVariant10 {
    #[serde(rename = "_tag")]
    pub _tag: LinkPullRequestErrorVariant10Tag,
    #[serde(rename = "cause")]
    pub cause: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LinkPullRequestErrorVariant11Tag {
    #[serde(rename = "PullRequestNotOpenError")]
    PullRequestNotOpenError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LinkPullRequestErrorVariant11 {
    #[serde(rename = "_tag")]
    pub _tag: LinkPullRequestErrorVariant11Tag,
    #[serde(rename = "state")]
    pub state: IsoDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LinkPullRequestError {
    Variant1(ToolFrameworkAiError),
    Variant2(McpCapabilityUnavailableError),
    Variant3(LinkPullRequestErrorVariant3),
    Variant4(LinkPullRequestErrorVariant4),
    Variant5(LinkPullRequestErrorVariant5),
    Variant6(LinkPullRequestErrorVariant6),
    Variant7(LinkPullRequestErrorVariant7),
    Variant8(LinkPullRequestErrorVariant8),
    Variant9(LinkPullRequestErrorVariant9),
    Variant10(LinkPullRequestErrorVariant10),
    Variant11(LinkPullRequestErrorVariant11),
    Variant12(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LinkPullRequestInput {
    #[serde(rename = "url", default, skip_serializing_if = "Optional::is_absent")]
    pub url: Optional<String>,
    #[serde(
        rename = "repository",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub repository: Optional<String>,
    #[serde(
        rename = "number",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub number: Optional<i64>,
    #[serde(rename = "host", default, skip_serializing_if = "Optional::is_absent")]
    pub host: Optional<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LinkPullRequestResult {
    #[serde(rename = "host")]
    pub host: IsoDateTime,
    #[serde(rename = "repository")]
    pub repository: IsoDateTime,
    #[serde(rename = "number")]
    pub number: i64,
    #[serde(rename = "url")]
    pub url: IsoDateTime,
    #[serde(rename = "alreadyLinked")]
    pub already_linked: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ListScheduledTasksError {
    Variant1(ToolFrameworkAiError),
    Variant2(OrchestratorMcpFailure),
    Variant3(ToolFrameworkExecutionFailure),
}

pub type ListScheduledTasksInput = BTreeMap<IsoDateTime, Never>;

pub type ListScheduledTasksResult = OrchestratorMcpListScheduledTasksResult;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ListThreadPullRequestsErrorVariant3Tag {
    #[serde(rename = "PullRequestUrlInvalidError")]
    PullRequestUrlInvalidError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListThreadPullRequestsErrorVariant3 {
    #[serde(rename = "_tag")]
    pub _tag: ListThreadPullRequestsErrorVariant3Tag,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ListThreadPullRequestsErrorVariant4Tag {
    #[serde(rename = "PullRequestTargetIncompleteError")]
    PullRequestTargetIncompleteError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListThreadPullRequestsErrorVariant4 {
    #[serde(rename = "_tag")]
    pub _tag: ListThreadPullRequestsErrorVariant4Tag,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ListThreadPullRequestsErrorVariant5Tag {
    #[serde(rename = "PullRequestHostRequiredError")]
    PullRequestHostRequiredError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListThreadPullRequestsErrorVariant5 {
    #[serde(rename = "_tag")]
    pub _tag: ListThreadPullRequestsErrorVariant5Tag,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ListThreadPullRequestsErrorVariant6Tag {
    #[serde(rename = "PullRequestThreadNotFoundError")]
    PullRequestThreadNotFoundError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListThreadPullRequestsErrorVariant6 {
    #[serde(rename = "_tag")]
    pub _tag: ListThreadPullRequestsErrorVariant6Tag,
    #[serde(rename = "threadId")]
    pub thread_id: IsoDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ListThreadPullRequestsErrorVariant7Tag {
    #[serde(rename = "PullRequestLinkFailedError")]
    PullRequestLinkFailedError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListThreadPullRequestsErrorVariant7 {
    #[serde(rename = "_tag")]
    pub _tag: ListThreadPullRequestsErrorVariant7Tag,
    #[serde(rename = "cause")]
    pub cause: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ListThreadPullRequestsErrorVariant8Tag {
    #[serde(rename = "PullRequestUnlinkFailedError")]
    PullRequestUnlinkFailedError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListThreadPullRequestsErrorVariant8 {
    #[serde(rename = "_tag")]
    pub _tag: ListThreadPullRequestsErrorVariant8Tag,
    #[serde(rename = "cause")]
    pub cause: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ListThreadPullRequestsErrorVariant9Tag {
    #[serde(rename = "PullRequestListFailedError")]
    PullRequestListFailedError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListThreadPullRequestsErrorVariant9 {
    #[serde(rename = "_tag")]
    pub _tag: ListThreadPullRequestsErrorVariant9Tag,
    #[serde(rename = "cause")]
    pub cause: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ListThreadPullRequestsErrorVariant10Tag {
    #[serde(rename = "PullRequestWatchFailedError")]
    PullRequestWatchFailedError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListThreadPullRequestsErrorVariant10 {
    #[serde(rename = "_tag")]
    pub _tag: ListThreadPullRequestsErrorVariant10Tag,
    #[serde(rename = "cause")]
    pub cause: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ListThreadPullRequestsErrorVariant11Tag {
    #[serde(rename = "PullRequestNotOpenError")]
    PullRequestNotOpenError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListThreadPullRequestsErrorVariant11 {
    #[serde(rename = "_tag")]
    pub _tag: ListThreadPullRequestsErrorVariant11Tag,
    #[serde(rename = "state")]
    pub state: IsoDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ListThreadPullRequestsError {
    Variant1(ToolFrameworkAiError),
    Variant2(McpCapabilityUnavailableError),
    Variant3(ListThreadPullRequestsErrorVariant3),
    Variant4(ListThreadPullRequestsErrorVariant4),
    Variant5(ListThreadPullRequestsErrorVariant5),
    Variant6(ListThreadPullRequestsErrorVariant6),
    Variant7(ListThreadPullRequestsErrorVariant7),
    Variant8(ListThreadPullRequestsErrorVariant8),
    Variant9(ListThreadPullRequestsErrorVariant9),
    Variant10(ListThreadPullRequestsErrorVariant10),
    Variant11(ListThreadPullRequestsErrorVariant11),
    Variant12(ToolFrameworkExecutionFailure),
}

pub type ListThreadPullRequestsInput = BTreeMap<IsoDateTime, Never>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ListThreadPullRequestsResultPullRequestsItemStackKind {
    #[serde(rename = "native")]
    Native,
    #[serde(rename = "derived")]
    Derived,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListThreadPullRequestsResultPullRequestsItemStack {
    #[serde(rename = "kind")]
    pub kind: ListThreadPullRequestsResultPullRequestsItemStackKind,
    #[serde(rename = "position")]
    pub position: i64,
    #[serde(rename = "size")]
    pub size: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListThreadPullRequestsResultPullRequestsItem {
    #[serde(rename = "host")]
    pub host: IsoDateTime,
    #[serde(rename = "repository")]
    pub repository: IsoDateTime,
    #[serde(rename = "number")]
    pub number: i64,
    #[serde(rename = "url")]
    pub url: IsoDateTime,
    #[serde(rename = "source")]
    pub source: ThreadPullRequestLinkSource,
    #[serde(rename = "watching")]
    pub watching: bool,
    #[serde(rename = "state", deserialize_with = "required_nullable")]
    pub state: Option<PullRequestState>,
    #[serde(rename = "title", deserialize_with = "required_nullable")]
    pub title: Option<IsoDateTime>,
    #[serde(rename = "headBranch", deserialize_with = "required_nullable")]
    pub head_branch: Option<IsoDateTime>,
    #[serde(rename = "baseBranch", deserialize_with = "required_nullable")]
    pub base_branch: Option<IsoDateTime>,
    #[serde(rename = "isDraft", deserialize_with = "required_nullable")]
    pub is_draft: Option<bool>,
    #[serde(rename = "stack", deserialize_with = "required_nullable")]
    pub stack: Option<ListThreadPullRequestsResultPullRequestsItemStack>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ListThreadPullRequestsResultChainsItemKind {
    #[serde(rename = "native")]
    Native,
    #[serde(rename = "derived")]
    Derived,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListThreadPullRequestsResultChainsItem {
    #[serde(rename = "kind")]
    pub kind: ListThreadPullRequestsResultChainsItemKind,
    #[serde(rename = "numbers")]
    pub numbers: Vec<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListThreadPullRequestsResult {
    #[serde(rename = "pullRequests")]
    pub pull_requests: Vec<ListThreadPullRequestsResultPullRequestsItem>,
    #[serde(rename = "chains")]
    pub chains: Vec<ListThreadPullRequestsResultChainsItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum McpCapabilityUnavailableErrorTag {
    #[serde(rename = "McpCapabilityUnavailableError")]
    McpCapabilityUnavailableError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpCapabilityUnavailableError {
    #[serde(rename = "_tag")]
    pub _tag: McpCapabilityUnavailableErrorTag,
    #[serde(rename = "capability")]
    pub capability: TrimmedNonEmptyString,
    #[serde(rename = "environmentId")]
    pub environment_id: EnvironmentId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "providerSessionId")]
    pub provider_session_id: TrimmedNonEmptyString,
    #[serde(rename = "providerInstanceId")]
    pub provider_instance_id: ProviderInstanceId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OrchestratorCapabilitiesError {
    Variant1(ToolFrameworkAiError),
    Variant2(OrchestratorMcpFailure),
    Variant3(ToolFrameworkExecutionFailure),
}

pub type OrchestratorCapabilitiesInput = BTreeMap<IsoDateTime, Never>;

pub type OrchestratorCapabilitiesResult = OrchestratorMcpCapabilitiesResult;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestratorMcpCapabilitiesResultFeatures {
    #[serde(rename = "appOwnedSubagents")]
    pub app_owned_subagents: bool,
    #[serde(rename = "asyncPolling")]
    pub async_polling: bool,
    #[serde(rename = "cancellation")]
    pub cancellation: bool,
    #[serde(rename = "batchThreadCreation")]
    pub batch_thread_creation: bool,
    #[serde(rename = "threadManagement")]
    pub thread_management: bool,
    #[serde(rename = "incrementalThreadRead")]
    pub incremental_thread_read: bool,
    #[serde(rename = "scheduledTasks")]
    pub scheduled_tasks: bool,
    #[serde(rename = "maxBatchThreads")]
    pub max_batch_threads: JsonNumber,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestratorMcpCapabilitiesResult {
    #[serde(rename = "parentThreadId")]
    pub parent_thread_id: ThreadId,
    #[serde(rename = "inheritedProviderInstanceId")]
    pub inherited_provider_instance_id: ProviderInstanceId,
    #[serde(rename = "inheritedModel")]
    pub inherited_model: IsoDateTime,
    #[serde(rename = "runtimeMode")]
    pub runtime_mode: RuntimeMode,
    #[serde(rename = "interactionMode")]
    pub interaction_mode: ProviderInteractionMode,
    #[serde(rename = "providers")]
    pub providers: Vec<OrchestratorMcpProviderCapability>,
    #[serde(rename = "features")]
    pub features: OrchestratorMcpCapabilitiesResultFeatures,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestratorMcpCreateThreadRequest {
    #[serde(
        rename = "prompt",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub prompt: Optional<String>,
    #[serde(rename = "title", default, skip_serializing_if = "Optional::is_absent")]
    pub title: Optional<String>,
    #[serde(
        rename = "target",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub target: Optional<OrchestratorMcpTarget>,
    #[serde(
        rename = "runtimeMode",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub runtime_mode: Optional<OrchestratorMcpRuntimeMode>,
    #[serde(
        rename = "interactionMode",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub interaction_mode: Optional<OrchestratorMcpInteractionMode>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestratorMcpCreateThreadsInput {
    #[serde(rename = "threads")]
    pub threads: Vec<OrchestratorMcpCreateThreadRequest>,
    #[serde(
        rename = "clientRequestId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub client_request_id: Optional<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestratorMcpCreateThreadsResult {
    #[serde(rename = "threads")]
    pub threads: Vec<OrchestratorMcpCreatedThread>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestratorMcpCreatedThread {
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "status")]
    pub status: OrchestratorMcpCreatedThreadStatus,
    #[serde(rename = "title")]
    pub title: IsoDateTime,
    #[serde(rename = "createdBy")]
    pub created_by: OrchestrationV2Actor,
    #[serde(rename = "creationSource")]
    pub creation_source: OrchestrationV2CreationSource,
    #[serde(rename = "providerInstanceId")]
    pub provider_instance_id: ProviderInstanceId,
    #[serde(rename = "model")]
    pub model: IsoDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestratorMcpCreatedThreadStatus {
    #[serde(rename = "idle")]
    Idle,
    #[serde(rename = "preparing")]
    Preparing,
    #[serde(rename = "starting")]
    Starting,
    #[serde(rename = "queued")]
    Queued,
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
    #[serde(rename = "rolled_back")]
    RolledBack,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestratorMcpDelegateTaskInputMode {
    #[serde(rename = "async")]
    Async,
    #[serde(rename = "wait")]
    Wait,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestratorMcpDelegateTaskInput {
    #[serde(rename = "task")]
    pub task: String,
    #[serde(
        rename = "target",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub target: Optional<OrchestratorMcpTarget>,
    #[serde(rename = "title", default, skip_serializing_if = "Optional::is_absent")]
    pub title: Optional<String>,
    #[serde(rename = "role", default, skip_serializing_if = "Optional::is_absent")]
    pub role: Optional<OrchestratorMcpTaskRole>,
    #[serde(rename = "mode", default, skip_serializing_if = "Optional::is_absent")]
    pub mode: Optional<OrchestratorMcpDelegateTaskInputMode>,
    #[serde(
        rename = "timeoutMs",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub timeout_ms: Optional<JsonNumber>,
    #[serde(
        rename = "clientRequestId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub client_request_id: Optional<String>,
    #[serde(
        rename = "runtimeMode",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub runtime_mode: Optional<OrchestratorMcpRuntimeMode>,
    #[serde(
        rename = "interactionMode",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub interaction_mode: Optional<OrchestratorMcpInteractionMode>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestratorMcpDelegateTaskResultWorkState {
    #[serde(rename = "working")]
    Working,
    #[serde(rename = "waiting_for_children")]
    WaitingForChildren,
    #[serde(rename = "result_available")]
    ResultAvailable,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestratorMcpDelegateTaskResult {
    #[serde(rename = "taskId")]
    pub task_id: NodeId,
    #[serde(rename = "childThreadId")]
    pub child_thread_id: ThreadId,
    #[serde(rename = "childRunId", deserialize_with = "required_nullable")]
    pub child_run_id: Option<RunId>,
    #[serde(rename = "childNodeId")]
    pub child_node_id: NodeId,
    #[serde(rename = "status")]
    pub status: OrchestratorMcpDelegatedTaskStatus,
    #[serde(rename = "workState")]
    pub work_state: OrchestratorMcpDelegateTaskResultWorkState,
    #[serde(rename = "hasPendingChildRuns")]
    pub has_pending_child_runs: bool,
    #[serde(rename = "latestTerminalRunId", deserialize_with = "required_nullable")]
    pub latest_terminal_run_id: Option<RunId>,
    #[serde(
        rename = "latestTerminalStatus",
        deserialize_with = "required_nullable"
    )]
    pub latest_terminal_status: Option<OrchestratorMcpTerminalDelegatedTaskStatus>,
    #[serde(
        rename = "latestTerminalSummary",
        deserialize_with = "required_nullable"
    )]
    pub latest_terminal_summary: Option<IsoDateTime>,
    #[serde(
        rename = "latestTerminalResultContextTransferId",
        deserialize_with = "required_nullable"
    )]
    pub latest_terminal_result_context_transfer_id: Option<ContextTransferId>,
    #[serde(rename = "providerInstanceId")]
    pub provider_instance_id: ProviderInstanceId,
    #[serde(rename = "model", deserialize_with = "required_nullable")]
    pub model: Option<IsoDateTime>,
    #[serde(rename = "summary", deserialize_with = "required_nullable")]
    pub summary: Option<IsoDateTime>,
    #[serde(
        rename = "resultContextTransferId",
        deserialize_with = "required_nullable"
    )]
    pub result_context_transfer_id: Option<ContextTransferId>,
    #[serde(rename = "waitTimedOut")]
    pub wait_timed_out: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestratorMcpDelegatedTaskStatus {
    #[serde(rename = "queued")]
    Queued,
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
pub struct OrchestratorMcpDeleteScheduledTaskInput {
    #[serde(rename = "scheduledTaskId")]
    pub scheduled_task_id: ScheduledTaskId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestratorMcpDeleteScheduledTaskResult {
    #[serde(rename = "scheduledTaskId")]
    pub scheduled_task_id: ScheduledTaskId,
    #[serde(rename = "deleted")]
    pub deleted: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestratorMcpFailureTag {
    #[serde(rename = "OrchestratorMcpFailure")]
    OrchestratorMcpFailure,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestratorMcpFailureCode {
    #[serde(rename = "capability_denied")]
    CapabilityDenied,
    #[serde(rename = "parent_not_active")]
    ParentNotActive,
    #[serde(rename = "provider_unavailable")]
    ProviderUnavailable,
    #[serde(rename = "model_unavailable")]
    ModelUnavailable,
    #[serde(rename = "runtime_mode_escalation_denied")]
    RuntimeModeEscalationDenied,
    #[serde(rename = "interaction_mode_escalation_denied")]
    InteractionModeEscalationDenied,
    #[serde(rename = "task_not_found")]
    TaskNotFound,
    #[serde(rename = "task_not_cancellable")]
    TaskNotCancellable,
    #[serde(rename = "thread_not_found")]
    ThreadNotFound,
    #[serde(rename = "run_not_found")]
    RunNotFound,
    #[serde(rename = "thread_not_sendable")]
    ThreadNotSendable,
    #[serde(rename = "thread_not_interruptible")]
    ThreadNotInterruptible,
    #[serde(rename = "invalid_request")]
    InvalidRequest,
    #[serde(rename = "orchestration_error")]
    OrchestrationError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestratorMcpFailure {
    #[serde(rename = "_tag")]
    pub _tag: OrchestratorMcpFailureTag,
    #[serde(rename = "code")]
    pub code: OrchestratorMcpFailureCode,
    #[serde(rename = "message")]
    pub message: IsoDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestratorMcpInteractionMode {
    #[serde(rename = "inherit")]
    Inherit,
    #[serde(rename = "default")]
    Default,
    #[serde(rename = "plan")]
    Plan,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestratorMcpListScheduledTasksResult {
    #[serde(rename = "tasks")]
    pub tasks: Vec<OrchestratorMcpScheduleTaskResult>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestratorMcpRuntimeMode {
    #[serde(rename = "inherit")]
    Inherit,
    #[serde(rename = "approval-required")]
    ApprovalRequired,
    #[serde(rename = "auto-accept-edits")]
    AutoAcceptEdits,
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "full-access")]
    FullAccess,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestratorMcpScheduleTaskInputScheduleInterval {
    #[serde(rename = "everyMs")]
    pub every_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestratorMcpScheduleTaskInputScheduleFixedTime {
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
pub enum OrchestratorMcpScheduleTaskInputSchedule {
    #[serde(rename = "interval")]
    Interval(Box<OrchestratorMcpScheduleTaskInputScheduleInterval>),
    #[serde(rename = "fixed_time")]
    FixedTime(Box<OrchestratorMcpScheduleTaskInputScheduleFixedTime>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct OrchestratorMcpScheduleTaskInputCanonical {
    #[serde(rename = "prompt")]
    pub prompt: String,
    #[serde(rename = "schedule")]
    pub schedule: OrchestratorMcpScheduleTaskInputSchedule,
    #[serde(rename = "title", default, skip_serializing_if = "Optional::is_absent")]
    pub title: Optional<String>,
    #[serde(
        rename = "enabled",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub enabled: Optional<bool>,
    #[serde(
        rename = "bindToCurrentThread",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub bind_to_current_thread: Optional<bool>,
    #[serde(
        rename = "clientRequestId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub client_request_id: Optional<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "serde_json::Value")]
pub struct OrchestratorMcpScheduleTaskInput {
    #[serde(rename = "prompt")]
    pub prompt: String,
    #[serde(rename = "schedule")]
    pub schedule: OrchestratorMcpScheduleTaskInputSchedule,
    #[serde(rename = "title", default, skip_serializing_if = "Optional::is_absent")]
    pub title: Optional<String>,
    #[serde(
        rename = "enabled",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub enabled: Optional<bool>,
    #[serde(
        rename = "bindToCurrentThread",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub bind_to_current_thread: Optional<bool>,
    #[serde(
        rename = "clientRequestId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub client_request_id: Optional<String>,
}
impl TryFrom<serde_json::Value> for OrchestratorMcpScheduleTaskInput {
    type Error = String;
    fn try_from(value: serde_json::Value) -> Result<Self, String> {
        let value: OrchestratorMcpScheduleTaskInputCanonical = serde_json::from_value(
            normalize_contract("OrchestratorMcpScheduleTaskInput", value)?,
        )
        .map_err(|e| e.to_string())?;
        Ok(Self {
            prompt: value.prompt,
            schedule: value.schedule,
            title: value.title,
            enabled: value.enabled,
            bind_to_current_thread: value.bind_to_current_thread,
            client_request_id: value.client_request_id,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestratorMcpScheduleTaskResult {
    #[serde(rename = "scheduledTaskId")]
    pub scheduled_task_id: ScheduledTaskId,
    #[serde(rename = "title")]
    pub title: IsoDateTime,
    #[serde(rename = "prompt")]
    pub prompt: IsoDateTime,
    #[serde(rename = "enabled")]
    pub enabled: bool,
    #[serde(rename = "projectId")]
    pub project_id: ProjectId,
    #[serde(rename = "boundThreadId", deserialize_with = "required_nullable")]
    pub bound_thread_id: Option<ThreadId>,
    #[serde(rename = "schedule")]
    pub schedule: ScheduledTaskSchedule,
    #[serde(rename = "nextRunAt", deserialize_with = "required_nullable")]
    pub next_run_at: Option<IsoDateTime>,
    #[serde(rename = "lastRunStatus")]
    pub last_run_status: ScheduledTaskRunStatus,
}

pub type OrchestratorMcpScheduledTask = OrchestratorMcpScheduleTaskResult;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestratorMcpTarget {
    #[serde(
        rename = "providerInstanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider_instance_id: Optional<ProviderInstanceId>,
    #[serde(
        rename = "driverKind",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub driver_kind: Optional<ProviderDriverKind>,
    #[serde(rename = "model", default, skip_serializing_if = "Optional::is_absent")]
    pub model: Optional<String>,
    #[serde(
        rename = "options",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub options: Optional<Vec<ProviderOptionSelection>>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(try_from = "serde_json::Value")]
pub struct OrchestratorMcpTargetOptions(pub Vec<ProviderOptionSelection>);
impl TryFrom<serde_json::Value> for OrchestratorMcpTargetOptions {
    type Error = String;
    fn try_from(value: serde_json::Value) -> Result<Self, String> {
        serde_json::from_value(normalize_contract("OrchestratorMcpTargetOptions", value)?)
            .map(Self)
            .map_err(|e| e.to_string())
    }
}
impl Serialize for OrchestratorMcpTargetOptions {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestratorMcpTaskCancelInput {
    #[serde(rename = "taskId")]
    pub task_id: NodeId,
    #[serde(
        rename = "reason",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub reason: Optional<String>,
    #[serde(
        rename = "clientRequestId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub client_request_id: Optional<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestratorMcpTaskCancelResultStatus {
    #[serde(rename = "cancel_requested")]
    CancelRequested,
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
pub struct OrchestratorMcpTaskCancelResult {
    #[serde(rename = "taskId")]
    pub task_id: NodeId,
    #[serde(rename = "status")]
    pub status: OrchestratorMcpTaskCancelResultStatus,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestratorMcpTaskRole {
    #[serde(rename = "implementation")]
    Implementation,
    #[serde(rename = "research")]
    Research,
    #[serde(rename = "review")]
    Review,
    #[serde(rename = "design")]
    Design,
    #[serde(rename = "test")]
    Test,
    #[serde(rename = "general")]
    General,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestratorMcpTaskStatusInput {
    #[serde(rename = "taskId")]
    pub task_id: NodeId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestratorMcpTerminalDelegatedTaskStatus {
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
pub enum OrchestratorMcpThreadDetailRelationshipToParent {
    #[serde(rename = "fork")]
    Fork,
    #[serde(rename = "subagent")]
    Subagent,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestratorMcpThreadDetail {
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "projectId")]
    pub project_id: ProjectId,
    #[serde(rename = "title")]
    pub title: IsoDateTime,
    #[serde(rename = "createdBy")]
    pub created_by: OrchestrationV2Actor,
    #[serde(rename = "creationSource")]
    pub creation_source: OrchestrationV2CreationSource,
    #[serde(rename = "status")]
    pub status: OrchestratorMcpThreadStatus,
    #[serde(rename = "latestRunId", deserialize_with = "required_nullable")]
    pub latest_run_id: Option<RunId>,
    #[serde(rename = "activeRunId", deserialize_with = "required_nullable")]
    pub active_run_id: Option<RunId>,
    #[serde(rename = "providerInstanceId")]
    pub provider_instance_id: ProviderInstanceId,
    #[serde(rename = "model")]
    pub model: IsoDateTime,
    #[serde(rename = "runtimeMode")]
    pub runtime_mode: RuntimeMode,
    #[serde(rename = "interactionMode")]
    pub interaction_mode: ProviderInteractionMode,
    #[serde(rename = "linkedPullRequest", deserialize_with = "required_nullable")]
    pub linked_pull_request: Option<ThreadLinkedPullRequest>,
    #[serde(rename = "titleRegeneration", deserialize_with = "required_nullable")]
    pub title_regeneration: Option<ThreadTitleRegeneration>,
    #[serde(rename = "branch", deserialize_with = "required_nullable")]
    pub branch: Option<IsoDateTime>,
    #[serde(rename = "worktreePath", deserialize_with = "required_nullable")]
    pub worktree_path: Option<IsoDateTime>,
    #[serde(rename = "parentThreadId", deserialize_with = "required_nullable")]
    pub parent_thread_id: Option<ThreadId>,
    #[serde(
        rename = "relationshipToParent",
        deserialize_with = "required_nullable"
    )]
    pub relationship_to_parent: Option<OrchestratorMcpThreadDetailRelationshipToParent>,
    #[serde(rename = "runCount")]
    pub run_count: NonNegativeInt,
    #[serde(rename = "itemCount")]
    pub item_count: NonNegativeInt,
    #[serde(rename = "pendingRequestCount")]
    pub pending_request_count: NonNegativeInt,
    #[serde(rename = "archived")]
    pub archived: bool,
    #[serde(rename = "settled")]
    pub settled: bool,
    #[serde(rename = "settledAt", deserialize_with = "required_nullable")]
    pub settled_at: Option<IsoDateTime>,
    #[serde(rename = "createdAt")]
    pub created_at: IsoDateTime,
    #[serde(rename = "updatedAt")]
    pub updated_at: IsoDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestratorMcpThreadInterruptInput {
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "reason",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub reason: Optional<String>,
    #[serde(
        rename = "clientRequestId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub client_request_id: Optional<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestratorMcpThreadInterruptResultStatus {
    #[serde(rename = "interrupt_requested")]
    InterruptRequested,
    #[serde(rename = "no_active_run")]
    NoActiveRun,
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "cancelled")]
    Cancelled,
    #[serde(rename = "interrupted")]
    Interrupted,
    #[serde(rename = "rolled_back")]
    RolledBack,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestratorMcpThreadInterruptResult {
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "status")]
    pub status: OrchestratorMcpThreadInterruptResultStatus,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestratorMcpThreadListInput {
    #[serde(
        rename = "statuses",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub statuses: Optional<Vec<OrchestratorMcpThreadStatus>>,
    #[serde(
        rename = "titleContains",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub title_contains: Optional<String>,
    #[serde(
        rename = "settled",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub settled: Optional<bool>,
    #[serde(
        rename = "includeSubagents",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub include_subagents: Optional<bool>,
    #[serde(
        rename = "cursor",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub cursor: Optional<NonNegativeInt>,
    #[serde(rename = "limit", default, skip_serializing_if = "Optional::is_absent")]
    pub limit: Optional<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestratorMcpThreadListItemRelationshipToParent {
    #[serde(rename = "fork")]
    Fork,
    #[serde(rename = "subagent")]
    Subagent,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestratorMcpThreadListItem {
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "title")]
    pub title: IsoDateTime,
    #[serde(rename = "createdBy")]
    pub created_by: OrchestrationV2Actor,
    #[serde(rename = "creationSource")]
    pub creation_source: OrchestrationV2CreationSource,
    #[serde(rename = "status")]
    pub status: OrchestratorMcpThreadStatus,
    #[serde(rename = "latestRunId", deserialize_with = "required_nullable")]
    pub latest_run_id: Option<RunId>,
    #[serde(rename = "providerInstanceId")]
    pub provider_instance_id: ProviderInstanceId,
    #[serde(rename = "model")]
    pub model: IsoDateTime,
    #[serde(rename = "runtimeMode")]
    pub runtime_mode: RuntimeMode,
    #[serde(rename = "interactionMode")]
    pub interaction_mode: ProviderInteractionMode,
    #[serde(rename = "linkedPullRequest", deserialize_with = "required_nullable")]
    pub linked_pull_request: Option<ThreadLinkedPullRequest>,
    #[serde(rename = "settled")]
    pub settled: bool,
    #[serde(rename = "settledAt", deserialize_with = "required_nullable")]
    pub settled_at: Option<IsoDateTime>,
    #[serde(rename = "parentThreadId", deserialize_with = "required_nullable")]
    pub parent_thread_id: Option<ThreadId>,
    #[serde(
        rename = "relationshipToParent",
        deserialize_with = "required_nullable"
    )]
    pub relationship_to_parent: Option<OrchestratorMcpThreadListItemRelationshipToParent>,
    #[serde(rename = "itemCount")]
    pub item_count: NonNegativeInt,
    #[serde(rename = "createdAt")]
    pub created_at: IsoDateTime,
    #[serde(rename = "updatedAt")]
    pub updated_at: IsoDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestratorMcpThreadListResult {
    #[serde(rename = "projectId")]
    pub project_id: ProjectId,
    #[serde(rename = "currentThreadId")]
    pub current_thread_id: ThreadId,
    #[serde(rename = "threads")]
    pub threads: Vec<OrchestratorMcpThreadListItem>,
    #[serde(rename = "nextCursor", deserialize_with = "required_nullable")]
    pub next_cursor: Option<NonNegativeInt>,
    #[serde(rename = "total")]
    pub total: NonNegativeInt,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestratorMcpThreadReadInputView {
    #[serde(rename = "messages")]
    Messages,
    #[serde(rename = "activity")]
    Activity,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestratorMcpThreadReadInput {
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(
        rename = "itemId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub item_id: Optional<TurnItemId>,
    #[serde(
        rename = "textOffset",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub text_offset: Optional<NonNegativeInt>,
    #[serde(rename = "view", default, skip_serializing_if = "Optional::is_absent")]
    pub view: Optional<OrchestratorMcpThreadReadInputView>,
    #[serde(
        rename = "afterPosition",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub after_position: Optional<NonNegativeInt>,
    #[serde(rename = "limit", default, skip_serializing_if = "Optional::is_absent")]
    pub limit: Optional<i64>,
    #[serde(
        rename = "runLimit",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub run_limit: Optional<i64>,
    #[serde(
        rename = "maxCharsPerItem",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub max_chars_per_item: Optional<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestratorMcpThreadReadResult {
    #[serde(rename = "thread")]
    pub thread: OrchestratorMcpThreadDetail,
    #[serde(rename = "recentRuns")]
    pub recent_runs: Vec<OrchestratorMcpThreadRun>,
    #[serde(rename = "items")]
    pub items: Vec<OrchestratorMcpThreadTimelineItem>,
    #[serde(rename = "nextPosition", deserialize_with = "required_nullable")]
    pub next_position: Option<NonNegativeInt>,
    #[serde(rename = "hasMore")]
    pub has_more: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestratorMcpThreadRun {
    #[serde(rename = "runId")]
    pub run_id: RunId,
    #[serde(rename = "ordinal")]
    pub ordinal: PositiveInt,
    #[serde(rename = "status")]
    pub status: OrchestrationV2RunStatus,
    #[serde(rename = "providerInstanceId")]
    pub provider_instance_id: ProviderInstanceId,
    #[serde(rename = "model")]
    pub model: IsoDateTime,
    #[serde(rename = "requestedAt")]
    pub requested_at: IsoDateTime,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<IsoDateTime>,
    #[serde(rename = "completedAt", deserialize_with = "required_nullable")]
    pub completed_at: Option<IsoDateTime>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestratorMcpThreadSendInputMode {
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "queue")]
    Queue,
    #[serde(rename = "steer")]
    Steer,
    #[serde(rename = "restart")]
    Restart,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestratorMcpThreadSendInput {
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "message")]
    pub message: String,
    #[serde(rename = "mode", default, skip_serializing_if = "Optional::is_absent")]
    pub mode: Optional<OrchestratorMcpThreadSendInputMode>,
    #[serde(
        rename = "clientRequestId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub client_request_id: Optional<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestratorMcpThreadSendResultDelivery {
    #[serde(rename = "started")]
    Started,
    #[serde(rename = "queued")]
    Queued,
    #[serde(rename = "steered")]
    Steered,
    #[serde(rename = "restarted")]
    Restarted,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestratorMcpThreadSendResult {
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "messageId")]
    pub message_id: MessageId,
    #[serde(rename = "runId")]
    pub run_id: RunId,
    #[serde(rename = "status")]
    pub status: OrchestrationV2RunStatus,
    #[serde(rename = "delivery")]
    pub delivery: OrchestratorMcpThreadSendResultDelivery,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrchestratorMcpThreadStatus {
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
pub enum OrchestratorMcpThreadTimelineItemVisibility {
    #[serde(rename = "local")]
    Local,
    #[serde(rename = "inherited")]
    Inherited,
    #[serde(rename = "synthetic")]
    Synthetic,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestratorMcpThreadTimelineItem {
    #[serde(rename = "position")]
    pub position: NonNegativeInt,
    #[serde(rename = "visibility")]
    pub visibility: OrchestratorMcpThreadTimelineItemVisibility,
    #[serde(rename = "sourceThreadId")]
    pub source_thread_id: ThreadId,
    #[serde(rename = "itemId")]
    pub item_id: TurnItemId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "messageId", deserialize_with = "required_nullable")]
    pub message_id: Option<MessageId>,
    #[serde(rename = "createdBy", deserialize_with = "required_nullable")]
    pub created_by: Option<OrchestrationV2Actor>,
    #[serde(rename = "creationSource", deserialize_with = "required_nullable")]
    pub creation_source: Option<OrchestrationV2CreationSource>,
    #[serde(rename = "type")]
    pub r#type: IsoDateTime,
    #[serde(rename = "status")]
    pub status: OrchestrationV2TurnItemStatus,
    #[serde(rename = "title", deserialize_with = "required_nullable")]
    pub title: Option<IsoDateTime>,
    #[serde(rename = "text", deserialize_with = "required_nullable")]
    pub text: Option<IsoDateTime>,
    #[serde(rename = "textTruncated")]
    pub text_truncated: bool,
    #[serde(
        rename = "nextTextOffset",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub next_text_offset: Optional<Option<NonNegativeInt>>,
    #[serde(rename = "updatedAt")]
    pub updated_at: IsoDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestratorMcpThreadWaitInput {
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<RunId>,
    #[serde(
        rename = "timeoutMs",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub timeout_ms: Optional<JsonNumber>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestratorMcpThreadWaitResult {
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "status")]
    pub status: OrchestratorMcpThreadStatus,
    #[serde(rename = "timedOut")]
    pub timed_out: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestratorMcpUpdateScheduledTaskInputScheduleInterval {
    #[serde(rename = "everyMs")]
    pub every_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestratorMcpUpdateScheduledTaskInputScheduleFixedTime {
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
pub enum OrchestratorMcpUpdateScheduledTaskInputSchedule {
    #[serde(rename = "interval")]
    Interval(Box<OrchestratorMcpUpdateScheduledTaskInputScheduleInterval>),
    #[serde(rename = "fixed_time")]
    FixedTime(Box<OrchestratorMcpUpdateScheduledTaskInputScheduleFixedTime>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestratorMcpUpdateScheduledTaskInput {
    #[serde(rename = "scheduledTaskId")]
    pub scheduled_task_id: ScheduledTaskId,
    #[serde(
        rename = "prompt",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub prompt: Optional<String>,
    #[serde(rename = "title", default, skip_serializing_if = "Optional::is_absent")]
    pub title: Optional<String>,
    #[serde(
        rename = "schedule",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub schedule: Optional<OrchestratorMcpUpdateScheduledTaskInputSchedule>,
    #[serde(
        rename = "enabled",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub enabled: Optional<bool>,
    #[serde(
        rename = "bindToCurrentThread",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub bind_to_current_thread: Optional<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewAutomationActionEventStatus {
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "succeeded")]
    Succeeded,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "interrupted")]
    Interrupted,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewAutomationActionEvent {
    #[serde(rename = "id")]
    pub id: IsoDateTime,
    #[serde(rename = "action")]
    pub action: IsoDateTime,
    #[serde(rename = "status")]
    pub status: PreviewAutomationActionEventStatus,
    #[serde(rename = "startedAt")]
    pub started_at: IsoDateTime,
    #[serde(
        rename = "completedAt",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub completed_at: Optional<IsoDateTime>,
    #[serde(rename = "error", default, skip_serializing_if = "Optional::is_absent")]
    pub error: Optional<IsoDateTime>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewAutomationClientDisconnectedErrorTag {
    #[serde(rename = "PreviewAutomationClientDisconnectedError")]
    PreviewAutomationClientDisconnectedError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewAutomationClientDisconnectedError {
    #[serde(rename = "_tag")]
    pub _tag: PreviewAutomationClientDisconnectedErrorTag,
    #[serde(rename = "operation")]
    pub operation: PreviewAutomationOperation,
    #[serde(rename = "environmentId")]
    pub environment_id: EnvironmentId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "providerSessionId")]
    pub provider_session_id: TrimmedNonEmptyString,
    #[serde(rename = "providerInstanceId")]
    pub provider_instance_id: ProviderInstanceId,
    #[serde(rename = "clientId")]
    pub client_id: TrimmedNonEmptyString,
    #[serde(rename = "connectionId")]
    pub connection_id: PreviewAutomationConnectionId,
    #[serde(rename = "requestId")]
    pub request_id: TrimmedNonEmptyString,
    #[serde(rename = "tabId", default, skip_serializing_if = "Optional::is_absent")]
    pub tab_id: Optional<PreviewTabId>,
    #[serde(rename = "timeoutMs")]
    pub timeout_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewAutomationColorScheme {
    #[serde(rename = "system")]
    System,
    #[serde(rename = "light")]
    Light,
    #[serde(rename = "dark")]
    Dark,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PreviewAutomationConnectionId(pub String);
impl From<String> for PreviewAutomationConnectionId {
    fn from(value: String) -> Self {
        Self(value)
    }
}
impl From<&str> for PreviewAutomationConnectionId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}
impl AsRef<str> for PreviewAutomationConnectionId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
impl std::fmt::Display for PreviewAutomationConnectionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewAutomationConsoleEntry {
    #[serde(rename = "level")]
    pub level: IsoDateTime,
    #[serde(rename = "text")]
    pub text: IsoDateTime,
    #[serde(rename = "timestamp")]
    pub timestamp: IsoDateTime,
    #[serde(
        rename = "source",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub source: Optional<IsoDateTime>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewAutomationControlInterruptedErrorTag {
    #[serde(rename = "PreviewAutomationControlInterruptedError")]
    PreviewAutomationControlInterruptedError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewAutomationControlInterruptedErrorRemoteDetailKind {
    #[serde(rename = "null")]
    Null,
    #[serde(rename = "array")]
    Array,
    #[serde(rename = "object")]
    Object,
    #[serde(rename = "string")]
    String,
    #[serde(rename = "number")]
    Number,
    #[serde(rename = "boolean")]
    Boolean,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewAutomationControlInterruptedError {
    #[serde(rename = "_tag")]
    pub _tag: PreviewAutomationControlInterruptedErrorTag,
    #[serde(rename = "operation")]
    pub operation: PreviewAutomationOperation,
    #[serde(rename = "environmentId")]
    pub environment_id: EnvironmentId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "providerSessionId")]
    pub provider_session_id: TrimmedNonEmptyString,
    #[serde(rename = "providerInstanceId")]
    pub provider_instance_id: ProviderInstanceId,
    #[serde(rename = "clientId")]
    pub client_id: TrimmedNonEmptyString,
    #[serde(rename = "connectionId")]
    pub connection_id: PreviewAutomationConnectionId,
    #[serde(rename = "requestId")]
    pub request_id: TrimmedNonEmptyString,
    #[serde(rename = "tabId", default, skip_serializing_if = "Optional::is_absent")]
    pub tab_id: Optional<PreviewTabId>,
    #[serde(rename = "timeoutMs")]
    pub timeout_ms: i64,
    #[serde(rename = "remoteTag")]
    pub remote_tag: TrimmedNonEmptyString,
    #[serde(rename = "remoteMessageLength")]
    pub remote_message_length: i64,
    #[serde(
        rename = "remoteDetailKind",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub remote_detail_kind: Optional<PreviewAutomationControlInterruptedErrorRemoteDetailKind>,
    #[serde(rename = "cause")]
    pub cause: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewAutomationElement {
    #[serde(rename = "tag")]
    pub tag: IsoDateTime,
    #[serde(rename = "role", deserialize_with = "required_nullable")]
    pub role: Option<IsoDateTime>,
    #[serde(rename = "name")]
    pub name: IsoDateTime,
    #[serde(rename = "selector")]
    pub selector: IsoDateTime,
    #[serde(rename = "x")]
    pub x: JsonNumber,
    #[serde(rename = "y")]
    pub y: JsonNumber,
    #[serde(rename = "width")]
    pub width: JsonNumber,
    #[serde(rename = "height")]
    pub height: JsonNumber,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewAutomationErrorPreviewAutomationRecordingTransferError {
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "cause", default, skip_serializing_if = "Optional::is_absent")]
    pub cause: Optional<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewAutomationErrorPreviewAutomationRecordingDesktopUpdateRequiredError {
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "cause", default, skip_serializing_if = "Optional::is_absent")]
    pub cause: Optional<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewAutomationErrorPreviewAutomationRecordingTooLargeError {
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "cause", default, skip_serializing_if = "Optional::is_absent")]
    pub cause: Optional<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewAutomationErrorPreviewAutomationRecordingDeadlineExpiredError {
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "cause", default, skip_serializing_if = "Optional::is_absent")]
    pub cause: Optional<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewAutomationErrorPreviewAutomationUnavailableErrorCapability {
    #[serde(rename = "preview")]
    Preview,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewAutomationErrorPreviewAutomationUnavailableError {
    #[serde(rename = "capability")]
    pub capability: PreviewAutomationErrorPreviewAutomationUnavailableErrorCapability,
    #[serde(rename = "environmentId")]
    pub environment_id: EnvironmentId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "providerSessionId")]
    pub provider_session_id: TrimmedNonEmptyString,
    #[serde(rename = "providerInstanceId")]
    pub provider_instance_id: ProviderInstanceId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewAutomationErrorPreviewAutomationNoAvailableHostErrorRemoteDetailKind {
    #[serde(rename = "null")]
    Null,
    #[serde(rename = "array")]
    Array,
    #[serde(rename = "object")]
    Object,
    #[serde(rename = "string")]
    String,
    #[serde(rename = "number")]
    Number,
    #[serde(rename = "boolean")]
    Boolean,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewAutomationErrorPreviewAutomationNoAvailableHostError {
    #[serde(rename = "operation")]
    pub operation: PreviewAutomationOperation,
    #[serde(rename = "environmentId")]
    pub environment_id: EnvironmentId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "providerSessionId")]
    pub provider_session_id: TrimmedNonEmptyString,
    #[serde(rename = "providerInstanceId")]
    pub provider_instance_id: ProviderInstanceId,
    #[serde(
        rename = "clientId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub client_id: Optional<TrimmedNonEmptyString>,
    #[serde(
        rename = "connectionId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub connection_id: Optional<PreviewAutomationConnectionId>,
    #[serde(
        rename = "requestId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub request_id: Optional<TrimmedNonEmptyString>,
    #[serde(rename = "tabId", default, skip_serializing_if = "Optional::is_absent")]
    pub tab_id: Optional<PreviewTabId>,
    #[serde(
        rename = "timeoutMs",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub timeout_ms: Optional<i64>,
    #[serde(
        rename = "remoteTag",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub remote_tag: Optional<TrimmedNonEmptyString>,
    #[serde(
        rename = "remoteMessageLength",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub remote_message_length: Optional<i64>,
    #[serde(
        rename = "remoteDetailKind",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub remote_detail_kind:
        Optional<PreviewAutomationErrorPreviewAutomationNoAvailableHostErrorRemoteDetailKind>,
    #[serde(rename = "cause", default, skip_serializing_if = "Optional::is_absent")]
    pub cause: Optional<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewAutomationErrorPreviewAutomationUnsupportedClientErrorRemoteDetailKind {
    #[serde(rename = "null")]
    Null,
    #[serde(rename = "array")]
    Array,
    #[serde(rename = "object")]
    Object,
    #[serde(rename = "string")]
    String,
    #[serde(rename = "number")]
    Number,
    #[serde(rename = "boolean")]
    Boolean,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewAutomationErrorPreviewAutomationUnsupportedClientError {
    #[serde(rename = "operation")]
    pub operation: PreviewAutomationOperation,
    #[serde(rename = "environmentId")]
    pub environment_id: EnvironmentId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "providerSessionId")]
    pub provider_session_id: TrimmedNonEmptyString,
    #[serde(rename = "providerInstanceId")]
    pub provider_instance_id: ProviderInstanceId,
    #[serde(rename = "clientId")]
    pub client_id: TrimmedNonEmptyString,
    #[serde(rename = "connectionId")]
    pub connection_id: PreviewAutomationConnectionId,
    #[serde(rename = "requestId")]
    pub request_id: TrimmedNonEmptyString,
    #[serde(rename = "tabId", default, skip_serializing_if = "Optional::is_absent")]
    pub tab_id: Optional<PreviewTabId>,
    #[serde(rename = "timeoutMs")]
    pub timeout_ms: i64,
    #[serde(rename = "remoteTag")]
    pub remote_tag: TrimmedNonEmptyString,
    #[serde(rename = "remoteMessageLength")]
    pub remote_message_length: i64,
    #[serde(
        rename = "remoteDetailKind",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub remote_detail_kind:
        Optional<PreviewAutomationErrorPreviewAutomationUnsupportedClientErrorRemoteDetailKind>,
    #[serde(rename = "cause")]
    pub cause: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewAutomationErrorPreviewAutomationTabNotFoundErrorRemoteDetailKind {
    #[serde(rename = "null")]
    Null,
    #[serde(rename = "array")]
    Array,
    #[serde(rename = "object")]
    Object,
    #[serde(rename = "string")]
    String,
    #[serde(rename = "number")]
    Number,
    #[serde(rename = "boolean")]
    Boolean,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewAutomationErrorPreviewAutomationTabNotFoundError {
    #[serde(rename = "operation")]
    pub operation: PreviewAutomationOperation,
    #[serde(rename = "environmentId")]
    pub environment_id: EnvironmentId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "providerSessionId")]
    pub provider_session_id: TrimmedNonEmptyString,
    #[serde(rename = "providerInstanceId")]
    pub provider_instance_id: ProviderInstanceId,
    #[serde(rename = "clientId")]
    pub client_id: TrimmedNonEmptyString,
    #[serde(rename = "connectionId")]
    pub connection_id: PreviewAutomationConnectionId,
    #[serde(rename = "requestId")]
    pub request_id: TrimmedNonEmptyString,
    #[serde(rename = "tabId", default, skip_serializing_if = "Optional::is_absent")]
    pub tab_id: Optional<PreviewTabId>,
    #[serde(rename = "timeoutMs")]
    pub timeout_ms: i64,
    #[serde(rename = "remoteTag")]
    pub remote_tag: TrimmedNonEmptyString,
    #[serde(rename = "remoteMessageLength")]
    pub remote_message_length: i64,
    #[serde(
        rename = "remoteDetailKind",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub remote_detail_kind:
        Optional<PreviewAutomationErrorPreviewAutomationTabNotFoundErrorRemoteDetailKind>,
    #[serde(rename = "cause")]
    pub cause: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewAutomationErrorPreviewAutomationTimeoutErrorRemoteDetailKind {
    #[serde(rename = "null")]
    Null,
    #[serde(rename = "array")]
    Array,
    #[serde(rename = "object")]
    Object,
    #[serde(rename = "string")]
    String,
    #[serde(rename = "number")]
    Number,
    #[serde(rename = "boolean")]
    Boolean,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewAutomationErrorPreviewAutomationTimeoutError {
    #[serde(rename = "operation")]
    pub operation: PreviewAutomationOperation,
    #[serde(rename = "environmentId")]
    pub environment_id: EnvironmentId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "providerSessionId")]
    pub provider_session_id: TrimmedNonEmptyString,
    #[serde(rename = "providerInstanceId")]
    pub provider_instance_id: ProviderInstanceId,
    #[serde(rename = "clientId")]
    pub client_id: TrimmedNonEmptyString,
    #[serde(rename = "connectionId")]
    pub connection_id: PreviewAutomationConnectionId,
    #[serde(rename = "requestId")]
    pub request_id: TrimmedNonEmptyString,
    #[serde(rename = "tabId", default, skip_serializing_if = "Optional::is_absent")]
    pub tab_id: Optional<PreviewTabId>,
    #[serde(rename = "timeoutMs")]
    pub timeout_ms: i64,
    #[serde(
        rename = "remoteTag",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub remote_tag: Optional<TrimmedNonEmptyString>,
    #[serde(
        rename = "remoteMessageLength",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub remote_message_length: Optional<i64>,
    #[serde(
        rename = "remoteDetailKind",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub remote_detail_kind:
        Optional<PreviewAutomationErrorPreviewAutomationTimeoutErrorRemoteDetailKind>,
    #[serde(rename = "cause", default, skip_serializing_if = "Optional::is_absent")]
    pub cause: Optional<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewAutomationErrorPreviewAutomationControlInterruptedErrorRemoteDetailKind {
    #[serde(rename = "null")]
    Null,
    #[serde(rename = "array")]
    Array,
    #[serde(rename = "object")]
    Object,
    #[serde(rename = "string")]
    String,
    #[serde(rename = "number")]
    Number,
    #[serde(rename = "boolean")]
    Boolean,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewAutomationErrorPreviewAutomationControlInterruptedError {
    #[serde(rename = "operation")]
    pub operation: PreviewAutomationOperation,
    #[serde(rename = "environmentId")]
    pub environment_id: EnvironmentId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "providerSessionId")]
    pub provider_session_id: TrimmedNonEmptyString,
    #[serde(rename = "providerInstanceId")]
    pub provider_instance_id: ProviderInstanceId,
    #[serde(rename = "clientId")]
    pub client_id: TrimmedNonEmptyString,
    #[serde(rename = "connectionId")]
    pub connection_id: PreviewAutomationConnectionId,
    #[serde(rename = "requestId")]
    pub request_id: TrimmedNonEmptyString,
    #[serde(rename = "tabId", default, skip_serializing_if = "Optional::is_absent")]
    pub tab_id: Optional<PreviewTabId>,
    #[serde(rename = "timeoutMs")]
    pub timeout_ms: i64,
    #[serde(rename = "remoteTag")]
    pub remote_tag: TrimmedNonEmptyString,
    #[serde(rename = "remoteMessageLength")]
    pub remote_message_length: i64,
    #[serde(
        rename = "remoteDetailKind",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub remote_detail_kind:
        Optional<PreviewAutomationErrorPreviewAutomationControlInterruptedErrorRemoteDetailKind>,
    #[serde(rename = "cause")]
    pub cause: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewAutomationErrorPreviewAutomationExecutionErrorRemoteDetailKind {
    #[serde(rename = "null")]
    Null,
    #[serde(rename = "array")]
    Array,
    #[serde(rename = "object")]
    Object,
    #[serde(rename = "string")]
    String,
    #[serde(rename = "number")]
    Number,
    #[serde(rename = "boolean")]
    Boolean,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewAutomationErrorPreviewAutomationExecutionError {
    #[serde(rename = "operation")]
    pub operation: PreviewAutomationOperation,
    #[serde(rename = "environmentId")]
    pub environment_id: EnvironmentId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "providerSessionId")]
    pub provider_session_id: TrimmedNonEmptyString,
    #[serde(rename = "providerInstanceId")]
    pub provider_instance_id: ProviderInstanceId,
    #[serde(rename = "clientId")]
    pub client_id: TrimmedNonEmptyString,
    #[serde(rename = "connectionId")]
    pub connection_id: PreviewAutomationConnectionId,
    #[serde(rename = "requestId")]
    pub request_id: TrimmedNonEmptyString,
    #[serde(rename = "tabId", default, skip_serializing_if = "Optional::is_absent")]
    pub tab_id: Optional<PreviewTabId>,
    #[serde(rename = "timeoutMs")]
    pub timeout_ms: i64,
    #[serde(rename = "remoteTag")]
    pub remote_tag: TrimmedNonEmptyString,
    #[serde(rename = "remoteMessageLength")]
    pub remote_message_length: i64,
    #[serde(
        rename = "remoteDetailKind",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub remote_detail_kind:
        Optional<PreviewAutomationErrorPreviewAutomationExecutionErrorRemoteDetailKind>,
    #[serde(rename = "cause")]
    pub cause: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewAutomationErrorPreviewAutomationInvalidSelectorErrorRemoteDetailKind {
    #[serde(rename = "null")]
    Null,
    #[serde(rename = "array")]
    Array,
    #[serde(rename = "object")]
    Object,
    #[serde(rename = "string")]
    String,
    #[serde(rename = "number")]
    Number,
    #[serde(rename = "boolean")]
    Boolean,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewAutomationErrorPreviewAutomationInvalidSelectorErrorSelectorKind {
    #[serde(rename = "locator")]
    Locator,
    #[serde(rename = "selector")]
    Selector,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewAutomationErrorPreviewAutomationInvalidSelectorError {
    #[serde(rename = "operation")]
    pub operation: PreviewAutomationOperation,
    #[serde(rename = "environmentId")]
    pub environment_id: EnvironmentId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "providerSessionId")]
    pub provider_session_id: TrimmedNonEmptyString,
    #[serde(rename = "providerInstanceId")]
    pub provider_instance_id: ProviderInstanceId,
    #[serde(rename = "clientId")]
    pub client_id: TrimmedNonEmptyString,
    #[serde(rename = "connectionId")]
    pub connection_id: PreviewAutomationConnectionId,
    #[serde(rename = "requestId")]
    pub request_id: TrimmedNonEmptyString,
    #[serde(rename = "tabId", default, skip_serializing_if = "Optional::is_absent")]
    pub tab_id: Optional<PreviewTabId>,
    #[serde(rename = "timeoutMs")]
    pub timeout_ms: i64,
    #[serde(rename = "remoteTag")]
    pub remote_tag: TrimmedNonEmptyString,
    #[serde(rename = "remoteMessageLength")]
    pub remote_message_length: i64,
    #[serde(
        rename = "remoteDetailKind",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub remote_detail_kind:
        Optional<PreviewAutomationErrorPreviewAutomationInvalidSelectorErrorRemoteDetailKind>,
    #[serde(rename = "cause")]
    pub cause: serde_json::Value,
    #[serde(
        rename = "selectorKind",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub selector_kind:
        Optional<PreviewAutomationErrorPreviewAutomationInvalidSelectorErrorSelectorKind>,
    #[serde(
        rename = "selectorLength",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub selector_length: Optional<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewAutomationErrorPreviewAutomationTargetNotEditableErrorRemoteDetailKind {
    #[serde(rename = "null")]
    Null,
    #[serde(rename = "array")]
    Array,
    #[serde(rename = "object")]
    Object,
    #[serde(rename = "string")]
    String,
    #[serde(rename = "number")]
    Number,
    #[serde(rename = "boolean")]
    Boolean,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewAutomationErrorPreviewAutomationTargetNotEditableErrorSelectorKind {
    #[serde(rename = "focused-element")]
    FocusedElement,
    #[serde(rename = "locator")]
    Locator,
    #[serde(rename = "selector")]
    Selector,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewAutomationErrorPreviewAutomationTargetNotEditableError {
    #[serde(rename = "operation")]
    pub operation: PreviewAutomationOperation,
    #[serde(rename = "environmentId")]
    pub environment_id: EnvironmentId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "providerSessionId")]
    pub provider_session_id: TrimmedNonEmptyString,
    #[serde(rename = "providerInstanceId")]
    pub provider_instance_id: ProviderInstanceId,
    #[serde(rename = "clientId")]
    pub client_id: TrimmedNonEmptyString,
    #[serde(rename = "connectionId")]
    pub connection_id: PreviewAutomationConnectionId,
    #[serde(rename = "requestId")]
    pub request_id: TrimmedNonEmptyString,
    #[serde(rename = "tabId", default, skip_serializing_if = "Optional::is_absent")]
    pub tab_id: Optional<PreviewTabId>,
    #[serde(rename = "timeoutMs")]
    pub timeout_ms: i64,
    #[serde(rename = "remoteTag")]
    pub remote_tag: TrimmedNonEmptyString,
    #[serde(rename = "remoteMessageLength")]
    pub remote_message_length: i64,
    #[serde(
        rename = "remoteDetailKind",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub remote_detail_kind:
        Optional<PreviewAutomationErrorPreviewAutomationTargetNotEditableErrorRemoteDetailKind>,
    #[serde(rename = "cause")]
    pub cause: serde_json::Value,
    #[serde(
        rename = "selectorKind",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub selector_kind:
        Optional<PreviewAutomationErrorPreviewAutomationTargetNotEditableErrorSelectorKind>,
    #[serde(
        rename = "selectorLength",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub selector_length: Optional<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewAutomationErrorPreviewAutomationResultTooLargeErrorRemoteDetailKind {
    #[serde(rename = "null")]
    Null,
    #[serde(rename = "array")]
    Array,
    #[serde(rename = "object")]
    Object,
    #[serde(rename = "string")]
    String,
    #[serde(rename = "number")]
    Number,
    #[serde(rename = "boolean")]
    Boolean,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewAutomationErrorPreviewAutomationResultTooLargeError {
    #[serde(rename = "operation")]
    pub operation: PreviewAutomationOperation,
    #[serde(rename = "environmentId")]
    pub environment_id: EnvironmentId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "providerSessionId")]
    pub provider_session_id: TrimmedNonEmptyString,
    #[serde(rename = "providerInstanceId")]
    pub provider_instance_id: ProviderInstanceId,
    #[serde(rename = "clientId")]
    pub client_id: TrimmedNonEmptyString,
    #[serde(rename = "connectionId")]
    pub connection_id: PreviewAutomationConnectionId,
    #[serde(rename = "requestId")]
    pub request_id: TrimmedNonEmptyString,
    #[serde(rename = "tabId", default, skip_serializing_if = "Optional::is_absent")]
    pub tab_id: Optional<PreviewTabId>,
    #[serde(rename = "timeoutMs")]
    pub timeout_ms: i64,
    #[serde(rename = "remoteTag")]
    pub remote_tag: TrimmedNonEmptyString,
    #[serde(rename = "remoteMessageLength")]
    pub remote_message_length: i64,
    #[serde(
        rename = "remoteDetailKind",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub remote_detail_kind:
        Optional<PreviewAutomationErrorPreviewAutomationResultTooLargeErrorRemoteDetailKind>,
    #[serde(rename = "cause")]
    pub cause: serde_json::Value,
    #[serde(
        rename = "maximumBytes",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub maximum_bytes: Optional<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewAutomationErrorPreviewAutomationClientDisconnectedError {
    #[serde(rename = "operation")]
    pub operation: PreviewAutomationOperation,
    #[serde(rename = "environmentId")]
    pub environment_id: EnvironmentId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "providerSessionId")]
    pub provider_session_id: TrimmedNonEmptyString,
    #[serde(rename = "providerInstanceId")]
    pub provider_instance_id: ProviderInstanceId,
    #[serde(rename = "clientId")]
    pub client_id: TrimmedNonEmptyString,
    #[serde(rename = "connectionId")]
    pub connection_id: PreviewAutomationConnectionId,
    #[serde(rename = "requestId")]
    pub request_id: TrimmedNonEmptyString,
    #[serde(rename = "tabId", default, skip_serializing_if = "Optional::is_absent")]
    pub tab_id: Optional<PreviewTabId>,
    #[serde(rename = "timeoutMs")]
    pub timeout_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewAutomationErrorPreviewAutomationRequestQueueClosedError {
    #[serde(rename = "operation")]
    pub operation: PreviewAutomationOperation,
    #[serde(rename = "environmentId")]
    pub environment_id: EnvironmentId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "providerSessionId")]
    pub provider_session_id: TrimmedNonEmptyString,
    #[serde(rename = "providerInstanceId")]
    pub provider_instance_id: ProviderInstanceId,
    #[serde(rename = "clientId")]
    pub client_id: TrimmedNonEmptyString,
    #[serde(rename = "connectionId")]
    pub connection_id: PreviewAutomationConnectionId,
    #[serde(rename = "requestId")]
    pub request_id: TrimmedNonEmptyString,
    #[serde(rename = "tabId", default, skip_serializing_if = "Optional::is_absent")]
    pub tab_id: Optional<PreviewTabId>,
    #[serde(rename = "timeoutMs")]
    pub timeout_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewAutomationErrorPreviewAutomationRemoteUnavailableErrorRemoteDetailKind {
    #[serde(rename = "null")]
    Null,
    #[serde(rename = "array")]
    Array,
    #[serde(rename = "object")]
    Object,
    #[serde(rename = "string")]
    String,
    #[serde(rename = "number")]
    Number,
    #[serde(rename = "boolean")]
    Boolean,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewAutomationErrorPreviewAutomationRemoteUnavailableError {
    #[serde(rename = "operation")]
    pub operation: PreviewAutomationOperation,
    #[serde(rename = "environmentId")]
    pub environment_id: EnvironmentId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "providerSessionId")]
    pub provider_session_id: TrimmedNonEmptyString,
    #[serde(rename = "providerInstanceId")]
    pub provider_instance_id: ProviderInstanceId,
    #[serde(rename = "clientId")]
    pub client_id: TrimmedNonEmptyString,
    #[serde(rename = "connectionId")]
    pub connection_id: PreviewAutomationConnectionId,
    #[serde(rename = "requestId")]
    pub request_id: TrimmedNonEmptyString,
    #[serde(rename = "tabId", default, skip_serializing_if = "Optional::is_absent")]
    pub tab_id: Optional<PreviewTabId>,
    #[serde(rename = "timeoutMs")]
    pub timeout_ms: i64,
    #[serde(rename = "remoteTag")]
    pub remote_tag: TrimmedNonEmptyString,
    #[serde(rename = "remoteMessageLength")]
    pub remote_message_length: i64,
    #[serde(
        rename = "remoteDetailKind",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub remote_detail_kind:
        Optional<PreviewAutomationErrorPreviewAutomationRemoteUnavailableErrorRemoteDetailKind>,
    #[serde(rename = "cause")]
    pub cause: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewAutomationErrorPreviewAutomationMalformedResponseError {
    #[serde(rename = "operation")]
    pub operation: PreviewAutomationOperation,
    #[serde(rename = "environmentId")]
    pub environment_id: EnvironmentId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "providerSessionId")]
    pub provider_session_id: TrimmedNonEmptyString,
    #[serde(rename = "providerInstanceId")]
    pub provider_instance_id: ProviderInstanceId,
    #[serde(rename = "clientId")]
    pub client_id: TrimmedNonEmptyString,
    #[serde(rename = "connectionId")]
    pub connection_id: PreviewAutomationConnectionId,
    #[serde(rename = "requestId")]
    pub request_id: TrimmedNonEmptyString,
    #[serde(rename = "tabId", default, skip_serializing_if = "Optional::is_absent")]
    pub tab_id: Optional<PreviewTabId>,
    #[serde(rename = "timeoutMs")]
    pub timeout_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "_tag")]
pub enum PreviewAutomationError {
    #[serde(rename = "PreviewAutomationRecordingTransferError")]
    PreviewAutomationRecordingTransferError(
        Box<PreviewAutomationErrorPreviewAutomationRecordingTransferError>,
    ),
    #[serde(rename = "PreviewAutomationRecordingDesktopUpdateRequiredError")]
    PreviewAutomationRecordingDesktopUpdateRequiredError(
        Box<PreviewAutomationErrorPreviewAutomationRecordingDesktopUpdateRequiredError>,
    ),
    #[serde(rename = "PreviewAutomationRecordingTooLargeError")]
    PreviewAutomationRecordingTooLargeError(
        Box<PreviewAutomationErrorPreviewAutomationRecordingTooLargeError>,
    ),
    #[serde(rename = "PreviewAutomationRecordingDeadlineExpiredError")]
    PreviewAutomationRecordingDeadlineExpiredError(
        Box<PreviewAutomationErrorPreviewAutomationRecordingDeadlineExpiredError>,
    ),
    #[serde(rename = "PreviewAutomationUnavailableError")]
    PreviewAutomationUnavailableError(Box<PreviewAutomationErrorPreviewAutomationUnavailableError>),
    #[serde(rename = "PreviewAutomationNoAvailableHostError")]
    PreviewAutomationNoAvailableHostError(
        Box<PreviewAutomationErrorPreviewAutomationNoAvailableHostError>,
    ),
    #[serde(rename = "PreviewAutomationUnsupportedClientError")]
    PreviewAutomationUnsupportedClientError(
        Box<PreviewAutomationErrorPreviewAutomationUnsupportedClientError>,
    ),
    #[serde(rename = "PreviewAutomationTabNotFoundError")]
    PreviewAutomationTabNotFoundError(Box<PreviewAutomationErrorPreviewAutomationTabNotFoundError>),
    #[serde(rename = "PreviewAutomationTimeoutError")]
    PreviewAutomationTimeoutError(Box<PreviewAutomationErrorPreviewAutomationTimeoutError>),
    #[serde(rename = "PreviewAutomationControlInterruptedError")]
    PreviewAutomationControlInterruptedError(
        Box<PreviewAutomationErrorPreviewAutomationControlInterruptedError>,
    ),
    #[serde(rename = "PreviewAutomationExecutionError")]
    PreviewAutomationExecutionError(Box<PreviewAutomationErrorPreviewAutomationExecutionError>),
    #[serde(rename = "PreviewAutomationInvalidSelectorError")]
    PreviewAutomationInvalidSelectorError(
        Box<PreviewAutomationErrorPreviewAutomationInvalidSelectorError>,
    ),
    #[serde(rename = "PreviewAutomationTargetNotEditableError")]
    PreviewAutomationTargetNotEditableError(
        Box<PreviewAutomationErrorPreviewAutomationTargetNotEditableError>,
    ),
    #[serde(rename = "PreviewAutomationResultTooLargeError")]
    PreviewAutomationResultTooLargeError(
        Box<PreviewAutomationErrorPreviewAutomationResultTooLargeError>,
    ),
    #[serde(rename = "PreviewAutomationClientDisconnectedError")]
    PreviewAutomationClientDisconnectedError(
        Box<PreviewAutomationErrorPreviewAutomationClientDisconnectedError>,
    ),
    #[serde(rename = "PreviewAutomationRequestQueueClosedError")]
    PreviewAutomationRequestQueueClosedError(
        Box<PreviewAutomationErrorPreviewAutomationRequestQueueClosedError>,
    ),
    #[serde(rename = "PreviewAutomationRemoteUnavailableError")]
    PreviewAutomationRemoteUnavailableError(
        Box<PreviewAutomationErrorPreviewAutomationRemoteUnavailableError>,
    ),
    #[serde(rename = "PreviewAutomationMalformedResponseError")]
    PreviewAutomationMalformedResponseError(
        Box<PreviewAutomationErrorPreviewAutomationMalformedResponseError>,
    ),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewAutomationExecutionErrorTag {
    #[serde(rename = "PreviewAutomationExecutionError")]
    PreviewAutomationExecutionError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewAutomationExecutionErrorRemoteDetailKind {
    #[serde(rename = "null")]
    Null,
    #[serde(rename = "array")]
    Array,
    #[serde(rename = "object")]
    Object,
    #[serde(rename = "string")]
    String,
    #[serde(rename = "number")]
    Number,
    #[serde(rename = "boolean")]
    Boolean,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewAutomationExecutionError {
    #[serde(rename = "_tag")]
    pub _tag: PreviewAutomationExecutionErrorTag,
    #[serde(rename = "operation")]
    pub operation: PreviewAutomationOperation,
    #[serde(rename = "environmentId")]
    pub environment_id: EnvironmentId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "providerSessionId")]
    pub provider_session_id: TrimmedNonEmptyString,
    #[serde(rename = "providerInstanceId")]
    pub provider_instance_id: ProviderInstanceId,
    #[serde(rename = "clientId")]
    pub client_id: TrimmedNonEmptyString,
    #[serde(rename = "connectionId")]
    pub connection_id: PreviewAutomationConnectionId,
    #[serde(rename = "requestId")]
    pub request_id: TrimmedNonEmptyString,
    #[serde(rename = "tabId", default, skip_serializing_if = "Optional::is_absent")]
    pub tab_id: Optional<PreviewTabId>,
    #[serde(rename = "timeoutMs")]
    pub timeout_ms: i64,
    #[serde(rename = "remoteTag")]
    pub remote_tag: TrimmedNonEmptyString,
    #[serde(rename = "remoteMessageLength")]
    pub remote_message_length: i64,
    #[serde(
        rename = "remoteDetailKind",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub remote_detail_kind: Optional<PreviewAutomationExecutionErrorRemoteDetailKind>,
    #[serde(rename = "cause")]
    pub cause: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewAutomationInvalidSelectorErrorTag {
    #[serde(rename = "PreviewAutomationInvalidSelectorError")]
    PreviewAutomationInvalidSelectorError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewAutomationInvalidSelectorErrorRemoteDetailKind {
    #[serde(rename = "null")]
    Null,
    #[serde(rename = "array")]
    Array,
    #[serde(rename = "object")]
    Object,
    #[serde(rename = "string")]
    String,
    #[serde(rename = "number")]
    Number,
    #[serde(rename = "boolean")]
    Boolean,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewAutomationInvalidSelectorErrorSelectorKind {
    #[serde(rename = "locator")]
    Locator,
    #[serde(rename = "selector")]
    Selector,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewAutomationInvalidSelectorError {
    #[serde(rename = "_tag")]
    pub _tag: PreviewAutomationInvalidSelectorErrorTag,
    #[serde(rename = "operation")]
    pub operation: PreviewAutomationOperation,
    #[serde(rename = "environmentId")]
    pub environment_id: EnvironmentId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "providerSessionId")]
    pub provider_session_id: TrimmedNonEmptyString,
    #[serde(rename = "providerInstanceId")]
    pub provider_instance_id: ProviderInstanceId,
    #[serde(rename = "clientId")]
    pub client_id: TrimmedNonEmptyString,
    #[serde(rename = "connectionId")]
    pub connection_id: PreviewAutomationConnectionId,
    #[serde(rename = "requestId")]
    pub request_id: TrimmedNonEmptyString,
    #[serde(rename = "tabId", default, skip_serializing_if = "Optional::is_absent")]
    pub tab_id: Optional<PreviewTabId>,
    #[serde(rename = "timeoutMs")]
    pub timeout_ms: i64,
    #[serde(rename = "remoteTag")]
    pub remote_tag: TrimmedNonEmptyString,
    #[serde(rename = "remoteMessageLength")]
    pub remote_message_length: i64,
    #[serde(
        rename = "remoteDetailKind",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub remote_detail_kind: Optional<PreviewAutomationInvalidSelectorErrorRemoteDetailKind>,
    #[serde(rename = "cause")]
    pub cause: serde_json::Value,
    #[serde(
        rename = "selectorKind",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub selector_kind: Optional<PreviewAutomationInvalidSelectorErrorSelectorKind>,
    #[serde(
        rename = "selectorLength",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub selector_length: Optional<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewAutomationMalformedResponseErrorTag {
    #[serde(rename = "PreviewAutomationMalformedResponseError")]
    PreviewAutomationMalformedResponseError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewAutomationMalformedResponseError {
    #[serde(rename = "_tag")]
    pub _tag: PreviewAutomationMalformedResponseErrorTag,
    #[serde(rename = "operation")]
    pub operation: PreviewAutomationOperation,
    #[serde(rename = "environmentId")]
    pub environment_id: EnvironmentId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "providerSessionId")]
    pub provider_session_id: TrimmedNonEmptyString,
    #[serde(rename = "providerInstanceId")]
    pub provider_instance_id: ProviderInstanceId,
    #[serde(rename = "clientId")]
    pub client_id: TrimmedNonEmptyString,
    #[serde(rename = "connectionId")]
    pub connection_id: PreviewAutomationConnectionId,
    #[serde(rename = "requestId")]
    pub request_id: TrimmedNonEmptyString,
    #[serde(rename = "tabId", default, skip_serializing_if = "Optional::is_absent")]
    pub tab_id: Optional<PreviewTabId>,
    #[serde(rename = "timeoutMs")]
    pub timeout_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewAutomationNetworkEntry {
    #[serde(rename = "url")]
    pub url: IsoDateTime,
    #[serde(rename = "method")]
    pub method: IsoDateTime,
    #[serde(rename = "status", deserialize_with = "required_nullable")]
    pub status: Option<JsonNumber>,
    #[serde(rename = "failed")]
    pub failed: bool,
    #[serde(
        rename = "errorText",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub error_text: Optional<IsoDateTime>,
    #[serde(rename = "timestamp")]
    pub timestamp: IsoDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewAutomationNoAvailableHostErrorTag {
    #[serde(rename = "PreviewAutomationNoAvailableHostError")]
    PreviewAutomationNoAvailableHostError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewAutomationNoAvailableHostErrorRemoteDetailKind {
    #[serde(rename = "null")]
    Null,
    #[serde(rename = "array")]
    Array,
    #[serde(rename = "object")]
    Object,
    #[serde(rename = "string")]
    String,
    #[serde(rename = "number")]
    Number,
    #[serde(rename = "boolean")]
    Boolean,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewAutomationNoAvailableHostError {
    #[serde(rename = "_tag")]
    pub _tag: PreviewAutomationNoAvailableHostErrorTag,
    #[serde(rename = "operation")]
    pub operation: PreviewAutomationOperation,
    #[serde(rename = "environmentId")]
    pub environment_id: EnvironmentId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "providerSessionId")]
    pub provider_session_id: TrimmedNonEmptyString,
    #[serde(rename = "providerInstanceId")]
    pub provider_instance_id: ProviderInstanceId,
    #[serde(
        rename = "clientId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub client_id: Optional<TrimmedNonEmptyString>,
    #[serde(
        rename = "connectionId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub connection_id: Optional<PreviewAutomationConnectionId>,
    #[serde(
        rename = "requestId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub request_id: Optional<TrimmedNonEmptyString>,
    #[serde(rename = "tabId", default, skip_serializing_if = "Optional::is_absent")]
    pub tab_id: Optional<PreviewTabId>,
    #[serde(
        rename = "timeoutMs",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub timeout_ms: Optional<i64>,
    #[serde(
        rename = "remoteTag",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub remote_tag: Optional<TrimmedNonEmptyString>,
    #[serde(
        rename = "remoteMessageLength",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub remote_message_length: Optional<i64>,
    #[serde(
        rename = "remoteDetailKind",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub remote_detail_kind: Optional<PreviewAutomationNoAvailableHostErrorRemoteDetailKind>,
    #[serde(rename = "cause", default, skip_serializing_if = "Optional::is_absent")]
    pub cause: Optional<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewAutomationOperation {
    #[serde(rename = "status")]
    Status,
    #[serde(rename = "open")]
    Open,
    #[serde(rename = "navigate")]
    Navigate,
    #[serde(rename = "snapshot")]
    Snapshot,
    #[serde(rename = "click")]
    Click,
    #[serde(rename = "type")]
    Type,
    #[serde(rename = "press")]
    Press,
    #[serde(rename = "scroll")]
    Scroll,
    #[serde(rename = "evaluate")]
    Evaluate,
    #[serde(rename = "waitFor")]
    WaitFor,
    #[serde(rename = "recordingStart")]
    RecordingStart,
    #[serde(rename = "recordingStop")]
    RecordingStop,
    #[serde(rename = "resize")]
    Resize,
    #[serde(rename = "setColorScheme")]
    SetColorScheme,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewAutomationRecordingDeadlineExpiredErrorTag {
    #[serde(rename = "PreviewAutomationRecordingDeadlineExpiredError")]
    PreviewAutomationRecordingDeadlineExpiredError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewAutomationRecordingDeadlineExpiredError {
    #[serde(rename = "_tag")]
    pub _tag: PreviewAutomationRecordingDeadlineExpiredErrorTag,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "cause", default, skip_serializing_if = "Optional::is_absent")]
    pub cause: Optional<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewAutomationRecordingDesktopUpdateRequiredErrorTag {
    #[serde(rename = "PreviewAutomationRecordingDesktopUpdateRequiredError")]
    PreviewAutomationRecordingDesktopUpdateRequiredError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewAutomationRecordingDesktopUpdateRequiredError {
    #[serde(rename = "_tag")]
    pub _tag: PreviewAutomationRecordingDesktopUpdateRequiredErrorTag,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "cause", default, skip_serializing_if = "Optional::is_absent")]
    pub cause: Optional<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewAutomationRecordingTooLargeErrorTag {
    #[serde(rename = "PreviewAutomationRecordingTooLargeError")]
    PreviewAutomationRecordingTooLargeError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewAutomationRecordingTooLargeError {
    #[serde(rename = "_tag")]
    pub _tag: PreviewAutomationRecordingTooLargeErrorTag,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "cause", default, skip_serializing_if = "Optional::is_absent")]
    pub cause: Optional<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewAutomationRecordingTransferErrorTag {
    #[serde(rename = "PreviewAutomationRecordingTransferError")]
    PreviewAutomationRecordingTransferError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewAutomationRecordingTransferError {
    #[serde(rename = "_tag")]
    pub _tag: PreviewAutomationRecordingTransferErrorTag,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "cause", default, skip_serializing_if = "Optional::is_absent")]
    pub cause: Optional<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewAutomationRemoteUnavailableErrorTag {
    #[serde(rename = "PreviewAutomationRemoteUnavailableError")]
    PreviewAutomationRemoteUnavailableError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewAutomationRemoteUnavailableErrorRemoteDetailKind {
    #[serde(rename = "null")]
    Null,
    #[serde(rename = "array")]
    Array,
    #[serde(rename = "object")]
    Object,
    #[serde(rename = "string")]
    String,
    #[serde(rename = "number")]
    Number,
    #[serde(rename = "boolean")]
    Boolean,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewAutomationRemoteUnavailableError {
    #[serde(rename = "_tag")]
    pub _tag: PreviewAutomationRemoteUnavailableErrorTag,
    #[serde(rename = "operation")]
    pub operation: PreviewAutomationOperation,
    #[serde(rename = "environmentId")]
    pub environment_id: EnvironmentId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "providerSessionId")]
    pub provider_session_id: TrimmedNonEmptyString,
    #[serde(rename = "providerInstanceId")]
    pub provider_instance_id: ProviderInstanceId,
    #[serde(rename = "clientId")]
    pub client_id: TrimmedNonEmptyString,
    #[serde(rename = "connectionId")]
    pub connection_id: PreviewAutomationConnectionId,
    #[serde(rename = "requestId")]
    pub request_id: TrimmedNonEmptyString,
    #[serde(rename = "tabId", default, skip_serializing_if = "Optional::is_absent")]
    pub tab_id: Optional<PreviewTabId>,
    #[serde(rename = "timeoutMs")]
    pub timeout_ms: i64,
    #[serde(rename = "remoteTag")]
    pub remote_tag: TrimmedNonEmptyString,
    #[serde(rename = "remoteMessageLength")]
    pub remote_message_length: i64,
    #[serde(
        rename = "remoteDetailKind",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub remote_detail_kind: Optional<PreviewAutomationRemoteUnavailableErrorRemoteDetailKind>,
    #[serde(rename = "cause")]
    pub cause: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewAutomationRequestQueueClosedErrorTag {
    #[serde(rename = "PreviewAutomationRequestQueueClosedError")]
    PreviewAutomationRequestQueueClosedError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewAutomationRequestQueueClosedError {
    #[serde(rename = "_tag")]
    pub _tag: PreviewAutomationRequestQueueClosedErrorTag,
    #[serde(rename = "operation")]
    pub operation: PreviewAutomationOperation,
    #[serde(rename = "environmentId")]
    pub environment_id: EnvironmentId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "providerSessionId")]
    pub provider_session_id: TrimmedNonEmptyString,
    #[serde(rename = "providerInstanceId")]
    pub provider_instance_id: ProviderInstanceId,
    #[serde(rename = "clientId")]
    pub client_id: TrimmedNonEmptyString,
    #[serde(rename = "connectionId")]
    pub connection_id: PreviewAutomationConnectionId,
    #[serde(rename = "requestId")]
    pub request_id: TrimmedNonEmptyString,
    #[serde(rename = "tabId", default, skip_serializing_if = "Optional::is_absent")]
    pub tab_id: Optional<PreviewTabId>,
    #[serde(rename = "timeoutMs")]
    pub timeout_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewAutomationResultTooLargeErrorTag {
    #[serde(rename = "PreviewAutomationResultTooLargeError")]
    PreviewAutomationResultTooLargeError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewAutomationResultTooLargeErrorRemoteDetailKind {
    #[serde(rename = "null")]
    Null,
    #[serde(rename = "array")]
    Array,
    #[serde(rename = "object")]
    Object,
    #[serde(rename = "string")]
    String,
    #[serde(rename = "number")]
    Number,
    #[serde(rename = "boolean")]
    Boolean,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewAutomationResultTooLargeError {
    #[serde(rename = "_tag")]
    pub _tag: PreviewAutomationResultTooLargeErrorTag,
    #[serde(rename = "operation")]
    pub operation: PreviewAutomationOperation,
    #[serde(rename = "environmentId")]
    pub environment_id: EnvironmentId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "providerSessionId")]
    pub provider_session_id: TrimmedNonEmptyString,
    #[serde(rename = "providerInstanceId")]
    pub provider_instance_id: ProviderInstanceId,
    #[serde(rename = "clientId")]
    pub client_id: TrimmedNonEmptyString,
    #[serde(rename = "connectionId")]
    pub connection_id: PreviewAutomationConnectionId,
    #[serde(rename = "requestId")]
    pub request_id: TrimmedNonEmptyString,
    #[serde(rename = "tabId", default, skip_serializing_if = "Optional::is_absent")]
    pub tab_id: Optional<PreviewTabId>,
    #[serde(rename = "timeoutMs")]
    pub timeout_ms: i64,
    #[serde(rename = "remoteTag")]
    pub remote_tag: TrimmedNonEmptyString,
    #[serde(rename = "remoteMessageLength")]
    pub remote_message_length: i64,
    #[serde(
        rename = "remoteDetailKind",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub remote_detail_kind: Optional<PreviewAutomationResultTooLargeErrorRemoteDetailKind>,
    #[serde(rename = "cause")]
    pub cause: serde_json::Value,
    #[serde(
        rename = "maximumBytes",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub maximum_bytes: Optional<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewAutomationSnapshotScreenshotMimeType {
    #[serde(rename = "image/png")]
    ImagePng,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewAutomationSnapshotScreenshot {
    #[serde(rename = "mimeType")]
    pub mime_type: PreviewAutomationSnapshotScreenshotMimeType,
    #[serde(rename = "data")]
    pub data: IsoDateTime,
    #[serde(rename = "width")]
    pub width: i64,
    #[serde(rename = "height")]
    pub height: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewAutomationSnapshot {
    #[serde(rename = "url")]
    pub url: IsoDateTime,
    #[serde(rename = "title")]
    pub title: IsoDateTime,
    #[serde(rename = "loading")]
    pub loading: bool,
    #[serde(rename = "visibleText")]
    pub visible_text: IsoDateTime,
    #[serde(rename = "interactiveElements")]
    pub interactive_elements: Vec<PreviewAutomationElement>,
    #[serde(rename = "accessibilityTree")]
    pub accessibility_tree: serde_json::Value,
    #[serde(rename = "consoleEntries")]
    pub console_entries: Vec<PreviewAutomationConsoleEntry>,
    #[serde(rename = "networkEntries")]
    pub network_entries: Vec<PreviewAutomationNetworkEntry>,
    #[serde(rename = "actionTimeline")]
    pub action_timeline: Vec<PreviewAutomationActionEvent>,
    #[serde(rename = "screenshot")]
    pub screenshot: PreviewAutomationSnapshotScreenshot,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewAutomationStatus {
    #[serde(rename = "available")]
    pub available: bool,
    #[serde(rename = "visible")]
    pub visible: bool,
    #[serde(rename = "tabId", deserialize_with = "required_nullable")]
    pub tab_id: Option<PreviewTabId>,
    #[serde(rename = "url", deserialize_with = "required_nullable")]
    pub url: Option<IsoDateTime>,
    #[serde(rename = "title", deserialize_with = "required_nullable")]
    pub title: Option<IsoDateTime>,
    #[serde(rename = "loading")]
    pub loading: bool,
    #[serde(
        rename = "viewportSetting",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub viewport_setting: Optional<PreviewViewportSetting>,
    #[serde(
        rename = "viewport",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub viewport: Optional<PreviewRenderedViewportSize>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewAutomationTabNotFoundErrorTag {
    #[serde(rename = "PreviewAutomationTabNotFoundError")]
    PreviewAutomationTabNotFoundError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewAutomationTabNotFoundErrorRemoteDetailKind {
    #[serde(rename = "null")]
    Null,
    #[serde(rename = "array")]
    Array,
    #[serde(rename = "object")]
    Object,
    #[serde(rename = "string")]
    String,
    #[serde(rename = "number")]
    Number,
    #[serde(rename = "boolean")]
    Boolean,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewAutomationTabNotFoundError {
    #[serde(rename = "_tag")]
    pub _tag: PreviewAutomationTabNotFoundErrorTag,
    #[serde(rename = "operation")]
    pub operation: PreviewAutomationOperation,
    #[serde(rename = "environmentId")]
    pub environment_id: EnvironmentId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "providerSessionId")]
    pub provider_session_id: TrimmedNonEmptyString,
    #[serde(rename = "providerInstanceId")]
    pub provider_instance_id: ProviderInstanceId,
    #[serde(rename = "clientId")]
    pub client_id: TrimmedNonEmptyString,
    #[serde(rename = "connectionId")]
    pub connection_id: PreviewAutomationConnectionId,
    #[serde(rename = "requestId")]
    pub request_id: TrimmedNonEmptyString,
    #[serde(rename = "tabId", default, skip_serializing_if = "Optional::is_absent")]
    pub tab_id: Optional<PreviewTabId>,
    #[serde(rename = "timeoutMs")]
    pub timeout_ms: i64,
    #[serde(rename = "remoteTag")]
    pub remote_tag: TrimmedNonEmptyString,
    #[serde(rename = "remoteMessageLength")]
    pub remote_message_length: i64,
    #[serde(
        rename = "remoteDetailKind",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub remote_detail_kind: Optional<PreviewAutomationTabNotFoundErrorRemoteDetailKind>,
    #[serde(rename = "cause")]
    pub cause: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewAutomationTargetNotEditableErrorTag {
    #[serde(rename = "PreviewAutomationTargetNotEditableError")]
    PreviewAutomationTargetNotEditableError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewAutomationTargetNotEditableErrorRemoteDetailKind {
    #[serde(rename = "null")]
    Null,
    #[serde(rename = "array")]
    Array,
    #[serde(rename = "object")]
    Object,
    #[serde(rename = "string")]
    String,
    #[serde(rename = "number")]
    Number,
    #[serde(rename = "boolean")]
    Boolean,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewAutomationTargetNotEditableErrorSelectorKind {
    #[serde(rename = "focused-element")]
    FocusedElement,
    #[serde(rename = "locator")]
    Locator,
    #[serde(rename = "selector")]
    Selector,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewAutomationTargetNotEditableError {
    #[serde(rename = "_tag")]
    pub _tag: PreviewAutomationTargetNotEditableErrorTag,
    #[serde(rename = "operation")]
    pub operation: PreviewAutomationOperation,
    #[serde(rename = "environmentId")]
    pub environment_id: EnvironmentId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "providerSessionId")]
    pub provider_session_id: TrimmedNonEmptyString,
    #[serde(rename = "providerInstanceId")]
    pub provider_instance_id: ProviderInstanceId,
    #[serde(rename = "clientId")]
    pub client_id: TrimmedNonEmptyString,
    #[serde(rename = "connectionId")]
    pub connection_id: PreviewAutomationConnectionId,
    #[serde(rename = "requestId")]
    pub request_id: TrimmedNonEmptyString,
    #[serde(rename = "tabId", default, skip_serializing_if = "Optional::is_absent")]
    pub tab_id: Optional<PreviewTabId>,
    #[serde(rename = "timeoutMs")]
    pub timeout_ms: i64,
    #[serde(rename = "remoteTag")]
    pub remote_tag: TrimmedNonEmptyString,
    #[serde(rename = "remoteMessageLength")]
    pub remote_message_length: i64,
    #[serde(
        rename = "remoteDetailKind",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub remote_detail_kind: Optional<PreviewAutomationTargetNotEditableErrorRemoteDetailKind>,
    #[serde(rename = "cause")]
    pub cause: serde_json::Value,
    #[serde(
        rename = "selectorKind",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub selector_kind: Optional<PreviewAutomationTargetNotEditableErrorSelectorKind>,
    #[serde(
        rename = "selectorLength",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub selector_length: Optional<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewAutomationTimeoutErrorTag {
    #[serde(rename = "PreviewAutomationTimeoutError")]
    PreviewAutomationTimeoutError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewAutomationTimeoutErrorRemoteDetailKind {
    #[serde(rename = "null")]
    Null,
    #[serde(rename = "array")]
    Array,
    #[serde(rename = "object")]
    Object,
    #[serde(rename = "string")]
    String,
    #[serde(rename = "number")]
    Number,
    #[serde(rename = "boolean")]
    Boolean,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewAutomationTimeoutError {
    #[serde(rename = "_tag")]
    pub _tag: PreviewAutomationTimeoutErrorTag,
    #[serde(rename = "operation")]
    pub operation: PreviewAutomationOperation,
    #[serde(rename = "environmentId")]
    pub environment_id: EnvironmentId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "providerSessionId")]
    pub provider_session_id: TrimmedNonEmptyString,
    #[serde(rename = "providerInstanceId")]
    pub provider_instance_id: ProviderInstanceId,
    #[serde(rename = "clientId")]
    pub client_id: TrimmedNonEmptyString,
    #[serde(rename = "connectionId")]
    pub connection_id: PreviewAutomationConnectionId,
    #[serde(rename = "requestId")]
    pub request_id: TrimmedNonEmptyString,
    #[serde(rename = "tabId", default, skip_serializing_if = "Optional::is_absent")]
    pub tab_id: Optional<PreviewTabId>,
    #[serde(rename = "timeoutMs")]
    pub timeout_ms: i64,
    #[serde(
        rename = "remoteTag",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub remote_tag: Optional<TrimmedNonEmptyString>,
    #[serde(
        rename = "remoteMessageLength",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub remote_message_length: Optional<i64>,
    #[serde(
        rename = "remoteDetailKind",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub remote_detail_kind: Optional<PreviewAutomationTimeoutErrorRemoteDetailKind>,
    #[serde(rename = "cause", default, skip_serializing_if = "Optional::is_absent")]
    pub cause: Optional<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewAutomationUnavailableErrorTag {
    #[serde(rename = "PreviewAutomationUnavailableError")]
    PreviewAutomationUnavailableError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewAutomationUnavailableErrorCapability {
    #[serde(rename = "preview")]
    Preview,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewAutomationUnavailableError {
    #[serde(rename = "_tag")]
    pub _tag: PreviewAutomationUnavailableErrorTag,
    #[serde(rename = "capability")]
    pub capability: PreviewAutomationUnavailableErrorCapability,
    #[serde(rename = "environmentId")]
    pub environment_id: EnvironmentId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "providerSessionId")]
    pub provider_session_id: TrimmedNonEmptyString,
    #[serde(rename = "providerInstanceId")]
    pub provider_instance_id: ProviderInstanceId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewAutomationUnsupportedClientErrorTag {
    #[serde(rename = "PreviewAutomationUnsupportedClientError")]
    PreviewAutomationUnsupportedClientError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewAutomationUnsupportedClientErrorRemoteDetailKind {
    #[serde(rename = "null")]
    Null,
    #[serde(rename = "array")]
    Array,
    #[serde(rename = "object")]
    Object,
    #[serde(rename = "string")]
    String,
    #[serde(rename = "number")]
    Number,
    #[serde(rename = "boolean")]
    Boolean,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewAutomationUnsupportedClientError {
    #[serde(rename = "_tag")]
    pub _tag: PreviewAutomationUnsupportedClientErrorTag,
    #[serde(rename = "operation")]
    pub operation: PreviewAutomationOperation,
    #[serde(rename = "environmentId")]
    pub environment_id: EnvironmentId,
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "providerSessionId")]
    pub provider_session_id: TrimmedNonEmptyString,
    #[serde(rename = "providerInstanceId")]
    pub provider_instance_id: ProviderInstanceId,
    #[serde(rename = "clientId")]
    pub client_id: TrimmedNonEmptyString,
    #[serde(rename = "connectionId")]
    pub connection_id: PreviewAutomationConnectionId,
    #[serde(rename = "requestId")]
    pub request_id: TrimmedNonEmptyString,
    #[serde(rename = "tabId", default, skip_serializing_if = "Optional::is_absent")]
    pub tab_id: Optional<PreviewTabId>,
    #[serde(rename = "timeoutMs")]
    pub timeout_ms: i64,
    #[serde(rename = "remoteTag")]
    pub remote_tag: TrimmedNonEmptyString,
    #[serde(rename = "remoteMessageLength")]
    pub remote_message_length: i64,
    #[serde(
        rename = "remoteDetailKind",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub remote_detail_kind: Optional<PreviewAutomationUnsupportedClientErrorRemoteDetailKind>,
    #[serde(rename = "cause")]
    pub cause: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PreviewClickError {
    Variant1(ToolFrameworkAiError),
    Variant2(PreviewAutomationError),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewClickInput {
    #[serde(rename = "tabId", default, skip_serializing_if = "Optional::is_absent")]
    pub tab_id: Optional<String>,
    #[serde(
        rename = "selector",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub selector: Optional<String>,
    #[serde(
        rename = "locator",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub locator: Optional<String>,
    #[serde(rename = "x", default, skip_serializing_if = "Optional::is_absent")]
    pub x: Optional<JsonNumber>,
    #[serde(rename = "y", default, skip_serializing_if = "Optional::is_absent")]
    pub y: Optional<JsonNumber>,
    #[serde(
        rename = "timeoutMs",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub timeout_ms: Optional<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewClickResult {
    #[serde(
        rename = "toolIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_icon: Optional<ToolActivityIcon>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PreviewEvaluateError {
    Variant1(ToolFrameworkAiError),
    Variant2(PreviewAutomationError),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewEvaluateInput {
    #[serde(rename = "tabId", default, skip_serializing_if = "Optional::is_absent")]
    pub tab_id: Optional<String>,
    #[serde(rename = "expression")]
    pub expression: String,
    #[serde(
        rename = "awaitPromise",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub await_promise: Optional<bool>,
    #[serde(
        rename = "returnByValue",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub return_by_value: Optional<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewEvaluateResult {
    #[serde(
        rename = "toolIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_icon: Optional<ToolActivityIcon>,
    #[serde(rename = "value")]
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PreviewNavigateError {
    Variant1(ToolFrameworkAiError),
    Variant2(PreviewAutomationError),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewNavigateInputTargetUrl {
    #[serde(rename = "url")]
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewNavigateInputTargetEnvironmentPortProtocol {
    #[serde(rename = "http")]
    Http,
    #[serde(rename = "https")]
    Https,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewNavigateInputTargetEnvironmentPort {
    #[serde(rename = "port")]
    pub port: i64,
    #[serde(
        rename = "protocol",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub protocol: Optional<PreviewNavigateInputTargetEnvironmentPortProtocol>,
    #[serde(rename = "path", default, skip_serializing_if = "Optional::is_absent")]
    pub path: Optional<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum PreviewNavigateInputTarget {
    #[serde(rename = "url")]
    Url(Box<PreviewNavigateInputTargetUrl>),
    #[serde(rename = "environment-port")]
    EnvironmentPort(Box<PreviewNavigateInputTargetEnvironmentPort>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewNavigateInputReadiness {
    #[serde(rename = "load")]
    Load,
    #[serde(rename = "domContentLoaded")]
    DomContentLoaded,
    #[serde(rename = "none")]
    None,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewNavigateInput {
    #[serde(rename = "tabId", default, skip_serializing_if = "Optional::is_absent")]
    pub tab_id: Optional<String>,
    #[serde(rename = "url", default, skip_serializing_if = "Optional::is_absent")]
    pub url: Optional<String>,
    #[serde(
        rename = "target",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub target: Optional<PreviewNavigateInputTarget>,
    #[serde(
        rename = "readiness",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub readiness: Optional<PreviewNavigateInputReadiness>,
    #[serde(
        rename = "timeoutMs",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub timeout_ms: Optional<i64>,
}

pub type PreviewNavigateResult = PreviewAutomationStatus;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PreviewOpenError {
    Variant1(ToolFrameworkAiError),
    Variant2(PreviewAutomationError),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewOpenInput {
    #[serde(rename = "tabId", default, skip_serializing_if = "Optional::is_absent")]
    pub tab_id: Optional<String>,
    #[serde(rename = "url", default, skip_serializing_if = "Optional::is_absent")]
    pub url: Optional<String>,
    #[serde(rename = "open", default, skip_serializing_if = "Optional::is_absent")]
    pub open: Optional<bool>,
    #[serde(rename = "show", default, skip_serializing_if = "Optional::is_absent")]
    pub show: Optional<bool>,
    #[serde(
        rename = "reuseExistingTab",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub reuse_existing_tab: Optional<bool>,
}

pub type PreviewOpenResult = PreviewAutomationStatus;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PreviewPressError {
    Variant1(ToolFrameworkAiError),
    Variant2(PreviewAutomationError),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewPressInputModifiersItem {
    #[serde(rename = "Alt")]
    Alt,
    #[serde(rename = "Control")]
    Control,
    #[serde(rename = "Meta")]
    Meta,
    #[serde(rename = "Shift")]
    Shift,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewPressInput {
    #[serde(rename = "tabId", default, skip_serializing_if = "Optional::is_absent")]
    pub tab_id: Optional<String>,
    #[serde(rename = "key")]
    pub key: String,
    #[serde(
        rename = "modifiers",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub modifiers: Optional<Vec<PreviewPressInputModifiersItem>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewPressResult {
    #[serde(
        rename = "toolIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_icon: Optional<ToolActivityIcon>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PreviewRecordingStartError {
    Variant1(ToolFrameworkAiError),
    Variant2(PreviewAutomationError),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewRecordingStartInput {
    #[serde(rename = "tabId", default, skip_serializing_if = "Optional::is_absent")]
    pub tab_id: Optional<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewRecordingStartResult {
    #[serde(rename = "tabId")]
    pub tab_id: PreviewTabId,
    #[serde(rename = "recording")]
    pub recording: bool,
    #[serde(rename = "startedAt", deserialize_with = "required_nullable")]
    pub started_at: Option<IsoDateTime>,
    #[serde(
        rename = "toolIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_icon: Optional<ToolActivityIcon>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PreviewRecordingStopError {
    Variant1(ToolFrameworkAiError),
    Variant2(PreviewAutomationError),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewRecordingStopInput {
    #[serde(rename = "tabId", default, skip_serializing_if = "Optional::is_absent")]
    pub tab_id: Optional<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewRecordingStopResult {
    #[serde(rename = "id")]
    pub id: IsoDateTime,
    #[serde(rename = "tabId")]
    pub tab_id: PreviewTabId,
    #[serde(rename = "path")]
    pub path: IsoDateTime,
    #[serde(rename = "mimeType")]
    pub mime_type: IsoDateTime,
    #[serde(rename = "sizeBytes")]
    pub size_bytes: i64,
    #[serde(rename = "createdAt")]
    pub created_at: IsoDateTime,
    #[serde(
        rename = "toolIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_icon: Optional<ToolActivityIcon>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PreviewResizeError {
    Variant1(ToolFrameworkAiError),
    Variant2(PreviewAutomationError),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewResizeInputMode {
    #[serde(rename = "fill")]
    Fill,
    #[serde(rename = "freeform")]
    Freeform,
    #[serde(rename = "preset")]
    Preset,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewResizeInputPreset {
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
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewResizeInputOrientation {
    #[serde(rename = "portrait")]
    Portrait,
    #[serde(rename = "landscape")]
    Landscape,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewResizeInput {
    #[serde(rename = "tabId", default, skip_serializing_if = "Optional::is_absent")]
    pub tab_id: Optional<String>,
    #[serde(rename = "mode")]
    pub mode: PreviewResizeInputMode,
    #[serde(
        rename = "preset",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub preset: Optional<PreviewResizeInputPreset>,
    #[serde(rename = "width", default, skip_serializing_if = "Optional::is_absent")]
    pub width: Optional<i64>,
    #[serde(
        rename = "height",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub height: Optional<i64>,
    #[serde(
        rename = "orientation",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub orientation: Optional<PreviewResizeInputOrientation>,
    #[serde(
        rename = "timeoutMs",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub timeout_ms: Optional<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewResizeResult {
    #[serde(rename = "tabId")]
    pub tab_id: PreviewTabId,
    #[serde(rename = "setting")]
    pub setting: PreviewViewportSetting,
    #[serde(rename = "viewport")]
    pub viewport: PreviewRenderedViewportSize,
    #[serde(
        rename = "toolIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_icon: Optional<ToolActivityIcon>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PreviewScrollError {
    Variant1(ToolFrameworkAiError),
    Variant2(PreviewAutomationError),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewScrollInput {
    #[serde(rename = "tabId", default, skip_serializing_if = "Optional::is_absent")]
    pub tab_id: Optional<String>,
    #[serde(
        rename = "deltaX",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub delta_x: Optional<JsonNumber>,
    #[serde(
        rename = "deltaY",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub delta_y: Optional<JsonNumber>,
    #[serde(
        rename = "selector",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub selector: Optional<String>,
    #[serde(
        rename = "locator",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub locator: Optional<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewScrollResult {
    #[serde(
        rename = "toolIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_icon: Optional<ToolActivityIcon>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PreviewSetAppearanceError {
    Variant1(ToolFrameworkAiError),
    Variant2(PreviewAutomationError),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewSetAppearanceInputColorScheme {
    #[serde(rename = "system")]
    System,
    #[serde(rename = "light")]
    Light,
    #[serde(rename = "dark")]
    Dark,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewSetAppearanceInput {
    #[serde(rename = "tabId", default, skip_serializing_if = "Optional::is_absent")]
    pub tab_id: Optional<String>,
    #[serde(rename = "colorScheme")]
    pub color_scheme: PreviewSetAppearanceInputColorScheme,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewSetAppearanceResult {
    #[serde(rename = "tabId")]
    pub tab_id: PreviewTabId,
    #[serde(rename = "colorScheme")]
    pub color_scheme: PreviewAutomationColorScheme,
    #[serde(
        rename = "toolIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_icon: Optional<ToolActivityIcon>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PreviewSnapshotError {
    Variant1(ToolFrameworkAiError),
    Variant2(PreviewAutomationError),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewSnapshotInput {
    #[serde(rename = "tabId", default, skip_serializing_if = "Optional::is_absent")]
    pub tab_id: Optional<String>,
    #[serde(
        rename = "includeImage",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub include_image: Optional<bool>,
    #[serde(rename = "save", default, skip_serializing_if = "Optional::is_absent")]
    pub save: Optional<bool>,
}

pub type PreviewSnapshotResult = PreviewAutomationSnapshot;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PreviewStatusError {
    Variant1(ToolFrameworkAiError),
    Variant2(PreviewAutomationError),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewStatusInput {
    #[serde(rename = "tabId", default, skip_serializing_if = "Optional::is_absent")]
    pub tab_id: Optional<String>,
}

pub type PreviewStatusResult = PreviewAutomationStatus;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PreviewTypeError {
    Variant1(ToolFrameworkAiError),
    Variant2(PreviewAutomationError),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewTypeInput {
    #[serde(rename = "tabId", default, skip_serializing_if = "Optional::is_absent")]
    pub tab_id: Optional<String>,
    #[serde(rename = "text")]
    pub text: String,
    #[serde(
        rename = "selector",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub selector: Optional<String>,
    #[serde(
        rename = "locator",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub locator: Optional<String>,
    #[serde(rename = "clear", default, skip_serializing_if = "Optional::is_absent")]
    pub clear: Optional<bool>,
    #[serde(
        rename = "timeoutMs",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub timeout_ms: Optional<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewTypeResult {
    #[serde(
        rename = "toolIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_icon: Optional<ToolActivityIcon>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PreviewWaitForError {
    Variant1(ToolFrameworkAiError),
    Variant2(PreviewAutomationError),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewWaitForInput {
    #[serde(rename = "tabId", default, skip_serializing_if = "Optional::is_absent")]
    pub tab_id: Optional<String>,
    #[serde(
        rename = "selector",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub selector: Optional<String>,
    #[serde(
        rename = "locator",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub locator: Optional<String>,
    #[serde(rename = "text", default, skip_serializing_if = "Optional::is_absent")]
    pub text: Optional<String>,
    #[serde(
        rename = "urlIncludes",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub url_includes: Optional<String>,
    #[serde(
        rename = "timeoutMs",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub timeout_ms: Optional<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewWaitForResult {
    #[serde(
        rename = "toolIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub tool_icon: Optional<ToolActivityIcon>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RunScheduledTaskNowError {
    Variant1(ToolFrameworkAiError),
    Variant2(OrchestratorMcpFailure),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunScheduledTaskNowInput {
    #[serde(rename = "taskId")]
    pub task_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunScheduledTaskNowResult {
    #[serde(rename = "taskId")]
    pub task_id: ScheduledTaskId,
    #[serde(rename = "threadId", deserialize_with = "required_nullable")]
    pub thread_id: Option<ThreadId>,
    #[serde(rename = "lastRunStatus")]
    pub last_run_status: ScheduledTaskRunStatus,
    #[serde(rename = "runCount")]
    pub run_count: NonNegativeInt,
    #[serde(rename = "nextRunAt", deserialize_with = "required_nullable")]
    pub next_run_at: Option<IsoDateTime>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ScheduleTaskError {
    Variant1(ToolFrameworkAiError),
    Variant2(OrchestratorMcpFailure),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ScheduleTaskInputScheduleVariant1Type {
    #[serde(rename = "interval")]
    Interval,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduleTaskInputScheduleVariant1 {
    #[serde(rename = "type")]
    pub r#type: ScheduleTaskInputScheduleVariant1Type,
    #[serde(rename = "everyMs")]
    pub every_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ScheduleTaskInputScheduleVariant2Type {
    #[serde(rename = "fixed_time")]
    FixedTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduleTaskInputScheduleVariant2 {
    #[serde(rename = "type")]
    pub r#type: ScheduleTaskInputScheduleVariant2Type,
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
#[serde(untagged)]
pub enum ScheduleTaskInputSchedule {
    Variant1(ScheduleTaskInputScheduleVariant1),
    Variant2(ScheduleTaskInputScheduleVariant2),
    Variant3(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduleTaskInput {
    #[serde(rename = "prompt")]
    pub prompt: String,
    #[serde(rename = "schedule")]
    pub schedule: ScheduleTaskInputSchedule,
    #[serde(rename = "title", default, skip_serializing_if = "Optional::is_absent")]
    pub title: Optional<String>,
    #[serde(
        rename = "enabled",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub enabled: Optional<bool>,
    #[serde(
        rename = "bindToCurrentThread",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub bind_to_current_thread: Optional<bool>,
    #[serde(
        rename = "clientRequestId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub client_request_id: Optional<String>,
}

pub type ScheduleTaskResult = OrchestratorMcpScheduleTaskResult;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum T3AttachmentDiscardError {
    Variant1(ToolFrameworkAiError),
    Variant2(OrchestratorMcpFailure),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3AttachmentDiscardInput {
    #[serde(rename = "attachmentId")]
    pub attachment_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3AttachmentDiscardResult {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum T3AttachmentPrepareUploadError {
    Variant1(ToolFrameworkAiError),
    Variant2(OrchestratorMcpFailure),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum T3AttachmentPrepareUploadInputUploadVariant1Type {
    #[serde(rename = "image")]
    Image,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum T3AttachmentPrepareUploadInputUploadVariant1MimeType {
    #[serde(rename = "image/gif")]
    ImageGif,
    #[serde(rename = "image/jpeg")]
    ImageJpeg,
    #[serde(rename = "image/png")]
    ImagePng,
    #[serde(rename = "image/webp")]
    ImageWebp,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3AttachmentPrepareUploadInputUploadVariant1 {
    #[serde(rename = "type", default, skip_serializing_if = "Optional::is_absent")]
    pub r#type: Optional<T3AttachmentPrepareUploadInputUploadVariant1Type>,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "mimeType")]
    pub mime_type: T3AttachmentPrepareUploadInputUploadVariant1MimeType,
    #[serde(rename = "sizeBytes")]
    pub size_bytes: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum T3AttachmentPrepareUploadInputUploadVariant2Type {
    #[serde(rename = "file")]
    File,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3AttachmentPrepareUploadInputUploadVariant2 {
    #[serde(rename = "type")]
    pub r#type: T3AttachmentPrepareUploadInputUploadVariant2Type,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    #[serde(rename = "sizeBytes")]
    pub size_bytes: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum T3AttachmentPrepareUploadInputUpload {
    Variant1(T3AttachmentPrepareUploadInputUploadVariant1),
    Variant2(T3AttachmentPrepareUploadInputUploadVariant2),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3AttachmentPrepareUploadInput {
    #[serde(rename = "upload")]
    pub upload: T3AttachmentPrepareUploadInputUpload,
}

pub type T3AttachmentPrepareUploadResult = AttachmentCreateUploadUrlResult;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum T3EnvironmentPreferencesUpdateError {
    Variant1(ToolFrameworkAiError),
    Variant2(OrchestratorMcpFailure),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3EnvironmentPreferencesUpdateInputBackgroundActivity {
    #[serde(rename = "profile")]
    pub profile: BackgroundActivityProfile,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum T3EnvironmentPreferencesUpdateInputSourceControlWritingStyleMode {
    #[serde(rename = "repo_conventions")]
    RepoConventions,
    #[serde(rename = "conventional_commits")]
    ConventionalCommits,
    #[serde(rename = "custom")]
    Custom,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3EnvironmentPreferencesUpdateInputSourceControlWritingStyle {
    #[serde(rename = "mode", default, skip_serializing_if = "Optional::is_absent")]
    pub mode: Optional<T3EnvironmentPreferencesUpdateInputSourceControlWritingStyleMode>,
    #[serde(
        rename = "customInstructions",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub custom_instructions: Optional<String>,
    #[serde(
        rename = "followChangeRequestTemplates",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub follow_change_request_templates: Optional<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3EnvironmentPreferencesUpdateInput {
    #[serde(
        rename = "defaultThreadEnvMode",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub default_thread_env_mode: Optional<Option<ThreadEnvMode>>,
    #[serde(
        rename = "newWorktreesStartFromOrigin",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub new_worktrees_start_from_origin: Optional<bool>,
    #[serde(
        rename = "enableProviderUpdateChecks",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub enable_provider_update_checks: Optional<bool>,
    #[serde(
        rename = "backgroundActivity",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub background_activity: Optional<T3EnvironmentPreferencesUpdateInputBackgroundActivity>,
    #[serde(
        rename = "sourceControlWritingStyle",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub source_control_writing_style:
        Optional<T3EnvironmentPreferencesUpdateInputSourceControlWritingStyle>,
}

fn t3_environment_preferences_update_result_default_thread_env_mode_default()
-> Option<ThreadEnvMode> {
    serde_json::from_str("null").expect("upstream default")
}

fn t3_environment_preferences_update_result_new_worktrees_start_from_origin_default() -> bool {
    serde_json::from_str("true").expect("upstream default")
}

fn t3_environment_preferences_update_result_enable_provider_update_checks_default() -> bool {
    serde_json::from_str("true").expect("upstream default")
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3EnvironmentPreferencesUpdateResultBackgroundActivity {
    #[serde(rename = "profile")]
    pub profile: BackgroundActivityProfileSelection,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3EnvironmentPreferencesUpdateResultSourceControlWritingStyle {
    #[serde(rename = "mode")]
    pub mode: IsoDateTime,
    #[serde(rename = "followChangeRequestTemplates")]
    pub follow_change_request_templates: bool,
    #[serde(rename = "customInstructions")]
    pub custom_instructions: IsoDateTime,
    #[serde(rename = "truncated")]
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3EnvironmentPreferencesUpdateResult {
    #[serde(
        rename = "defaultThreadEnvMode",
        default = "t3_environment_preferences_update_result_default_thread_env_mode_default"
    )]
    pub default_thread_env_mode: Option<ThreadEnvMode>,
    #[serde(
        rename = "newWorktreesStartFromOrigin",
        default = "t3_environment_preferences_update_result_new_worktrees_start_from_origin_default"
    )]
    pub new_worktrees_start_from_origin: bool,
    #[serde(
        rename = "enableProviderUpdateChecks",
        default = "t3_environment_preferences_update_result_enable_provider_update_checks_default"
    )]
    pub enable_provider_update_checks: bool,
    #[serde(rename = "backgroundActivity")]
    pub background_activity: T3EnvironmentPreferencesUpdateResultBackgroundActivity,
    #[serde(rename = "sourceControlWritingStyle")]
    pub source_control_writing_style: T3EnvironmentPreferencesUpdateResultSourceControlWritingStyle,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum T3EnvironmentReadError {
    Variant1(ToolFrameworkAiError),
    Variant2(OrchestratorMcpFailure),
    Variant3(ToolFrameworkExecutionFailure),
}

pub type T3EnvironmentReadInput = BTreeMap<IsoDateTime, Never>;

fn t3_environment_read_result_preferences_default_thread_env_mode_default() -> Option<ThreadEnvMode>
{
    serde_json::from_str("null").expect("upstream default")
}

fn t3_environment_read_result_preferences_new_worktrees_start_from_origin_default() -> bool {
    serde_json::from_str("true").expect("upstream default")
}

fn t3_environment_read_result_preferences_enable_provider_update_checks_default() -> bool {
    serde_json::from_str("true").expect("upstream default")
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3EnvironmentReadResultPreferencesBackgroundActivity {
    #[serde(rename = "profile")]
    pub profile: BackgroundActivityProfileSelection,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3EnvironmentReadResultPreferencesSourceControlWritingStyle {
    #[serde(rename = "mode")]
    pub mode: IsoDateTime,
    #[serde(rename = "followChangeRequestTemplates")]
    pub follow_change_request_templates: bool,
    #[serde(rename = "customInstructions")]
    pub custom_instructions: IsoDateTime,
    #[serde(rename = "truncated")]
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3EnvironmentReadResultPreferences {
    #[serde(
        rename = "defaultThreadEnvMode",
        default = "t3_environment_read_result_preferences_default_thread_env_mode_default"
    )]
    pub default_thread_env_mode: Option<ThreadEnvMode>,
    #[serde(
        rename = "newWorktreesStartFromOrigin",
        default = "t3_environment_read_result_preferences_new_worktrees_start_from_origin_default"
    )]
    pub new_worktrees_start_from_origin: bool,
    #[serde(
        rename = "enableProviderUpdateChecks",
        default = "t3_environment_read_result_preferences_enable_provider_update_checks_default"
    )]
    pub enable_provider_update_checks: bool,
    #[serde(rename = "backgroundActivity")]
    pub background_activity: T3EnvironmentReadResultPreferencesBackgroundActivity,
    #[serde(rename = "sourceControlWritingStyle")]
    pub source_control_writing_style: T3EnvironmentReadResultPreferencesSourceControlWritingStyle,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3EnvironmentReadResult {
    #[serde(rename = "environmentId")]
    pub environment_id: EnvironmentId,
    #[serde(rename = "label")]
    pub label: IsoDateTime,
    #[serde(rename = "serverVersion")]
    pub server_version: IsoDateTime,
    #[serde(rename = "platform")]
    pub platform: ExecutionEnvironmentPlatform,
    #[serde(rename = "preferences")]
    pub preferences: T3EnvironmentReadResultPreferences,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum T3PendingRequestListError {
    Variant1(ToolFrameworkAiError),
    Variant2(OrchestratorMcpFailure),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3PendingRequestListInput {
    #[serde(
        rename = "threadId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub thread_id: Optional<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3PendingRequestListResult {
    #[serde(rename = "requestIds")]
    pub request_ids: Vec<RuntimeRequestId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum T3PendingRequestReadError {
    Variant1(ToolFrameworkAiError),
    Variant2(OrchestratorMcpFailure),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3PendingRequestReadInput {
    #[serde(
        rename = "threadId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub thread_id: Optional<String>,
    #[serde(rename = "requestId")]
    pub request_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3PendingRequestReadResultQuestionsItemOptionsItem {
    #[serde(rename = "label")]
    pub label: IsoDateTime,
    #[serde(rename = "description")]
    pub description: IsoDateTime,
    #[serde(rename = "value", default, skip_serializing_if = "Optional::is_absent")]
    pub value: Optional<IsoDateTime>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3PendingRequestReadResultQuestionsItem {
    #[serde(rename = "id")]
    pub id: IsoDateTime,
    #[serde(rename = "header")]
    pub header: IsoDateTime,
    #[serde(rename = "question")]
    pub question: IsoDateTime,
    #[serde(rename = "options")]
    pub options: Vec<T3PendingRequestReadResultQuestionsItemOptionsItem>,
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
pub struct T3PendingRequestReadResult {
    #[serde(rename = "requestId")]
    pub request_id: RuntimeRequestId,
    #[serde(rename = "questions")]
    pub questions: Vec<T3PendingRequestReadResultQuestionsItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum T3PendingRequestRespondError {
    Variant1(ToolFrameworkAiError),
    Variant2(OrchestratorMcpFailure),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3PendingRequestRespondInput {
    #[serde(
        rename = "threadId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub thread_id: Optional<String>,
    #[serde(rename = "requestId")]
    pub request_id: String,
    #[serde(rename = "answers")]
    pub answers: ProviderUserInputAnswers,
}

pub type T3PendingRequestRespondResult = OrchestrationV2DispatchCommandResult;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum T3PreviewCloseError {
    Variant1(ToolFrameworkAiError),
    Variant2(OrchestratorMcpFailure),
    Variant3(PreviewAutomationUnavailableError),
    Variant4(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3PreviewCloseInput {
    #[serde(rename = "tabId")]
    pub tab_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3PreviewCloseResult {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum T3PreviewListError {
    Variant1(ToolFrameworkAiError),
    Variant2(OrchestratorMcpFailure),
    Variant3(PreviewAutomationUnavailableError),
    Variant4(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3PreviewListInput {
    #[serde(
        rename = "cursor",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub cursor: Optional<NonNegativeInt>,
    #[serde(rename = "limit", default, skip_serializing_if = "Optional::is_absent")]
    pub limit: Optional<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3PreviewListResult {
    #[serde(rename = "sessions")]
    pub sessions: Vec<PreviewSessionSnapshot>,
    #[serde(rename = "serverEpoch")]
    pub server_epoch: TrimmedNonEmptyString,
    #[serde(rename = "revision")]
    pub revision: NonNegativeInt,
    #[serde(rename = "nextCursor", deserialize_with = "required_nullable")]
    pub next_cursor: Option<NonNegativeInt>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum T3ProjectCloneError {
    Variant1(ToolFrameworkAiError),
    Variant2(OrchestratorMcpFailure),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3ProjectCloneInput {
    #[serde(
        rename = "provider",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider: Optional<SourceControlProviderKind>,
    #[serde(
        rename = "repository",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub repository: Optional<String>,
    #[serde(
        rename = "remoteUrl",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub remote_url: Optional<String>,
    #[serde(rename = "destinationPath")]
    pub destination_path: String,
    #[serde(
        rename = "protocol",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub protocol: Optional<SourceControlCloneProtocol>,
}

pub type T3ProjectCloneResult = SourceControlCloneRepositoryResult;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum T3ProjectCreateError {
    Variant1(ToolFrameworkAiError),
    Variant2(OrchestratorMcpFailure),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3ProjectCreateInputDefaultModelSelection {
    #[serde(
        rename = "provider",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider: Optional<serde_json::Value>,
    #[serde(
        rename = "instanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub instance_id: Optional<serde_json::Value>,
    #[serde(rename = "model")]
    pub model: serde_json::Value,
    #[serde(
        rename = "options",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub options: Optional<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3ProjectCreateInputScriptsItem {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "command")]
    pub command: String,
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
    pub preview_url: Optional<String>,
    #[serde(
        rename = "autoOpenPreview",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub auto_open_preview: Optional<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3ProjectCreateInput {
    #[serde(rename = "title")]
    pub title: String,
    #[serde(
        rename = "workspaceRoot",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub workspace_root: Optional<String>,
    #[serde(
        rename = "createWorkspaceRootIfMissing",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub create_workspace_root_if_missing: Optional<bool>,
    #[serde(
        rename = "defaultModelSelection",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub default_model_selection: Optional<Option<T3ProjectCreateInputDefaultModelSelection>>,
    #[serde(
        rename = "scripts",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub scripts: Optional<Vec<T3ProjectCreateInputScriptsItem>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3ProjectCreateResult {
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
    #[serde(
        rename = "commitError",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub commit_error: Optional<IsoDateTime>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum T3ProjectDeleteError {
    Variant1(ToolFrameworkAiError),
    Variant2(OrchestratorMcpFailure),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3ProjectDeleteInput {
    #[serde(rename = "projectId")]
    pub project_id: String,
    #[serde(rename = "force", default, skip_serializing_if = "Optional::is_absent")]
    pub force: Optional<bool>,
}

pub type T3ProjectDeleteResult = Project;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum T3ProjectListError {
    Variant1(ToolFrameworkAiError),
    Variant2(OrchestratorMcpFailure),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3ProjectListInput {
    #[serde(
        rename = "cursor",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub cursor: Optional<NonNegativeInt>,
    #[serde(rename = "limit", default, skip_serializing_if = "Optional::is_absent")]
    pub limit: Optional<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3ProjectListResult {
    #[serde(rename = "projects")]
    pub projects: Vec<Project>,
    #[serde(rename = "nextCursor", deserialize_with = "required_nullable")]
    pub next_cursor: Option<NonNegativeInt>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum T3ProjectReadError {
    Variant1(ToolFrameworkAiError),
    Variant2(OrchestratorMcpFailure),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3ProjectReadInput {
    #[serde(rename = "projectId")]
    pub project_id: String,
}

pub type T3ProjectReadResult = Project;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum T3ProjectUpdateError {
    Variant1(ToolFrameworkAiError),
    Variant2(OrchestratorMcpFailure),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3ProjectUpdateInputDefaultModelSelection {
    #[serde(
        rename = "provider",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider: Optional<serde_json::Value>,
    #[serde(
        rename = "instanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub instance_id: Optional<serde_json::Value>,
    #[serde(rename = "model")]
    pub model: serde_json::Value,
    #[serde(
        rename = "options",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub options: Optional<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3ProjectUpdateInputProjectIconLucide {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "color")]
    pub color: ProjectIconColor,
    #[serde(
        rename = "monogramText",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub monogram_text: Optional<String>,
    #[serde(
        rename = "monogram",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub monogram: Optional<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3ProjectUpdateInputProjectIconEmoji {
    #[serde(rename = "emoji")]
    pub emoji: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3ProjectUpdateInputProjectIconMonogram {
    #[serde(rename = "text")]
    pub text: String,
    #[serde(rename = "color")]
    pub color: ProjectIconColor,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum T3ProjectUpdateInputProjectIcon {
    #[serde(rename = "lucide")]
    Lucide(Box<T3ProjectUpdateInputProjectIconLucide>),
    #[serde(rename = "emoji")]
    Emoji(Box<T3ProjectUpdateInputProjectIconEmoji>),
    #[serde(rename = "monogram")]
    Monogram(Box<T3ProjectUpdateInputProjectIconMonogram>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3ProjectUpdateInputScriptsItem {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "command")]
    pub command: String,
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
    pub preview_url: Optional<String>,
    #[serde(
        rename = "autoOpenPreview",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub auto_open_preview: Optional<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3ProjectUpdateInput {
    #[serde(rename = "projectId")]
    pub project_id: String,
    #[serde(rename = "title", default, skip_serializing_if = "Optional::is_absent")]
    pub title: Optional<String>,
    #[serde(
        rename = "workspaceRoot",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub workspace_root: Optional<String>,
    #[serde(
        rename = "defaultModelSelection",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub default_model_selection: Optional<Option<T3ProjectUpdateInputDefaultModelSelection>>,
    #[serde(
        rename = "autoPull",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub auto_pull: Optional<bool>,
    #[serde(
        rename = "projectIcon",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub project_icon: Optional<Option<T3ProjectUpdateInputProjectIcon>>,
    #[serde(
        rename = "faviconPath",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub favicon_path: Optional<Option<String>>,
    #[serde(
        rename = "defaultThreadEnvMode",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub default_thread_env_mode: Optional<Option<ThreadEnvMode>>,
    #[serde(
        rename = "scripts",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub scripts: Optional<Vec<T3ProjectUpdateInputScriptsItem>>,
}

pub type T3ProjectUpdateResult = Project;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum T3QueueCancelError {
    Variant1(ToolFrameworkAiError),
    Variant2(OrchestratorMcpFailure),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3QueueCancelInput {
    #[serde(
        rename = "threadId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub thread_id: Optional<String>,
    #[serde(rename = "queuedRunId")]
    pub queued_run_id: String,
}

pub type T3QueueCancelResult = OrchestrationV2DispatchCommandResult;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum T3QueueEditError {
    Variant1(ToolFrameworkAiError),
    Variant2(OrchestratorMcpFailure),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3QueueEditInput {
    #[serde(
        rename = "threadId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub thread_id: Optional<String>,
    #[serde(rename = "queuedRunId")]
    pub queued_run_id: String,
    #[serde(rename = "text")]
    pub text: String,
}

pub type T3QueueEditResult = OrchestrationV2DispatchCommandResult;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum T3QueueListError {
    Variant1(ToolFrameworkAiError),
    Variant2(OrchestratorMcpFailure),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3QueueListInput {
    #[serde(
        rename = "threadId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub thread_id: Optional<String>,
    #[serde(
        rename = "cursor",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub cursor: Optional<NonNegativeInt>,
    #[serde(rename = "limit", default, skip_serializing_if = "Optional::is_absent")]
    pub limit: Optional<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3QueueListResultItemsItem {
    #[serde(rename = "queuedRunId")]
    pub queued_run_id: RunId,
    #[serde(rename = "text")]
    pub text: IsoDateTime,
    #[serde(rename = "truncated")]
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3QueueListResult {
    #[serde(rename = "items")]
    pub items: Vec<T3QueueListResultItemsItem>,
    #[serde(rename = "nextCursor", deserialize_with = "required_nullable")]
    pub next_cursor: Option<NonNegativeInt>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum T3QueuePromoteToSteerError {
    Variant1(ToolFrameworkAiError),
    Variant2(OrchestratorMcpFailure),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3QueuePromoteToSteerInput {
    #[serde(
        rename = "threadId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub thread_id: Optional<String>,
    #[serde(rename = "queuedRunId")]
    pub queued_run_id: String,
    #[serde(rename = "targetRunId")]
    pub target_run_id: String,
}

pub type T3QueuePromoteToSteerResult = OrchestrationV2DispatchCommandResult;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum T3QueueReadError {
    Variant1(ToolFrameworkAiError),
    Variant2(OrchestratorMcpFailure),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3QueueReadInput {
    #[serde(
        rename = "threadId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub thread_id: Optional<String>,
    #[serde(rename = "queuedRunId")]
    pub queued_run_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3QueueReadResult {
    #[serde(rename = "queuedRunId")]
    pub queued_run_id: RunId,
    #[serde(rename = "text")]
    pub text: IsoDateTime,
    #[serde(rename = "truncated")]
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum T3QueueReorderError {
    Variant1(ToolFrameworkAiError),
    Variant2(OrchestratorMcpFailure),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3QueueReorderInput {
    #[serde(
        rename = "threadId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub thread_id: Optional<String>,
    #[serde(rename = "queuedRunId")]
    pub queued_run_id: String,
    #[serde(rename = "beforeRunId", deserialize_with = "required_nullable")]
    pub before_run_id: Option<String>,
}

pub type T3QueueReorderResult = OrchestrationV2DispatchCommandResult;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum T3ThreadConfigurationError {
    Variant1(ToolFrameworkAiError),
    Variant2(OrchestratorMcpFailure),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3ThreadConfigurationInput {
    #[serde(
        rename = "threadId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub thread_id: Optional<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3ThreadConfigurationResult {
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "modelSelection")]
    pub model_selection: ModelSelection,
    #[serde(rename = "runtimeMode")]
    pub runtime_mode: RuntimeMode,
    #[serde(rename = "interactionMode")]
    pub interaction_mode: ProviderInteractionMode,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum T3ThreadConfigureError {
    Variant1(ToolFrameworkAiError),
    Variant2(OrchestratorMcpFailure),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3ThreadConfigureInputModelSelection {
    #[serde(
        rename = "provider",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider: Optional<serde_json::Value>,
    #[serde(
        rename = "instanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub instance_id: Optional<serde_json::Value>,
    #[serde(rename = "model")]
    pub model: serde_json::Value,
    #[serde(
        rename = "options",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub options: Optional<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3ThreadConfigureInput {
    #[serde(rename = "modelSelection")]
    pub model_selection: T3ThreadConfigureInputModelSelection,
}

pub type T3ThreadConfigureResult = OrchestrationV2DispatchCommandResult;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum T3ThreadForkError {
    Variant1(ToolFrameworkAiError),
    Variant2(OrchestratorMcpFailure),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3ThreadForkInputSourcePointLatestStable {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3ThreadForkInputSourcePointRun {
    #[serde(rename = "runId")]
    pub run_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3ThreadForkInputSourcePointCheckpoint {
    #[serde(rename = "checkpointId")]
    pub checkpoint_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum T3ThreadForkInputSourcePoint {
    #[serde(rename = "latest_stable")]
    LatestStable(Box<T3ThreadForkInputSourcePointLatestStable>),
    #[serde(rename = "run")]
    Run(Box<T3ThreadForkInputSourcePointRun>),
    #[serde(rename = "checkpoint")]
    Checkpoint(Box<T3ThreadForkInputSourcePointCheckpoint>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3ThreadForkInput {
    #[serde(rename = "sourcePoint")]
    pub source_point: T3ThreadForkInputSourcePoint,
    #[serde(rename = "title", default, skip_serializing_if = "Optional::is_absent")]
    pub title: Optional<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3ThreadForkResult {
    #[serde(rename = "sequence")]
    pub sequence: NonNegativeInt,
    #[serde(rename = "targetThreadId")]
    pub target_thread_id: ThreadId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum T3ThreadInterruptError {
    Variant1(ToolFrameworkAiError),
    Variant2(OrchestratorMcpFailure),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3ThreadInterruptInput {
    #[serde(rename = "threadId")]
    pub thread_id: String,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<String>,
    #[serde(
        rename = "reason",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub reason: Optional<String>,
    #[serde(
        rename = "clientRequestId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub client_request_id: Optional<String>,
}

pub type T3ThreadInterruptResult = OrchestratorMcpThreadInterruptResult;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum T3ThreadLaunchError {
    Variant1(ToolFrameworkAiError),
    Variant2(OrchestratorMcpFailure),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3ThreadLaunchInputModelSelection {
    #[serde(
        rename = "provider",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub provider: Optional<serde_json::Value>,
    #[serde(
        rename = "instanceId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub instance_id: Optional<serde_json::Value>,
    #[serde(rename = "model")]
    pub model: serde_json::Value,
    #[serde(
        rename = "options",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub options: Optional<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3ThreadLaunchInputWorkspaceStrategyRoot {
    #[serde(
        rename = "branch",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub branch: Optional<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3ThreadLaunchInputWorkspaceStrategyExistingWorktree {
    #[serde(rename = "worktreePath")]
    pub worktree_path: String,
    #[serde(
        rename = "branch",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub branch: Optional<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3ThreadLaunchInputWorkspaceStrategyWorktree {
    #[serde(rename = "baseRef")]
    pub base_ref: String,
    #[serde(
        rename = "branch",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub branch: Optional<String>,
    #[serde(
        rename = "startFromOrigin",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub start_from_origin: Optional<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum T3ThreadLaunchInputWorkspaceStrategy {
    #[serde(rename = "root")]
    Root(Box<T3ThreadLaunchInputWorkspaceStrategyRoot>),
    #[serde(rename = "existing_worktree")]
    ExistingWorktree(Box<T3ThreadLaunchInputWorkspaceStrategyExistingWorktree>),
    #[serde(rename = "worktree")]
    Worktree(Box<T3ThreadLaunchInputWorkspaceStrategyWorktree>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3ThreadLaunchInputAttachmentsItemImage {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    #[serde(rename = "sizeBytes")]
    pub size_bytes: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3ThreadLaunchInputAttachmentsItemFile {
    #[serde(rename = "id")]
    pub id: String,
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
pub enum T3ThreadLaunchInputAttachmentsItem {
    #[serde(rename = "image")]
    Image(Box<T3ThreadLaunchInputAttachmentsItemImage>),
    #[serde(rename = "file")]
    File(Box<T3ThreadLaunchInputAttachmentsItemFile>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3ThreadLaunchInput {
    #[serde(
        rename = "projectId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub project_id: Optional<String>,
    #[serde(
        rename = "scratch",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub scratch: Optional<bool>,
    #[serde(rename = "title")]
    pub title: String,
    #[serde(
        rename = "modelSelection",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub model_selection: Optional<T3ThreadLaunchInputModelSelection>,
    #[serde(
        rename = "runtimeMode",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub runtime_mode: Optional<RuntimeMode>,
    #[serde(
        rename = "interactionMode",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub interaction_mode: Optional<ProviderInteractionMode>,
    #[serde(
        rename = "workspaceStrategy",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub workspace_strategy: Optional<T3ThreadLaunchInputWorkspaceStrategy>,
    #[serde(
        rename = "message",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub message: Optional<String>,
    #[serde(
        rename = "attachments",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub attachments: Optional<Vec<T3ThreadLaunchInputAttachmentsItem>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3ThreadLaunchResult {
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "projectId")]
    pub project_id: ProjectId,
    #[serde(rename = "modelSelection")]
    pub model_selection: ModelSelection,
    #[serde(rename = "runId", deserialize_with = "required_nullable")]
    pub run_id: Option<RunId>,
    #[serde(rename = "status", deserialize_with = "required_nullable")]
    pub status: Option<OrchestrationV2RunStatus>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum T3ThreadListError {
    Variant1(ToolFrameworkAiError),
    Variant2(OrchestratorMcpFailure),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3ThreadListInput {
    #[serde(
        rename = "statuses",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub statuses: Optional<Vec<OrchestratorMcpThreadStatus>>,
    #[serde(
        rename = "titleContains",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub title_contains: Optional<String>,
    #[serde(
        rename = "settled",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub settled: Optional<bool>,
    #[serde(
        rename = "includeSubagents",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub include_subagents: Optional<bool>,
    #[serde(
        rename = "cursor",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub cursor: Optional<NonNegativeInt>,
    #[serde(rename = "limit", default, skip_serializing_if = "Optional::is_absent")]
    pub limit: Optional<i64>,
}

pub type T3ThreadListResult = OrchestratorMcpThreadListResult;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum T3ThreadMergeBackError {
    Variant1(ToolFrameworkAiError),
    Variant2(OrchestratorMcpFailure),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3ThreadMergeBackInputSourcePointLatestStable {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3ThreadMergeBackInputSourcePointRun {
    #[serde(rename = "runId")]
    pub run_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3ThreadMergeBackInputSourcePointCheckpoint {
    #[serde(rename = "checkpointId")]
    pub checkpoint_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum T3ThreadMergeBackInputSourcePoint {
    #[serde(rename = "latest_stable")]
    LatestStable(Box<T3ThreadMergeBackInputSourcePointLatestStable>),
    #[serde(rename = "run")]
    Run(Box<T3ThreadMergeBackInputSourcePointRun>),
    #[serde(rename = "checkpoint")]
    Checkpoint(Box<T3ThreadMergeBackInputSourcePointCheckpoint>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3ThreadMergeBackInput {
    #[serde(rename = "targetThreadId")]
    pub target_thread_id: String,
    #[serde(rename = "sourcePoint")]
    pub source_point: T3ThreadMergeBackInputSourcePoint,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3ThreadMergeBackResult {
    #[serde(rename = "sequence")]
    pub sequence: NonNegativeInt,
    #[serde(rename = "targetThreadId")]
    pub target_thread_id: ThreadId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum T3ThreadOrganizeError {
    Variant1(ToolFrameworkAiError),
    Variant2(OrchestratorMcpFailure),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum T3ThreadOrganizeInputAction {
    #[serde(rename = "pin")]
    Pin,
    #[serde(rename = "unpin")]
    Unpin,
    #[serde(rename = "snooze")]
    Snooze,
    #[serde(rename = "unsnooze")]
    Unsnooze,
    #[serde(rename = "settle")]
    Settle,
    #[serde(rename = "unsettle")]
    Unsettle,
    #[serde(rename = "archive")]
    Archive,
    #[serde(rename = "unarchive")]
    Unarchive,
    #[serde(rename = "mark_unread")]
    MarkUnread,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3ThreadOrganizeInput {
    #[serde(
        rename = "threadId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub thread_id: Optional<String>,
    #[serde(rename = "action")]
    pub action: T3ThreadOrganizeInputAction,
    #[serde(
        rename = "snoozedUntil",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub snoozed_until: Optional<IsoDateTime>,
}

pub type T3ThreadOrganizeResult = OrchestrationV2DispatchCommandResult;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum T3ThreadReadError {
    Variant1(ToolFrameworkAiError),
    Variant2(OrchestratorMcpFailure),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum T3ThreadReadInputView {
    #[serde(rename = "messages")]
    Messages,
    #[serde(rename = "activity")]
    Activity,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3ThreadReadInput {
    #[serde(rename = "threadId")]
    pub thread_id: String,
    #[serde(
        rename = "itemId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub item_id: Optional<String>,
    #[serde(
        rename = "textOffset",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub text_offset: Optional<NonNegativeInt>,
    #[serde(rename = "view", default, skip_serializing_if = "Optional::is_absent")]
    pub view: Optional<T3ThreadReadInputView>,
    #[serde(
        rename = "afterPosition",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub after_position: Optional<NonNegativeInt>,
    #[serde(rename = "limit", default, skip_serializing_if = "Optional::is_absent")]
    pub limit: Optional<i64>,
    #[serde(
        rename = "runLimit",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub run_limit: Optional<i64>,
    #[serde(
        rename = "maxCharsPerItem",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub max_chars_per_item: Optional<i64>,
}

pub type T3ThreadReadResult = OrchestratorMcpThreadReadResult;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum T3ThreadSearchError {
    Variant1(ToolFrameworkAiError),
    Variant2(OrchestratorMcpFailure),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3ThreadSearchInput {
    #[serde(rename = "query")]
    pub query: String,
    #[serde(rename = "limit", default, skip_serializing_if = "Optional::is_absent")]
    pub limit: Optional<i64>,
}

pub type T3ThreadSearchResult = OrchestrationSearchThreadsResult;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum T3ThreadSendAttachmentsError {
    Variant1(ToolFrameworkAiError),
    Variant2(OrchestratorMcpFailure),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3ThreadSendAttachmentsInputAttachmentsItemImage {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    #[serde(rename = "sizeBytes")]
    pub size_bytes: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3ThreadSendAttachmentsInputAttachmentsItemFile {
    #[serde(rename = "id")]
    pub id: String,
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
pub enum T3ThreadSendAttachmentsInputAttachmentsItem {
    #[serde(rename = "image")]
    Image(Box<T3ThreadSendAttachmentsInputAttachmentsItemImage>),
    #[serde(rename = "file")]
    File(Box<T3ThreadSendAttachmentsInputAttachmentsItemFile>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3ThreadSendAttachmentsInput {
    #[serde(
        rename = "threadId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub thread_id: Optional<String>,
    #[serde(
        rename = "message",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub message: Optional<String>,
    #[serde(rename = "attachments")]
    pub attachments: Vec<T3ThreadSendAttachmentsInputAttachmentsItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3ThreadSendAttachmentsResult {
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "messageId")]
    pub message_id: MessageId,
    #[serde(rename = "runId")]
    pub run_id: RunId,
    #[serde(rename = "status")]
    pub status: OrchestrationV2RunStatus,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum T3ThreadSendError {
    Variant1(ToolFrameworkAiError),
    Variant2(OrchestratorMcpFailure),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum T3ThreadSendInputMode {
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "queue")]
    Queue,
    #[serde(rename = "steer")]
    Steer,
    #[serde(rename = "restart")]
    Restart,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3ThreadSendInput {
    #[serde(rename = "threadId")]
    pub thread_id: String,
    #[serde(rename = "message")]
    pub message: String,
    #[serde(rename = "mode", default, skip_serializing_if = "Optional::is_absent")]
    pub mode: Optional<T3ThreadSendInputMode>,
    #[serde(
        rename = "clientRequestId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub client_request_id: Optional<String>,
}

pub type T3ThreadSendResult = OrchestratorMcpThreadSendResult;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum T3ThreadTransfersError {
    Variant1(ToolFrameworkAiError),
    Variant2(OrchestratorMcpFailure),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3ThreadTransfersInput {
    #[serde(
        rename = "threadId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub thread_id: Optional<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum T3ThreadTransfersResultTransfersItemStatus {
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
pub struct T3ThreadTransfersResultTransfersItem {
    #[serde(rename = "id")]
    pub id: ContextTransferId,
    #[serde(rename = "sourceThreadId")]
    pub source_thread_id: ThreadId,
    #[serde(rename = "targetThreadId")]
    pub target_thread_id: ThreadId,
    #[serde(rename = "status")]
    pub status: T3ThreadTransfersResultTransfersItemStatus,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3ThreadTransfersResult {
    #[serde(rename = "transfers")]
    pub transfers: Vec<T3ThreadTransfersResultTransfersItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum T3ThreadUpdateError {
    Variant1(ToolFrameworkAiError),
    Variant2(OrchestratorMcpFailure),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3ThreadUpdateInputPullRequest {
    #[serde(rename = "repository")]
    pub repository: String,
    #[serde(rename = "number")]
    pub number: i64,
    #[serde(rename = "url")]
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3ThreadUpdateInput {
    #[serde(
        rename = "threadId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub thread_id: Optional<String>,
    #[serde(rename = "action")]
    pub action: ThreadMetadataMcpAction,
    #[serde(rename = "title", default, skip_serializing_if = "Optional::is_absent")]
    pub title: Optional<String>,
    #[serde(
        rename = "pullRequest",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub pull_request: Optional<T3ThreadUpdateInputPullRequest>,
    #[serde(
        rename = "clientRequestId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub client_request_id: Optional<String>,
}

pub type T3ThreadUpdateResult = ThreadMetadataMcpUpdateResult;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum T3ThreadWaitError {
    Variant1(ToolFrameworkAiError),
    Variant2(OrchestratorMcpFailure),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3ThreadWaitInput {
    #[serde(rename = "threadId")]
    pub thread_id: String,
    #[serde(rename = "runId", default, skip_serializing_if = "Optional::is_absent")]
    pub run_id: Optional<String>,
    #[serde(
        rename = "timeoutMs",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub timeout_ms: Optional<JsonNumber>,
}

pub type T3ThreadWaitResult = OrchestratorMcpThreadWaitResult;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum T3WorktreeHandoffError {
    Variant1(ToolFrameworkAiError),
    Variant2(WorktreeMcpFailure),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3WorktreeHandoffInput {
    #[serde(rename = "branch")]
    pub branch: String,
    #[serde(
        rename = "baseRef",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub base_ref: Optional<String>,
    #[serde(
        rename = "startFromOrigin",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub start_from_origin: Optional<bool>,
    #[serde(rename = "path", default, skip_serializing_if = "Optional::is_absent")]
    pub path: Optional<String>,
    #[serde(
        rename = "runSetupScript",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub run_setup_script: Optional<bool>,
    #[serde(
        rename = "continuationPrompt",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub continuation_prompt: Optional<String>,
}

pub type T3WorktreeHandoffResult = WorktreeMcpHandoffResult;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum T3WorktreeListError {
    Variant1(ToolFrameworkAiError),
    Variant2(OrchestratorMcpFailure),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum T3WorktreeListInputRefKind {
    #[serde(rename = "all")]
    All,
    #[serde(rename = "local")]
    Local,
    #[serde(rename = "remote")]
    Remote,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct T3WorktreeListInput {
    #[serde(rename = "query", default, skip_serializing_if = "Optional::is_absent")]
    pub query: Optional<String>,
    #[serde(
        rename = "cursor",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub cursor: Optional<NonNegativeInt>,
    #[serde(rename = "limit", default, skip_serializing_if = "Optional::is_absent")]
    pub limit: Optional<i64>,
    #[serde(
        rename = "refKind",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub ref_kind: Optional<T3WorktreeListInputRefKind>,
    #[serde(
        rename = "includeMatchingRemoteRefs",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub include_matching_remote_refs: Optional<bool>,
}

pub type T3WorktreeListResult = VcsListRefsResult;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum T3WorktreeStatusError {
    Variant1(ToolFrameworkAiError),
    Variant2(WorktreeMcpFailure),
    Variant3(ToolFrameworkExecutionFailure),
}

pub type T3WorktreeStatusInput = BTreeMap<IsoDateTime, Never>;

pub type T3WorktreeStatusResult = WorktreeMcpStatusResult;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TaskCancelError {
    Variant1(ToolFrameworkAiError),
    Variant2(OrchestratorMcpFailure),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskCancelInput {
    #[serde(rename = "taskId")]
    pub task_id: String,
    #[serde(
        rename = "reason",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub reason: Optional<String>,
    #[serde(
        rename = "clientRequestId",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub client_request_id: Optional<String>,
}

pub type TaskCancelResult = OrchestratorMcpTaskCancelResult;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TaskStatusError {
    Variant1(ToolFrameworkAiError),
    Variant2(OrchestratorMcpFailure),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskStatusInput {
    #[serde(rename = "taskId")]
    pub task_id: String,
}

pub type TaskStatusResult = OrchestratorMcpDelegateTaskResult;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ThreadMetadataMcpAction {
    #[serde(rename = "rename")]
    Rename,
    #[serde(rename = "regenerate_title")]
    RegenerateTitle,
    #[serde(rename = "link_pull_request")]
    LinkPullRequest,
    #[serde(rename = "unlink_pull_request")]
    UnlinkPullRequest,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThreadMetadataMcpUpdateResult {
    #[serde(rename = "threadId")]
    pub thread_id: ThreadId,
    #[serde(rename = "action")]
    pub action: ThreadMetadataMcpAction,
    #[serde(rename = "commandId")]
    pub command_id: CommandId,
    #[serde(rename = "sequence")]
    pub sequence: NonNegativeInt,
    #[serde(rename = "title")]
    pub title: IsoDateTime,
    #[serde(rename = "titleRegeneration", deserialize_with = "required_nullable")]
    pub title_regeneration: Option<ThreadTitleRegeneration>,
    #[serde(rename = "linkedPullRequest", deserialize_with = "required_nullable")]
    pub linked_pull_request: Option<ThreadLinkedPullRequest>,
    #[serde(rename = "updatedAt")]
    pub updated_at: IsoDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ToolFrameworkAiErrorTag {
    #[serde(rename = "AiError")]
    AiError,
}

fn tool_framework_ai_error_reason_rate_limit_error_metadata_default()
-> BTreeMap<IsoDateTime, Option<serde_json::Value>> {
    serde_json::from_str("{}").expect("upstream default")
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ToolFrameworkAiErrorReasonRateLimitErrorHttpRequestMethod {
    #[serde(rename = "GET")]
    GET,
    #[serde(rename = "POST")]
    POST,
    #[serde(rename = "PATCH")]
    PATCH,
    #[serde(rename = "PUT")]
    PUT,
    #[serde(rename = "DELETE")]
    DELETE,
    #[serde(rename = "HEAD")]
    HEAD,
    #[serde(rename = "OPTIONS")]
    OPTIONS,
    #[serde(rename = "TRACE")]
    TRACE,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolFrameworkAiErrorReasonRateLimitErrorHttpRequestHeadersValue {
    Variant1(IsoDateTime),
    Variant2(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolFrameworkAiErrorReasonRateLimitErrorHttpRequest {
    #[serde(rename = "method")]
    pub method: ToolFrameworkAiErrorReasonRateLimitErrorHttpRequestMethod,
    #[serde(rename = "url")]
    pub url: IsoDateTime,
    #[serde(rename = "urlParams")]
    pub url_params: Vec<(IsoDateTime, IsoDateTime)>,
    #[serde(rename = "hash", default, skip_serializing_if = "Optional::is_absent")]
    pub hash: Optional<IsoDateTime>,
    #[serde(rename = "headers")]
    pub headers:
        BTreeMap<IsoDateTime, ToolFrameworkAiErrorReasonRateLimitErrorHttpRequestHeadersValue>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolFrameworkAiErrorReasonRateLimitErrorHttpResponseHeadersValue {
    Variant1(IsoDateTime),
    Variant2(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolFrameworkAiErrorReasonRateLimitErrorHttpResponse {
    #[serde(rename = "status")]
    pub status: i64,
    #[serde(rename = "headers")]
    pub headers:
        BTreeMap<IsoDateTime, ToolFrameworkAiErrorReasonRateLimitErrorHttpResponseHeadersValue>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolFrameworkAiErrorReasonRateLimitErrorHttp {
    #[serde(rename = "request")]
    pub request: ToolFrameworkAiErrorReasonRateLimitErrorHttpRequest,
    #[serde(
        rename = "response",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub response: Optional<ToolFrameworkAiErrorReasonRateLimitErrorHttpResponse>,
    #[serde(rename = "body", default, skip_serializing_if = "Optional::is_absent")]
    pub body: Optional<IsoDateTime>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolFrameworkAiErrorReasonRateLimitError {
    #[serde(
        rename = "retryAfter",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub retry_after: Optional<serde_json::Value>,
    #[serde(
        rename = "metadata",
        default = "tool_framework_ai_error_reason_rate_limit_error_metadata_default"
    )]
    pub metadata: BTreeMap<IsoDateTime, Option<serde_json::Value>>,
    #[serde(rename = "http", default, skip_serializing_if = "Optional::is_absent")]
    pub http: Optional<ToolFrameworkAiErrorReasonRateLimitErrorHttp>,
}

fn tool_framework_ai_error_reason_quota_exhausted_error_metadata_default()
-> BTreeMap<IsoDateTime, Option<serde_json::Value>> {
    serde_json::from_str("{}").expect("upstream default")
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ToolFrameworkAiErrorReasonQuotaExhaustedErrorHttpRequestMethod {
    #[serde(rename = "GET")]
    GET,
    #[serde(rename = "POST")]
    POST,
    #[serde(rename = "PATCH")]
    PATCH,
    #[serde(rename = "PUT")]
    PUT,
    #[serde(rename = "DELETE")]
    DELETE,
    #[serde(rename = "HEAD")]
    HEAD,
    #[serde(rename = "OPTIONS")]
    OPTIONS,
    #[serde(rename = "TRACE")]
    TRACE,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolFrameworkAiErrorReasonQuotaExhaustedErrorHttpRequestHeadersValue {
    Variant1(IsoDateTime),
    Variant2(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolFrameworkAiErrorReasonQuotaExhaustedErrorHttpRequest {
    #[serde(rename = "method")]
    pub method: ToolFrameworkAiErrorReasonQuotaExhaustedErrorHttpRequestMethod,
    #[serde(rename = "url")]
    pub url: IsoDateTime,
    #[serde(rename = "urlParams")]
    pub url_params: Vec<(IsoDateTime, IsoDateTime)>,
    #[serde(rename = "hash", default, skip_serializing_if = "Optional::is_absent")]
    pub hash: Optional<IsoDateTime>,
    #[serde(rename = "headers")]
    pub headers:
        BTreeMap<IsoDateTime, ToolFrameworkAiErrorReasonQuotaExhaustedErrorHttpRequestHeadersValue>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolFrameworkAiErrorReasonQuotaExhaustedErrorHttpResponseHeadersValue {
    Variant1(IsoDateTime),
    Variant2(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolFrameworkAiErrorReasonQuotaExhaustedErrorHttpResponse {
    #[serde(rename = "status")]
    pub status: i64,
    #[serde(rename = "headers")]
    pub headers: BTreeMap<
        IsoDateTime,
        ToolFrameworkAiErrorReasonQuotaExhaustedErrorHttpResponseHeadersValue,
    >,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolFrameworkAiErrorReasonQuotaExhaustedErrorHttp {
    #[serde(rename = "request")]
    pub request: ToolFrameworkAiErrorReasonQuotaExhaustedErrorHttpRequest,
    #[serde(
        rename = "response",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub response: Optional<ToolFrameworkAiErrorReasonQuotaExhaustedErrorHttpResponse>,
    #[serde(rename = "body", default, skip_serializing_if = "Optional::is_absent")]
    pub body: Optional<IsoDateTime>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolFrameworkAiErrorReasonQuotaExhaustedError {
    #[serde(
        rename = "resetAt",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub reset_at: Optional<String>,
    #[serde(
        rename = "metadata",
        default = "tool_framework_ai_error_reason_quota_exhausted_error_metadata_default"
    )]
    pub metadata: BTreeMap<IsoDateTime, Option<serde_json::Value>>,
    #[serde(rename = "http", default, skip_serializing_if = "Optional::is_absent")]
    pub http: Optional<ToolFrameworkAiErrorReasonQuotaExhaustedErrorHttp>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ToolFrameworkAiErrorReasonAuthenticationErrorKind {
    #[serde(rename = "InvalidKey")]
    InvalidKey,
    #[serde(rename = "ExpiredKey")]
    ExpiredKey,
    #[serde(rename = "MissingKey")]
    MissingKey,
    #[serde(rename = "InsufficientPermissions")]
    InsufficientPermissions,
    #[serde(rename = "Unknown")]
    Unknown,
}

fn tool_framework_ai_error_reason_authentication_error_metadata_default()
-> BTreeMap<IsoDateTime, Option<serde_json::Value>> {
    serde_json::from_str("{}").expect("upstream default")
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ToolFrameworkAiErrorReasonAuthenticationErrorHttpRequestMethod {
    #[serde(rename = "GET")]
    GET,
    #[serde(rename = "POST")]
    POST,
    #[serde(rename = "PATCH")]
    PATCH,
    #[serde(rename = "PUT")]
    PUT,
    #[serde(rename = "DELETE")]
    DELETE,
    #[serde(rename = "HEAD")]
    HEAD,
    #[serde(rename = "OPTIONS")]
    OPTIONS,
    #[serde(rename = "TRACE")]
    TRACE,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolFrameworkAiErrorReasonAuthenticationErrorHttpRequestHeadersValue {
    Variant1(IsoDateTime),
    Variant2(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolFrameworkAiErrorReasonAuthenticationErrorHttpRequest {
    #[serde(rename = "method")]
    pub method: ToolFrameworkAiErrorReasonAuthenticationErrorHttpRequestMethod,
    #[serde(rename = "url")]
    pub url: IsoDateTime,
    #[serde(rename = "urlParams")]
    pub url_params: Vec<(IsoDateTime, IsoDateTime)>,
    #[serde(rename = "hash", default, skip_serializing_if = "Optional::is_absent")]
    pub hash: Optional<IsoDateTime>,
    #[serde(rename = "headers")]
    pub headers:
        BTreeMap<IsoDateTime, ToolFrameworkAiErrorReasonAuthenticationErrorHttpRequestHeadersValue>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolFrameworkAiErrorReasonAuthenticationErrorHttpResponseHeadersValue {
    Variant1(IsoDateTime),
    Variant2(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolFrameworkAiErrorReasonAuthenticationErrorHttpResponse {
    #[serde(rename = "status")]
    pub status: i64,
    #[serde(rename = "headers")]
    pub headers: BTreeMap<
        IsoDateTime,
        ToolFrameworkAiErrorReasonAuthenticationErrorHttpResponseHeadersValue,
    >,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolFrameworkAiErrorReasonAuthenticationErrorHttp {
    #[serde(rename = "request")]
    pub request: ToolFrameworkAiErrorReasonAuthenticationErrorHttpRequest,
    #[serde(
        rename = "response",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub response: Optional<ToolFrameworkAiErrorReasonAuthenticationErrorHttpResponse>,
    #[serde(rename = "body", default, skip_serializing_if = "Optional::is_absent")]
    pub body: Optional<IsoDateTime>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolFrameworkAiErrorReasonAuthenticationError {
    #[serde(rename = "kind")]
    pub kind: ToolFrameworkAiErrorReasonAuthenticationErrorKind,
    #[serde(
        rename = "description",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub description: Optional<IsoDateTime>,
    #[serde(
        rename = "metadata",
        default = "tool_framework_ai_error_reason_authentication_error_metadata_default"
    )]
    pub metadata: BTreeMap<IsoDateTime, Option<serde_json::Value>>,
    #[serde(rename = "http", default, skip_serializing_if = "Optional::is_absent")]
    pub http: Optional<ToolFrameworkAiErrorReasonAuthenticationErrorHttp>,
}

fn tool_framework_ai_error_reason_content_policy_error_metadata_default()
-> BTreeMap<IsoDateTime, Option<serde_json::Value>> {
    serde_json::from_str("{}").expect("upstream default")
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ToolFrameworkAiErrorReasonContentPolicyErrorHttpRequestMethod {
    #[serde(rename = "GET")]
    GET,
    #[serde(rename = "POST")]
    POST,
    #[serde(rename = "PATCH")]
    PATCH,
    #[serde(rename = "PUT")]
    PUT,
    #[serde(rename = "DELETE")]
    DELETE,
    #[serde(rename = "HEAD")]
    HEAD,
    #[serde(rename = "OPTIONS")]
    OPTIONS,
    #[serde(rename = "TRACE")]
    TRACE,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolFrameworkAiErrorReasonContentPolicyErrorHttpRequestHeadersValue {
    Variant1(IsoDateTime),
    Variant2(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolFrameworkAiErrorReasonContentPolicyErrorHttpRequest {
    #[serde(rename = "method")]
    pub method: ToolFrameworkAiErrorReasonContentPolicyErrorHttpRequestMethod,
    #[serde(rename = "url")]
    pub url: IsoDateTime,
    #[serde(rename = "urlParams")]
    pub url_params: Vec<(IsoDateTime, IsoDateTime)>,
    #[serde(rename = "hash", default, skip_serializing_if = "Optional::is_absent")]
    pub hash: Optional<IsoDateTime>,
    #[serde(rename = "headers")]
    pub headers:
        BTreeMap<IsoDateTime, ToolFrameworkAiErrorReasonContentPolicyErrorHttpRequestHeadersValue>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolFrameworkAiErrorReasonContentPolicyErrorHttpResponseHeadersValue {
    Variant1(IsoDateTime),
    Variant2(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolFrameworkAiErrorReasonContentPolicyErrorHttpResponse {
    #[serde(rename = "status")]
    pub status: i64,
    #[serde(rename = "headers")]
    pub headers:
        BTreeMap<IsoDateTime, ToolFrameworkAiErrorReasonContentPolicyErrorHttpResponseHeadersValue>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolFrameworkAiErrorReasonContentPolicyErrorHttp {
    #[serde(rename = "request")]
    pub request: ToolFrameworkAiErrorReasonContentPolicyErrorHttpRequest,
    #[serde(
        rename = "response",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub response: Optional<ToolFrameworkAiErrorReasonContentPolicyErrorHttpResponse>,
    #[serde(rename = "body", default, skip_serializing_if = "Optional::is_absent")]
    pub body: Optional<IsoDateTime>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolFrameworkAiErrorReasonContentPolicyError {
    #[serde(rename = "description")]
    pub description: IsoDateTime,
    #[serde(
        rename = "metadata",
        default = "tool_framework_ai_error_reason_content_policy_error_metadata_default"
    )]
    pub metadata: BTreeMap<IsoDateTime, Option<serde_json::Value>>,
    #[serde(rename = "http", default, skip_serializing_if = "Optional::is_absent")]
    pub http: Optional<ToolFrameworkAiErrorReasonContentPolicyErrorHttp>,
}

fn tool_framework_ai_error_reason_invalid_request_error_metadata_default()
-> BTreeMap<IsoDateTime, Option<serde_json::Value>> {
    serde_json::from_str("{}").expect("upstream default")
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ToolFrameworkAiErrorReasonInvalidRequestErrorHttpRequestMethod {
    #[serde(rename = "GET")]
    GET,
    #[serde(rename = "POST")]
    POST,
    #[serde(rename = "PATCH")]
    PATCH,
    #[serde(rename = "PUT")]
    PUT,
    #[serde(rename = "DELETE")]
    DELETE,
    #[serde(rename = "HEAD")]
    HEAD,
    #[serde(rename = "OPTIONS")]
    OPTIONS,
    #[serde(rename = "TRACE")]
    TRACE,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolFrameworkAiErrorReasonInvalidRequestErrorHttpRequestHeadersValue {
    Variant1(IsoDateTime),
    Variant2(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolFrameworkAiErrorReasonInvalidRequestErrorHttpRequest {
    #[serde(rename = "method")]
    pub method: ToolFrameworkAiErrorReasonInvalidRequestErrorHttpRequestMethod,
    #[serde(rename = "url")]
    pub url: IsoDateTime,
    #[serde(rename = "urlParams")]
    pub url_params: Vec<(IsoDateTime, IsoDateTime)>,
    #[serde(rename = "hash", default, skip_serializing_if = "Optional::is_absent")]
    pub hash: Optional<IsoDateTime>,
    #[serde(rename = "headers")]
    pub headers:
        BTreeMap<IsoDateTime, ToolFrameworkAiErrorReasonInvalidRequestErrorHttpRequestHeadersValue>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolFrameworkAiErrorReasonInvalidRequestErrorHttpResponseHeadersValue {
    Variant1(IsoDateTime),
    Variant2(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolFrameworkAiErrorReasonInvalidRequestErrorHttpResponse {
    #[serde(rename = "status")]
    pub status: i64,
    #[serde(rename = "headers")]
    pub headers: BTreeMap<
        IsoDateTime,
        ToolFrameworkAiErrorReasonInvalidRequestErrorHttpResponseHeadersValue,
    >,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolFrameworkAiErrorReasonInvalidRequestErrorHttp {
    #[serde(rename = "request")]
    pub request: ToolFrameworkAiErrorReasonInvalidRequestErrorHttpRequest,
    #[serde(
        rename = "response",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub response: Optional<ToolFrameworkAiErrorReasonInvalidRequestErrorHttpResponse>,
    #[serde(rename = "body", default, skip_serializing_if = "Optional::is_absent")]
    pub body: Optional<IsoDateTime>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolFrameworkAiErrorReasonInvalidRequestError {
    #[serde(
        rename = "parameter",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub parameter: Optional<IsoDateTime>,
    #[serde(
        rename = "constraint",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub constraint: Optional<IsoDateTime>,
    #[serde(
        rename = "description",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub description: Optional<IsoDateTime>,
    #[serde(
        rename = "metadata",
        default = "tool_framework_ai_error_reason_invalid_request_error_metadata_default"
    )]
    pub metadata: BTreeMap<IsoDateTime, Option<serde_json::Value>>,
    #[serde(rename = "http", default, skip_serializing_if = "Optional::is_absent")]
    pub http: Optional<ToolFrameworkAiErrorReasonInvalidRequestErrorHttp>,
}

fn tool_framework_ai_error_reason_internal_provider_error_metadata_default()
-> BTreeMap<IsoDateTime, Option<serde_json::Value>> {
    serde_json::from_str("{}").expect("upstream default")
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ToolFrameworkAiErrorReasonInternalProviderErrorHttpRequestMethod {
    #[serde(rename = "GET")]
    GET,
    #[serde(rename = "POST")]
    POST,
    #[serde(rename = "PATCH")]
    PATCH,
    #[serde(rename = "PUT")]
    PUT,
    #[serde(rename = "DELETE")]
    DELETE,
    #[serde(rename = "HEAD")]
    HEAD,
    #[serde(rename = "OPTIONS")]
    OPTIONS,
    #[serde(rename = "TRACE")]
    TRACE,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolFrameworkAiErrorReasonInternalProviderErrorHttpRequestHeadersValue {
    Variant1(IsoDateTime),
    Variant2(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolFrameworkAiErrorReasonInternalProviderErrorHttpRequest {
    #[serde(rename = "method")]
    pub method: ToolFrameworkAiErrorReasonInternalProviderErrorHttpRequestMethod,
    #[serde(rename = "url")]
    pub url: IsoDateTime,
    #[serde(rename = "urlParams")]
    pub url_params: Vec<(IsoDateTime, IsoDateTime)>,
    #[serde(rename = "hash", default, skip_serializing_if = "Optional::is_absent")]
    pub hash: Optional<IsoDateTime>,
    #[serde(rename = "headers")]
    pub headers: BTreeMap<
        IsoDateTime,
        ToolFrameworkAiErrorReasonInternalProviderErrorHttpRequestHeadersValue,
    >,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolFrameworkAiErrorReasonInternalProviderErrorHttpResponseHeadersValue {
    Variant1(IsoDateTime),
    Variant2(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolFrameworkAiErrorReasonInternalProviderErrorHttpResponse {
    #[serde(rename = "status")]
    pub status: i64,
    #[serde(rename = "headers")]
    pub headers: BTreeMap<
        IsoDateTime,
        ToolFrameworkAiErrorReasonInternalProviderErrorHttpResponseHeadersValue,
    >,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolFrameworkAiErrorReasonInternalProviderErrorHttp {
    #[serde(rename = "request")]
    pub request: ToolFrameworkAiErrorReasonInternalProviderErrorHttpRequest,
    #[serde(
        rename = "response",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub response: Optional<ToolFrameworkAiErrorReasonInternalProviderErrorHttpResponse>,
    #[serde(rename = "body", default, skip_serializing_if = "Optional::is_absent")]
    pub body: Optional<IsoDateTime>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolFrameworkAiErrorReasonInternalProviderError {
    #[serde(rename = "description")]
    pub description: IsoDateTime,
    #[serde(
        rename = "metadata",
        default = "tool_framework_ai_error_reason_internal_provider_error_metadata_default"
    )]
    pub metadata: BTreeMap<IsoDateTime, Option<serde_json::Value>>,
    #[serde(rename = "http", default, skip_serializing_if = "Optional::is_absent")]
    pub http: Optional<ToolFrameworkAiErrorReasonInternalProviderErrorHttp>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ToolFrameworkAiErrorReasonNetworkErrorReason {
    #[serde(rename = "TransportError")]
    TransportError,
    #[serde(rename = "EncodeError")]
    EncodeError,
    #[serde(rename = "InvalidUrlError")]
    InvalidUrlError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ToolFrameworkAiErrorReasonNetworkErrorRequestMethod {
    #[serde(rename = "GET")]
    GET,
    #[serde(rename = "POST")]
    POST,
    #[serde(rename = "PATCH")]
    PATCH,
    #[serde(rename = "PUT")]
    PUT,
    #[serde(rename = "DELETE")]
    DELETE,
    #[serde(rename = "HEAD")]
    HEAD,
    #[serde(rename = "OPTIONS")]
    OPTIONS,
    #[serde(rename = "TRACE")]
    TRACE,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolFrameworkAiErrorReasonNetworkErrorRequestHeadersValue {
    Variant1(IsoDateTime),
    Variant2(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolFrameworkAiErrorReasonNetworkErrorRequest {
    #[serde(rename = "method")]
    pub method: ToolFrameworkAiErrorReasonNetworkErrorRequestMethod,
    #[serde(rename = "url")]
    pub url: IsoDateTime,
    #[serde(rename = "urlParams")]
    pub url_params: Vec<(IsoDateTime, IsoDateTime)>,
    #[serde(rename = "hash", default, skip_serializing_if = "Optional::is_absent")]
    pub hash: Optional<IsoDateTime>,
    #[serde(rename = "headers")]
    pub headers: BTreeMap<IsoDateTime, ToolFrameworkAiErrorReasonNetworkErrorRequestHeadersValue>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolFrameworkAiErrorReasonNetworkError {
    #[serde(rename = "reason")]
    pub reason: ToolFrameworkAiErrorReasonNetworkErrorReason,
    #[serde(rename = "request")]
    pub request: ToolFrameworkAiErrorReasonNetworkErrorRequest,
    #[serde(
        rename = "description",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub description: Optional<IsoDateTime>,
}

fn tool_framework_ai_error_reason_invalid_output_error_metadata_default()
-> BTreeMap<IsoDateTime, Option<serde_json::Value>> {
    serde_json::from_str("{}").expect("upstream default")
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolFrameworkAiErrorReasonInvalidOutputErrorUsage {
    #[serde(
        rename = "promptTokens",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub prompt_tokens: Optional<i64>,
    #[serde(
        rename = "completionTokens",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub completion_tokens: Optional<i64>,
    #[serde(
        rename = "totalTokens",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub total_tokens: Optional<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolFrameworkAiErrorReasonInvalidOutputError {
    #[serde(rename = "description")]
    pub description: IsoDateTime,
    #[serde(
        rename = "metadata",
        default = "tool_framework_ai_error_reason_invalid_output_error_metadata_default"
    )]
    pub metadata: BTreeMap<IsoDateTime, Option<serde_json::Value>>,
    #[serde(rename = "usage", default, skip_serializing_if = "Optional::is_absent")]
    pub usage: Optional<ToolFrameworkAiErrorReasonInvalidOutputErrorUsage>,
}

fn tool_framework_ai_error_reason_structured_output_error_metadata_default()
-> BTreeMap<IsoDateTime, Option<serde_json::Value>> {
    serde_json::from_str("{}").expect("upstream default")
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolFrameworkAiErrorReasonStructuredOutputErrorUsage {
    #[serde(
        rename = "promptTokens",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub prompt_tokens: Optional<i64>,
    #[serde(
        rename = "completionTokens",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub completion_tokens: Optional<i64>,
    #[serde(
        rename = "totalTokens",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub total_tokens: Optional<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolFrameworkAiErrorReasonStructuredOutputError {
    #[serde(rename = "description")]
    pub description: IsoDateTime,
    #[serde(rename = "responseText")]
    pub response_text: IsoDateTime,
    #[serde(
        rename = "metadata",
        default = "tool_framework_ai_error_reason_structured_output_error_metadata_default"
    )]
    pub metadata: BTreeMap<IsoDateTime, Option<serde_json::Value>>,
    #[serde(rename = "usage", default, skip_serializing_if = "Optional::is_absent")]
    pub usage: Optional<ToolFrameworkAiErrorReasonStructuredOutputErrorUsage>,
}

fn tool_framework_ai_error_reason_unsupported_schema_error_metadata_default()
-> BTreeMap<IsoDateTime, Option<serde_json::Value>> {
    serde_json::from_str("{}").expect("upstream default")
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolFrameworkAiErrorReasonUnsupportedSchemaError {
    #[serde(rename = "description")]
    pub description: IsoDateTime,
    #[serde(
        rename = "metadata",
        default = "tool_framework_ai_error_reason_unsupported_schema_error_metadata_default"
    )]
    pub metadata: BTreeMap<IsoDateTime, Option<serde_json::Value>>,
}

fn tool_framework_ai_error_reason_unknown_error_metadata_default()
-> BTreeMap<IsoDateTime, Option<serde_json::Value>> {
    serde_json::from_str("{}").expect("upstream default")
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ToolFrameworkAiErrorReasonUnknownErrorHttpRequestMethod {
    #[serde(rename = "GET")]
    GET,
    #[serde(rename = "POST")]
    POST,
    #[serde(rename = "PATCH")]
    PATCH,
    #[serde(rename = "PUT")]
    PUT,
    #[serde(rename = "DELETE")]
    DELETE,
    #[serde(rename = "HEAD")]
    HEAD,
    #[serde(rename = "OPTIONS")]
    OPTIONS,
    #[serde(rename = "TRACE")]
    TRACE,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolFrameworkAiErrorReasonUnknownErrorHttpRequestHeadersValue {
    Variant1(IsoDateTime),
    Variant2(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolFrameworkAiErrorReasonUnknownErrorHttpRequest {
    #[serde(rename = "method")]
    pub method: ToolFrameworkAiErrorReasonUnknownErrorHttpRequestMethod,
    #[serde(rename = "url")]
    pub url: IsoDateTime,
    #[serde(rename = "urlParams")]
    pub url_params: Vec<(IsoDateTime, IsoDateTime)>,
    #[serde(rename = "hash", default, skip_serializing_if = "Optional::is_absent")]
    pub hash: Optional<IsoDateTime>,
    #[serde(rename = "headers")]
    pub headers:
        BTreeMap<IsoDateTime, ToolFrameworkAiErrorReasonUnknownErrorHttpRequestHeadersValue>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolFrameworkAiErrorReasonUnknownErrorHttpResponseHeadersValue {
    Variant1(IsoDateTime),
    Variant2(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolFrameworkAiErrorReasonUnknownErrorHttpResponse {
    #[serde(rename = "status")]
    pub status: i64,
    #[serde(rename = "headers")]
    pub headers:
        BTreeMap<IsoDateTime, ToolFrameworkAiErrorReasonUnknownErrorHttpResponseHeadersValue>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolFrameworkAiErrorReasonUnknownErrorHttp {
    #[serde(rename = "request")]
    pub request: ToolFrameworkAiErrorReasonUnknownErrorHttpRequest,
    #[serde(
        rename = "response",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub response: Optional<ToolFrameworkAiErrorReasonUnknownErrorHttpResponse>,
    #[serde(rename = "body", default, skip_serializing_if = "Optional::is_absent")]
    pub body: Optional<IsoDateTime>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolFrameworkAiErrorReasonUnknownError {
    #[serde(
        rename = "description",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub description: Optional<IsoDateTime>,
    #[serde(
        rename = "metadata",
        default = "tool_framework_ai_error_reason_unknown_error_metadata_default"
    )]
    pub metadata: BTreeMap<IsoDateTime, Option<serde_json::Value>>,
    #[serde(rename = "http", default, skip_serializing_if = "Optional::is_absent")]
    pub http: Optional<ToolFrameworkAiErrorReasonUnknownErrorHttp>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolFrameworkAiErrorReasonToolNotFoundError {
    #[serde(rename = "toolName")]
    pub tool_name: IsoDateTime,
    #[serde(rename = "availableTools")]
    pub available_tools: Vec<IsoDateTime>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolFrameworkAiErrorReasonToolParameterValidationError {
    #[serde(rename = "toolName")]
    pub tool_name: IsoDateTime,
    #[serde(rename = "description")]
    pub description: IsoDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolFrameworkAiErrorReasonInvalidToolResultError {
    #[serde(rename = "toolName")]
    pub tool_name: IsoDateTime,
    #[serde(rename = "description")]
    pub description: IsoDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolFrameworkAiErrorReasonToolResultEncodingError {
    #[serde(rename = "toolName")]
    pub tool_name: IsoDateTime,
    #[serde(rename = "toolResult")]
    pub tool_result: serde_json::Value,
    #[serde(rename = "description")]
    pub description: IsoDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolFrameworkAiErrorReasonToolConfigurationError {
    #[serde(rename = "toolName")]
    pub tool_name: IsoDateTime,
    #[serde(rename = "description")]
    pub description: IsoDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolFrameworkAiErrorReasonToolkitRequiredError {
    #[serde(rename = "pendingApprovals")]
    pub pending_approvals: Vec<IsoDateTime>,
    #[serde(
        rename = "description",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub description: Optional<IsoDateTime>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolFrameworkAiErrorReasonInvalidUserInputError {
    #[serde(rename = "description")]
    pub description: IsoDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "_tag")]
pub enum ToolFrameworkAiErrorReason {
    #[serde(rename = "RateLimitError")]
    RateLimitError(Box<ToolFrameworkAiErrorReasonRateLimitError>),
    #[serde(rename = "QuotaExhaustedError")]
    QuotaExhaustedError(Box<ToolFrameworkAiErrorReasonQuotaExhaustedError>),
    #[serde(rename = "AuthenticationError")]
    AuthenticationError(Box<ToolFrameworkAiErrorReasonAuthenticationError>),
    #[serde(rename = "ContentPolicyError")]
    ContentPolicyError(Box<ToolFrameworkAiErrorReasonContentPolicyError>),
    #[serde(rename = "InvalidRequestError")]
    InvalidRequestError(Box<ToolFrameworkAiErrorReasonInvalidRequestError>),
    #[serde(rename = "InternalProviderError")]
    InternalProviderError(Box<ToolFrameworkAiErrorReasonInternalProviderError>),
    #[serde(rename = "NetworkError")]
    NetworkError(Box<ToolFrameworkAiErrorReasonNetworkError>),
    #[serde(rename = "InvalidOutputError")]
    InvalidOutputError(Box<ToolFrameworkAiErrorReasonInvalidOutputError>),
    #[serde(rename = "StructuredOutputError")]
    StructuredOutputError(Box<ToolFrameworkAiErrorReasonStructuredOutputError>),
    #[serde(rename = "UnsupportedSchemaError")]
    UnsupportedSchemaError(Box<ToolFrameworkAiErrorReasonUnsupportedSchemaError>),
    #[serde(rename = "UnknownError")]
    UnknownError(Box<ToolFrameworkAiErrorReasonUnknownError>),
    #[serde(rename = "ToolNotFoundError")]
    ToolNotFoundError(Box<ToolFrameworkAiErrorReasonToolNotFoundError>),
    #[serde(rename = "ToolParameterValidationError")]
    ToolParameterValidationError(Box<ToolFrameworkAiErrorReasonToolParameterValidationError>),
    #[serde(rename = "InvalidToolResultError")]
    InvalidToolResultError(Box<ToolFrameworkAiErrorReasonInvalidToolResultError>),
    #[serde(rename = "ToolResultEncodingError")]
    ToolResultEncodingError(Box<ToolFrameworkAiErrorReasonToolResultEncodingError>),
    #[serde(rename = "ToolConfigurationError")]
    ToolConfigurationError(Box<ToolFrameworkAiErrorReasonToolConfigurationError>),
    #[serde(rename = "ToolkitRequiredError")]
    ToolkitRequiredError(Box<ToolFrameworkAiErrorReasonToolkitRequiredError>),
    #[serde(rename = "InvalidUserInputError")]
    InvalidUserInputError(Box<ToolFrameworkAiErrorReasonInvalidUserInputError>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolFrameworkAiError {
    #[serde(rename = "_tag")]
    pub _tag: ToolFrameworkAiErrorTag,
    #[serde(rename = "module")]
    pub module: IsoDateTime,
    #[serde(rename = "method")]
    pub method: IsoDateTime,
    #[serde(rename = "reason")]
    pub reason: ToolFrameworkAiErrorReason,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ToolFrameworkExecutionFailureType {
    #[serde(rename = "execution-denied")]
    ExecutionDenied,
    #[serde(rename = "execution-interrupted")]
    ExecutionInterrupted,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolFrameworkExecutionFailure {
    #[serde(rename = "type")]
    pub r#type: ToolFrameworkExecutionFailureType,
    #[serde(rename = "reason")]
    pub reason: IsoDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UnlinkPullRequestErrorVariant3Tag {
    #[serde(rename = "PullRequestUrlInvalidError")]
    PullRequestUrlInvalidError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnlinkPullRequestErrorVariant3 {
    #[serde(rename = "_tag")]
    pub _tag: UnlinkPullRequestErrorVariant3Tag,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UnlinkPullRequestErrorVariant4Tag {
    #[serde(rename = "PullRequestTargetIncompleteError")]
    PullRequestTargetIncompleteError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnlinkPullRequestErrorVariant4 {
    #[serde(rename = "_tag")]
    pub _tag: UnlinkPullRequestErrorVariant4Tag,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UnlinkPullRequestErrorVariant5Tag {
    #[serde(rename = "PullRequestHostRequiredError")]
    PullRequestHostRequiredError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnlinkPullRequestErrorVariant5 {
    #[serde(rename = "_tag")]
    pub _tag: UnlinkPullRequestErrorVariant5Tag,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UnlinkPullRequestErrorVariant6Tag {
    #[serde(rename = "PullRequestThreadNotFoundError")]
    PullRequestThreadNotFoundError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnlinkPullRequestErrorVariant6 {
    #[serde(rename = "_tag")]
    pub _tag: UnlinkPullRequestErrorVariant6Tag,
    #[serde(rename = "threadId")]
    pub thread_id: IsoDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UnlinkPullRequestErrorVariant7Tag {
    #[serde(rename = "PullRequestLinkFailedError")]
    PullRequestLinkFailedError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnlinkPullRequestErrorVariant7 {
    #[serde(rename = "_tag")]
    pub _tag: UnlinkPullRequestErrorVariant7Tag,
    #[serde(rename = "cause")]
    pub cause: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UnlinkPullRequestErrorVariant8Tag {
    #[serde(rename = "PullRequestUnlinkFailedError")]
    PullRequestUnlinkFailedError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnlinkPullRequestErrorVariant8 {
    #[serde(rename = "_tag")]
    pub _tag: UnlinkPullRequestErrorVariant8Tag,
    #[serde(rename = "cause")]
    pub cause: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UnlinkPullRequestErrorVariant9Tag {
    #[serde(rename = "PullRequestListFailedError")]
    PullRequestListFailedError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnlinkPullRequestErrorVariant9 {
    #[serde(rename = "_tag")]
    pub _tag: UnlinkPullRequestErrorVariant9Tag,
    #[serde(rename = "cause")]
    pub cause: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UnlinkPullRequestErrorVariant10Tag {
    #[serde(rename = "PullRequestWatchFailedError")]
    PullRequestWatchFailedError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnlinkPullRequestErrorVariant10 {
    #[serde(rename = "_tag")]
    pub _tag: UnlinkPullRequestErrorVariant10Tag,
    #[serde(rename = "cause")]
    pub cause: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UnlinkPullRequestErrorVariant11Tag {
    #[serde(rename = "PullRequestNotOpenError")]
    PullRequestNotOpenError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnlinkPullRequestErrorVariant11 {
    #[serde(rename = "_tag")]
    pub _tag: UnlinkPullRequestErrorVariant11Tag,
    #[serde(rename = "state")]
    pub state: IsoDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UnlinkPullRequestError {
    Variant1(ToolFrameworkAiError),
    Variant2(McpCapabilityUnavailableError),
    Variant3(UnlinkPullRequestErrorVariant3),
    Variant4(UnlinkPullRequestErrorVariant4),
    Variant5(UnlinkPullRequestErrorVariant5),
    Variant6(UnlinkPullRequestErrorVariant6),
    Variant7(UnlinkPullRequestErrorVariant7),
    Variant8(UnlinkPullRequestErrorVariant8),
    Variant9(UnlinkPullRequestErrorVariant9),
    Variant10(UnlinkPullRequestErrorVariant10),
    Variant11(UnlinkPullRequestErrorVariant11),
    Variant12(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnlinkPullRequestInput {
    #[serde(rename = "url", default, skip_serializing_if = "Optional::is_absent")]
    pub url: Optional<String>,
    #[serde(
        rename = "repository",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub repository: Optional<String>,
    #[serde(
        rename = "number",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub number: Optional<i64>,
    #[serde(rename = "host", default, skip_serializing_if = "Optional::is_absent")]
    pub host: Optional<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnlinkPullRequestResult {
    #[serde(rename = "host")]
    pub host: IsoDateTime,
    #[serde(rename = "repository")]
    pub repository: IsoDateTime,
    #[serde(rename = "number")]
    pub number: i64,
    #[serde(rename = "wasLinked")]
    pub was_linked: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UnwatchPullRequestErrorVariant3Tag {
    #[serde(rename = "PullRequestUrlInvalidError")]
    PullRequestUrlInvalidError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnwatchPullRequestErrorVariant3 {
    #[serde(rename = "_tag")]
    pub _tag: UnwatchPullRequestErrorVariant3Tag,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UnwatchPullRequestErrorVariant4Tag {
    #[serde(rename = "PullRequestTargetIncompleteError")]
    PullRequestTargetIncompleteError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnwatchPullRequestErrorVariant4 {
    #[serde(rename = "_tag")]
    pub _tag: UnwatchPullRequestErrorVariant4Tag,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UnwatchPullRequestErrorVariant5Tag {
    #[serde(rename = "PullRequestHostRequiredError")]
    PullRequestHostRequiredError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnwatchPullRequestErrorVariant5 {
    #[serde(rename = "_tag")]
    pub _tag: UnwatchPullRequestErrorVariant5Tag,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UnwatchPullRequestErrorVariant6Tag {
    #[serde(rename = "PullRequestThreadNotFoundError")]
    PullRequestThreadNotFoundError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnwatchPullRequestErrorVariant6 {
    #[serde(rename = "_tag")]
    pub _tag: UnwatchPullRequestErrorVariant6Tag,
    #[serde(rename = "threadId")]
    pub thread_id: IsoDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UnwatchPullRequestErrorVariant7Tag {
    #[serde(rename = "PullRequestLinkFailedError")]
    PullRequestLinkFailedError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnwatchPullRequestErrorVariant7 {
    #[serde(rename = "_tag")]
    pub _tag: UnwatchPullRequestErrorVariant7Tag,
    #[serde(rename = "cause")]
    pub cause: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UnwatchPullRequestErrorVariant8Tag {
    #[serde(rename = "PullRequestUnlinkFailedError")]
    PullRequestUnlinkFailedError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnwatchPullRequestErrorVariant8 {
    #[serde(rename = "_tag")]
    pub _tag: UnwatchPullRequestErrorVariant8Tag,
    #[serde(rename = "cause")]
    pub cause: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UnwatchPullRequestErrorVariant9Tag {
    #[serde(rename = "PullRequestListFailedError")]
    PullRequestListFailedError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnwatchPullRequestErrorVariant9 {
    #[serde(rename = "_tag")]
    pub _tag: UnwatchPullRequestErrorVariant9Tag,
    #[serde(rename = "cause")]
    pub cause: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UnwatchPullRequestErrorVariant10Tag {
    #[serde(rename = "PullRequestWatchFailedError")]
    PullRequestWatchFailedError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnwatchPullRequestErrorVariant10 {
    #[serde(rename = "_tag")]
    pub _tag: UnwatchPullRequestErrorVariant10Tag,
    #[serde(rename = "cause")]
    pub cause: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UnwatchPullRequestErrorVariant11Tag {
    #[serde(rename = "PullRequestNotOpenError")]
    PullRequestNotOpenError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnwatchPullRequestErrorVariant11 {
    #[serde(rename = "_tag")]
    pub _tag: UnwatchPullRequestErrorVariant11Tag,
    #[serde(rename = "state")]
    pub state: IsoDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UnwatchPullRequestError {
    Variant1(ToolFrameworkAiError),
    Variant2(McpCapabilityUnavailableError),
    Variant3(UnwatchPullRequestErrorVariant3),
    Variant4(UnwatchPullRequestErrorVariant4),
    Variant5(UnwatchPullRequestErrorVariant5),
    Variant6(UnwatchPullRequestErrorVariant6),
    Variant7(UnwatchPullRequestErrorVariant7),
    Variant8(UnwatchPullRequestErrorVariant8),
    Variant9(UnwatchPullRequestErrorVariant9),
    Variant10(UnwatchPullRequestErrorVariant10),
    Variant11(UnwatchPullRequestErrorVariant11),
    Variant12(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnwatchPullRequestInput {
    #[serde(rename = "url", default, skip_serializing_if = "Optional::is_absent")]
    pub url: Optional<String>,
    #[serde(
        rename = "repository",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub repository: Optional<String>,
    #[serde(
        rename = "number",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub number: Optional<i64>,
    #[serde(rename = "host", default, skip_serializing_if = "Optional::is_absent")]
    pub host: Optional<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnwatchPullRequestResult {
    #[serde(rename = "host")]
    pub host: IsoDateTime,
    #[serde(rename = "repository")]
    pub repository: IsoDateTime,
    #[serde(rename = "number")]
    pub number: i64,
    #[serde(rename = "url")]
    pub url: IsoDateTime,
    #[serde(rename = "watching")]
    pub watching: bool,
    #[serde(rename = "wasWatching")]
    pub was_watching: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UpdateScheduledTaskError {
    Variant1(ToolFrameworkAiError),
    Variant2(OrchestratorMcpFailure),
    Variant3(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UpdateScheduledTaskInputScheduleVariant1Type {
    #[serde(rename = "interval")]
    Interval,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpdateScheduledTaskInputScheduleVariant1 {
    #[serde(rename = "type")]
    pub r#type: UpdateScheduledTaskInputScheduleVariant1Type,
    #[serde(rename = "everyMs")]
    pub every_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UpdateScheduledTaskInputScheduleVariant2Type {
    #[serde(rename = "fixed_time")]
    FixedTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpdateScheduledTaskInputScheduleVariant2 {
    #[serde(rename = "type")]
    pub r#type: UpdateScheduledTaskInputScheduleVariant2Type,
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
#[serde(untagged)]
pub enum UpdateScheduledTaskInputSchedule {
    Variant1(UpdateScheduledTaskInputScheduleVariant1),
    Variant2(UpdateScheduledTaskInputScheduleVariant2),
    Variant3(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpdateScheduledTaskInput {
    #[serde(rename = "scheduledTaskId")]
    pub scheduled_task_id: String,
    #[serde(
        rename = "prompt",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub prompt: Optional<String>,
    #[serde(rename = "title", default, skip_serializing_if = "Optional::is_absent")]
    pub title: Optional<String>,
    #[serde(
        rename = "schedule",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub schedule: Optional<UpdateScheduledTaskInputSchedule>,
    #[serde(
        rename = "enabled",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub enabled: Optional<bool>,
    #[serde(
        rename = "bindToCurrentThread",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub bind_to_current_thread: Optional<bool>,
}

pub type UpdateScheduledTaskResult = OrchestratorMcpScheduleTaskResult;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WatchPullRequestErrorVariant3Tag {
    #[serde(rename = "PullRequestUrlInvalidError")]
    PullRequestUrlInvalidError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WatchPullRequestErrorVariant3 {
    #[serde(rename = "_tag")]
    pub _tag: WatchPullRequestErrorVariant3Tag,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WatchPullRequestErrorVariant4Tag {
    #[serde(rename = "PullRequestTargetIncompleteError")]
    PullRequestTargetIncompleteError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WatchPullRequestErrorVariant4 {
    #[serde(rename = "_tag")]
    pub _tag: WatchPullRequestErrorVariant4Tag,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WatchPullRequestErrorVariant5Tag {
    #[serde(rename = "PullRequestHostRequiredError")]
    PullRequestHostRequiredError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WatchPullRequestErrorVariant5 {
    #[serde(rename = "_tag")]
    pub _tag: WatchPullRequestErrorVariant5Tag,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WatchPullRequestErrorVariant6Tag {
    #[serde(rename = "PullRequestThreadNotFoundError")]
    PullRequestThreadNotFoundError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WatchPullRequestErrorVariant6 {
    #[serde(rename = "_tag")]
    pub _tag: WatchPullRequestErrorVariant6Tag,
    #[serde(rename = "threadId")]
    pub thread_id: IsoDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WatchPullRequestErrorVariant7Tag {
    #[serde(rename = "PullRequestLinkFailedError")]
    PullRequestLinkFailedError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WatchPullRequestErrorVariant7 {
    #[serde(rename = "_tag")]
    pub _tag: WatchPullRequestErrorVariant7Tag,
    #[serde(rename = "cause")]
    pub cause: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WatchPullRequestErrorVariant8Tag {
    #[serde(rename = "PullRequestUnlinkFailedError")]
    PullRequestUnlinkFailedError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WatchPullRequestErrorVariant8 {
    #[serde(rename = "_tag")]
    pub _tag: WatchPullRequestErrorVariant8Tag,
    #[serde(rename = "cause")]
    pub cause: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WatchPullRequestErrorVariant9Tag {
    #[serde(rename = "PullRequestListFailedError")]
    PullRequestListFailedError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WatchPullRequestErrorVariant9 {
    #[serde(rename = "_tag")]
    pub _tag: WatchPullRequestErrorVariant9Tag,
    #[serde(rename = "cause")]
    pub cause: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WatchPullRequestErrorVariant10Tag {
    #[serde(rename = "PullRequestWatchFailedError")]
    PullRequestWatchFailedError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WatchPullRequestErrorVariant10 {
    #[serde(rename = "_tag")]
    pub _tag: WatchPullRequestErrorVariant10Tag,
    #[serde(rename = "cause")]
    pub cause: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WatchPullRequestErrorVariant11Tag {
    #[serde(rename = "PullRequestNotOpenError")]
    PullRequestNotOpenError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WatchPullRequestErrorVariant11 {
    #[serde(rename = "_tag")]
    pub _tag: WatchPullRequestErrorVariant11Tag,
    #[serde(rename = "state")]
    pub state: IsoDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WatchPullRequestError {
    Variant1(ToolFrameworkAiError),
    Variant2(McpCapabilityUnavailableError),
    Variant3(WatchPullRequestErrorVariant3),
    Variant4(WatchPullRequestErrorVariant4),
    Variant5(WatchPullRequestErrorVariant5),
    Variant6(WatchPullRequestErrorVariant6),
    Variant7(WatchPullRequestErrorVariant7),
    Variant8(WatchPullRequestErrorVariant8),
    Variant9(WatchPullRequestErrorVariant9),
    Variant10(WatchPullRequestErrorVariant10),
    Variant11(WatchPullRequestErrorVariant11),
    Variant12(ToolFrameworkExecutionFailure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WatchPullRequestInput {
    #[serde(rename = "url", default, skip_serializing_if = "Optional::is_absent")]
    pub url: Optional<String>,
    #[serde(
        rename = "repository",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub repository: Optional<String>,
    #[serde(
        rename = "number",
        default,
        skip_serializing_if = "Optional::is_absent"
    )]
    pub number: Optional<i64>,
    #[serde(rename = "host", default, skip_serializing_if = "Optional::is_absent")]
    pub host: Optional<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WatchPullRequestResult {
    #[serde(rename = "host")]
    pub host: IsoDateTime,
    #[serde(rename = "repository")]
    pub repository: IsoDateTime,
    #[serde(rename = "number")]
    pub number: i64,
    #[serde(rename = "url")]
    pub url: IsoDateTime,
    #[serde(rename = "watching")]
    pub watching: bool,
    #[serde(rename = "wasWatching")]
    pub was_watching: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WorktreeMcpContinuationStatusVariant1Status {
    #[serde(rename = "scheduled")]
    Scheduled,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WorktreeMcpContinuationStatusVariant1Delivery {
    #[serde(rename = "started")]
    Started,
    #[serde(rename = "queued")]
    Queued,
    #[serde(rename = "steered")]
    Steered,
    #[serde(rename = "restarted")]
    Restarted,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorktreeMcpContinuationStatusVariant1 {
    #[serde(rename = "status")]
    pub status: WorktreeMcpContinuationStatusVariant1Status,
    #[serde(rename = "delivery")]
    pub delivery: WorktreeMcpContinuationStatusVariant1Delivery,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WorktreeMcpContinuationStatusVariant2Status {
    #[serde(rename = "skipped")]
    Skipped,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorktreeMcpContinuationStatusVariant2 {
    #[serde(rename = "status")]
    pub status: WorktreeMcpContinuationStatusVariant2Status,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WorktreeMcpContinuationStatusVariant3Status {
    #[serde(rename = "failed")]
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorktreeMcpContinuationStatusVariant3 {
    #[serde(rename = "status")]
    pub status: WorktreeMcpContinuationStatusVariant3Status,
    #[serde(rename = "detail")]
    pub detail: IsoDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WorktreeMcpContinuationStatus {
    Variant1(WorktreeMcpContinuationStatusVariant1),
    Variant2(WorktreeMcpContinuationStatusVariant2),
    Variant3(WorktreeMcpContinuationStatusVariant3),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WorktreeMcpFailureTag {
    #[serde(rename = "WorktreeMcpFailure")]
    WorktreeMcpFailure,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WorktreeMcpFailureCode {
    #[serde(rename = "capability_denied")]
    CapabilityDenied,
    #[serde(rename = "thread_not_found")]
    ThreadNotFound,
    #[serde(rename = "project_not_found")]
    ProjectNotFound,
    #[serde(rename = "already_in_worktree")]
    AlreadyInWorktree,
    #[serde(rename = "handoff_in_progress")]
    HandoffInProgress,
    #[serde(rename = "invalid_request")]
    InvalidRequest,
    #[serde(rename = "operation_failed")]
    OperationFailed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorktreeMcpFailure {
    #[serde(rename = "_tag")]
    pub _tag: WorktreeMcpFailureTag,
    #[serde(rename = "code")]
    pub code: WorktreeMcpFailureCode,
    #[serde(rename = "message")]
    pub message: IsoDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorktreeMcpHandoffResult {
    #[serde(rename = "worktreePath")]
    pub worktree_path: TrimmedNonEmptyString,
    #[serde(rename = "branch")]
    pub branch: TrimmedNonEmptyString,
    #[serde(rename = "baseRef")]
    pub base_ref: TrimmedNonEmptyString,
    #[serde(rename = "startedFromOrigin")]
    pub started_from_origin: bool,
    #[serde(rename = "setupScript")]
    pub setup_script: WorktreeMcpSetupScriptStatus,
    #[serde(rename = "continuation")]
    pub continuation: WorktreeMcpContinuationStatus,
    #[serde(rename = "note")]
    pub note: IsoDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WorktreeMcpSetupScriptStatusVariant1Status {
    #[serde(rename = "started")]
    Started,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorktreeMcpSetupScriptStatusVariant1 {
    #[serde(rename = "status")]
    pub status: WorktreeMcpSetupScriptStatusVariant1Status,
    #[serde(rename = "scriptName")]
    pub script_name: TrimmedNonEmptyString,
    #[serde(rename = "terminalId")]
    pub terminal_id: TrimmedNonEmptyString,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WorktreeMcpSetupScriptStatusVariant2Status {
    #[serde(rename = "no-script")]
    NoScript,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorktreeMcpSetupScriptStatusVariant2 {
    #[serde(rename = "status")]
    pub status: WorktreeMcpSetupScriptStatusVariant2Status,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WorktreeMcpSetupScriptStatusVariant3Status {
    #[serde(rename = "skipped")]
    Skipped,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorktreeMcpSetupScriptStatusVariant3 {
    #[serde(rename = "status")]
    pub status: WorktreeMcpSetupScriptStatusVariant3Status,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WorktreeMcpSetupScriptStatusVariant4Status {
    #[serde(rename = "failed")]
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorktreeMcpSetupScriptStatusVariant4 {
    #[serde(rename = "status")]
    pub status: WorktreeMcpSetupScriptStatusVariant4Status,
    #[serde(rename = "detail")]
    pub detail: IsoDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WorktreeMcpSetupScriptStatus {
    Variant1(WorktreeMcpSetupScriptStatusVariant1),
    Variant2(WorktreeMcpSetupScriptStatusVariant2),
    Variant3(WorktreeMcpSetupScriptStatusVariant3),
    Variant4(WorktreeMcpSetupScriptStatusVariant4),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorktreeMcpStatusResult {
    #[serde(rename = "attached")]
    pub attached: bool,
    #[serde(rename = "worktreePath", deserialize_with = "required_nullable")]
    pub worktree_path: Option<TrimmedNonEmptyString>,
    #[serde(rename = "branch", deserialize_with = "required_nullable")]
    pub branch: Option<TrimmedNonEmptyString>,
    #[serde(rename = "projectWorkspaceRoot")]
    pub project_workspace_root: String,
    #[serde(rename = "defaultStartFromOrigin")]
    pub default_start_from_origin: bool,
}

/// Pinned descriptor digest: includes schemas, defaults, annotations and error tags.
pub const TOOL_CONTRACT_DIGESTS: &[(&str, &str)] = &[
    (
        "orchestrator_capabilities",
        "4f23b5f149bb9c0b827353399d45d14b4d174049af99867ee62e90647d1ae832",
    ),
    (
        "delegate_task",
        "a2ec9f9dbf1e7b62880a6148725920027422f71bfce9177193d930f0402f2089",
    ),
    (
        "task_status",
        "21a2ddec56020783ade239734778addfb51cc1f9b8a4b8479bcf643d205733fc",
    ),
    (
        "task_cancel",
        "5a46e92a256320832ac6dc43fb0c86d055cdfe1b7dbe4ed0721e124b49d97427",
    ),
    (
        "schedule_task",
        "6dc0db08274983910dcf57bde42716b3a80a8ba760a613f18ab0e0092e5f978a",
    ),
    (
        "list_scheduled_tasks",
        "70574b79e65cbee57dbdbfdc7172f60bee3686b18ad614ec101eac018f9a929e",
    ),
    (
        "update_scheduled_task",
        "b6db4264f280b87c156ab5d59b3ec8cab085a0237bed06124f08af61c00a8efa",
    ),
    (
        "delete_scheduled_task",
        "7d55f099cc7a5dcc7fe723a01a4844623a2722c5ce51829932d9d73074be90cf",
    ),
    (
        "create_threads",
        "2c21de060cf7ae43085039a8e16cb86e88377cba5f8b6b446476db1608709ffc",
    ),
    (
        "t3_thread_list",
        "eb7aebd31283b118b7e526a5d8eb9b978e5dc68137dbbf40f14223d6e9709794",
    ),
    (
        "t3_thread_read",
        "c48487d0d20b6d5c08a5bca7c3842946e7920e959e48ce9cbd544cafb3583c57",
    ),
    (
        "t3_thread_update",
        "28f1b9a64c83d7ba7db8833d44a6971736f6ac4c69a1feb5fdc1d06290337bfb",
    ),
    (
        "t3_thread_send",
        "b36cd447e1caeda70c7a127c538d1744be10fbb889968eab60e85e019acf0652",
    ),
    (
        "t3_thread_wait",
        "d404486f139e3299a4c9fa26a9aa8c3f8df81664ef74d5474f3fe69b65318f23",
    ),
    (
        "t3_thread_interrupt",
        "645aa7e25375c4d5ffa05a2c6ad031dd60c47e7627b8ab5b7e035c110b1a337f",
    ),
    (
        "run_scheduled_task_now",
        "e8151ab2128955aeee8e0900544bc60eae43a628f8885ff7cb1f236eb3fe0b75",
    ),
    (
        "t3_thread_search",
        "8f5a3f9fd0343fb34d2d18f2c4be25087b13161903da6177858e8f7dd9741544",
    ),
    (
        "t3_thread_fork",
        "028b73915fc46110fc8eba815aabc173977a79fd0b6d5c19afc3b6a6e2eef66e",
    ),
    (
        "t3_thread_merge_back",
        "0211ba0c10fcfade27525b48625232cd125a4660678be68e7bed3d08d085c544",
    ),
    (
        "t3_thread_transfers",
        "51cab6780836114850d5bed079bc99701944a1b1fc5e86fd79ad3db7ed74be3c",
    ),
    (
        "t3_thread_configuration",
        "2e6cd337c2dc20d9a90a53149154354cc1e25e246992cb3ec639a18a2c93275e",
    ),
    (
        "t3_thread_configure",
        "6760203bbdfadc8f14d9ff4df7b5f21867ae07561b7025e4e57834ed32f6169d",
    ),
    (
        "t3_pending_request_list",
        "bdbfb9a0da792a2f36623cf671c1c832015bb9c01c484c6ba1b6d37a00721b03",
    ),
    (
        "t3_pending_request_read",
        "596f447692f1919532e394f674e3877c859eb029434b8d593020abc437cda94a",
    ),
    (
        "t3_pending_request_respond",
        "6a33aec43f5b2088a142efb685251b138e72f6d7502c4b3b71cb83f75e1720b1",
    ),
    (
        "t3_thread_organize",
        "62434f1411580def780b8f6b745da07dc0057dd068d526e14f4b9c4119503ccc",
    ),
    (
        "t3_queue_list",
        "7ba0ed06f9cb661ab23d246b1a76a9583be91941ad1db2d08bc05df1fec23a3c",
    ),
    (
        "t3_queue_read",
        "cdd11928b5ce14189a1b80f210d6b570d663319b426702f5a6fb0aa8d38025b7",
    ),
    (
        "t3_queue_edit",
        "3958cf6ba25e81c147372436f9d865aa3bafb7c99a2399692276711b11ee6285",
    ),
    (
        "t3_queue_cancel",
        "e0333d08a41cc1ece8dc65df09e2f8d9f5c54436591da66022a93037266269f7",
    ),
    (
        "t3_queue_reorder",
        "a5e47ebd290470c28435150a0fe98f60d9e671b068f426938917256f63290178",
    ),
    (
        "t3_queue_promote_to_steer",
        "2d6c3c28a5ca02ad327025f28bd918c39c0f5fc7568a0a5d6836d03bbb5c34f1",
    ),
    (
        "t3_worktree_handoff",
        "f4871df48e7099ab0a24c43f3d87f10964d49ead4dd3859a267b5723bdd9babd",
    ),
    (
        "t3_worktree_status",
        "417bfa139714ed90279d0fce2aab0f560cf30176a963406745ab817689be7acc",
    ),
    (
        "t3_worktree_list",
        "970c61127a7101444b7e642680ac96d6a96cb1da41e3a62e41c15b0de3ca88cf",
    ),
    (
        "link_pull_request",
        "b0d076eb5472c9908017b5462a35fce3aa65d7ec5569ce41cde49fb2fdd99125",
    ),
    (
        "unlink_pull_request",
        "96d231892304d5bdd2fd492b118194884918defc08d95192be775a754eb270bd",
    ),
    (
        "list_thread_pull_requests",
        "aa7772b9c66a62ecef819cd02e4df78c541264546b0523a57924994c88a229d1",
    ),
    (
        "watch_pull_request",
        "8617368f197e2bd9db2b79afd0e6f700d2295440888f23ef8d5ba99c76eb36cc",
    ),
    (
        "unwatch_pull_request",
        "6088cc3f50920e60f1bb079e9e6bc80a2b193c07dbe0b207779a6a991c076967",
    ),
    (
        "t3_thread_launch",
        "03212e02e032db2737aa69a453ba52040dc0f6a70cf9d85d62ebbcc6f05c0158",
    ),
    (
        "t3_project_list",
        "9358a3ae84449bbd720bb00fac35726181d884470513d9f112c44e1ebf87c577",
    ),
    (
        "t3_project_read",
        "268642af48425863660354c9f426c43bd0082fd3e56bb044bc1ea2875de2ac4e",
    ),
    (
        "t3_project_create",
        "148aff3f9c6f34d6e2de320097ba4dfca58acea7018286afc09d1e7a993b4fda",
    ),
    (
        "t3_project_update",
        "ddc95ad24ef75cfed99f716491050c444e72e927b571f88063e354e537caf024",
    ),
    (
        "t3_project_delete",
        "981ca6bf09cfa092ffc5f85fd69bbdebfa49e03161d79c4f5ab61ed9e312ebbd",
    ),
    (
        "t3_project_clone",
        "2a8930e99e863edb6567cc2dbf58a8dcc9b478db2d205f1bb8baf33b3435bd63",
    ),
    (
        "t3_environment_read",
        "edab88f8a41ca7a3978cae13b5f6f8e880d1a3b79c6eb362e33b5cbb734a1ca3",
    ),
    (
        "t3_environment_preferences_update",
        "e368bdc5d52c749e86424568126ee2663b377397cffd4007be4d025651839cdd",
    ),
    (
        "t3_attachment_prepare_upload",
        "cd03c322901a16a3d514136e4252ac38dd2221582169b1f21186606c809885d7",
    ),
    (
        "t3_attachment_discard",
        "23d1d91f665d00c5f45bbfe03be34ab558e8528eb2ea0d4ed68ce640a0a65030",
    ),
    (
        "t3_thread_send_attachments",
        "c6ce5a5d77ef9c7336d5a3553f846f25969bd48ebb2859d32f9e658f32462b95",
    ),
    (
        "preview_snapshot",
        "39111707544dfc2a6b624a2fbb2527383b1bd006c29e0489b7c7ce08932fda90",
    ),
    (
        "preview_status",
        "ff5fff3513b0dc3b8085c55c5c5383177e706f79dafe0aa3750af14a3b40f049",
    ),
    (
        "preview_open",
        "1e0da73e21aaa20d9fcfac6a5b9db3e1ce9962a564386ce7824aaa8c67a86abe",
    ),
    (
        "preview_navigate",
        "8813a1b94648ec692ec1985706d7da4175ea8f884460b6990d3871a0ff282a83",
    ),
    (
        "preview_resize",
        "bcf9b6a4f0dabf5453365b957e1e2ab4e49b881c47a713fae6d12bcda0ba78b5",
    ),
    (
        "preview_set_appearance",
        "347a0db4b37fd0207c28745c48a819cb9cfaf61e51653ebd1ff742277da2f886",
    ),
    (
        "preview_click",
        "48ac8199cc685fd6c1a296aadff3e255f9b91979903c0f13eb205f099d06f183",
    ),
    (
        "preview_type",
        "aa97ee9018cdaf51ba10dd006783104b85ddf8f55a326fa1c14ea40f82a6398e",
    ),
    (
        "preview_press",
        "fa526e0e45463eae11de36b0a04d9a05cc587af1b59a84f9a670cc8ae6598040",
    ),
    (
        "preview_scroll",
        "831cafd108bf003e8edcf7d5dbf53b199ff8782116419cb72812d4d2d6fd3006",
    ),
    (
        "preview_evaluate",
        "ce86f06f58a1b0136a2281ef9a34922b6f150bb7846e01cdf5675e5c8fc90648",
    ),
    (
        "preview_wait_for",
        "8d5f6fccf54e250a167110d18dbf0bf682e49e806f75b3cddfe934d623662e1e",
    ),
    (
        "preview_recording_start",
        "c4716138c43bc29fd331c583d25f025680b3102829405354fcf030d32df7975b",
    ),
    (
        "preview_recording_stop",
        "46b44ee937db2f0cc2f0d9b8d8a333807c8350b9692be17bee956a2c0345afad",
    ),
    (
        "t3_preview_list",
        "7a4ebcaeed64ffafbbbc038d60425874f62eba5ac6122fb4b47bb1f0f9417c30",
    ),
    (
        "t3_preview_close",
        "8042986d953003d77f7dd58edd9db96d8ffa9aed0d1a1d10def2f3bdd8334eca",
    ),
    (
        "device_screenshot",
        "d86d7a622246009a70d0eaf12d6c70d29796bcce4265d83438b58f2c5a78139c",
    ),
    (
        "device_list",
        "72fbba7dd8e7fb6919771f8abbc9998347d3bb34b3abdfe1bbe6cae474072ec1",
    ),
    (
        "device_open",
        "1ce35778a48fb86d322a56784d399886173595906d2d5069d1f33d0fb1b1d63b",
    ),
    (
        "device_close",
        "b0b2b4ebce0f3047e19d8990619f6b2457e6d3ea4daeb00527320a8422427c92",
    ),
];
pub const DOMAIN_CONTRACT_DIGESTS: &[(&str, &str)] = &[
    (
        "AttachmentCreateUploadUrlResult",
        "efeb2449e88a30e93c4ee055cd7c2da7e9b8f75884cd81c505ef9eaf8f8c736b",
    ),
    (
        "BackgroundActivityProfile",
        "b3a3fc17826828e4c8b027bcd44603b6fe5e99ae17ca481927a241a273741b93",
    ),
    (
        "BackgroundActivityProfileSelection",
        "d944641c112063e169042a008d93838a14193fe23302c8da90a11b8392bba682",
    ),
    (
        "BooleanProviderOptionDescriptor",
        "23a026c0af8335b3dc6608d926f8a3cb091628449c8a14b9271fc5ad9d167ea3",
    ),
    (
        "BrowserProfileId",
        "9bddc1fdfb55932e41cf34e4a0bf062564cf848d3e0070d86dcd3a998fdce008",
    ),
    (
        "ChatAttachment",
        "b390a32cc036dd4638a52caee22f24efdba4e77741b4fe49d7a20aa24c2e47ac",
    ),
    (
        "ChatAttachmentId",
        "9bddc1fdfb55932e41cf34e4a0bf062564cf848d3e0070d86dcd3a998fdce008",
    ),
    (
        "ChatFileAttachment",
        "5d55874de6a06d1b19ac1c17505af82a3eea5fb469822b2f5538ec2b91554d54",
    ),
    (
        "ChatImageAttachment",
        "eace619c4cad35f7eeb5600e52a29aafe2b45599d87b0c0961ad8c4a6975f33a",
    ),
    (
        "ChatUnknownAttachment",
        "5ab179ac98c3f38bf6daaa91beab0ba70d4fd31f48d2b82315277d7777de36d1",
    ),
    (
        "CheckpointId",
        "9bddc1fdfb55932e41cf34e4a0bf062564cf848d3e0070d86dcd3a998fdce008",
    ),
    (
        "CheckpointRef",
        "9bddc1fdfb55932e41cf34e4a0bf062564cf848d3e0070d86dcd3a998fdce008",
    ),
    (
        "CheckpointScopeId",
        "9bddc1fdfb55932e41cf34e4a0bf062564cf848d3e0070d86dcd3a998fdce008",
    ),
    (
        "CommandId",
        "9bddc1fdfb55932e41cf34e4a0bf062564cf848d3e0070d86dcd3a998fdce008",
    ),
    (
        "ComposerContextId",
        "9bddc1fdfb55932e41cf34e4a0bf062564cf848d3e0070d86dcd3a998fdce008",
    ),
    (
        "ComposerContextRecord",
        "02d2af165af2f7482d71951f9f1702dee24ba815bc8162089183faedf051b0a4",
    ),
    (
        "ContextHandoffId",
        "9bddc1fdfb55932e41cf34e4a0bf062564cf848d3e0070d86dcd3a998fdce008",
    ),
    (
        "ContextTransferId",
        "9bddc1fdfb55932e41cf34e4a0bf062564cf848d3e0070d86dcd3a998fdce008",
    ),
    (
        "CreateThreadsError",
        "029ec0b2d47ec1133a238247d5b48a4733fdf689b727aaad538947b905fe9040",
    ),
    (
        "CreateThreadsInput",
        "4400bf0000b89972ce854a255aab7f8e0e5f712e125272ddbc4ffb418683fb59",
    ),
    (
        "CreateThreadsResult",
        "1836c94fa0becdaf8da78861c1c029bda1524930fafdcce4c8cdd04a87eaab26",
    ),
    (
        "CustomModelEntry",
        "5388d38f491fb777507c0efe32b78276b32c8844b6185fd6049c981b9050546a",
    ),
    (
        "CustomModelSetting",
        "9f018b51e9c21dbe748f5c5fac20c33a48f49a7734dff6ecad336825d4e31462",
    ),
    (
        "DelegateTaskError",
        "029ec0b2d47ec1133a238247d5b48a4733fdf689b727aaad538947b905fe9040",
    ),
    (
        "DelegateTaskInput",
        "86e6bd29b2fa8caa5b5535eb173835d8fa91ed2b786ce56899c4b67718d36271",
    ),
    (
        "DelegateTaskResult",
        "c2bbb732663e757b927e08d2aae6f278477b203cecc438ec2ad6372a21fdf5e3",
    ),
    (
        "DeleteScheduledTaskError",
        "029ec0b2d47ec1133a238247d5b48a4733fdf689b727aaad538947b905fe9040",
    ),
    (
        "DeleteScheduledTaskInput",
        "985f199fe36eb6aca4317b1f56929d04242ce27c5ff5dbdb0d8a41e43359a091",
    ),
    (
        "DeleteScheduledTaskResult",
        "888ca9dce9c7c2eb7ef16a74639295ec6e2cf2e8d8e3b689bee6c84420421763",
    ),
    (
        "DeviceActionUnavailableError",
        "88b39165b35bd7a80d226b721d37207ee58f02e84daca4de215ca0ec03b7f13d",
    ),
    (
        "DeviceBootError",
        "92ea83c12715c1d07081a146b6dcb48400068aa99f640a69e2dbeadcd27232f0",
    ),
    (
        "DeviceCloseError",
        "1cb72f9f6503f0dd765418180b50ef65af8d7471c06cc387b4bdf1551295a174",
    ),
    (
        "DeviceCloseInput",
        "0bdcc5b0ed224c00c78e61b37e3a02cd87a8df492000b7805493194bc500421b",
    ),
    (
        "DeviceCloseResult",
        "3e7cd0c7da63c31f23d31e81653eeb201e43a51bae91d944318c3caf8172d41e",
    ),
    (
        "DeviceHostId",
        "9bddc1fdfb55932e41cf34e4a0bf062564cf848d3e0070d86dcd3a998fdce008",
    ),
    (
        "DeviceHostStatus",
        "76d3a1701fff094e6a216913d52f9f30311426e440dfc1c646269d9a1b82d226",
    ),
    (
        "DeviceHostSummary",
        "ef1169dbb7d9d23133508ed8225eedf5dd9b9be7d2f857d265f4018f78a573f9",
    ),
    (
        "DeviceHostUnavailableError",
        "035e9d9d22641df77c9b2f7d9286a391a26f82e0edeec1fd8ca8bdbc70f353fc",
    ),
    (
        "DeviceId",
        "9bddc1fdfb55932e41cf34e4a0bf062564cf848d3e0070d86dcd3a998fdce008",
    ),
    (
        "DeviceListError",
        "1cb72f9f6503f0dd765418180b50ef65af8d7471c06cc387b4bdf1551295a174",
    ),
    (
        "DeviceListInput",
        "358d3b3a3d316d338c84a883eeb07a17c3c3ee3ea6a39d7fbfc73110794919b1",
    ),
    (
        "DeviceListResult",
        "f40f1ff3c722a642797fd0bc2f1d4afc6edf46678a710a3df9cb831e6ac5ed9f",
    ),
    (
        "DeviceNotFoundError",
        "3b358e5c1e63220783848165699c4d793e6a3006bc27bed3476044d3b05f4e1a",
    ),
    (
        "DeviceOpenError",
        "1cb72f9f6503f0dd765418180b50ef65af8d7471c06cc387b4bdf1551295a174",
    ),
    (
        "DeviceOpenInput",
        "1a8647d09d24b71d57aaf4dd8a1b2177e0f372e43e99c1a33b032b4663ea2b62",
    ),
    (
        "DeviceOpenResult",
        "24ec250bd39f34726228e18bdcaabd0e581bf690e27e9a2e9d795a6e57339432",
    ),
    (
        "DeviceOperationError",
        "e4cadb6a64ee7297f6693e1f087a02c3c9eb66b0dcd328912368d0249a14db8c",
    ),
    (
        "DevicePlatform",
        "3c9ac412a117a28427de74a9bcfae1db5fb3ff5e8c7ea98f03188beb742834a2",
    ),
    (
        "DevicePlatformAvailability",
        "072260d72151e05d3d5d463f9fd10187ce0467ad0ff16b87f5f53b392baa450b",
    ),
    (
        "DevicePlatformUnavailableError",
        "7db22e38975db2219dca162ca3a945a5f23b7b29da4272c73288422f9e04319e",
    ),
    (
        "DeviceScreenshotError",
        "1cb72f9f6503f0dd765418180b50ef65af8d7471c06cc387b4bdf1551295a174",
    ),
    (
        "DeviceScreenshotInput",
        "8f7f55e6167c3ca55a25cf583c7a4762690b28e9504a8233ef4840e2d9207b1b",
    ),
    (
        "DeviceScreenshotResult",
        "bcca0c64edff96eaa132ea6d73c1d98db3136820da4b7602661ef21c8ea375b5",
    ),
    (
        "DeviceSummary",
        "2cc34cbd22ec3df4d1923d56a84c26faf8d7d8762b6dbef2325c787cf99cc79f",
    ),
    (
        "DeviceToolError",
        "a6034d69d9cfa445a515c57ead038dbc547090a7d52f384a7cdc5275d897b7e0",
    ),
    (
        "DeviceToolListResult",
        "1f6142a8c3406f21bb6ac684946d956e288c8e988bc36b0a5d02055ea2e0c90c",
    ),
    (
        "DeviceToolOpenResult",
        "e89c9d04cc5ad55d00b8d3fce7b365a1a3cc487e1c7f3adbe9a80f0ff9ced529",
    ),
    (
        "DeviceToolScreenshotResult",
        "fd8525fedb445d0f900f4d89f6d4783447686760f9a904968d76af7a11f42c32",
    ),
    (
        "DeviceToolUnavailableError",
        "cc235bb06080e6a14b66f92be0b07fcf28c65e80a29c1613d11becbd905ff800",
    ),
    (
        "DeviceToolVersion",
        "76f6e2b41bff9d09ff022611e3e172fea09b465173adabbf3fb6274eef54e666",
    ),
    (
        "DeviceToolVersions",
        "ad74b743ad2ddc50ca4efb230a08f23bd415b3af91e27dcd79552623cfa98f44",
    ),
    (
        "ElementContextDetails",
        "3d14a4278c6c577f6c6e0f811a9165519a58c2ee8691f50d4c54998d1454fc48",
    ),
    (
        "ElementContextRecord",
        "6199a74a8a2193ba9896e7c9d332d431cc83eb04284cca33c90693ba7f2cc3aa",
    ),
    (
        "ElementContextSource",
        "6b51c0befd1b5e719a95d134781d65bb703c04c2de85a16b8c698ad420a58b2e",
    ),
    (
        "EnvironmentId",
        "9bddc1fdfb55932e41cf34e4a0bf062564cf848d3e0070d86dcd3a998fdce008",
    ),
    (
        "EnvironmentMachineKind",
        "59cec0b6dc1ee10102a21aa78896722590f66192c26b0a01ee18e4a120c64c7b",
    ),
    (
        "EventId",
        "9bddc1fdfb55932e41cf34e4a0bf062564cf848d3e0070d86dcd3a998fdce008",
    ),
    (
        "ExecutionEnvironmentPlatform",
        "e85d07d329067ac9a8f250335f143dc914ebf16de685f7d1255614217f2b0d38",
    ),
    (
        "ExecutionEnvironmentPlatformArch",
        "b1eca2b8ccb59fd0f6fbc2904a315dc00dd0458e9bf6cbc4c9521975bc6f4adc",
    ),
    (
        "ExecutionEnvironmentPlatformOs",
        "6a60ca2f443ad0429ebd07b7c5fb167090bb988c87c0e144568bb5e0754a2ecd",
    ),
    (
        "FileContextRecord",
        "6bca115a8796b8674a30ed9fc238d0c180a15344c4e3a1c21b42dd4620db82e5",
    ),
    (
        "ImageContextRecord",
        "e5599a497656fe8b7d5c72c5359383303acc099fbf787ff05ed03d7e0d8d9fbf",
    ),
    (
        "IsoDateTime",
        "9bddc1fdfb55932e41cf34e4a0bf062564cf848d3e0070d86dcd3a998fdce008",
    ),
    (
        "LinkPullRequestError",
        "9467654a0069596fc9d535edd0f61c9e55775435778ee650ca520a8bf286e8bb",
    ),
    (
        "LinkPullRequestInput",
        "19bc9074da7d7c6953574f5570e131667b5fbae6f2d03e27d61f6f5ade07d406",
    ),
    (
        "LinkPullRequestResult",
        "3272cae7181999c5713d2217f352c09f23af410a0479be2a0cd4f00b0eeb7d08",
    ),
    (
        "ListScheduledTasksError",
        "029ec0b2d47ec1133a238247d5b48a4733fdf689b727aaad538947b905fe9040",
    ),
    (
        "ListScheduledTasksInput",
        "3e7cd0c7da63c31f23d31e81653eeb201e43a51bae91d944318c3caf8172d41e",
    ),
    (
        "ListScheduledTasksResult",
        "624afe6b9f15236c2f55d54e97b5b5401149f381d7dd41ab89ce776d958f646b",
    ),
    (
        "ListThreadPullRequestsError",
        "9467654a0069596fc9d535edd0f61c9e55775435778ee650ca520a8bf286e8bb",
    ),
    (
        "ListThreadPullRequestsInput",
        "3e7cd0c7da63c31f23d31e81653eeb201e43a51bae91d944318c3caf8172d41e",
    ),
    (
        "ListThreadPullRequestsResult",
        "846db4b877fd132f370c72323f9e89e906d3ab087b8bac642ad85fcd9281138f",
    ),
    (
        "McpCapabilityUnavailableError",
        "5feeb6e39eabbc22d3ade193462269ea9f615de4a1269ca11ff2a0d02c3fd72c",
    ),
    (
        "MentionContextRecord",
        "ea07fd8a894dcba23228c197119ec533309f9e03975f1750b3562995a55fb647",
    ),
    (
        "MessageId",
        "9bddc1fdfb55932e41cf34e4a0bf062564cf848d3e0070d86dcd3a998fdce008",
    ),
    (
        "ModelCapabilities",
        "c02967c001986beaba4159c511b695463786036ad9d0948d426a17c532992922",
    ),
    (
        "ModelSelection",
        "cf5a6df671393d167d9d0b4e910d5a84651e4ec7c3b3d374ccbb41076f603ac9",
    ),
    (
        "NodeId",
        "9bddc1fdfb55932e41cf34e4a0bf062564cf848d3e0070d86dcd3a998fdce008",
    ),
    (
        "NonNegativeInt",
        "96de37a86fda3c92e55456d8471fb3536852eab7757820b7d1383f22f012a501",
    ),
    (
        "OrchestrationGetWorkflowScriptError",
        "11aff23dec11aff5ff53f0bc44a5c3518a4dee65cf9d8780ff81eaf0398a7839",
    ),
    (
        "OrchestrationMessageContext",
        "8ffd29b3aa8ddaac57b80cefe7b720ab99c590139daf05b1477cd372d1826304",
    ),
    (
        "OrchestrationProjectShell",
        "1caeb2de95e1aeb1402a6d9fbe9f1e8ad536a1e50d07fcf115dd09bc266ff84a",
    ),
    (
        "OrchestrationSearchThreadsResult",
        "aa009e6b7376827f53ce8e6347ac1c45919789496b0ea7aa404b7b30ab4a0bff",
    ),
    (
        "OrchestrationThreadSearchMatch",
        "bc696db7d5e88e46227b7cb3629ef377c0eef0814f0157cf998657f716cc1122",
    ),
    (
        "OrchestrationThreadSearchSource",
        "badbcbbc1064abfb57c9455f1bcd1a171f480c2ec5159092ad9f7fcf6ecdeb9f",
    ),
    (
        "OrchestrationV2Actor",
        "2b0f8898a5d6927e76614c05ab0d430b153050ca3c067e3b06fb6e08c102c8b9",
    ),
    (
        "OrchestrationV2AppThread",
        "c6c538919d8891fccbe37d6ee4b24f27bbd78fe0bd0076fa7ac94c2d48add727",
    ),
    (
        "OrchestrationV2AppThreadJson",
        "c6c538919d8891fccbe37d6ee4b24f27bbd78fe0bd0076fa7ac94c2d48add727",
    ),
    (
        "OrchestrationV2AppThreadLineage",
        "9061224fa314b2d825b6aa8cdedc59879ed410f99e68d722d88e71192149cbd3",
    ),
    (
        "OrchestrationV2ApprovalCapabilities",
        "5d2f916558304034e8b8af7b30307b0c93fc856b1e25b865cb8f74269071892b",
    ),
    (
        "OrchestrationV2ArchivedShellSnapshot",
        "43a3d24345a697d993485e5cdbce1ad8c16239f78260dd582ab39b9af36472db",
    ),
    (
        "OrchestrationV2ArchivedShellStreamItem",
        "7d975286c3e51fc95346df49558059de65729aeaaad661f183c56bd30963d0d3",
    ),
    (
        "OrchestrationV2Checkpoint",
        "5ce16836e3191cdea5a3259cc1fc525484ef11d98444cdf3c010eea6dd2a1545",
    ),
    (
        "OrchestrationV2CheckpointCapabilities",
        "80071cca97e443e3430793e9c59337b7beafbe7626f750ac6da2a8a26809fe2b",
    ),
    (
        "OrchestrationV2CheckpointFileSummary",
        "6967d1772da97e923bee885f902d686b37f6edd1d411073fd731b911a7e5a537",
    ),
    (
        "OrchestrationV2CheckpointJson",
        "5ce16836e3191cdea5a3259cc1fc525484ef11d98444cdf3c010eea6dd2a1545",
    ),
    (
        "OrchestrationV2CheckpointRollbackRequest",
        "daf647328ed386658bbe2724e701f04d44590026f4184c736029408c1c8d4684",
    ),
    (
        "OrchestrationV2CheckpointRollbackRequestJson",
        "daf647328ed386658bbe2724e701f04d44590026f4184c736029408c1c8d4684",
    ),
    (
        "OrchestrationV2CheckpointScope",
        "d1435b07762315efd03fa29bfe9602aff5eee54c9b11ebf92129f5863c641ed3",
    ),
    (
        "OrchestrationV2CheckpointScopeJson",
        "d1435b07762315efd03fa29bfe9602aff5eee54c9b11ebf92129f5863c641ed3",
    ),
    (
        "OrchestrationV2CheckpointUnavailableError",
        "531af898a0f39943ddce496ef4c16fadbd12842d32a6b72bec6db6d28c9f1b0a",
    ),
    (
        "OrchestrationV2Command",
        "7f6f9e8810041d2b138aef02a57e61ebf7bdfd4cfd9dfb8fc69815aee602f8ae",
    ),
    (
        "OrchestrationV2ContextCapabilities",
        "12bd3f9fcf8892c25dc46ffd68dca7aca0c917996476d21889048efc20d99f2c",
    ),
    (
        "OrchestrationV2ContextHandoff",
        "208695c47bf578c2f9c56c2778e963de2d30f5418b73aeae31ab94656c654a48",
    ),
    (
        "OrchestrationV2ContextHandoffJson",
        "208695c47bf578c2f9c56c2778e963de2d30f5418b73aeae31ab94656c654a48",
    ),
    (
        "OrchestrationV2ContextSourcePoint",
        "e2f6c56e77231af5d3a60af3a282db83b44ae789a2659581dce3cdaaaa3e8761",
    ),
    (
        "OrchestrationV2ContextTransfer",
        "729e5586e752e63d052905620bca27a7f452f5c23daac825834156064f03837f",
    ),
    (
        "OrchestrationV2ContextTransferJson",
        "729e5586e752e63d052905620bca27a7f452f5c23daac825834156064f03837f",
    ),
    (
        "OrchestrationV2ContextTransferResolution",
        "9e39ecb87d09c85e1d587bed1c8a115b4cf60fce2695b905d380993a88c984bd",
    ),
    (
        "OrchestrationV2ContextTransferType",
        "47ae2340a0cd7489cc32a5133db2698ce79d13aafb9ba82d09abd4ac09761b18",
    ),
    (
        "OrchestrationV2ConversationMessage",
        "92dce0d8afcca404e5e4b71734bfc3b67e7059187c883a90f98ba23c1ab00a93",
    ),
    (
        "OrchestrationV2ConversationMessageJson",
        "92dce0d8afcca404e5e4b71734bfc3b67e7059187c883a90f98ba23c1ab00a93",
    ),
    (
        "OrchestrationV2CreationSource",
        "25952ecbe0aec2cee26fdfb1b53e864d5cfab87e864122c28938caae42bd96e3",
    ),
    (
        "OrchestrationV2DelegatedCompletionCohort",
        "eeefa63311d77718fb1e32926d4bc3af265fc4ae5e180b5adee13be0130bd245",
    ),
    (
        "OrchestrationV2DelegatedCompletionDelivery",
        "475a7872a4ab7dbc8764f131d53849f3d46cfeb3ef98493ac7c8f5e27418ddb7",
    ),
    (
        "OrchestrationV2DelegatedCompletionTaskDelivery",
        "2cf0f036f6407e8e31b371de463c9e2c7703d744061d1256448d0745d2327281",
    ),
    (
        "OrchestrationV2DelegatedCompletionTaskDeliveryState",
        "2ce824e82d31d8f9ac4da033450d3b671c78e4b2df219bff6a6095c33bf4ba5a",
    ),
    (
        "OrchestrationV2DispatchCommandError",
        "a3127b02e746a39ff93c8cc7d2e81074d524d3b352cb1840a4a5f1f15a767d20",
    ),
    (
        "OrchestrationV2DispatchCommandResult",
        "446276f537f0f849783d1a2a4663eb9537a28519625f67adb3b4e75acfdb0760",
    ),
    (
        "OrchestrationV2DomainEvent",
        "fd8ff4a4d93dfd6fde53756c481a22cf2ad268ec99bcbacde70fa09c475516c8",
    ),
    (
        "OrchestrationV2DomainEventJson",
        "a5bc70fd92711f2f599f446773a09024791ab4406c5b98c58b20644ba47f5f05",
    ),
    (
        "OrchestrationV2ExecutionNode",
        "86445b8e2788df6a88812c9957b2a7b4321e2e3f9b98ec1bfe75b8896f409a75",
    ),
    (
        "OrchestrationV2ExecutionNodeJson",
        "86445b8e2788df6a88812c9957b2a7b4321e2e3f9b98ec1bfe75b8896f409a75",
    ),
    (
        "OrchestrationV2FileChangeDetail",
        "f4afa90c42363e35b56d29ef6c456c1068322684a74d881c68913b763ee9b428",
    ),
    (
        "OrchestrationV2FileSearchResult",
        "b14b563d932c42e1c8f43cc89e03314122a863b858ad0cbdb00404e332501c7b",
    ),
    (
        "OrchestrationV2GetShellSnapshotError",
        "aa5fa2d286fafa1e536feb3c00ff6fbfc619ea422c8df908e068f55124f85def",
    ),
    (
        "OrchestrationV2GetThreadProjectionError",
        "9681684761fcb05f651dd0594fb83258f6a7daaa0da7dc489f402114fc7dae2d",
    ),
    (
        "OrchestrationV2GetThreadProjectionInput",
        "d950248bf42c118968b9977ebe2c4a2333d0edfe7adc1e3959eeba7022cb840e",
    ),
    (
        "OrchestrationV2GetWorkflowScriptInput",
        "6be251847835461aa66f4d29773d289e6c5f83fda55f5db98d3015df29aeaf81",
    ),
    (
        "OrchestrationV2GetWorkflowScriptResult",
        "322abd905d7740f3fa9c683f8cd503712012a9a2d5ed06b9d5712b92595aea53",
    ),
    (
        "OrchestrationV2HistoricalMessage",
        "6d1e93371a9ce65dcb35a6613c333710660b9d99403cfe6c800ac9d207275f0d",
    ),
    (
        "OrchestrationV2IdentityCapabilities",
        "8d3973d9e75bea8b4116f72e52c0f4f0b40cdef264b0cba6f7204c41d53f1506",
    ),
    (
        "OrchestrationV2LatestVisibleMessageSummary",
        "2cbe6c74c15f0a9f24e16353076d4f27733c7dbd2390c73d3f403a3a715f8b8c",
    ),
    (
        "OrchestrationV2LatestVisibleMessageSummaryJson",
        "2cbe6c74c15f0a9f24e16353076d4f27733c7dbd2390c73d3f403a3a715f8b8c",
    ),
    (
        "OrchestrationV2LimitRecovery",
        "3345630ff05dcd30d86aaa4caf33576df2c8182ddf44c3bba7376630372d3616",
    ),
    (
        "OrchestrationV2LimitRecoveryUpdate",
        "42c9c73eb5b11b908cf7557c86601e5f9db0b7147b1586ba15ff65062e56daf0",
    ),
    (
        "OrchestrationV2NativeRefStrength",
        "d9569d323bfa56dcb2c42051e8e2ec0e77b2736465754cfa98ed996cafb7f6fb",
    ),
    (
        "OrchestrationV2Notification",
        "824d385f27fe9aca5fa382a2f4d1e5936dc46eb700e11c63a96e686986a0c762",
    ),
    (
        "OrchestrationV2NotificationSource",
        "80bda23e1af543f2405fa04e724e2ddd59fc000557a8db8ae9c1f10085b22d12",
    ),
    (
        "OrchestrationV2PendingBackgroundTask",
        "bc4a82d8d99040c7014cf12be4cc1c26dd97ecdd705c23df519ba88395e53e7e",
    ),
    (
        "OrchestrationV2PendingRuntimeRequestSummary",
        "8434da239f9b50db162efe5e7eac83805920b26feb8ad255fe4b3f725564e02f",
    ),
    (
        "OrchestrationV2PendingRuntimeRequestSummaryJson",
        "8434da239f9b50db162efe5e7eac83805920b26feb8ad255fe4b3f725564e02f",
    ),
    (
        "OrchestrationV2PlanArtifact",
        "8e78c0c6b7c7f7418707dd8ec52187c5e9c627114fbb828d8fe2e120f3cc3d2a",
    ),
    (
        "OrchestrationV2PlanStep",
        "dbb9460f751e5aaa0577f3f3dcdbdb091d21ea97584798f7f098b4256d4a2c16",
    ),
    (
        "OrchestrationV2PlanningCapabilities",
        "74d91c34084fd6c01146ee93321d4ebd25a1ade57c86e8719fd29028927b88a2",
    ),
    (
        "OrchestrationV2ProjectedTurnItem",
        "2676d6d26be64380a28a6cc577de7f06fbae715409d343df771896443f6efcf3",
    ),
    (
        "OrchestrationV2ProjectedTurnItemJson",
        "0c9b41321b370c89935d02fae7004b65bd1a4a46f802ebc7292361920bbc9df4",
    ),
    (
        "OrchestrationV2ProviderCapabilities",
        "9d0b28de9ec428d29803150814e36dc35670d3258744c36406a9f887f65c400e",
    ),
    (
        "OrchestrationV2ProviderFailure",
        "71d49c41279e5aee72419db373273f24032ee59f6574b29aa9d56e3c33ecafd2",
    ),
    (
        "OrchestrationV2ProviderFailureClass",
        "e85c31af15a5fbc51235f6ddc24fbb5652de22f12712dd95d2344cd085e76ab7",
    ),
    (
        "OrchestrationV2ProviderRef",
        "702eac98921b62432fc618b39c50de88a6abf9d4e8889c3942bd7db993bece5c",
    ),
    (
        "OrchestrationV2ProviderRetry",
        "6a963fc5c009c36d8c53a53971fb4e12945a2f2d3f0978d109f088783964c5cf",
    ),
    (
        "OrchestrationV2ProviderSession",
        "371f4d2ae7b03a98b06fd642a90edadc647b1c99de4d61e0690821e9c441e779",
    ),
    (
        "OrchestrationV2ProviderSessionDetached",
        "375eba6e9a6daf5922c42dc93ef4f073e6568092e337e2483061269fd5ba6007",
    ),
    (
        "OrchestrationV2ProviderSessionDetachedJson",
        "375eba6e9a6daf5922c42dc93ef4f073e6568092e337e2483061269fd5ba6007",
    ),
    (
        "OrchestrationV2ProviderSessionJson",
        "371f4d2ae7b03a98b06fd642a90edadc647b1c99de4d61e0690821e9c441e779",
    ),
    (
        "OrchestrationV2ProviderThread",
        "19ce21e19ae6cbd5f975eab4dc8e1e8131b92c08ac1a8b096b2a01a3528406a1",
    ),
    (
        "OrchestrationV2ProviderThreadDisposition",
        "aeb33ca7699380567ec7ee020c423eaf96733aaa91654e6fdb7cf37f34132a0a",
    ),
    (
        "OrchestrationV2ProviderThreadJson",
        "19ce21e19ae6cbd5f975eab4dc8e1e8131b92c08ac1a8b096b2a01a3528406a1",
    ),
    (
        "OrchestrationV2ProviderThreadNativeMetadata",
        "188ccc658c59b7617898a7ecca277ca378e37de59a9f91a6066d396f14d4a140",
    ),
    (
        "OrchestrationV2ProviderTurn",
        "f981dcdfdaa0bef306b20078331fad5a04735942568adb0cfc7efeb9fc3d9995",
    ),
    (
        "OrchestrationV2ProviderTurnJson",
        "f981dcdfdaa0bef306b20078331fad5a04735942568adb0cfc7efeb9fc3d9995",
    ),
    (
        "OrchestrationV2ProviderTurnTokenUsage",
        "70c57d2c5dd5110e15c285da1aea44516bb29077f72f8d74a0b272e909d1e68b",
    ),
    (
        "OrchestrationV2RawProviderEvent",
        "575c3a525214edc5fe061cc8fe5f483c95cd8c2a858c33743b3a6e7ccb9c664e",
    ),
    (
        "OrchestrationV2RawProviderEventJson",
        "575c3a525214edc5fe061cc8fe5f483c95cd8c2a858c33743b3a6e7ccb9c664e",
    ),
    (
        "OrchestrationV2RestartCancelledBackgroundWork",
        "8ae62d996735d2eb1fe7ef258e53ba0636d1bda8122b8f6138f3f5d08ff8a800",
    ),
    (
        "OrchestrationV2RpcError",
        "0c0f39e2209829cc57cb4b8b19af08b81e82d01c7d1d0fda4434145bc099e024",
    ),
    (
        "OrchestrationV2Run",
        "732e75b97440885c7fcb72b2c274ca377e20e093635be6681946fa1c60724a1f",
    ),
    (
        "OrchestrationV2RunAttempt",
        "d9fa676d8034b4aaec7549e363a8a0de2e56dda843b960ad7d894f9cd088b566",
    ),
    (
        "OrchestrationV2RunAttemptJson",
        "d9fa676d8034b4aaec7549e363a8a0de2e56dda843b960ad7d894f9cd088b566",
    ),
    (
        "OrchestrationV2RunBackgroundWorkCancelled",
        "82ee14597bd6780b0dd0bdb33c6a548e795ca0dc560a31e1f8b0a59f710cd71d",
    ),
    (
        "OrchestrationV2RunJson",
        "732e75b97440885c7fcb72b2c274ca377e20e093635be6681946fa1c60724a1f",
    ),
    (
        "OrchestrationV2RunStatus",
        "276e83d39e6bcb2c01183875587e841d221c01c9a1cd6c58dd0d495515c1f88c",
    ),
    (
        "OrchestrationV2RuntimePolicyCapabilities",
        "51cdd9f504cbdafe7a4ede704e907d68eb2144bbeb8db3918bd8bbf6a0d645fd",
    ),
    (
        "OrchestrationV2RuntimeRequest",
        "b04ab61d54f32e2df913b48f68480d7ef48946f46ae9d75733a0188784c7f010",
    ),
    (
        "OrchestrationV2RuntimeRequestJson",
        "b04ab61d54f32e2df913b48f68480d7ef48946f46ae9d75733a0188784c7f010",
    ),
    (
        "OrchestrationV2SessionCapabilities",
        "36395f78e449846ddd153d66f555c03d637178429838057202716c0e73069bc2",
    ),
    (
        "OrchestrationV2ShellSnapshot",
        "18c278e042c7cfa03a6d61a10e62a214f98671e178b55bf79c40a096fae59f4b",
    ),
    (
        "OrchestrationV2ShellSnapshotJson",
        "1272ed93ccbb53aa4f3d6652b5416fe61b6ec29420964e7950a862dd63152344",
    ),
    (
        "OrchestrationV2ShellStreamItem",
        "9f3d5cd78c0c8d1903667b264db75e464cadf915c2d2d3b77159d27b714ae6b4",
    ),
    (
        "OrchestrationV2ShellThreadStatus",
        "73e9f0c03bab54c2fa9e55f01467e5fec63d1fb6cba4315af7cfc72338d5619c",
    ),
    (
        "OrchestrationV2StoredEvent",
        "1ccd70781bbe5a3ba28c1721a322a72fded1a37b6ecfdb7518d97dcea4952442",
    ),
    (
        "OrchestrationV2StoredEventJson",
        "acd46e2c10518c70f0b6000353558f6e98274bccbe5b4cfc3eea01ccc4e6b5a2",
    ),
    (
        "OrchestrationV2StreamingCapabilities",
        "7603c7d1048d10478b905a938d787c522374cfc53f7a9c1d068736ea45f3d046",
    ),
    (
        "OrchestrationV2Subagent",
        "3ff1a19b284e15a490137758e76a89d811594e677168bca73e4907eb5f9fdaa4",
    ),
    (
        "OrchestrationV2SubagentCapabilities",
        "b29024305f480ab88872c2e5cfc4d5c72db88558e054533f88e32a3ce5cd11fb",
    ),
    (
        "OrchestrationV2SubagentJson",
        "3ff1a19b284e15a490137758e76a89d811594e677168bca73e4907eb5f9fdaa4",
    ),
    (
        "OrchestrationV2SubscribeShellInput",
        "41b5976af834e0850103a6fcef45d2c30174cf5b4265d0fab6e2cc7ebedcdc9f",
    ),
    (
        "OrchestrationV2SubscribeThreadInput",
        "1d98b243ee1ec682013479d465296298891169e43ee66821798aa5eb51c3b619",
    ),
    (
        "OrchestrationV2ThreadBoundedSnapshot",
        "17b76c269cd8e5dbb62429324c6d77b2cb3125d951c0faa35dab9abe10fab5c5",
    ),
    (
        "OrchestrationV2ThreadCapabilities",
        "e9d1aa435db81714b9e9001c547874d3507f09347f5c4a448c60f8b4cad735ce",
    ),
    (
        "OrchestrationV2ThreadDetailSnapshot",
        "bb2fd9cb15550b46b119cb54db58fd70b7ebf76cb066a6ac8f691ff5c858c6e0",
    ),
    (
        "OrchestrationV2ThreadForkSourcePoint",
        "2d76e7e490cb8d75e23a2f71f6832d69c33c2eca8961c6288753f363d1d4a5ab",
    ),
    (
        "OrchestrationV2ThreadHistoryOrigin",
        "38665728389fa08394b600afcd2a4f350173d9d112cad4b3ca3ad3268633d757",
    ),
    (
        "OrchestrationV2ThreadHistoryPage",
        "a4b72ab047ba219bc089f3c64a1405314cdaef701091bcde2fbe75ccc2299d5e",
    ),
    (
        "OrchestrationV2ThreadLaunchError",
        "4dfa2a73a0322889eebab33e46fd104301f2e01203891744391a302954283c2b",
    ),
    (
        "OrchestrationV2ThreadLaunchInput",
        "603e60b8a3865e1730ce8919e42b61fc27f8e7bcda80b5308d74c6d1e701d5d1",
    ),
    (
        "OrchestrationV2ThreadLaunchResult",
        "9ca7ecf3e2cbe402a8809aeae52cb1f8049180452bfdefb6d6ade89e330fa311",
    ),
    (
        "OrchestrationV2ThreadLaunchWorkspaceStrategy",
        "b307c3d5382c2cd9b9d32ba766d4c1b0aaf5323f60d627b9b29e5f93758a514b",
    ),
    (
        "OrchestrationV2ThreadProjection",
        "fad883840cb897f9e74a8b2d66dededdcf4b73bef53b0b708b26456e354edd51",
    ),
    (
        "OrchestrationV2ThreadProjectionJson",
        "ec459b65e3d513c46d04e8e3dabb196b6176d600d1ba8741eec34bccc31cc7b8",
    ),
    (
        "OrchestrationV2ThreadShell",
        "26612206fea4530dd6ea55aa18264c6a1fb99a724144d1ff40d37d41dc5f7c99",
    ),
    (
        "OrchestrationV2ThreadShellJson",
        "62a65ac384356fd5d07d46f625b8448ef2071046749b2f9e01705a6edc5bd0f7",
    ),
    (
        "OrchestrationV2ThreadShellSnapshot",
        "5c13ff249d758a20ad41ab0b5d78c77417a18db770f5e28bbf2492366aba20cd",
    ),
    (
        "OrchestrationV2ThreadStreamItem",
        "28f86bf8bf67a77bd945af4ff8d2b3531d11c4f8c235d8069f10e733c984cbb7",
    ),
    (
        "OrchestrationV2ToolCapabilities",
        "ed55108234112e6d433d894fb532fdbac4cf900fea41925bad8662c998b27efd",
    ),
    (
        "OrchestrationV2TurnCapabilities",
        "6668d2402ce2909616ce8120890858366bd08914919e026a0fe111742fe0cfc3",
    ),
    (
        "OrchestrationV2TurnItem",
        "b046954f05440f84a69257fb441698296c07469a19c19a1e94899ee4624fa371",
    ),
    (
        "OrchestrationV2TurnItemJson",
        "b046954f05440f84a69257fb441698296c07469a19c19a1e94899ee4624fa371",
    ),
    (
        "OrchestrationV2TurnItemStatus",
        "099ee182a1b43715943c159d9f2a37073e5d6ceb32f58515ae0a23a1ed698162",
    ),
    (
        "OrchestrationV2UserInputQuestion",
        "5c501214c39fd8829364e771f5887d7e3d7741b6335c6d4a2744b9b3c2a239a5",
    ),
    (
        "OrchestrationV2UserMessageInputIntent",
        "3d25ba6ea9cdd0cd6a8596785db5fbf478366d374109caae27fb5b5706ef6fda",
    ),
    (
        "OrchestrationV2WebSearchResult",
        "0a132e0295d1d487bffa547ec1c4aca9f9eb16b8871f152c44e5fa2a622128b9",
    ),
    (
        "OrchestratorCapabilitiesError",
        "029ec0b2d47ec1133a238247d5b48a4733fdf689b727aaad538947b905fe9040",
    ),
    (
        "OrchestratorCapabilitiesInput",
        "3e7cd0c7da63c31f23d31e81653eeb201e43a51bae91d944318c3caf8172d41e",
    ),
    (
        "OrchestratorCapabilitiesResult",
        "019a37e48563679c3ec4c005f4e497e7dcc188ea93f52c5ef476d9c25839eb03",
    ),
    (
        "OrchestratorMcpCapabilitiesResult",
        "0c000514ba0ea0e17e2b75f5c1a90e74d187ddad0292329bca59d86d6acc8069",
    ),
    (
        "OrchestratorMcpCreateThreadRequest",
        "7f2d6bf46f4cba3a5e03b2234d7f141739607d01784e12735fe5942b4295e9cf",
    ),
    (
        "OrchestratorMcpCreateThreadsInput",
        "8a63daa917bd9cefcaaca22d8dd8a9b20862612440c89949730cc8060dc4d204",
    ),
    (
        "OrchestratorMcpCreateThreadsResult",
        "f08f2977d31818de7b551d80885fc082cef03ff42ea335a6ea9118b01d45e585",
    ),
    (
        "OrchestratorMcpCreatedThread",
        "1daf0e1deefb20dba2af78079da5c7ab0fbd827631a22c566cfc32df3f73a496",
    ),
    (
        "OrchestratorMcpCreatedThreadStatus",
        "bfa89ddbbe88b3cb7e703e579f300b39b977f329f5bb1b64cccf017765582833",
    ),
    (
        "OrchestratorMcpDelegateTaskInput",
        "2ec85f96741682229775b15769a907f41ba8efc81b672d330fa66e26c4590d4f",
    ),
    (
        "OrchestratorMcpDelegateTaskResult",
        "8d5525295a303182a86e7016d4808441ffdeddbc8c73521100f7c8162d825a5c",
    ),
    (
        "OrchestratorMcpDelegatedTaskStatus",
        "dd5052ee992e40300c9cc2b7d3f1f668c5e9198529e5263e7a45014a889b2ccd",
    ),
    (
        "OrchestratorMcpDeleteScheduledTaskInput",
        "3379b2fd2055999a7599774fc9417c770864c841f1b9e42240475b535d8af85f",
    ),
    (
        "OrchestratorMcpDeleteScheduledTaskResult",
        "1b5e307f434f088c38f4868cb6fbeab188d0cb57afa56498aa726ddcc1e53308",
    ),
    (
        "OrchestratorMcpFailure",
        "90d21741c3dea4271fbdb84b58e1715d846cb4c32c81b6453b54386684339a51",
    ),
    (
        "OrchestratorMcpInteractionMode",
        "aa90de0a1db92d4557cea6ee317031e678960b78841b044bda0911209bcd04da",
    ),
    (
        "OrchestratorMcpListScheduledTasksResult",
        "aac45c644b5073994df6471db41d33f8d7b03f359782ef334a57edd51ea0511c",
    ),
    (
        "OrchestratorMcpProviderCapability",
        "1263f4157fdc81fbd68e7f002446f884a5c5fc46029a94a464eca17b0ed3ce85",
    ),
    (
        "OrchestratorMcpRuntimeMode",
        "08aebb96ffa9ed199e972db094ac69d23657ec6da2d526eb9e59af494eaec1c8",
    ),
    (
        "OrchestratorMcpScheduleTaskInput",
        "b9d743a6b34af0c4d71c2223b2f1ae69d3a3224521a05f7062bde35557e6d1ed",
    ),
    (
        "OrchestratorMcpScheduleTaskResult",
        "3eae0bdd6d87b02b4661c3dfdcb7fd0ec3d2579af59eed74bf78fe5a402b77a5",
    ),
    (
        "OrchestratorMcpScheduledTask",
        "8961e9c29cc34b54d79fc2680ef1282a86ada704d7cdae2738b015f48cdfc79a",
    ),
    (
        "OrchestratorMcpTarget",
        "464fc43001dbc8bc1c967503e6971ae57065fa627501beabf2521d99286c84ad",
    ),
    (
        "OrchestratorMcpTargetOptions",
        "3775d5a2ecd86c5b17283e4618dc39927eaca5e55fe432103374d2f78a49a71a",
    ),
    (
        "OrchestratorMcpTaskCancelInput",
        "ae4e0939e27594939779b5211dd6db303057a14faa2ec08666e458c0fa68868f",
    ),
    (
        "OrchestratorMcpTaskCancelResult",
        "af7b7b3f1cce5a7e1ca2ec46d8dab1cfeeef391af99bdce7888fd6b0adc3d963",
    ),
    (
        "OrchestratorMcpTaskRole",
        "9e3ae37a8484520a8f4384d48ec3cd4d3e2b3247cd4520e5f12975d637e8365e",
    ),
    (
        "OrchestratorMcpTaskStatusInput",
        "78374b124ddc0d93ff2d82b7c856a53d48e4ec02bbaee210a29dffd628b91849",
    ),
    (
        "OrchestratorMcpTerminalDelegatedTaskStatus",
        "3cf6f7ca63606fe53d8e101ec895b58b439ce87485b126f14cbd3181897e3cba",
    ),
    (
        "OrchestratorMcpThreadDetail",
        "cc67a6a4e0701ba6d9639cd3c131ad08b933eadd6ddb35213f94adfea5cf0258",
    ),
    (
        "OrchestratorMcpThreadInterruptInput",
        "b190aa4ffcb9de231ebb8b909806015ebd3720becb9f913a1ddb9ce60ac2ef07",
    ),
    (
        "OrchestratorMcpThreadInterruptResult",
        "d4d8beeb00728f6e83855f4c6becc4ab195f875fd843187737d775c2ee1e00e3",
    ),
    (
        "OrchestratorMcpThreadListInput",
        "87383792c2e2409d831d11ad6bc882705732eb749e4ea52979eb7e2ffabf2b40",
    ),
    (
        "OrchestratorMcpThreadListItem",
        "59d882f65d68c741ee9695675ec74e5d49d36f805df48bd473d0b9c120d2d9c3",
    ),
    (
        "OrchestratorMcpThreadListResult",
        "3323efc8211a2b59bf6e9c655e1e714a5ee7d49675f7a227992a840dcea00847",
    ),
    (
        "OrchestratorMcpThreadReadInput",
        "b9f3c4a58a05f5fda1502feceb3a4b35aff9930a60d5a6fd15ce92eb32a0250f",
    ),
    (
        "OrchestratorMcpThreadReadResult",
        "2bd95605e0abeb6c057b077b50842118b03d92dd348b7ec3fdc0b8861a6fbf12",
    ),
    (
        "OrchestratorMcpThreadRun",
        "3cfdf8677f7fbe0a3143b354a9015c550de5069f7abd20dcb9ba84eb1f0223c2",
    ),
    (
        "OrchestratorMcpThreadSendInput",
        "9e745611822940104fba8047fe02bd6d05e9b8724ec9e4c7fcfe21b31ea5ef22",
    ),
    (
        "OrchestratorMcpThreadSendResult",
        "ca679b3b787fcb1476ececd3265ae94c20af0347826f231db444e2356b564bea",
    ),
    (
        "OrchestratorMcpThreadStatus",
        "73e9f0c03bab54c2fa9e55f01467e5fec63d1fb6cba4315af7cfc72338d5619c",
    ),
    (
        "OrchestratorMcpThreadTimelineItem",
        "3df6c64f8bfbaf195ded4cd1bf2b6492d5030062b45b257fe628df2e690a60ed",
    ),
    (
        "OrchestratorMcpThreadWaitInput",
        "68fdc3e747cf9c911c113a5070fec07fe07cd081d0fd6cd6b84f7ac3de7a2112",
    ),
    (
        "OrchestratorMcpThreadWaitResult",
        "7cd7b8ee8fd8f0d6eaa59b09cee5dfb6f3430ca0ebf2f1a2855652ddfb1ad4e1",
    ),
    (
        "OrchestratorMcpUpdateScheduledTaskInput",
        "aab7a4a0c5b2506e692a07df8e6a26ee452f2ff7a6e10978e8165dc0b932787a",
    ),
    (
        "PastedTextAttachmentSource",
        "803f0a8493a7355665292d5a63dc5d219f9c5f558e0676c268d21b3c8878922d",
    ),
    (
        "PlanId",
        "9bddc1fdfb55932e41cf34e4a0bf062564cf848d3e0070d86dcd3a998fdce008",
    ),
    (
        "PositiveInt",
        "96de37a86fda3c92e55456d8471fb3536852eab7757820b7d1383f22f012a501",
    ),
    (
        "PreviewAnnotationContextRecord",
        "3e54f553e904415b31a169e9b8590c560a9de17cbd634442d7ce36379e416a9e",
    ),
    (
        "PreviewAutomationActionEvent",
        "f5b2c09614e73450039e91a800345ad84af4e2f47d84fc78357564821c3ea501",
    ),
    (
        "PreviewAutomationClientDisconnectedError",
        "ba3e5418ffb70db7948f57a9a168c2eca529b860c5fc02053d67e51d5e308b46",
    ),
    (
        "PreviewAutomationColorScheme",
        "0a4ebb1af69c162041f6c9dbaf17b0fdcacb61075e18b214277b7355f098bf49",
    ),
    (
        "PreviewAutomationConnectionId",
        "9bddc1fdfb55932e41cf34e4a0bf062564cf848d3e0070d86dcd3a998fdce008",
    ),
    (
        "PreviewAutomationConsoleEntry",
        "65bc5545c6277b85071c0c5ef247788071b89adb623f1d76ad6df0facb732564",
    ),
    (
        "PreviewAutomationControlInterruptedError",
        "281e054a7b44fc44e4b079fd5da7bc72d8e1d6a7cf62e738273e392118b50fdb",
    ),
    (
        "PreviewAutomationElement",
        "d5d3dcd4d7c5977e8726db48e0863ba4b12ba4296dae29e19dbd0b0644406ac7",
    ),
    (
        "PreviewAutomationError",
        "7255ddfaa26e36eb6f5f3936a9a108e15aecee0175aa7cb9da8b7a2fc8587b73",
    ),
    (
        "PreviewAutomationExecutionError",
        "45ca316f572c99abc9d7a1d5c74bf92b7da5143bedeb3475ecd3480559eab067",
    ),
    (
        "PreviewAutomationInvalidSelectorError",
        "b33284e76c85b18c84c7bc04db999af7b3594dba50f4aeb653d80cffff3fcefa",
    ),
    (
        "PreviewAutomationMalformedResponseError",
        "baa200e7e3ab0694505e2ba3c06dd6fd80ae84c928b9b60f8c2c5ca0c05abe9b",
    ),
    (
        "PreviewAutomationNetworkEntry",
        "a36c3bebbe17d82ed86e6113c3ca48bfe14fa61210a3bab0740afadfa64a62ff",
    ),
    (
        "PreviewAutomationNoAvailableHostError",
        "9d6e9bd1ae9830ecea2a23facb79ea1c401ee74316631365da889fdf038cf4e7",
    ),
    (
        "PreviewAutomationOperation",
        "128ad4dc067786bff4fc516d902f4ef92b0f6b4a3e086fd30f757759b8c0f969",
    ),
    (
        "PreviewAutomationRecordingDeadlineExpiredError",
        "2046a542b471531094b4a9a069a4bc45827b427d00c654321ddd90a619cf1214",
    ),
    (
        "PreviewAutomationRecordingDesktopUpdateRequiredError",
        "8120353223033dd8d3abc231d4cc7d8e2e018c4f66831551e2a289e7ef43a1fd",
    ),
    (
        "PreviewAutomationRecordingTooLargeError",
        "96f1e2246ea7489a1f59dcabf82a1c63692c4eda09dddf78d48ece62e576064d",
    ),
    (
        "PreviewAutomationRecordingTransferError",
        "286c0f38dea74991c845ae2094166aa36530a527342be58be16b05239ada40bc",
    ),
    (
        "PreviewAutomationRemoteUnavailableError",
        "2c325c9d4607e5b5d1f7517423c81248b8e33b028c5e742f2bad9a31c89f00a8",
    ),
    (
        "PreviewAutomationRequestQueueClosedError",
        "fef3bd7519f6416f8d52773c96e54ccaadab28ae13f30eaef60343e275588c0e",
    ),
    (
        "PreviewAutomationResultTooLargeError",
        "57c5a4d558c420f6109ac552958a11baceb1306129542d65e4989b65f9412ed2",
    ),
    (
        "PreviewAutomationSnapshot",
        "8da477044ef7d63428712a66d87d9e5f9c1ae8cc90d3d5b6e9ed606460d607d9",
    ),
    (
        "PreviewAutomationStatus",
        "21ede6a215be28b745fd475866eb73bef58430d5e9270e27fed3a8ef534c9609",
    ),
    (
        "PreviewAutomationTabNotFoundError",
        "c61acb37b41b58eb6cba4eec5ebd05656893ae69157b29375ae797600284bc7a",
    ),
    (
        "PreviewAutomationTargetNotEditableError",
        "d2153a26bae98b8739117762e8db1f2917141677d566ec89f1dc32467f3f7931",
    ),
    (
        "PreviewAutomationTimeoutError",
        "6b52733ddf2a08fe46f040061c500d162e26f459848d814f663809dc75da918f",
    ),
    (
        "PreviewAutomationUnavailableError",
        "7fe1937285ef35e979bf91866c7dc21d1a71fc814ef0836f7722cfc1547b32c1",
    ),
    (
        "PreviewAutomationUnsupportedClientError",
        "1cdd3ea603467915ff64a08116690beb7cf51ca8047fb08ebcb2e99695122fc1",
    ),
    (
        "PreviewClickError",
        "f94199d44e7a4b6d7defdbea31c1ee38b5ec51ef7b83c903f222f7028bab6cc4",
    ),
    (
        "PreviewClickInput",
        "ca3fd60245a440ad21d61855cdc3a6e4048d9db5832b66268debefa8dd905f1b",
    ),
    (
        "PreviewClickResult",
        "b75515c883789a7c60ed1b3724fd7bb470dbd05a0571e4503b35e6fa9801d4b6",
    ),
    (
        "PreviewEvaluateError",
        "f94199d44e7a4b6d7defdbea31c1ee38b5ec51ef7b83c903f222f7028bab6cc4",
    ),
    (
        "PreviewEvaluateInput",
        "ae6536d27470a46f354d0151cee710268581a511c8b8fd4275e7edc9e27d50c1",
    ),
    (
        "PreviewEvaluateResult",
        "24c587c0cd781abca96a0db228a14aed56ef7e242b4aee5c37789359427d3e8b",
    ),
    (
        "PreviewNavStatus",
        "c688cf4df7e23d7a9ee10ceeac30cc84d7cfaf0526f09928f168d167a910f767",
    ),
    (
        "PreviewNavigateError",
        "f94199d44e7a4b6d7defdbea31c1ee38b5ec51ef7b83c903f222f7028bab6cc4",
    ),
    (
        "PreviewNavigateInput",
        "98436d0edf5441e2110c635f08299aa3c95ae13e9149a30815298cd86a76ba80",
    ),
    (
        "PreviewNavigateResult",
        "656b8e9b3f9609f2861aae056ad79ccb2b5a370daa5f16cb12d7f6c4d519ec2b",
    ),
    (
        "PreviewOpenError",
        "f94199d44e7a4b6d7defdbea31c1ee38b5ec51ef7b83c903f222f7028bab6cc4",
    ),
    (
        "PreviewOpenInput",
        "cd8185ebbe4fd9692c88332fc0a85620e2dbd770891b53a34af84f4878b6b9a7",
    ),
    (
        "PreviewOpenResult",
        "656b8e9b3f9609f2861aae056ad79ccb2b5a370daa5f16cb12d7f6c4d519ec2b",
    ),
    (
        "PreviewPressError",
        "f94199d44e7a4b6d7defdbea31c1ee38b5ec51ef7b83c903f222f7028bab6cc4",
    ),
    (
        "PreviewPressInput",
        "3b77d5f58e5f520c35e88ff691566828b4593b6ea4c9959683a6e40a2684fa5b",
    ),
    (
        "PreviewPressResult",
        "b75515c883789a7c60ed1b3724fd7bb470dbd05a0571e4503b35e6fa9801d4b6",
    ),
    (
        "PreviewRecordingStartError",
        "f94199d44e7a4b6d7defdbea31c1ee38b5ec51ef7b83c903f222f7028bab6cc4",
    ),
    (
        "PreviewRecordingStartInput",
        "21ccee1123214bd41193ae3dbabb61e94624ab9c037d665838572519b6797b2c",
    ),
    (
        "PreviewRecordingStartResult",
        "c19ce175f0bad59d888be994bc965c9c07cae0033daebd65fbbfd8b26e9d4226",
    ),
    (
        "PreviewRecordingStopError",
        "f94199d44e7a4b6d7defdbea31c1ee38b5ec51ef7b83c903f222f7028bab6cc4",
    ),
    (
        "PreviewRecordingStopInput",
        "21ccee1123214bd41193ae3dbabb61e94624ab9c037d665838572519b6797b2c",
    ),
    (
        "PreviewRecordingStopResult",
        "51f0290d48851c20fdf7ac98ebdd44f0ba009921c15185d773f8d3e3f303f4d6",
    ),
    (
        "PreviewRenderedViewportSize",
        "a1324a3bba34781ff714a7d5cb92bdd89dee6a5140b140c6fea62dfbb2e3eecb",
    ),
    (
        "PreviewResizeError",
        "f94199d44e7a4b6d7defdbea31c1ee38b5ec51ef7b83c903f222f7028bab6cc4",
    ),
    (
        "PreviewResizeInput",
        "195f2ce8138bc0eebbca6c949198e614be2c0eb13c86aee3142b3054080abe8f",
    ),
    (
        "PreviewResizeResult",
        "3a5963fc4c7a151fc8da1f3677b087034492a46fe689c149f0c2d2b180856545",
    ),
    (
        "PreviewScrollError",
        "f94199d44e7a4b6d7defdbea31c1ee38b5ec51ef7b83c903f222f7028bab6cc4",
    ),
    (
        "PreviewScrollInput",
        "4470b1858cbc58d8edb6cb240c23ea4abc276c9179505a640c4aed7f160ec3b0",
    ),
    (
        "PreviewScrollResult",
        "b75515c883789a7c60ed1b3724fd7bb470dbd05a0571e4503b35e6fa9801d4b6",
    ),
    (
        "PreviewSessionSnapshot",
        "e87e60a13d7333f4143a620dc5efca982fdbd9cd7f4f82ffc81102133f18e27f",
    ),
    (
        "PreviewSetAppearanceError",
        "f94199d44e7a4b6d7defdbea31c1ee38b5ec51ef7b83c903f222f7028bab6cc4",
    ),
    (
        "PreviewSetAppearanceInput",
        "2d467e3d605ea4857958e2c1f4ac1027aa3708661eebb56dfba7453d2fa7da44",
    ),
    (
        "PreviewSetAppearanceResult",
        "2b586a53577753f433b53be68c010ab872d1c4504bb356c10a5ea88780e539c1",
    ),
    (
        "PreviewSnapshotError",
        "f94199d44e7a4b6d7defdbea31c1ee38b5ec51ef7b83c903f222f7028bab6cc4",
    ),
    (
        "PreviewSnapshotInput",
        "65f7d002a2b83fd8eed8fce4539fb1a9ee050a8d068d334842488f2938fd779c",
    ),
    (
        "PreviewSnapshotResult",
        "7d757136793950f1ca360cbbe220d2d990cde80191fc1147dd7d5a99fb7fe197",
    ),
    (
        "PreviewStatusError",
        "f94199d44e7a4b6d7defdbea31c1ee38b5ec51ef7b83c903f222f7028bab6cc4",
    ),
    (
        "PreviewStatusInput",
        "21ccee1123214bd41193ae3dbabb61e94624ab9c037d665838572519b6797b2c",
    ),
    (
        "PreviewStatusResult",
        "656b8e9b3f9609f2861aae056ad79ccb2b5a370daa5f16cb12d7f6c4d519ec2b",
    ),
    (
        "PreviewTabId",
        "9bddc1fdfb55932e41cf34e4a0bf062564cf848d3e0070d86dcd3a998fdce008",
    ),
    (
        "PreviewTypeError",
        "f94199d44e7a4b6d7defdbea31c1ee38b5ec51ef7b83c903f222f7028bab6cc4",
    ),
    (
        "PreviewTypeInput",
        "41bf297e8b195c9f942a7a9ce69cdb7b8ea76292a5c03e3e789628df68bc177c",
    ),
    (
        "PreviewTypeResult",
        "b75515c883789a7c60ed1b3724fd7bb470dbd05a0571e4503b35e6fa9801d4b6",
    ),
    (
        "PreviewViewportSetting",
        "dfb338472ece3aa0bb6f4b037401cf327cbd610dbba2b21647d327d114d01e9b",
    ),
    (
        "PreviewWaitForError",
        "f94199d44e7a4b6d7defdbea31c1ee38b5ec51ef7b83c903f222f7028bab6cc4",
    ),
    (
        "PreviewWaitForInput",
        "cd3a59b1b41d678807dc72711f1ad0b2a1367eae91dedac8aaf8b1416c242f8e",
    ),
    (
        "PreviewWaitForResult",
        "b75515c883789a7c60ed1b3724fd7bb470dbd05a0571e4503b35e6fa9801d4b6",
    ),
    (
        "Project",
        "dd0b4e0f3dc15ba1c4e0283abbf886ead92b92670b353d1da8ac6849db81fa44",
    ),
    (
        "ProjectIconColor",
        "d7ab0a0e9ac6aa456fc316ca88068d61b3655a27364c5df78be29307fdf8d70e",
    ),
    (
        "ProjectIconOverride",
        "12c4ed71ce86175768d1b3c929e007909cf2b231e0225729a5f856a4e61e3b8a",
    ),
    (
        "ProjectId",
        "9bddc1fdfb55932e41cf34e4a0bf062564cf848d3e0070d86dcd3a998fdce008",
    ),
    (
        "ProjectMonogramText",
        "9bddc1fdfb55932e41cf34e4a0bf062564cf848d3e0070d86dcd3a998fdce008",
    ),
    (
        "ProjectScript",
        "774531bf22a458837763f6ea1023f525321ba56cf4e03af7d1bdaedf9483f130",
    ),
    (
        "ProjectScriptIcon",
        "9cc89c4b17a812638537df0f864bcabb9df2be80929ed3ab3cbc82883316d354",
    ),
    (
        "ProviderApprovalDecision",
        "1569072318e06c3ee07150dbd5fdaf8b12934174813003b1400659b5c7abcb6d",
    ),
    (
        "ProviderApprovalOption",
        "99a530de925414e3741951d4723e44ffa721070296f599cb9976a5d00c98b214",
    ),
    (
        "ProviderDriverKind",
        "9bddc1fdfb55932e41cf34e4a0bf062564cf848d3e0070d86dcd3a998fdce008",
    ),
    (
        "ProviderInstanceConfig",
        "c90be9253d18eb39234cd6bcd161fc6e80f0862f0611244a6a2cf69fb6eba665",
    ),
    (
        "ProviderInstanceConfigMap",
        "639426285a7d2fd98c56e56cc82e096445243b58ad472ee9ca392b4d7ee15198",
    ),
    (
        "ProviderInstanceEnvironment",
        "7d6fa510430efbb43e0a04236063fed1a535cf969e3f719f0d34fe01cbc604b6",
    ),
    (
        "ProviderInstanceEnvironmentVariable",
        "6b3e52dc520ba31f97d4e6b586da86d243d15ec4f251fddf7ca8df882ad84d7b",
    ),
    (
        "ProviderInstanceEnvironmentVariableName",
        "9bddc1fdfb55932e41cf34e4a0bf062564cf848d3e0070d86dcd3a998fdce008",
    ),
    (
        "ProviderInstanceId",
        "9bddc1fdfb55932e41cf34e4a0bf062564cf848d3e0070d86dcd3a998fdce008",
    ),
    (
        "ProviderInstanceMutation",
        "58ef020b82fd5021f63ea907a54c3327c5721632b252e559973a8570d85af112",
    ),
    (
        "ProviderInstanceRef",
        "de7345ec3f6f5b81740ebe4d645c22a09a0eed62fde6bb38eba9b0a92061d2b6",
    ),
    (
        "ProviderInteractionMode",
        "dde251aac011a422167853a0a59fbbd624072d33242eae8f2b9c6a842e09b0fa",
    ),
    (
        "ProviderOptionChoice",
        "7f4314ea89935b38f427e10ef76071f1c19f31a640071aca61f47a795d31f2e7",
    ),
    (
        "ProviderOptionDescriptor",
        "fd38919f522d91c782eadce6534d1565870cf09be43d8ad3c2a3d2ecb1116440",
    ),
    (
        "ProviderOptionDescriptorType",
        "8c0a0a319068dcf5fd533c90238c03ed6b467ded57462caae1b572bb737d5407",
    ),
    (
        "ProviderOptionSelection",
        "02ba0f9b0a85211f50f64f493737e424be917f19c1ce3d11361e0907f321a30b",
    ),
    (
        "ProviderOptionSelectionValue",
        "9a16340ddc3384b030ca54567590fe69371da4222944b6523c55561e4868cffa",
    ),
    (
        "ProviderOptionSelections",
        "3775d5a2ecd86c5b17283e4618dc39927eaca5e55fe432103374d2f78a49a71a",
    ),
    (
        "ProviderReplayEntry",
        "e42ec105e5858e41106e8779cac097a8e3233e77413f4e5a7e779d2da852402e",
    ),
    (
        "ProviderReplayNdjsonRecord",
        "0b9ccb9547dd19fae059dbe2dc0581435af5df81dec899b3729bf1c9383e256b",
    ),
    (
        "ProviderReplayTranscript",
        "1519027c5140e0f4476053fda7233b3acef315ecebcafff70553bd4f97f34d1b",
    ),
    (
        "ProviderReplayTranscriptHeader",
        "d2d9803ca784a50fb70b7447b730c09883c5d7a370a7d160f9020524a217b6dd",
    ),
    (
        "ProviderRequestKind",
        "69fc11f3c72fac2d3b328b6a156e0f9882299224a05f9c241a20648f49407e06",
    ),
    (
        "ProviderSessionId",
        "9bddc1fdfb55932e41cf34e4a0bf062564cf848d3e0070d86dcd3a998fdce008",
    ),
    (
        "ProviderThreadId",
        "9bddc1fdfb55932e41cf34e4a0bf062564cf848d3e0070d86dcd3a998fdce008",
    ),
    (
        "ProviderTurnId",
        "9bddc1fdfb55932e41cf34e4a0bf062564cf848d3e0070d86dcd3a998fdce008",
    ),
    (
        "ProviderUserInputAnswers",
        "03231190a47a3e4455755ea656d4a83915f7d0c5ad479135a02e79c900ca6552",
    ),
    (
        "PullRequestActor",
        "bd5329d362d5c7a7a6a62dbebab6a4bbea8a430eae1c696b12acebed36c0386a",
    ),
    (
        "PullRequestChecksState",
        "68950495284d6431743fcca915567a83d5e72bbd5b2faad76a92acfee9cbc543",
    ),
    (
        "PullRequestContextMetadata",
        "5ec2ca607094a168a0378a5905b59d55feddd704e0b7f4d5d0b1c68314fd2e1f",
    ),
    (
        "PullRequestMergeability",
        "445293d498a989a2702115afe61306a227abcebfa03d3caf8979f82b9f5642a4",
    ),
    (
        "PullRequestReviewDecision",
        "8a864cc57a49b99cd2045a2e3bb28c1f9a25d13334d8cdd21a568c97f9462565",
    ),
    (
        "PullRequestState",
        "6787d73b8b6814b043c6acaeb2e00cd9352dcdc196e90ad9572a2098a863c019",
    ),
    (
        "RawEventId",
        "9bddc1fdfb55932e41cf34e4a0bf062564cf848d3e0070d86dcd3a998fdce008",
    ),
    (
        "RepositoryIdentity",
        "56670815ce9d79268f8346f329c1948c708647d09a21e2d6e8837b78a5b58b80",
    ),
    (
        "RepositoryIdentityLocator",
        "837d6765d087dc2915d2bdbf83e1e55e0c3d17f53aab00eb9a4407f583723536",
    ),
    (
        "ReviewCommentContextRecord",
        "3896142b70fddcd4f4e6211845257891c3b3e285bad0ddf596f7d91e794d2b53",
    ),
    (
        "RunAttemptId",
        "9bddc1fdfb55932e41cf34e4a0bf062564cf848d3e0070d86dcd3a998fdce008",
    ),
    (
        "RunId",
        "9bddc1fdfb55932e41cf34e4a0bf062564cf848d3e0070d86dcd3a998fdce008",
    ),
    (
        "RunScheduledTaskNowError",
        "029ec0b2d47ec1133a238247d5b48a4733fdf689b727aaad538947b905fe9040",
    ),
    (
        "RunScheduledTaskNowInput",
        "25e6d317cf2cc2bbce6cc1204f59b95ffe60fd6105930a38d64dd4be1a12ec1e",
    ),
    (
        "RunScheduledTaskNowResult",
        "53c876c82e8271376213e40b0794820ccc179dfe081d2963c528474a5a9f67f3",
    ),
    (
        "RuntimeMode",
        "518f800910499351f793ce29268c9f898b3e45d89969b7333522e246018c75bb",
    ),
    (
        "RuntimeRequestId",
        "9bddc1fdfb55932e41cf34e4a0bf062564cf848d3e0070d86dcd3a998fdce008",
    ),
    (
        "ScheduleTaskError",
        "029ec0b2d47ec1133a238247d5b48a4733fdf689b727aaad538947b905fe9040",
    ),
    (
        "ScheduleTaskInput",
        "b7228f3bae1d696eda25fd61dcdd1cca659d67254107cb5a0847915788f1961b",
    ),
    (
        "ScheduleTaskResult",
        "8961e9c29cc34b54d79fc2680ef1282a86ada704d7cdae2738b015f48cdfc79a",
    ),
    (
        "ScheduledTask",
        "6f2c3423995c326ff71be2b733dfd0ddbe4bc036228fd8fd7e484a808d9d30be",
    ),
    (
        "ScheduledTaskDeleteInput",
        "9ebb27ff8b1b50460b7e41e480f85fa9cd5048f11012e614c2ef9c73f4bda403",
    ),
    (
        "ScheduledTaskDeleteResult",
        "9ebb27ff8b1b50460b7e41e480f85fa9cd5048f11012e614c2ef9c73f4bda403",
    ),
    (
        "ScheduledTaskError",
        "0d38273573fe8585a07a8ba0b08412cd9b73ba0479ea8d241e0117de8469b152",
    ),
    (
        "ScheduledTaskId",
        "9bddc1fdfb55932e41cf34e4a0bf062564cf848d3e0070d86dcd3a998fdce008",
    ),
    (
        "ScheduledTaskListInput",
        "80278b7424f70554d467388720b1d2fda798bca4a452ef3d6240cecf5bbd2aff",
    ),
    (
        "ScheduledTaskListResult",
        "c91128507b54f65438dd5bfd5182f02584ada697f81ac70cb9e2fbf69eee545f",
    ),
    (
        "ScheduledTaskMutationResult",
        "3780072eee4fa2f27607238b3c728bdf9f0b74b5267dfd1779c267f405f53c68",
    ),
    (
        "ScheduledTaskRunNowInput",
        "9ebb27ff8b1b50460b7e41e480f85fa9cd5048f11012e614c2ef9c73f4bda403",
    ),
    (
        "ScheduledTaskRunNowResult",
        "3780072eee4fa2f27607238b3c728bdf9f0b74b5267dfd1779c267f405f53c68",
    ),
    (
        "ScheduledTaskRunStatus",
        "0bb536fa0922ae73800b8235aebe40d187dc7012db6e53e2af27f211ea7440a8",
    ),
    (
        "ScheduledTaskSchedule",
        "c483b99441a8a21682a970c326f69f83bec258cbbe0183a8f08d91449bb20d76",
    ),
    (
        "ScheduledTaskSetEnabledInput",
        "ac792c9c95d74155f485d66c8adddb27dd0e20c3b3b0982647a6197f5c196f76",
    ),
    (
        "ScheduledTaskUpsertInput",
        "8dd236ae8e77761351b4263db2688d8696ae49be823979761f4ac6333b12bb75",
    ),
    (
        "ScheduledTaskUpsertSchedule",
        "c483b99441a8a21682a970c326f69f83bec258cbbe0183a8f08d91449bb20d76",
    ),
    (
        "SelectProviderOptionDescriptor",
        "8cc02b3b690e5db4461c17c4a16bfcae37c8abf5847ed786ff2811c2935c6c1c",
    ),
    (
        "SkillContextRecord",
        "655a00513bedbb5ae84ac686e732de0496762664fd88565b932f2aca3b7a6ffa",
    ),
    (
        "SnapShotAccessibility",
        "33f81ac8440051caeac301fd24073d317a4b8567907868ff4fe2d620ef139677",
    ),
    (
        "SnapShotAccessibilityNode",
        "bc3ba52c7bc1d9321e4180b1e985a85e3ea435139189ade9c572028b7011c1c8",
    ),
    (
        "SnapShotSource",
        "26017e01c6bd5dc8af23495affeb4ef545d5dcb7e8e44ba353474da31eb9a1b8",
    ),
    (
        "SourceControlCloneProtocol",
        "85105a53953ceb61174959b49ac377ca787b2cc6ff81f7f801c3536779a6daa3",
    ),
    (
        "SourceControlCloneRepositoryResult",
        "1eb8c31942930c759917f2de39e85be4f93fa556120075f80d7b7aabdef86880",
    ),
    (
        "SourceControlProviderKind",
        "853d4a20e8e4a880b27b10636f810d78162a8925e85c01926beb7b3cf8fc0d03",
    ),
    (
        "SourceControlRepositoryInfo",
        "fa7547e045ac48531ab886870845e2aef69b65ccdc96b65b4f0d4a6250929c85",
    ),
    (
        "T3AttachmentDiscardError",
        "029ec0b2d47ec1133a238247d5b48a4733fdf689b727aaad538947b905fe9040",
    ),
    (
        "T3AttachmentDiscardInput",
        "3a8a93da369803f5ef40b27629ac8abe0da6525c17dab775fd1b3e382dc22d6d",
    ),
    (
        "T3AttachmentDiscardResult",
        "80278b7424f70554d467388720b1d2fda798bca4a452ef3d6240cecf5bbd2aff",
    ),
    (
        "T3AttachmentPrepareUploadError",
        "029ec0b2d47ec1133a238247d5b48a4733fdf689b727aaad538947b905fe9040",
    ),
    (
        "T3AttachmentPrepareUploadInput",
        "62ffc2c4a7b6dee7f686fbbef307c7c5d1f5a17b067ecb0144db03eac46a025f",
    ),
    (
        "T3AttachmentPrepareUploadResult",
        "2ce786bec1e74db26d27c45dc3f456e9f12208e6d0f1c0d37779a65e6de8a195",
    ),
    (
        "T3EnvironmentPreferencesUpdateError",
        "029ec0b2d47ec1133a238247d5b48a4733fdf689b727aaad538947b905fe9040",
    ),
    (
        "T3EnvironmentPreferencesUpdateInput",
        "e89e1660abf4d32ee17538998e86db5508ba6a67de00bc807081720e088248f9",
    ),
    (
        "T3EnvironmentPreferencesUpdateResult",
        "eb5fa508367a91564860ca7a5d1160faa92275edb54bf954c4cbe94df8a71ef2",
    ),
    (
        "T3EnvironmentReadError",
        "029ec0b2d47ec1133a238247d5b48a4733fdf689b727aaad538947b905fe9040",
    ),
    (
        "T3EnvironmentReadInput",
        "3e7cd0c7da63c31f23d31e81653eeb201e43a51bae91d944318c3caf8172d41e",
    ),
    (
        "T3EnvironmentReadResult",
        "b72d6dbdf6669df79930a6e491c78532b1a21a447cb1894e8d44450c6fb6bf31",
    ),
    (
        "T3PendingRequestListError",
        "029ec0b2d47ec1133a238247d5b48a4733fdf689b727aaad538947b905fe9040",
    ),
    (
        "T3PendingRequestListInput",
        "51f42ece1937a449c913a32e28ad8e73c7f4a63529b364a9f64d2a069fc2297b",
    ),
    (
        "T3PendingRequestListResult",
        "157172d50aa731cef8b8695724414d1e3e4b8b93976f2f5b874bdbc776e66dab",
    ),
    (
        "T3PendingRequestReadError",
        "029ec0b2d47ec1133a238247d5b48a4733fdf689b727aaad538947b905fe9040",
    ),
    (
        "T3PendingRequestReadInput",
        "7c338822530c4418a1c4a57f90405a4a4c1d60d07a70cefe1f27bf1b0cf3dcae",
    ),
    (
        "T3PendingRequestReadResult",
        "672aa3330ea975c658f15c3bfdc15c01a419be6d205e3f01ef655cc67057787f",
    ),
    (
        "T3PendingRequestRespondError",
        "029ec0b2d47ec1133a238247d5b48a4733fdf689b727aaad538947b905fe9040",
    ),
    (
        "T3PendingRequestRespondInput",
        "a3213163df6048c8ea11f5fd8c0e52d709d07301c11bc812910b7832c91d1c8d",
    ),
    (
        "T3PendingRequestRespondResult",
        "7df2b694de677acf09176580adfa2937d5a2b266103ef9350f3fd8c8140c11d4",
    ),
    (
        "T3PreviewCloseError",
        "6f65cfdcb32cabafe9bc08e3b6c5391602e7df87b455900e6b139f0dee6a5b96",
    ),
    (
        "T3PreviewCloseInput",
        "1537544f7cff90108d3f64bd0a1e2c9f80b3b038c3b84933ba4472bc940fd0fe",
    ),
    (
        "T3PreviewCloseResult",
        "80278b7424f70554d467388720b1d2fda798bca4a452ef3d6240cecf5bbd2aff",
    ),
    (
        "T3PreviewListError",
        "6f65cfdcb32cabafe9bc08e3b6c5391602e7df87b455900e6b139f0dee6a5b96",
    ),
    (
        "T3PreviewListInput",
        "6d446bc77e4dbe9475d6c4ccc14da0f5df4efcdfd2f7ad393837376979c1847d",
    ),
    (
        "T3PreviewListResult",
        "b15862eb535cd7d60c9caa0860970c8bf7ec7a41333c767429565f9ab1336c4e",
    ),
    (
        "T3ProjectCloneError",
        "029ec0b2d47ec1133a238247d5b48a4733fdf689b727aaad538947b905fe9040",
    ),
    (
        "T3ProjectCloneInput",
        "e5aab11ee6610c193252b5ccd38d5366b67ea13bd34d5294f6f00d42fad41c91",
    ),
    (
        "T3ProjectCloneResult",
        "ce4b704de0306b326ff7494b9cbd6c2d9fcb03b633d5dcf086a8d7689f6e17b5",
    ),
    (
        "T3ProjectCreateError",
        "029ec0b2d47ec1133a238247d5b48a4733fdf689b727aaad538947b905fe9040",
    ),
    (
        "T3ProjectCreateInput",
        "f362253cfcbef53617396c367bf4fe16eac1f1c21fca8367c3fcecef43d6a1c4",
    ),
    (
        "T3ProjectCreateResult",
        "12a23e3a5c663a84ccafa54f523722ee02ba43014476d68aad064787e502716e",
    ),
    (
        "T3ProjectDeleteError",
        "029ec0b2d47ec1133a238247d5b48a4733fdf689b727aaad538947b905fe9040",
    ),
    (
        "T3ProjectDeleteInput",
        "4fe97ddd929926bb9ec6a36265da8be8a2bf644e09c38cc14368a50c52e73063",
    ),
    (
        "T3ProjectDeleteResult",
        "0f2c39f8e77c9be5fbb79511204895a9e9b7290a3e401750f7147a0093b4376f",
    ),
    (
        "T3ProjectListError",
        "029ec0b2d47ec1133a238247d5b48a4733fdf689b727aaad538947b905fe9040",
    ),
    (
        "T3ProjectListInput",
        "6d446bc77e4dbe9475d6c4ccc14da0f5df4efcdfd2f7ad393837376979c1847d",
    ),
    (
        "T3ProjectListResult",
        "84aefa35b1dc7b2702197abe3a323c96c861e6c8be7b6bb153765f160d62a69d",
    ),
    (
        "T3ProjectReadError",
        "029ec0b2d47ec1133a238247d5b48a4733fdf689b727aaad538947b905fe9040",
    ),
    (
        "T3ProjectReadInput",
        "1c8d8686e291a9535cc0a9744b262f4b695f9b5b73a048e21fcf4c01956096d9",
    ),
    (
        "T3ProjectReadResult",
        "0f2c39f8e77c9be5fbb79511204895a9e9b7290a3e401750f7147a0093b4376f",
    ),
    (
        "T3ProjectUpdateError",
        "029ec0b2d47ec1133a238247d5b48a4733fdf689b727aaad538947b905fe9040",
    ),
    (
        "T3ProjectUpdateInput",
        "a6eaf0cbad02ddffa9987566a9863f8fce84ee78c9700aaf94dc6d2c1fed2974",
    ),
    (
        "T3ProjectUpdateResult",
        "0f2c39f8e77c9be5fbb79511204895a9e9b7290a3e401750f7147a0093b4376f",
    ),
    (
        "T3QueueCancelError",
        "029ec0b2d47ec1133a238247d5b48a4733fdf689b727aaad538947b905fe9040",
    ),
    (
        "T3QueueCancelInput",
        "a63acd5b8bebd4932bbb8f818436e1b59ce14408e15cefde4acee2d300298625",
    ),
    (
        "T3QueueCancelResult",
        "7df2b694de677acf09176580adfa2937d5a2b266103ef9350f3fd8c8140c11d4",
    ),
    (
        "T3QueueEditError",
        "029ec0b2d47ec1133a238247d5b48a4733fdf689b727aaad538947b905fe9040",
    ),
    (
        "T3QueueEditInput",
        "d37916191b8808c705bb0ff11f41f0f1acc0fb2f69ca2fb16999cb763b264c4a",
    ),
    (
        "T3QueueEditResult",
        "7df2b694de677acf09176580adfa2937d5a2b266103ef9350f3fd8c8140c11d4",
    ),
    (
        "T3QueueListError",
        "029ec0b2d47ec1133a238247d5b48a4733fdf689b727aaad538947b905fe9040",
    ),
    (
        "T3QueueListInput",
        "4bdd3f5e5d392000aea865b2bf20acf98c821e4e77495751e3c7ae0bcda5a257",
    ),
    (
        "T3QueueListResult",
        "aad74141e175768db09c0f2585ba1747cd5948453d3653a3ade526c585f27976",
    ),
    (
        "T3QueuePromoteToSteerError",
        "029ec0b2d47ec1133a238247d5b48a4733fdf689b727aaad538947b905fe9040",
    ),
    (
        "T3QueuePromoteToSteerInput",
        "d779c2df897c57c10626d37fdcbbc79efe46f797e64d138f363986cf44369c79",
    ),
    (
        "T3QueuePromoteToSteerResult",
        "7df2b694de677acf09176580adfa2937d5a2b266103ef9350f3fd8c8140c11d4",
    ),
    (
        "T3QueueReadError",
        "029ec0b2d47ec1133a238247d5b48a4733fdf689b727aaad538947b905fe9040",
    ),
    (
        "T3QueueReadInput",
        "a63acd5b8bebd4932bbb8f818436e1b59ce14408e15cefde4acee2d300298625",
    ),
    (
        "T3QueueReadResult",
        "4cb9e9cec058f30e26b95287af5813382d0618c366df247df33b9fbed00a6c2d",
    ),
    (
        "T3QueueReorderError",
        "029ec0b2d47ec1133a238247d5b48a4733fdf689b727aaad538947b905fe9040",
    ),
    (
        "T3QueueReorderInput",
        "6c1d57dfde54e297c504b5323b1c5983bab60972598c055932c7f2d5c5cc6d63",
    ),
    (
        "T3QueueReorderResult",
        "7df2b694de677acf09176580adfa2937d5a2b266103ef9350f3fd8c8140c11d4",
    ),
    (
        "T3ThreadConfigurationError",
        "029ec0b2d47ec1133a238247d5b48a4733fdf689b727aaad538947b905fe9040",
    ),
    (
        "T3ThreadConfigurationInput",
        "51f42ece1937a449c913a32e28ad8e73c7f4a63529b364a9f64d2a069fc2297b",
    ),
    (
        "T3ThreadConfigurationResult",
        "9204c9f3271ecef411d882aaaf6688414902b36ed83aa6e4501299baf39814da",
    ),
    (
        "T3ThreadConfigureError",
        "029ec0b2d47ec1133a238247d5b48a4733fdf689b727aaad538947b905fe9040",
    ),
    (
        "T3ThreadConfigureInput",
        "2cfd5e8cc09535bbe11b5039baf014e0df4f7941d9e2f9a06c8feeeaf5129b0c",
    ),
    (
        "T3ThreadConfigureResult",
        "7df2b694de677acf09176580adfa2937d5a2b266103ef9350f3fd8c8140c11d4",
    ),
    (
        "T3ThreadForkError",
        "029ec0b2d47ec1133a238247d5b48a4733fdf689b727aaad538947b905fe9040",
    ),
    (
        "T3ThreadForkInput",
        "5a6a8e584bd2ccc7cdc7e4f6f9b831118c31c263201b1aa541d4d5caaca3375c",
    ),
    (
        "T3ThreadForkResult",
        "12dae5fd3ecd1160021b5e74e7c4e84eaf2f725da2cad6a12756eb92cb6edbdb",
    ),
    (
        "T3ThreadInterruptError",
        "029ec0b2d47ec1133a238247d5b48a4733fdf689b727aaad538947b905fe9040",
    ),
    (
        "T3ThreadInterruptInput",
        "08b73bf3f2a2aa2c81a9cee5daf9b385d6d963506dad66191399e01f92638173",
    ),
    (
        "T3ThreadInterruptResult",
        "7862ba66b9ddf488e5cc7846041992e0f7cc4395ff6fde35fa6e3baaca33ca99",
    ),
    (
        "T3ThreadLaunchError",
        "029ec0b2d47ec1133a238247d5b48a4733fdf689b727aaad538947b905fe9040",
    ),
    (
        "T3ThreadLaunchInput",
        "74f0c9c2c9e748063a7c521364cc9a91f0b0f857aebde153640d040a381259f5",
    ),
    (
        "T3ThreadLaunchResult",
        "79c193a4dfc1859cb7d3d1c1589ecacada0824738ea94e932c0607119d17b8bc",
    ),
    (
        "T3ThreadListError",
        "029ec0b2d47ec1133a238247d5b48a4733fdf689b727aaad538947b905fe9040",
    ),
    (
        "T3ThreadListInput",
        "87383792c2e2409d831d11ad6bc882705732eb749e4ea52979eb7e2ffabf2b40",
    ),
    (
        "T3ThreadListResult",
        "d4ebf78bf5ae0b3f9278da7ac00a2bd0bffb883de6e6f5b61cfc9a1f343f095e",
    ),
    (
        "T3ThreadMergeBackError",
        "029ec0b2d47ec1133a238247d5b48a4733fdf689b727aaad538947b905fe9040",
    ),
    (
        "T3ThreadMergeBackInput",
        "98f8f4b8b98e246a14afdc58d5a39597f89c7cdd48f640fbc07d7c059ad62c50",
    ),
    (
        "T3ThreadMergeBackResult",
        "12dae5fd3ecd1160021b5e74e7c4e84eaf2f725da2cad6a12756eb92cb6edbdb",
    ),
    (
        "T3ThreadOrganizeError",
        "029ec0b2d47ec1133a238247d5b48a4733fdf689b727aaad538947b905fe9040",
    ),
    (
        "T3ThreadOrganizeInput",
        "249aa3c202d5a3125f0e3fa999b377c37d9c0668afd236fe4bd86a238b16a19d",
    ),
    (
        "T3ThreadOrganizeResult",
        "7df2b694de677acf09176580adfa2937d5a2b266103ef9350f3fd8c8140c11d4",
    ),
    (
        "T3ThreadReadError",
        "029ec0b2d47ec1133a238247d5b48a4733fdf689b727aaad538947b905fe9040",
    ),
    (
        "T3ThreadReadInput",
        "02eded2802d4b9c68ad8384b28dd1329cf241e57f67ec7fc3347a9cc336f3517",
    ),
    (
        "T3ThreadReadResult",
        "c440f9e9e3a06e9d46bf7e1db08b055060ca0d952401b870c152580b330a3d92",
    ),
    (
        "T3ThreadSearchError",
        "029ec0b2d47ec1133a238247d5b48a4733fdf689b727aaad538947b905fe9040",
    ),
    (
        "T3ThreadSearchInput",
        "f3fe6f72b4db6e8fd805f9fb6df00f35b21f0020a8ca9590d900b0989c4f4320",
    ),
    (
        "T3ThreadSearchResult",
        "713f351d573142c7ec823f0da215ad4ec7ad8dab9274264c8fbc3c614d8c5ef7",
    ),
    (
        "T3ThreadSendAttachmentsError",
        "029ec0b2d47ec1133a238247d5b48a4733fdf689b727aaad538947b905fe9040",
    ),
    (
        "T3ThreadSendAttachmentsInput",
        "5374a7f0ffc38d180d7a3aef602ed78b6981273cb01947c2f2ca3190e70cca2e",
    ),
    (
        "T3ThreadSendAttachmentsResult",
        "1ad468efcce31753a483d1077092035f0ee207fcea20f4045b87a9f4a1f5c465",
    ),
    (
        "T3ThreadSendError",
        "029ec0b2d47ec1133a238247d5b48a4733fdf689b727aaad538947b905fe9040",
    ),
    (
        "T3ThreadSendInput",
        "f8a8c91b1fae49f977d3256b38a96e2feec265af82e5b8d8d1a11882647528d5",
    ),
    (
        "T3ThreadSendResult",
        "cd5f6976ed3378a12cab924d479d1d4476f151394caf9797ba5114b1d01f3901",
    ),
    (
        "T3ThreadTransfersError",
        "029ec0b2d47ec1133a238247d5b48a4733fdf689b727aaad538947b905fe9040",
    ),
    (
        "T3ThreadTransfersInput",
        "51f42ece1937a449c913a32e28ad8e73c7f4a63529b364a9f64d2a069fc2297b",
    ),
    (
        "T3ThreadTransfersResult",
        "a26563184ea82e760d5ebccd902dd3c40dfcaede7a3e051f7a41051fd7003440",
    ),
    (
        "T3ThreadUpdateError",
        "029ec0b2d47ec1133a238247d5b48a4733fdf689b727aaad538947b905fe9040",
    ),
    (
        "T3ThreadUpdateInput",
        "da958bf51d1852b126ad51c60df67404d9558f7eae0031348c0d6c3b6910386b",
    ),
    (
        "T3ThreadUpdateResult",
        "aff52ddddb1be4c75c8a96672261ab61af1da907793978ed5ada9ef9e082c9b5",
    ),
    (
        "T3ThreadWaitError",
        "029ec0b2d47ec1133a238247d5b48a4733fdf689b727aaad538947b905fe9040",
    ),
    (
        "T3ThreadWaitInput",
        "882782690199fa78fec39162f8a8e37b364f32a84dfb20d9771852361bf35941",
    ),
    (
        "T3ThreadWaitResult",
        "3d88d8d89fc10e0d8a032b463e73de8435bd20d8e723282fff6d0be7174c5d7b",
    ),
    (
        "T3WorktreeHandoffError",
        "a6425788b7352ccbd7b129dfe25c0a0fe5e66935339172a6f2e17738137a28c0",
    ),
    (
        "T3WorktreeHandoffInput",
        "48d4a6b47eea425441056cff1d3d0e2ed118055df7837c698025c9049cab71a7",
    ),
    (
        "T3WorktreeHandoffResult",
        "e40052c17390ed4b1c252022cb62673b06f19609bc80c254694f3ebb1ab79786",
    ),
    (
        "T3WorktreeListError",
        "029ec0b2d47ec1133a238247d5b48a4733fdf689b727aaad538947b905fe9040",
    ),
    (
        "T3WorktreeListInput",
        "3aa2412ba8dbbe68e263e0cc62f7e4426312f725f55bd03e6e089f04039b09b5",
    ),
    (
        "T3WorktreeListResult",
        "f69b677b9da1bc1595fbd50198dadc1d636a56ac1fb4a2aa345ffa9517fb843d",
    ),
    (
        "T3WorktreeStatusError",
        "a6425788b7352ccbd7b129dfe25c0a0fe5e66935339172a6f2e17738137a28c0",
    ),
    (
        "T3WorktreeStatusInput",
        "3e7cd0c7da63c31f23d31e81653eeb201e43a51bae91d944318c3caf8172d41e",
    ),
    (
        "T3WorktreeStatusResult",
        "faa28feafaa53fb8a2d0c8ecfb1c9fc8cce6e06f1b2488f05f0795659075b890",
    ),
    (
        "TaskCancelError",
        "029ec0b2d47ec1133a238247d5b48a4733fdf689b727aaad538947b905fe9040",
    ),
    (
        "TaskCancelInput",
        "750a29bd7826b15e74ac65fba60d0100dd54ff0164ff9351f59bed523cf63f86",
    ),
    (
        "TaskCancelResult",
        "0e250e064c9dfea09f8c780a3f3461e4f710edbbb91499817dc172b38b62f4f8",
    ),
    (
        "TaskStatusError",
        "029ec0b2d47ec1133a238247d5b48a4733fdf689b727aaad538947b905fe9040",
    ),
    (
        "TaskStatusInput",
        "25e6d317cf2cc2bbce6cc1204f59b95ffe60fd6105930a38d64dd4be1a12ec1e",
    ),
    (
        "TaskStatusResult",
        "c2bbb732663e757b927e08d2aae6f278477b203cecc438ec2ad6372a21fdf5e3",
    ),
    (
        "TerminalContextRecord",
        "826365a3e7e39d95669a7d7dcf930610d45359e8cd9dcd57a35144000590b53d",
    ),
    (
        "ThreadContextRecord",
        "2337bbdd1f5446feb5ce53e699084064db39e501017419af9966195c52f3ce0c",
    ),
    (
        "ThreadEnvMode",
        "5f2a5ba82a78ad8bce01948d4d6ceb9a51f7a007393b1446cb54a0377f20ace6",
    ),
    (
        "ThreadId",
        "9bddc1fdfb55932e41cf34e4a0bf062564cf848d3e0070d86dcd3a998fdce008",
    ),
    (
        "ThreadLinkedPullRequest",
        "6e0edd8f3d81838b8cec7fe9f031aec6b5eca73d129d6c5ec7f45dfb32b329d5",
    ),
    (
        "ThreadMetadataMcpAction",
        "045c3ec99fb8b093952193ff702928a79d33ea0c17c44103aa092bc1496013bc",
    ),
    (
        "ThreadMetadataMcpUpdateResult",
        "080a61eb88ec9b53280123641ee599133be2d6af523806df7b73ed74cbfca53f",
    ),
    (
        "ThreadPullRequestKey",
        "fa6dcfc2473308b4fc1402a3ae50490d354ad8eca748e5e13443f382c9893501",
    ),
    (
        "ThreadPullRequestLink",
        "a4ae047a372b374254773b3eb52ffd0f928e53cf2e5aa70c3decebb9a5075edd",
    ),
    (
        "ThreadPullRequestLinkSource",
        "daf5b5555ce0eb426ec9a8014603a11c13bd2441a1343358287270b6997b0e40",
    ),
    (
        "ThreadPullRequestSnapshot",
        "53273217f05cb7d8627832e123bf3fa870d1bc4cbefbc0a561c40223b0d3c062",
    ),
    (
        "ThreadPullRequestStack",
        "de6bd884667490eddd0de2e4f623c0d951403b890aaa62eeed83f41d1385ef99",
    ),
    (
        "ThreadPullRequestStackLayer",
        "0e61df48705bea68476eead6ebfb264f551df24acce9719a9f0a0e11ecda31f9",
    ),
    (
        "ThreadPullRequestWatch",
        "ec78e2e6083411e8f2cae2530f2e32874d3d020c219393a786484e1e2be43b2a",
    ),
    (
        "ThreadTitleRegeneration",
        "2c32daa58a7720f8242ec4371a5f7b346cf942c2bb0b810dffa3570160e350cf",
    ),
    (
        "ThreadTokenUsageSnapshot",
        "0577ff9b41047a994ad5af6df6370a0fbc242be2621aa5914f38b5149611c282",
    ),
    (
        "ToolActivityIcon",
        "866fe99f134f17be930f34efa8d40e2edceda1d850e9c254b565b27e7f68489c",
    ),
    (
        "ToolActivityNativeAppReference",
        "454b420755b7da0c66eca864525f517cab77af1e124d2ce1767f039afb95f084",
    ),
    (
        "ToolActivitySource",
        "78930f01e3a9041c6d25081d140cc3944b074148b7dffe1fb9096e8c408c8a26",
    ),
    (
        "ToolActivitySurface",
        "bf25d971a186c1f1fad7691a8b88abf9781e959f250923e6f5f076c77a537475",
    ),
    (
        "ToolFrameworkAiError",
        "b5a6e9cd522ff23da30350c7cf49059a7ec7b781e4ad946a95fdeec52a2beca0",
    ),
    (
        "ToolFrameworkExecutionFailure",
        "96ea547c50ae5f3c31eb802fa737a6ccd5c939eeb2d2f2ad41143b43e0b9313a",
    ),
    (
        "TrimmedNonEmptyString",
        "9bddc1fdfb55932e41cf34e4a0bf062564cf848d3e0070d86dcd3a998fdce008",
    ),
    (
        "TurnItemId",
        "9bddc1fdfb55932e41cf34e4a0bf062564cf848d3e0070d86dcd3a998fdce008",
    ),
    (
        "TurnTokenUsage",
        "f7e26af59a18b0beac56ff3bbfae040b24583acdc202bfb7343e85ae7f46e34f",
    ),
    (
        "UnknownContextRecord",
        "32770de11180a74dae326a6cdd4b80e22a1bf9947edf3e7bf719cd61ce43b598",
    ),
    (
        "UnlinkPullRequestError",
        "9467654a0069596fc9d535edd0f61c9e55775435778ee650ca520a8bf286e8bb",
    ),
    (
        "UnlinkPullRequestInput",
        "19bc9074da7d7c6953574f5570e131667b5fbae6f2d03e27d61f6f5ade07d406",
    ),
    (
        "UnlinkPullRequestResult",
        "6e898ad788f645ebf400cec69204e04d0567492caf82a36cce830b080bdd7851",
    ),
    (
        "UnwatchPullRequestError",
        "9467654a0069596fc9d535edd0f61c9e55775435778ee650ca520a8bf286e8bb",
    ),
    (
        "UnwatchPullRequestInput",
        "19bc9074da7d7c6953574f5570e131667b5fbae6f2d03e27d61f6f5ade07d406",
    ),
    (
        "UnwatchPullRequestResult",
        "d7e2b3cbcdd2bd6f3c6227a6d111293035fc87ad14201d08d77fbcd56061a3ed",
    ),
    (
        "UpdateScheduledTaskError",
        "029ec0b2d47ec1133a238247d5b48a4733fdf689b727aaad538947b905fe9040",
    ),
    (
        "UpdateScheduledTaskInput",
        "eb8f1809fb7199d36c1440b105ef233f7b7efa6192e9461f9a4d06acacc0959c",
    ),
    (
        "UpdateScheduledTaskResult",
        "8961e9c29cc34b54d79fc2680ef1282a86ada704d7cdae2738b015f48cdfc79a",
    ),
    (
        "UserInputAttachmentAnswerPayload",
        "01226e1cde6ad2d17b14f2769686939035f85d10da1a23b643024fa8fc900c10",
    ),
    (
        "UserInputAttachments",
        "ba6a0a83201c89a49d932b3990e7310bde7e6c4a642e7f606137900ac9f7cc41",
    ),
    (
        "VcsListRefsResult",
        "0fad0434b1431f678a4b69fb0cce03188aa88881a0419b9fe7098be0b096be63",
    ),
    (
        "VcsRef",
        "5a32e13f13d53539bbec62176e1c0f50abca6b1a02403b8fdc598609fbc05e08",
    ),
    (
        "WatchPullRequestError",
        "9467654a0069596fc9d535edd0f61c9e55775435778ee650ca520a8bf286e8bb",
    ),
    (
        "WatchPullRequestInput",
        "19bc9074da7d7c6953574f5570e131667b5fbae6f2d03e27d61f6f5ade07d406",
    ),
    (
        "WatchPullRequestResult",
        "d7e2b3cbcdd2bd6f3c6227a6d111293035fc87ad14201d08d77fbcd56061a3ed",
    ),
    (
        "WorktreeMcpContinuationStatus",
        "f74bf176c45dd94663c7ffe7b64a0adead2c74a24843e470a5c02526b70225d9",
    ),
    (
        "WorktreeMcpFailure",
        "dd3e7fbb40d45e1d425046edfa3f61f425be752d1ac27929962834d5857a33d9",
    ),
    (
        "WorktreeMcpHandoffResult",
        "eaf6a4c923a46a4051e7a129260ef3942747c42701ebb6559ad48e8d96e7cff8",
    ),
    (
        "WorktreeMcpSetupScriptStatus",
        "7f23fde5f92eb0bde38de33472ad81f20a8cc6f776e90f454c161552f40912bc",
    ),
    (
        "WorktreeMcpStatusResult",
        "a5f5a9f2728df0780e53d826748a8580563057ec4c1f711930bfb7afad693975",
    ),
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "name", content = "arguments")]
pub enum OrchestrationToolInput {
    #[serde(rename = "orchestrator_capabilities")]
    OrchestratorCapabilities(Box<OrchestratorCapabilitiesInput>),
    #[serde(rename = "delegate_task")]
    DelegateTask(Box<DelegateTaskInput>),
    #[serde(rename = "task_status")]
    TaskStatus(Box<TaskStatusInput>),
    #[serde(rename = "task_cancel")]
    TaskCancel(Box<TaskCancelInput>),
    #[serde(rename = "schedule_task")]
    ScheduleTask(Box<ScheduleTaskInput>),
    #[serde(rename = "list_scheduled_tasks")]
    ListScheduledTasks(Box<ListScheduledTasksInput>),
    #[serde(rename = "update_scheduled_task")]
    UpdateScheduledTask(Box<UpdateScheduledTaskInput>),
    #[serde(rename = "delete_scheduled_task")]
    DeleteScheduledTask(Box<DeleteScheduledTaskInput>),
    #[serde(rename = "create_threads")]
    CreateThreads(Box<CreateThreadsInput>),
    #[serde(rename = "t3_thread_list")]
    T3ThreadList(Box<T3ThreadListInput>),
    #[serde(rename = "t3_thread_read")]
    T3ThreadRead(Box<T3ThreadReadInput>),
    #[serde(rename = "t3_thread_update")]
    T3ThreadUpdate(Box<T3ThreadUpdateInput>),
    #[serde(rename = "t3_thread_send")]
    T3ThreadSend(Box<T3ThreadSendInput>),
    #[serde(rename = "t3_thread_wait")]
    T3ThreadWait(Box<T3ThreadWaitInput>),
    #[serde(rename = "t3_thread_interrupt")]
    T3ThreadInterrupt(Box<T3ThreadInterruptInput>),
    #[serde(rename = "run_scheduled_task_now")]
    RunScheduledTaskNow(Box<RunScheduledTaskNowInput>),
    #[serde(rename = "t3_thread_search")]
    T3ThreadSearch(Box<T3ThreadSearchInput>),
    #[serde(rename = "t3_thread_fork")]
    T3ThreadFork(Box<T3ThreadForkInput>),
    #[serde(rename = "t3_thread_merge_back")]
    T3ThreadMergeBack(Box<T3ThreadMergeBackInput>),
    #[serde(rename = "t3_thread_transfers")]
    T3ThreadTransfers(Box<T3ThreadTransfersInput>),
    #[serde(rename = "t3_thread_configuration")]
    T3ThreadConfiguration(Box<T3ThreadConfigurationInput>),
    #[serde(rename = "t3_thread_configure")]
    T3ThreadConfigure(Box<T3ThreadConfigureInput>),
    #[serde(rename = "t3_pending_request_list")]
    T3PendingRequestList(Box<T3PendingRequestListInput>),
    #[serde(rename = "t3_pending_request_read")]
    T3PendingRequestRead(Box<T3PendingRequestReadInput>),
    #[serde(rename = "t3_pending_request_respond")]
    T3PendingRequestRespond(Box<T3PendingRequestRespondInput>),
    #[serde(rename = "t3_thread_organize")]
    T3ThreadOrganize(Box<T3ThreadOrganizeInput>),
    #[serde(rename = "t3_queue_list")]
    T3QueueList(Box<T3QueueListInput>),
    #[serde(rename = "t3_queue_read")]
    T3QueueRead(Box<T3QueueReadInput>),
    #[serde(rename = "t3_queue_edit")]
    T3QueueEdit(Box<T3QueueEditInput>),
    #[serde(rename = "t3_queue_cancel")]
    T3QueueCancel(Box<T3QueueCancelInput>),
    #[serde(rename = "t3_queue_reorder")]
    T3QueueReorder(Box<T3QueueReorderInput>),
    #[serde(rename = "t3_queue_promote_to_steer")]
    T3QueuePromoteToSteer(Box<T3QueuePromoteToSteerInput>),
    #[serde(rename = "t3_worktree_handoff")]
    T3WorktreeHandoff(Box<T3WorktreeHandoffInput>),
    #[serde(rename = "t3_worktree_status")]
    T3WorktreeStatus(Box<T3WorktreeStatusInput>),
    #[serde(rename = "t3_worktree_list")]
    T3WorktreeList(Box<T3WorktreeListInput>),
    #[serde(rename = "link_pull_request")]
    LinkPullRequest(Box<LinkPullRequestInput>),
    #[serde(rename = "unlink_pull_request")]
    UnlinkPullRequest(Box<UnlinkPullRequestInput>),
    #[serde(rename = "list_thread_pull_requests")]
    ListThreadPullRequests(Box<ListThreadPullRequestsInput>),
    #[serde(rename = "watch_pull_request")]
    WatchPullRequest(Box<WatchPullRequestInput>),
    #[serde(rename = "unwatch_pull_request")]
    UnwatchPullRequest(Box<UnwatchPullRequestInput>),
    #[serde(rename = "t3_thread_launch")]
    T3ThreadLaunch(Box<T3ThreadLaunchInput>),
    #[serde(rename = "t3_project_list")]
    T3ProjectList(Box<T3ProjectListInput>),
    #[serde(rename = "t3_project_read")]
    T3ProjectRead(Box<T3ProjectReadInput>),
    #[serde(rename = "t3_project_create")]
    T3ProjectCreate(Box<T3ProjectCreateInput>),
    #[serde(rename = "t3_project_update")]
    T3ProjectUpdate(Box<T3ProjectUpdateInput>),
    #[serde(rename = "t3_project_delete")]
    T3ProjectDelete(Box<T3ProjectDeleteInput>),
    #[serde(rename = "t3_project_clone")]
    T3ProjectClone(Box<T3ProjectCloneInput>),
    #[serde(rename = "t3_environment_read")]
    T3EnvironmentRead(Box<T3EnvironmentReadInput>),
    #[serde(rename = "t3_environment_preferences_update")]
    T3EnvironmentPreferencesUpdate(Box<T3EnvironmentPreferencesUpdateInput>),
    #[serde(rename = "t3_attachment_prepare_upload")]
    T3AttachmentPrepareUpload(Box<T3AttachmentPrepareUploadInput>),
    #[serde(rename = "t3_attachment_discard")]
    T3AttachmentDiscard(Box<T3AttachmentDiscardInput>),
    #[serde(rename = "t3_thread_send_attachments")]
    T3ThreadSendAttachments(Box<T3ThreadSendAttachmentsInput>),
    #[serde(rename = "preview_snapshot")]
    PreviewSnapshot(Box<PreviewSnapshotInput>),
    #[serde(rename = "preview_status")]
    PreviewStatus(Box<PreviewStatusInput>),
    #[serde(rename = "preview_open")]
    PreviewOpen(Box<PreviewOpenInput>),
    #[serde(rename = "preview_navigate")]
    PreviewNavigate(Box<PreviewNavigateInput>),
    #[serde(rename = "preview_resize")]
    PreviewResize(Box<PreviewResizeInput>),
    #[serde(rename = "preview_set_appearance")]
    PreviewSetAppearance(Box<PreviewSetAppearanceInput>),
    #[serde(rename = "preview_click")]
    PreviewClick(Box<PreviewClickInput>),
    #[serde(rename = "preview_type")]
    PreviewType(Box<PreviewTypeInput>),
    #[serde(rename = "preview_press")]
    PreviewPress(Box<PreviewPressInput>),
    #[serde(rename = "preview_scroll")]
    PreviewScroll(Box<PreviewScrollInput>),
    #[serde(rename = "preview_evaluate")]
    PreviewEvaluate(Box<PreviewEvaluateInput>),
    #[serde(rename = "preview_wait_for")]
    PreviewWaitFor(Box<PreviewWaitForInput>),
    #[serde(rename = "preview_recording_start")]
    PreviewRecordingStart(Box<PreviewRecordingStartInput>),
    #[serde(rename = "preview_recording_stop")]
    PreviewRecordingStop(Box<PreviewRecordingStopInput>),
    #[serde(rename = "t3_preview_list")]
    T3PreviewList(Box<T3PreviewListInput>),
    #[serde(rename = "t3_preview_close")]
    T3PreviewClose(Box<T3PreviewCloseInput>),
    #[serde(rename = "device_screenshot")]
    DeviceScreenshot(Box<DeviceScreenshotInput>),
    #[serde(rename = "device_list")]
    DeviceList(Box<DeviceListInput>),
    #[serde(rename = "device_open")]
    DeviceOpen(Box<DeviceOpenInput>),
    #[serde(rename = "device_close")]
    DeviceClose(Box<DeviceCloseInput>),
}

/// Annotations as actually exposed by tools/list, not inferred from tool names.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolAnnotations {
    #[serde(default, skip_serializing_if = "Optional::is_absent")]
    pub title: Optional<String>,
    #[serde(default, skip_serializing_if = "Optional::is_absent")]
    pub read_only_hint: Optional<bool>,
    #[serde(default, skip_serializing_if = "Optional::is_absent")]
    pub destructive_hint: Optional<bool>,
    #[serde(default, skip_serializing_if = "Optional::is_absent")]
    pub idempotent_hint: Optional<bool>,
    #[serde(default, skip_serializing_if = "Optional::is_absent")]
    pub open_world_hint: Optional<bool>,
}

/// Frozen metadata. Capability gating belongs to the engine, not this catalog.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolDescriptor {
    pub name: String,
    pub description: String,
    pub group: String,
    pub phase: ToolPhase,
    pub annotations: ToolAnnotations,
    pub input_schema: serde_json::Value,
    pub decoded_input_schema: serde_json::Value,
    pub result_schema: serde_json::Value,
    pub error_schema: serde_json::Value,
    pub failure_result_schema: serde_json::Value,
    pub error_tags: Vec<String>,
    pub declared_error_tags: Vec<String>,
    pub refusal_codes: Vec<String>,
    #[serde(default, skip_serializing_if = "Optional::is_absent")]
    pub failure_mode: Optional<String>,
    pub framing: serde_json::Value,
    #[serde(default, skip_serializing_if = "Optional::is_absent")]
    pub transport: Optional<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolPhase {
    Core,
    Later,
}

pub fn pinned_tool_inventory() -> Vec<ToolDescriptor> {
    serde_json::from_str(include_str!("../tests/t3_oracle/fixtures/tools.json"))
        .expect("checked-in pinned T3 tool inventory")
}

/// Hand-registered PNG tools report a transport error envelope, not the
/// underlying toolkit failure object. The tag can be a typed failure tag or
/// the source's PreviewSnapshotError/device_screenshotError fallback.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageToolError {
    #[serde(rename = "_tag")]
    pub tag: String,
    pub operation: String,
    pub failure_count: usize,
    #[serde(default, skip_serializing_if = "Optional::is_absent")]
    pub message: Optional<String>,
}

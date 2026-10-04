// Generated fixture dispatch; see generate.mjs.
fn check_case(name: &str, value: &serde_json::Value) {
    match name {
        "AttachmentCreateUploadUrlResult" => {
            round_trip::<crate::orchestration::AttachmentCreateUploadUrlResult>(value)
        }
        "BackgroundActivityProfile" => {
            round_trip::<crate::orchestration::BackgroundActivityProfile>(value)
        }
        "BackgroundActivityProfileSelection" => {
            round_trip::<crate::orchestration::BackgroundActivityProfileSelection>(value)
        }
        "BooleanProviderOptionDescriptor" => {
            round_trip::<crate::provider_instance::BooleanProviderOptionDescriptor>(value)
        }
        "BrowserProfileId" => round_trip::<crate::orchestration::BrowserProfileId>(value),
        "ChatAttachment" => round_trip::<crate::orchestration::ChatAttachment>(value),
        "ChatAttachmentId" => round_trip::<crate::orchestration::ChatAttachmentId>(value),
        "ChatFileAttachment" => round_trip::<crate::orchestration::ChatFileAttachment>(value),
        "ChatImageAttachment" => round_trip::<crate::orchestration::ChatImageAttachment>(value),
        "ChatUnknownAttachment" => round_trip::<crate::orchestration::ChatUnknownAttachment>(value),
        "CheckpointId" => round_trip::<crate::orchestration::CheckpointId>(value),
        "CheckpointRef" => round_trip::<crate::orchestration::CheckpointRef>(value),
        "CheckpointScopeId" => round_trip::<crate::orchestration::CheckpointScopeId>(value),
        "CommandId" => round_trip::<crate::orchestration::CommandId>(value),
        "ComposerContextId" => round_trip::<crate::orchestration::ComposerContextId>(value),
        "ComposerContextRecord" => round_trip::<crate::orchestration::ComposerContextRecord>(value),
        "ContextHandoffId" => round_trip::<crate::orchestration::ContextHandoffId>(value),
        "ContextTransferId" => round_trip::<crate::orchestration::ContextTransferId>(value),
        "CreateThreadsError" => round_trip::<crate::orchestration_mcp::CreateThreadsError>(value),
        "CreateThreadsInput" => round_trip::<crate::orchestration_mcp::CreateThreadsInput>(value),
        "CreateThreadsResult" => round_trip::<crate::orchestration_mcp::CreateThreadsResult>(value),
        "CustomModelEntry" => round_trip::<crate::provider_instance::CustomModelEntry>(value),
        "CustomModelSetting" => round_trip::<crate::provider_instance::CustomModelSetting>(value),
        "DelegateTaskError" => round_trip::<crate::orchestration_mcp::DelegateTaskError>(value),
        "DelegateTaskInput" => round_trip::<crate::orchestration_mcp::DelegateTaskInput>(value),
        "DelegateTaskResult" => round_trip::<crate::orchestration_mcp::DelegateTaskResult>(value),
        "DeleteScheduledTaskError" => {
            round_trip::<crate::orchestration_mcp::DeleteScheduledTaskError>(value)
        }
        "DeleteScheduledTaskInput" => {
            round_trip::<crate::orchestration_mcp::DeleteScheduledTaskInput>(value)
        }
        "DeleteScheduledTaskResult" => {
            round_trip::<crate::orchestration_mcp::DeleteScheduledTaskResult>(value)
        }
        "DeviceActionUnavailableError" => {
            round_trip::<crate::orchestration::DeviceActionUnavailableError>(value)
        }
        "DeviceBootError" => round_trip::<crate::orchestration::DeviceBootError>(value),
        "DeviceCloseError" => round_trip::<crate::orchestration_mcp::DeviceCloseError>(value),
        "DeviceCloseInput" => round_trip::<crate::orchestration_mcp::DeviceCloseInput>(value),
        "DeviceCloseResult" => round_trip::<crate::orchestration_mcp::DeviceCloseResult>(value),
        "DeviceHostId" => round_trip::<crate::orchestration::DeviceHostId>(value),
        "DeviceHostStatus" => round_trip::<crate::orchestration::DeviceHostStatus>(value),
        "DeviceHostSummary" => round_trip::<crate::orchestration::DeviceHostSummary>(value),
        "DeviceHostUnavailableError" => {
            round_trip::<crate::orchestration::DeviceHostUnavailableError>(value)
        }
        "DeviceId" => round_trip::<crate::orchestration::DeviceId>(value),
        "DeviceListError" => round_trip::<crate::orchestration_mcp::DeviceListError>(value),
        "DeviceListInput" => round_trip::<crate::orchestration_mcp::DeviceListInput>(value),
        "DeviceListResult" => round_trip::<crate::orchestration_mcp::DeviceListResult>(value),
        "DeviceNotFoundError" => round_trip::<crate::orchestration::DeviceNotFoundError>(value),
        "DeviceOpenError" => round_trip::<crate::orchestration_mcp::DeviceOpenError>(value),
        "DeviceOpenInput" => round_trip::<crate::orchestration_mcp::DeviceOpenInput>(value),
        "DeviceOpenResult" => round_trip::<crate::orchestration_mcp::DeviceOpenResult>(value),
        "DeviceOperationError" => round_trip::<crate::orchestration::DeviceOperationError>(value),
        "DevicePlatform" => round_trip::<crate::orchestration::DevicePlatform>(value),
        "DevicePlatformAvailability" => {
            round_trip::<crate::orchestration::DevicePlatformAvailability>(value)
        }
        "DevicePlatformUnavailableError" => {
            round_trip::<crate::orchestration::DevicePlatformUnavailableError>(value)
        }
        "DeviceScreenshotError" => {
            round_trip::<crate::orchestration_mcp::DeviceScreenshotError>(value)
        }
        "DeviceScreenshotInput" => {
            round_trip::<crate::orchestration_mcp::DeviceScreenshotInput>(value)
        }
        "DeviceScreenshotResult" => {
            round_trip::<crate::orchestration_mcp::DeviceScreenshotResult>(value)
        }
        "DeviceSummary" => round_trip::<crate::orchestration::DeviceSummary>(value),
        "DeviceToolError" => round_trip::<crate::orchestration::DeviceToolError>(value),
        "DeviceToolListResult" => round_trip::<crate::orchestration::DeviceToolListResult>(value),
        "DeviceToolOpenResult" => round_trip::<crate::orchestration::DeviceToolOpenResult>(value),
        "DeviceToolScreenshotResult" => {
            round_trip::<crate::orchestration::DeviceToolScreenshotResult>(value)
        }
        "DeviceToolUnavailableError" => {
            round_trip::<crate::orchestration::DeviceToolUnavailableError>(value)
        }
        "DeviceToolVersion" => round_trip::<crate::orchestration::DeviceToolVersion>(value),
        "DeviceToolVersions" => round_trip::<crate::orchestration::DeviceToolVersions>(value),
        "ElementContextDetails" => round_trip::<crate::orchestration::ElementContextDetails>(value),
        "ElementContextRecord" => round_trip::<crate::orchestration::ElementContextRecord>(value),
        "ElementContextSource" => round_trip::<crate::orchestration::ElementContextSource>(value),
        "EnvironmentId" => round_trip::<crate::orchestration::EnvironmentId>(value),
        "EnvironmentMachineKind" => {
            round_trip::<crate::orchestration::EnvironmentMachineKind>(value)
        }
        "EventId" => round_trip::<crate::orchestration::EventId>(value),
        "ExecutionEnvironmentPlatform" => {
            round_trip::<crate::orchestration::ExecutionEnvironmentPlatform>(value)
        }
        "ExecutionEnvironmentPlatformArch" => {
            round_trip::<crate::orchestration::ExecutionEnvironmentPlatformArch>(value)
        }
        "ExecutionEnvironmentPlatformOs" => {
            round_trip::<crate::orchestration::ExecutionEnvironmentPlatformOs>(value)
        }
        "FileContextRecord" => round_trip::<crate::orchestration::FileContextRecord>(value),
        "ImageContextRecord" => round_trip::<crate::orchestration::ImageContextRecord>(value),
        "IsoDateTime" => round_trip::<crate::orchestration::IsoDateTime>(value),
        "LinkPullRequestError" => {
            round_trip::<crate::orchestration_mcp::LinkPullRequestError>(value)
        }
        "LinkPullRequestInput" => {
            round_trip::<crate::orchestration_mcp::LinkPullRequestInput>(value)
        }
        "LinkPullRequestResult" => {
            round_trip::<crate::orchestration_mcp::LinkPullRequestResult>(value)
        }
        "ListScheduledTasksError" => {
            round_trip::<crate::orchestration_mcp::ListScheduledTasksError>(value)
        }
        "ListScheduledTasksInput" => {
            round_trip::<crate::orchestration_mcp::ListScheduledTasksInput>(value)
        }
        "ListScheduledTasksResult" => {
            round_trip::<crate::orchestration_mcp::ListScheduledTasksResult>(value)
        }
        "ListThreadPullRequestsError" => {
            round_trip::<crate::orchestration_mcp::ListThreadPullRequestsError>(value)
        }
        "ListThreadPullRequestsInput" => {
            round_trip::<crate::orchestration_mcp::ListThreadPullRequestsInput>(value)
        }
        "ListThreadPullRequestsResult" => {
            round_trip::<crate::orchestration_mcp::ListThreadPullRequestsResult>(value)
        }
        "McpCapabilityUnavailableError" => {
            round_trip::<crate::orchestration_mcp::McpCapabilityUnavailableError>(value)
        }
        "MentionContextRecord" => round_trip::<crate::orchestration::MentionContextRecord>(value),
        "MessageId" => round_trip::<crate::orchestration::MessageId>(value),
        "ModelCapabilities" => round_trip::<crate::provider_instance::ModelCapabilities>(value),
        "ModelSelection" => round_trip::<crate::provider_instance::ModelSelection>(value),
        "NodeId" => round_trip::<crate::orchestration::NodeId>(value),
        "NonNegativeInt" => round_trip::<crate::orchestration::NonNegativeInt>(value),
        "OrchestrationGetWorkflowScriptError" => {
            round_trip::<crate::orchestration::OrchestrationGetWorkflowScriptError>(value)
        }
        "OrchestrationMessageContext" => {
            round_trip::<crate::orchestration::OrchestrationMessageContext>(value)
        }
        "OrchestrationProjectShell" => {
            round_trip::<crate::orchestration::OrchestrationProjectShell>(value)
        }
        "OrchestrationSearchThreadsResult" => {
            round_trip::<crate::orchestration::OrchestrationSearchThreadsResult>(value)
        }
        "OrchestrationThreadSearchMatch" => {
            round_trip::<crate::orchestration::OrchestrationThreadSearchMatch>(value)
        }
        "OrchestrationThreadSearchSource" => {
            round_trip::<crate::orchestration::OrchestrationThreadSearchSource>(value)
        }
        "OrchestrationV2Actor" => round_trip::<crate::orchestration::OrchestrationV2Actor>(value),
        "OrchestrationV2AppThread" => {
            round_trip::<crate::orchestration::OrchestrationV2AppThread>(value)
        }
        "OrchestrationV2AppThreadJson" => {
            round_trip::<crate::orchestration::OrchestrationV2AppThreadJson>(value)
        }
        "OrchestrationV2AppThreadLineage" => {
            round_trip::<crate::orchestration::OrchestrationV2AppThreadLineage>(value)
        }
        "OrchestrationV2ApprovalCapabilities" => {
            round_trip::<crate::orchestration::OrchestrationV2ApprovalCapabilities>(value)
        }
        "OrchestrationV2ArchivedShellSnapshot" => {
            round_trip::<crate::orchestration::OrchestrationV2ArchivedShellSnapshot>(value)
        }
        "OrchestrationV2ArchivedShellStreamItem" => {
            round_trip::<crate::orchestration::OrchestrationV2ArchivedShellStreamItem>(value)
        }
        "OrchestrationV2Checkpoint" => {
            round_trip::<crate::orchestration::OrchestrationV2Checkpoint>(value)
        }
        "OrchestrationV2CheckpointCapabilities" => {
            round_trip::<crate::orchestration::OrchestrationV2CheckpointCapabilities>(value)
        }
        "OrchestrationV2CheckpointFileSummary" => {
            round_trip::<crate::orchestration::OrchestrationV2CheckpointFileSummary>(value)
        }
        "OrchestrationV2CheckpointJson" => {
            round_trip::<crate::orchestration::OrchestrationV2CheckpointJson>(value)
        }
        "OrchestrationV2CheckpointRollbackRequest" => {
            round_trip::<crate::orchestration::OrchestrationV2CheckpointRollbackRequest>(value)
        }
        "OrchestrationV2CheckpointRollbackRequestJson" => {
            round_trip::<crate::orchestration::OrchestrationV2CheckpointRollbackRequestJson>(value)
        }
        "OrchestrationV2CheckpointScope" => {
            round_trip::<crate::orchestration::OrchestrationV2CheckpointScope>(value)
        }
        "OrchestrationV2CheckpointScopeJson" => {
            round_trip::<crate::orchestration::OrchestrationV2CheckpointScopeJson>(value)
        }
        "OrchestrationV2CheckpointUnavailableError" => {
            round_trip::<crate::orchestration::OrchestrationV2CheckpointUnavailableError>(value)
        }
        "OrchestrationV2Command" => {
            round_trip::<crate::orchestration::OrchestrationV2Command>(value)
        }
        "OrchestrationV2ContextCapabilities" => {
            round_trip::<crate::orchestration::OrchestrationV2ContextCapabilities>(value)
        }
        "OrchestrationV2ContextHandoff" => {
            round_trip::<crate::orchestration::OrchestrationV2ContextHandoff>(value)
        }
        "OrchestrationV2ContextHandoffJson" => {
            round_trip::<crate::orchestration::OrchestrationV2ContextHandoffJson>(value)
        }
        "OrchestrationV2ContextSourcePoint" => {
            round_trip::<crate::orchestration::OrchestrationV2ContextSourcePoint>(value)
        }
        "OrchestrationV2ContextTransfer" => {
            round_trip::<crate::orchestration::OrchestrationV2ContextTransfer>(value)
        }
        "OrchestrationV2ContextTransferJson" => {
            round_trip::<crate::orchestration::OrchestrationV2ContextTransferJson>(value)
        }
        "OrchestrationV2ContextTransferResolution" => {
            round_trip::<crate::orchestration::OrchestrationV2ContextTransferResolution>(value)
        }
        "OrchestrationV2ContextTransferType" => {
            round_trip::<crate::orchestration::OrchestrationV2ContextTransferType>(value)
        }
        "OrchestrationV2ConversationMessage" => {
            round_trip::<crate::orchestration::OrchestrationV2ConversationMessage>(value)
        }
        "OrchestrationV2ConversationMessageJson" => {
            round_trip::<crate::orchestration::OrchestrationV2ConversationMessageJson>(value)
        }
        "OrchestrationV2CreationSource" => {
            round_trip::<crate::orchestration::OrchestrationV2CreationSource>(value)
        }
        "OrchestrationV2DelegatedCompletionCohort" => {
            round_trip::<crate::orchestration::OrchestrationV2DelegatedCompletionCohort>(value)
        }
        "OrchestrationV2DelegatedCompletionDelivery" => {
            round_trip::<crate::orchestration::OrchestrationV2DelegatedCompletionDelivery>(value)
        }
        "OrchestrationV2DelegatedCompletionTaskDelivery" => round_trip::<
            crate::orchestration::OrchestrationV2DelegatedCompletionTaskDelivery,
        >(value),
        "OrchestrationV2DelegatedCompletionTaskDeliveryState" => round_trip::<
            crate::orchestration::OrchestrationV2DelegatedCompletionTaskDeliveryState,
        >(value),
        "OrchestrationV2DispatchCommandError" => {
            round_trip::<crate::orchestration::OrchestrationV2DispatchCommandError>(value)
        }
        "OrchestrationV2DispatchCommandResult" => {
            round_trip::<crate::orchestration::OrchestrationV2DispatchCommandResult>(value)
        }
        "OrchestrationV2DomainEvent" => {
            round_trip::<crate::orchestration::OrchestrationV2DomainEvent>(value)
        }
        "OrchestrationV2DomainEventJson" => {
            round_trip::<crate::orchestration::OrchestrationV2DomainEventJson>(value)
        }
        "OrchestrationV2ExecutionNode" => {
            round_trip::<crate::orchestration::OrchestrationV2ExecutionNode>(value)
        }
        "OrchestrationV2ExecutionNodeJson" => {
            round_trip::<crate::orchestration::OrchestrationV2ExecutionNodeJson>(value)
        }
        "OrchestrationV2FileChangeDetail" => {
            round_trip::<crate::orchestration::OrchestrationV2FileChangeDetail>(value)
        }
        "OrchestrationV2FileSearchResult" => {
            round_trip::<crate::orchestration::OrchestrationV2FileSearchResult>(value)
        }
        "OrchestrationV2GetShellSnapshotError" => {
            round_trip::<crate::orchestration::OrchestrationV2GetShellSnapshotError>(value)
        }
        "OrchestrationV2GetThreadProjectionError" => {
            round_trip::<crate::orchestration::OrchestrationV2GetThreadProjectionError>(value)
        }
        "OrchestrationV2GetThreadProjectionInput" => {
            round_trip::<crate::orchestration::OrchestrationV2GetThreadProjectionInput>(value)
        }
        "OrchestrationV2GetWorkflowScriptInput" => {
            round_trip::<crate::orchestration::OrchestrationV2GetWorkflowScriptInput>(value)
        }
        "OrchestrationV2GetWorkflowScriptResult" => {
            round_trip::<crate::orchestration::OrchestrationV2GetWorkflowScriptResult>(value)
        }
        "OrchestrationV2HistoricalMessage" => {
            round_trip::<crate::orchestration::OrchestrationV2HistoricalMessage>(value)
        }
        "OrchestrationV2IdentityCapabilities" => {
            round_trip::<crate::orchestration::OrchestrationV2IdentityCapabilities>(value)
        }
        "OrchestrationV2LatestVisibleMessageSummary" => {
            round_trip::<crate::orchestration::OrchestrationV2LatestVisibleMessageSummary>(value)
        }
        "OrchestrationV2LatestVisibleMessageSummaryJson" => round_trip::<
            crate::orchestration::OrchestrationV2LatestVisibleMessageSummaryJson,
        >(value),
        "OrchestrationV2LimitRecovery" => {
            round_trip::<crate::orchestration::OrchestrationV2LimitRecovery>(value)
        }
        "OrchestrationV2LimitRecoveryUpdate" => {
            round_trip::<crate::orchestration::OrchestrationV2LimitRecoveryUpdate>(value)
        }
        "OrchestrationV2NativeRefStrength" => {
            round_trip::<crate::orchestration::OrchestrationV2NativeRefStrength>(value)
        }
        "OrchestrationV2Notification" => {
            round_trip::<crate::orchestration::OrchestrationV2Notification>(value)
        }
        "OrchestrationV2NotificationSource" => {
            round_trip::<crate::orchestration::OrchestrationV2NotificationSource>(value)
        }
        "OrchestrationV2PendingBackgroundTask" => {
            round_trip::<crate::orchestration::OrchestrationV2PendingBackgroundTask>(value)
        }
        "OrchestrationV2PendingRuntimeRequestSummary" => {
            round_trip::<crate::orchestration::OrchestrationV2PendingRuntimeRequestSummary>(value)
        }
        "OrchestrationV2PendingRuntimeRequestSummaryJson" => round_trip::<
            crate::orchestration::OrchestrationV2PendingRuntimeRequestSummaryJson,
        >(value),
        "OrchestrationV2PlanArtifact" => {
            round_trip::<crate::orchestration::OrchestrationV2PlanArtifact>(value)
        }
        "OrchestrationV2PlanStep" => {
            round_trip::<crate::orchestration::OrchestrationV2PlanStep>(value)
        }
        "OrchestrationV2PlanningCapabilities" => {
            round_trip::<crate::orchestration::OrchestrationV2PlanningCapabilities>(value)
        }
        "OrchestrationV2ProjectedTurnItem" => {
            round_trip::<crate::orchestration::OrchestrationV2ProjectedTurnItem>(value)
        }
        "OrchestrationV2ProjectedTurnItemJson" => {
            round_trip::<crate::orchestration::OrchestrationV2ProjectedTurnItemJson>(value)
        }
        "OrchestrationV2ProviderCapabilities" => {
            round_trip::<crate::orchestration::OrchestrationV2ProviderCapabilities>(value)
        }
        "OrchestrationV2ProviderFailure" => {
            round_trip::<crate::orchestration::OrchestrationV2ProviderFailure>(value)
        }
        "OrchestrationV2ProviderFailureClass" => {
            round_trip::<crate::orchestration::OrchestrationV2ProviderFailureClass>(value)
        }
        "OrchestrationV2ProviderRef" => {
            round_trip::<crate::orchestration::OrchestrationV2ProviderRef>(value)
        }
        "OrchestrationV2ProviderRetry" => {
            round_trip::<crate::orchestration::OrchestrationV2ProviderRetry>(value)
        }
        "OrchestrationV2ProviderSession" => {
            round_trip::<crate::orchestration::OrchestrationV2ProviderSession>(value)
        }
        "OrchestrationV2ProviderSessionDetached" => {
            round_trip::<crate::orchestration::OrchestrationV2ProviderSessionDetached>(value)
        }
        "OrchestrationV2ProviderSessionDetachedJson" => {
            round_trip::<crate::orchestration::OrchestrationV2ProviderSessionDetachedJson>(value)
        }
        "OrchestrationV2ProviderSessionJson" => {
            round_trip::<crate::orchestration::OrchestrationV2ProviderSessionJson>(value)
        }
        "OrchestrationV2ProviderThread" => {
            round_trip::<crate::orchestration::OrchestrationV2ProviderThread>(value)
        }
        "OrchestrationV2ProviderThreadDisposition" => {
            round_trip::<crate::orchestration::OrchestrationV2ProviderThreadDisposition>(value)
        }
        "OrchestrationV2ProviderThreadJson" => {
            round_trip::<crate::orchestration::OrchestrationV2ProviderThreadJson>(value)
        }
        "OrchestrationV2ProviderThreadNativeMetadata" => {
            round_trip::<crate::orchestration::OrchestrationV2ProviderThreadNativeMetadata>(value)
        }
        "OrchestrationV2ProviderTurn" => {
            round_trip::<crate::orchestration::OrchestrationV2ProviderTurn>(value)
        }
        "OrchestrationV2ProviderTurnJson" => {
            round_trip::<crate::orchestration::OrchestrationV2ProviderTurnJson>(value)
        }
        "OrchestrationV2ProviderTurnTokenUsage" => {
            round_trip::<crate::orchestration::OrchestrationV2ProviderTurnTokenUsage>(value)
        }
        "OrchestrationV2RawProviderEvent" => {
            round_trip::<crate::orchestration::OrchestrationV2RawProviderEvent>(value)
        }
        "OrchestrationV2RawProviderEventJson" => {
            round_trip::<crate::orchestration::OrchestrationV2RawProviderEventJson>(value)
        }
        "OrchestrationV2RestartCancelledBackgroundWork" => {
            round_trip::<crate::orchestration::OrchestrationV2RestartCancelledBackgroundWork>(value)
        }
        "OrchestrationV2RpcError" => {
            round_trip::<crate::orchestration::OrchestrationV2RpcError>(value)
        }
        "OrchestrationV2Run" => round_trip::<crate::orchestration::OrchestrationV2Run>(value),
        "OrchestrationV2RunAttempt" => {
            round_trip::<crate::orchestration::OrchestrationV2RunAttempt>(value)
        }
        "OrchestrationV2RunAttemptJson" => {
            round_trip::<crate::orchestration::OrchestrationV2RunAttemptJson>(value)
        }
        "OrchestrationV2RunBackgroundWorkCancelled" => {
            round_trip::<crate::orchestration::OrchestrationV2RunBackgroundWorkCancelled>(value)
        }
        "OrchestrationV2RunJson" => {
            round_trip::<crate::orchestration::OrchestrationV2RunJson>(value)
        }
        "OrchestrationV2RunStatus" => {
            round_trip::<crate::orchestration::OrchestrationV2RunStatus>(value)
        }
        "OrchestrationV2RuntimePolicyCapabilities" => {
            round_trip::<crate::orchestration::OrchestrationV2RuntimePolicyCapabilities>(value)
        }
        "OrchestrationV2RuntimeRequest" => {
            round_trip::<crate::orchestration::OrchestrationV2RuntimeRequest>(value)
        }
        "OrchestrationV2RuntimeRequestJson" => {
            round_trip::<crate::orchestration::OrchestrationV2RuntimeRequestJson>(value)
        }
        "OrchestrationV2SessionCapabilities" => {
            round_trip::<crate::orchestration::OrchestrationV2SessionCapabilities>(value)
        }
        "OrchestrationV2ShellSnapshot" => {
            round_trip::<crate::orchestration::OrchestrationV2ShellSnapshot>(value)
        }
        "OrchestrationV2ShellSnapshotJson" => {
            round_trip::<crate::orchestration::OrchestrationV2ShellSnapshotJson>(value)
        }
        "OrchestrationV2ShellStreamItem" => {
            round_trip::<crate::orchestration::OrchestrationV2ShellStreamItem>(value)
        }
        "OrchestrationV2ShellThreadStatus" => {
            round_trip::<crate::orchestration::OrchestrationV2ShellThreadStatus>(value)
        }
        "OrchestrationV2StoredEvent" => {
            round_trip::<crate::orchestration::OrchestrationV2StoredEvent>(value)
        }
        "OrchestrationV2StoredEventJson" => {
            round_trip::<crate::orchestration::OrchestrationV2StoredEventJson>(value)
        }
        "OrchestrationV2StreamingCapabilities" => {
            round_trip::<crate::orchestration::OrchestrationV2StreamingCapabilities>(value)
        }
        "OrchestrationV2Subagent" => {
            round_trip::<crate::orchestration::OrchestrationV2Subagent>(value)
        }
        "OrchestrationV2SubagentCapabilities" => {
            round_trip::<crate::orchestration::OrchestrationV2SubagentCapabilities>(value)
        }
        "OrchestrationV2SubagentJson" => {
            round_trip::<crate::orchestration::OrchestrationV2SubagentJson>(value)
        }
        "OrchestrationV2SubscribeShellInput" => {
            round_trip::<crate::orchestration::OrchestrationV2SubscribeShellInput>(value)
        }
        "OrchestrationV2SubscribeThreadInput" => {
            round_trip::<crate::orchestration::OrchestrationV2SubscribeThreadInput>(value)
        }
        "OrchestrationV2ThreadBoundedSnapshot" => {
            round_trip::<crate::orchestration::OrchestrationV2ThreadBoundedSnapshot>(value)
        }
        "OrchestrationV2ThreadCapabilities" => {
            round_trip::<crate::orchestration::OrchestrationV2ThreadCapabilities>(value)
        }
        "OrchestrationV2ThreadDetailSnapshot" => {
            round_trip::<crate::orchestration::OrchestrationV2ThreadDetailSnapshot>(value)
        }
        "OrchestrationV2ThreadForkSourcePoint" => {
            round_trip::<crate::orchestration::OrchestrationV2ThreadForkSourcePoint>(value)
        }
        "OrchestrationV2ThreadHistoryOrigin" => {
            round_trip::<crate::orchestration::OrchestrationV2ThreadHistoryOrigin>(value)
        }
        "OrchestrationV2ThreadHistoryPage" => {
            round_trip::<crate::orchestration::OrchestrationV2ThreadHistoryPage>(value)
        }
        "OrchestrationV2ThreadLaunchError" => {
            round_trip::<crate::orchestration::OrchestrationV2ThreadLaunchError>(value)
        }
        "OrchestrationV2ThreadLaunchInput" => {
            round_trip::<crate::orchestration::OrchestrationV2ThreadLaunchInput>(value)
        }
        "OrchestrationV2ThreadLaunchResult" => {
            round_trip::<crate::orchestration::OrchestrationV2ThreadLaunchResult>(value)
        }
        "OrchestrationV2ThreadLaunchWorkspaceStrategy" => {
            round_trip::<crate::orchestration::OrchestrationV2ThreadLaunchWorkspaceStrategy>(value)
        }
        "OrchestrationV2ThreadProjection" => {
            round_trip::<crate::orchestration::OrchestrationV2ThreadProjection>(value)
        }
        "OrchestrationV2ThreadProjectionJson" => {
            round_trip::<crate::orchestration::OrchestrationV2ThreadProjectionJson>(value)
        }
        "OrchestrationV2ThreadShell" => {
            round_trip::<crate::orchestration::OrchestrationV2ThreadShell>(value)
        }
        "OrchestrationV2ThreadShellJson" => {
            round_trip::<crate::orchestration::OrchestrationV2ThreadShellJson>(value)
        }
        "OrchestrationV2ThreadShellSnapshot" => {
            round_trip::<crate::orchestration::OrchestrationV2ThreadShellSnapshot>(value)
        }
        "OrchestrationV2ThreadStreamItem" => {
            round_trip::<crate::orchestration::OrchestrationV2ThreadStreamItem>(value)
        }
        "OrchestrationV2ToolCapabilities" => {
            round_trip::<crate::orchestration::OrchestrationV2ToolCapabilities>(value)
        }
        "OrchestrationV2TurnCapabilities" => {
            round_trip::<crate::orchestration::OrchestrationV2TurnCapabilities>(value)
        }
        "OrchestrationV2TurnItem" => {
            round_trip::<crate::orchestration::OrchestrationV2TurnItem>(value)
        }
        "OrchestrationV2TurnItemJson" => {
            round_trip::<crate::orchestration::OrchestrationV2TurnItemJson>(value)
        }
        "OrchestrationV2TurnItemStatus" => {
            round_trip::<crate::orchestration::OrchestrationV2TurnItemStatus>(value)
        }
        "OrchestrationV2UserInputQuestion" => {
            round_trip::<crate::orchestration::OrchestrationV2UserInputQuestion>(value)
        }
        "OrchestrationV2UserMessageInputIntent" => {
            round_trip::<crate::orchestration::OrchestrationV2UserMessageInputIntent>(value)
        }
        "OrchestrationV2WebSearchResult" => {
            round_trip::<crate::orchestration::OrchestrationV2WebSearchResult>(value)
        }
        "OrchestratorCapabilitiesError" => {
            round_trip::<crate::orchestration_mcp::OrchestratorCapabilitiesError>(value)
        }
        "OrchestratorCapabilitiesInput" => {
            round_trip::<crate::orchestration_mcp::OrchestratorCapabilitiesInput>(value)
        }
        "OrchestratorCapabilitiesResult" => {
            round_trip::<crate::orchestration_mcp::OrchestratorCapabilitiesResult>(value)
        }
        "OrchestratorMcpCapabilitiesResult" => {
            round_trip::<crate::orchestration_mcp::OrchestratorMcpCapabilitiesResult>(value)
        }
        "OrchestratorMcpCreateThreadRequest" => {
            round_trip::<crate::orchestration_mcp::OrchestratorMcpCreateThreadRequest>(value)
        }
        "OrchestratorMcpCreateThreadsInput" => {
            round_trip::<crate::orchestration_mcp::OrchestratorMcpCreateThreadsInput>(value)
        }
        "OrchestratorMcpCreateThreadsResult" => {
            round_trip::<crate::orchestration_mcp::OrchestratorMcpCreateThreadsResult>(value)
        }
        "OrchestratorMcpCreatedThread" => {
            round_trip::<crate::orchestration_mcp::OrchestratorMcpCreatedThread>(value)
        }
        "OrchestratorMcpCreatedThreadStatus" => {
            round_trip::<crate::orchestration_mcp::OrchestratorMcpCreatedThreadStatus>(value)
        }
        "OrchestratorMcpDelegateTaskInput" => {
            round_trip::<crate::orchestration_mcp::OrchestratorMcpDelegateTaskInput>(value)
        }
        "OrchestratorMcpDelegateTaskResult" => {
            round_trip::<crate::orchestration_mcp::OrchestratorMcpDelegateTaskResult>(value)
        }
        "OrchestratorMcpDelegatedTaskStatus" => {
            round_trip::<crate::orchestration_mcp::OrchestratorMcpDelegatedTaskStatus>(value)
        }
        "OrchestratorMcpDeleteScheduledTaskInput" => {
            round_trip::<crate::orchestration_mcp::OrchestratorMcpDeleteScheduledTaskInput>(value)
        }
        "OrchestratorMcpDeleteScheduledTaskResult" => {
            round_trip::<crate::orchestration_mcp::OrchestratorMcpDeleteScheduledTaskResult>(value)
        }
        "OrchestratorMcpFailure" => {
            round_trip::<crate::orchestration_mcp::OrchestratorMcpFailure>(value)
        }
        "OrchestratorMcpInteractionMode" => {
            round_trip::<crate::orchestration_mcp::OrchestratorMcpInteractionMode>(value)
        }
        "OrchestratorMcpListScheduledTasksResult" => {
            round_trip::<crate::orchestration_mcp::OrchestratorMcpListScheduledTasksResult>(value)
        }
        "OrchestratorMcpProviderCapability" => {
            round_trip::<crate::provider_instance::OrchestratorMcpProviderCapability>(value)
        }
        "OrchestratorMcpRuntimeMode" => {
            round_trip::<crate::orchestration_mcp::OrchestratorMcpRuntimeMode>(value)
        }
        "OrchestratorMcpScheduleTaskInput" => {
            round_trip::<crate::orchestration_mcp::OrchestratorMcpScheduleTaskInput>(value)
        }
        "OrchestratorMcpScheduleTaskResult" => {
            round_trip::<crate::orchestration_mcp::OrchestratorMcpScheduleTaskResult>(value)
        }
        "OrchestratorMcpScheduledTask" => {
            round_trip::<crate::orchestration_mcp::OrchestratorMcpScheduledTask>(value)
        }
        "OrchestratorMcpTarget" => {
            round_trip::<crate::orchestration_mcp::OrchestratorMcpTarget>(value)
        }
        "OrchestratorMcpTargetOptions" => {
            round_trip::<crate::orchestration_mcp::OrchestratorMcpTargetOptions>(value)
        }
        "OrchestratorMcpTaskCancelInput" => {
            round_trip::<crate::orchestration_mcp::OrchestratorMcpTaskCancelInput>(value)
        }
        "OrchestratorMcpTaskCancelResult" => {
            round_trip::<crate::orchestration_mcp::OrchestratorMcpTaskCancelResult>(value)
        }
        "OrchestratorMcpTaskRole" => {
            round_trip::<crate::orchestration_mcp::OrchestratorMcpTaskRole>(value)
        }
        "OrchestratorMcpTaskStatusInput" => {
            round_trip::<crate::orchestration_mcp::OrchestratorMcpTaskStatusInput>(value)
        }
        "OrchestratorMcpTerminalDelegatedTaskStatus" => round_trip::<
            crate::orchestration_mcp::OrchestratorMcpTerminalDelegatedTaskStatus,
        >(value),
        "OrchestratorMcpThreadDetail" => {
            round_trip::<crate::orchestration_mcp::OrchestratorMcpThreadDetail>(value)
        }
        "OrchestratorMcpThreadInterruptInput" => {
            round_trip::<crate::orchestration_mcp::OrchestratorMcpThreadInterruptInput>(value)
        }
        "OrchestratorMcpThreadInterruptResult" => {
            round_trip::<crate::orchestration_mcp::OrchestratorMcpThreadInterruptResult>(value)
        }
        "OrchestratorMcpThreadListInput" => {
            round_trip::<crate::orchestration_mcp::OrchestratorMcpThreadListInput>(value)
        }
        "OrchestratorMcpThreadListItem" => {
            round_trip::<crate::orchestration_mcp::OrchestratorMcpThreadListItem>(value)
        }
        "OrchestratorMcpThreadListResult" => {
            round_trip::<crate::orchestration_mcp::OrchestratorMcpThreadListResult>(value)
        }
        "OrchestratorMcpThreadReadInput" => {
            round_trip::<crate::orchestration_mcp::OrchestratorMcpThreadReadInput>(value)
        }
        "OrchestratorMcpThreadReadResult" => {
            round_trip::<crate::orchestration_mcp::OrchestratorMcpThreadReadResult>(value)
        }
        "OrchestratorMcpThreadRun" => {
            round_trip::<crate::orchestration_mcp::OrchestratorMcpThreadRun>(value)
        }
        "OrchestratorMcpThreadSendInput" => {
            round_trip::<crate::orchestration_mcp::OrchestratorMcpThreadSendInput>(value)
        }
        "OrchestratorMcpThreadSendResult" => {
            round_trip::<crate::orchestration_mcp::OrchestratorMcpThreadSendResult>(value)
        }
        "OrchestratorMcpThreadStatus" => {
            round_trip::<crate::orchestration_mcp::OrchestratorMcpThreadStatus>(value)
        }
        "OrchestratorMcpThreadTimelineItem" => {
            round_trip::<crate::orchestration_mcp::OrchestratorMcpThreadTimelineItem>(value)
        }
        "OrchestratorMcpThreadWaitInput" => {
            round_trip::<crate::orchestration_mcp::OrchestratorMcpThreadWaitInput>(value)
        }
        "OrchestratorMcpThreadWaitResult" => {
            round_trip::<crate::orchestration_mcp::OrchestratorMcpThreadWaitResult>(value)
        }
        "OrchestratorMcpUpdateScheduledTaskInput" => {
            round_trip::<crate::orchestration_mcp::OrchestratorMcpUpdateScheduledTaskInput>(value)
        }
        "PastedTextAttachmentSource" => {
            round_trip::<crate::orchestration::PastedTextAttachmentSource>(value)
        }
        "PlanId" => round_trip::<crate::orchestration::PlanId>(value),
        "PositiveInt" => round_trip::<crate::orchestration::PositiveInt>(value),
        "PreviewAnnotationContextRecord" => {
            round_trip::<crate::orchestration::PreviewAnnotationContextRecord>(value)
        }
        "PreviewAutomationActionEvent" => {
            round_trip::<crate::orchestration_mcp::PreviewAutomationActionEvent>(value)
        }
        "PreviewAutomationClientDisconnectedError" => {
            round_trip::<crate::orchestration_mcp::PreviewAutomationClientDisconnectedError>(value)
        }
        "PreviewAutomationColorScheme" => {
            round_trip::<crate::orchestration_mcp::PreviewAutomationColorScheme>(value)
        }
        "PreviewAutomationConnectionId" => {
            round_trip::<crate::orchestration_mcp::PreviewAutomationConnectionId>(value)
        }
        "PreviewAutomationConsoleEntry" => {
            round_trip::<crate::orchestration_mcp::PreviewAutomationConsoleEntry>(value)
        }
        "PreviewAutomationControlInterruptedError" => {
            round_trip::<crate::orchestration_mcp::PreviewAutomationControlInterruptedError>(value)
        }
        "PreviewAutomationElement" => {
            round_trip::<crate::orchestration_mcp::PreviewAutomationElement>(value)
        }
        "PreviewAutomationError" => {
            round_trip::<crate::orchestration_mcp::PreviewAutomationError>(value)
        }
        "PreviewAutomationExecutionError" => {
            round_trip::<crate::orchestration_mcp::PreviewAutomationExecutionError>(value)
        }
        "PreviewAutomationInvalidSelectorError" => {
            round_trip::<crate::orchestration_mcp::PreviewAutomationInvalidSelectorError>(value)
        }
        "PreviewAutomationMalformedResponseError" => {
            round_trip::<crate::orchestration_mcp::PreviewAutomationMalformedResponseError>(value)
        }
        "PreviewAutomationNetworkEntry" => {
            round_trip::<crate::orchestration_mcp::PreviewAutomationNetworkEntry>(value)
        }
        "PreviewAutomationNoAvailableHostError" => {
            round_trip::<crate::orchestration_mcp::PreviewAutomationNoAvailableHostError>(value)
        }
        "PreviewAutomationOperation" => {
            round_trip::<crate::orchestration_mcp::PreviewAutomationOperation>(value)
        }
        "PreviewAutomationRecordingDeadlineExpiredError" => round_trip::<
            crate::orchestration_mcp::PreviewAutomationRecordingDeadlineExpiredError,
        >(value),
        "PreviewAutomationRecordingDesktopUpdateRequiredError" => round_trip::<
            crate::orchestration_mcp::PreviewAutomationRecordingDesktopUpdateRequiredError,
        >(value),
        "PreviewAutomationRecordingTooLargeError" => {
            round_trip::<crate::orchestration_mcp::PreviewAutomationRecordingTooLargeError>(value)
        }
        "PreviewAutomationRecordingTransferError" => {
            round_trip::<crate::orchestration_mcp::PreviewAutomationRecordingTransferError>(value)
        }
        "PreviewAutomationRemoteUnavailableError" => {
            round_trip::<crate::orchestration_mcp::PreviewAutomationRemoteUnavailableError>(value)
        }
        "PreviewAutomationRequestQueueClosedError" => {
            round_trip::<crate::orchestration_mcp::PreviewAutomationRequestQueueClosedError>(value)
        }
        "PreviewAutomationResultTooLargeError" => {
            round_trip::<crate::orchestration_mcp::PreviewAutomationResultTooLargeError>(value)
        }
        "PreviewAutomationSnapshot" => {
            round_trip::<crate::orchestration_mcp::PreviewAutomationSnapshot>(value)
        }
        "PreviewAutomationStatus" => {
            round_trip::<crate::orchestration_mcp::PreviewAutomationStatus>(value)
        }
        "PreviewAutomationTabNotFoundError" => {
            round_trip::<crate::orchestration_mcp::PreviewAutomationTabNotFoundError>(value)
        }
        "PreviewAutomationTargetNotEditableError" => {
            round_trip::<crate::orchestration_mcp::PreviewAutomationTargetNotEditableError>(value)
        }
        "PreviewAutomationTimeoutError" => {
            round_trip::<crate::orchestration_mcp::PreviewAutomationTimeoutError>(value)
        }
        "PreviewAutomationUnavailableError" => {
            round_trip::<crate::orchestration_mcp::PreviewAutomationUnavailableError>(value)
        }
        "PreviewAutomationUnsupportedClientError" => {
            round_trip::<crate::orchestration_mcp::PreviewAutomationUnsupportedClientError>(value)
        }
        "PreviewClickError" => round_trip::<crate::orchestration_mcp::PreviewClickError>(value),
        "PreviewClickInput" => round_trip::<crate::orchestration_mcp::PreviewClickInput>(value),
        "PreviewClickResult" => round_trip::<crate::orchestration_mcp::PreviewClickResult>(value),
        "PreviewEvaluateError" => {
            round_trip::<crate::orchestration_mcp::PreviewEvaluateError>(value)
        }
        "PreviewEvaluateInput" => {
            round_trip::<crate::orchestration_mcp::PreviewEvaluateInput>(value)
        }
        "PreviewEvaluateResult" => {
            round_trip::<crate::orchestration_mcp::PreviewEvaluateResult>(value)
        }
        "PreviewNavStatus" => round_trip::<crate::orchestration::PreviewNavStatus>(value),
        "PreviewNavigateError" => {
            round_trip::<crate::orchestration_mcp::PreviewNavigateError>(value)
        }
        "PreviewNavigateInput" => {
            round_trip::<crate::orchestration_mcp::PreviewNavigateInput>(value)
        }
        "PreviewNavigateResult" => {
            round_trip::<crate::orchestration_mcp::PreviewNavigateResult>(value)
        }
        "PreviewOpenError" => round_trip::<crate::orchestration_mcp::PreviewOpenError>(value),
        "PreviewOpenInput" => round_trip::<crate::orchestration_mcp::PreviewOpenInput>(value),
        "PreviewOpenResult" => round_trip::<crate::orchestration_mcp::PreviewOpenResult>(value),
        "PreviewPressError" => round_trip::<crate::orchestration_mcp::PreviewPressError>(value),
        "PreviewPressInput" => round_trip::<crate::orchestration_mcp::PreviewPressInput>(value),
        "PreviewPressResult" => round_trip::<crate::orchestration_mcp::PreviewPressResult>(value),
        "PreviewRecordingStartError" => {
            round_trip::<crate::orchestration_mcp::PreviewRecordingStartError>(value)
        }
        "PreviewRecordingStartInput" => {
            round_trip::<crate::orchestration_mcp::PreviewRecordingStartInput>(value)
        }
        "PreviewRecordingStartResult" => {
            round_trip::<crate::orchestration_mcp::PreviewRecordingStartResult>(value)
        }
        "PreviewRecordingStopError" => {
            round_trip::<crate::orchestration_mcp::PreviewRecordingStopError>(value)
        }
        "PreviewRecordingStopInput" => {
            round_trip::<crate::orchestration_mcp::PreviewRecordingStopInput>(value)
        }
        "PreviewRecordingStopResult" => {
            round_trip::<crate::orchestration_mcp::PreviewRecordingStopResult>(value)
        }
        "PreviewRenderedViewportSize" => {
            round_trip::<crate::orchestration::PreviewRenderedViewportSize>(value)
        }
        "PreviewResizeError" => round_trip::<crate::orchestration_mcp::PreviewResizeError>(value),
        "PreviewResizeInput" => round_trip::<crate::orchestration_mcp::PreviewResizeInput>(value),
        "PreviewResizeResult" => round_trip::<crate::orchestration_mcp::PreviewResizeResult>(value),
        "PreviewScrollError" => round_trip::<crate::orchestration_mcp::PreviewScrollError>(value),
        "PreviewScrollInput" => round_trip::<crate::orchestration_mcp::PreviewScrollInput>(value),
        "PreviewScrollResult" => round_trip::<crate::orchestration_mcp::PreviewScrollResult>(value),
        "PreviewSessionSnapshot" => {
            round_trip::<crate::orchestration::PreviewSessionSnapshot>(value)
        }
        "PreviewSetAppearanceError" => {
            round_trip::<crate::orchestration_mcp::PreviewSetAppearanceError>(value)
        }
        "PreviewSetAppearanceInput" => {
            round_trip::<crate::orchestration_mcp::PreviewSetAppearanceInput>(value)
        }
        "PreviewSetAppearanceResult" => {
            round_trip::<crate::orchestration_mcp::PreviewSetAppearanceResult>(value)
        }
        "PreviewSnapshotError" => {
            round_trip::<crate::orchestration_mcp::PreviewSnapshotError>(value)
        }
        "PreviewSnapshotInput" => {
            round_trip::<crate::orchestration_mcp::PreviewSnapshotInput>(value)
        }
        "PreviewSnapshotResult" => {
            round_trip::<crate::orchestration_mcp::PreviewSnapshotResult>(value)
        }
        "PreviewStatusError" => round_trip::<crate::orchestration_mcp::PreviewStatusError>(value),
        "PreviewStatusInput" => round_trip::<crate::orchestration_mcp::PreviewStatusInput>(value),
        "PreviewStatusResult" => round_trip::<crate::orchestration_mcp::PreviewStatusResult>(value),
        "PreviewTabId" => round_trip::<crate::orchestration::PreviewTabId>(value),
        "PreviewTypeError" => round_trip::<crate::orchestration_mcp::PreviewTypeError>(value),
        "PreviewTypeInput" => round_trip::<crate::orchestration_mcp::PreviewTypeInput>(value),
        "PreviewTypeResult" => round_trip::<crate::orchestration_mcp::PreviewTypeResult>(value),
        "PreviewViewportSetting" => {
            round_trip::<crate::orchestration::PreviewViewportSetting>(value)
        }
        "PreviewWaitForError" => round_trip::<crate::orchestration_mcp::PreviewWaitForError>(value),
        "PreviewWaitForInput" => round_trip::<crate::orchestration_mcp::PreviewWaitForInput>(value),
        "PreviewWaitForResult" => {
            round_trip::<crate::orchestration_mcp::PreviewWaitForResult>(value)
        }
        "Project" => round_trip::<crate::orchestration::Project>(value),
        "ProjectIconColor" => round_trip::<crate::orchestration::ProjectIconColor>(value),
        "ProjectIconOverride" => round_trip::<crate::orchestration::ProjectIconOverride>(value),
        "ProjectId" => round_trip::<crate::orchestration::ProjectId>(value),
        "ProjectMonogramText" => round_trip::<crate::orchestration::ProjectMonogramText>(value),
        "ProjectScript" => round_trip::<crate::orchestration::ProjectScript>(value),
        "ProjectScriptIcon" => round_trip::<crate::orchestration::ProjectScriptIcon>(value),
        "ProviderApprovalDecision" => {
            round_trip::<crate::orchestration::ProviderApprovalDecision>(value)
        }
        "ProviderApprovalOption" => {
            round_trip::<crate::orchestration::ProviderApprovalOption>(value)
        }
        "ProviderDriverKind" => round_trip::<crate::provider_instance::ProviderDriverKind>(value),
        "ProviderInstanceConfig" => {
            round_trip::<crate::provider_instance::ProviderInstanceConfig>(value)
        }
        "ProviderInstanceConfigMap" => {
            round_trip::<crate::provider_instance::ProviderInstanceConfigMap>(value)
        }
        "ProviderInstanceEnvironment" => {
            round_trip::<crate::provider_instance::ProviderInstanceEnvironment>(value)
        }
        "ProviderInstanceEnvironmentVariable" => {
            round_trip::<crate::provider_instance::ProviderInstanceEnvironmentVariable>(value)
        }
        "ProviderInstanceEnvironmentVariableName" => {
            round_trip::<crate::provider_instance::ProviderInstanceEnvironmentVariableName>(value)
        }
        "ProviderInstanceId" => round_trip::<crate::provider_instance::ProviderInstanceId>(value),
        "ProviderInstanceMutation" => {
            round_trip::<crate::provider_instance::ProviderInstanceMutation>(value)
        }
        "ProviderInstanceRef" => round_trip::<crate::provider_instance::ProviderInstanceRef>(value),
        "ProviderInteractionMode" => {
            round_trip::<crate::orchestration::ProviderInteractionMode>(value)
        }
        "ProviderOptionChoice" => {
            round_trip::<crate::provider_instance::ProviderOptionChoice>(value)
        }
        "ProviderOptionDescriptor" => {
            round_trip::<crate::provider_instance::ProviderOptionDescriptor>(value)
        }
        "ProviderOptionDescriptorType" => {
            round_trip::<crate::provider_instance::ProviderOptionDescriptorType>(value)
        }
        "ProviderOptionSelection" => {
            round_trip::<crate::provider_instance::ProviderOptionSelection>(value)
        }
        "ProviderOptionSelectionValue" => {
            round_trip::<crate::provider_instance::ProviderOptionSelectionValue>(value)
        }
        "ProviderOptionSelections" => {
            round_trip::<crate::provider_instance::ProviderOptionSelections>(value)
        }
        "ProviderReplayEntry" => round_trip::<crate::orchestration::ProviderReplayEntry>(value),
        "ProviderReplayNdjsonRecord" => {
            round_trip::<crate::orchestration::ProviderReplayNdjsonRecord>(value)
        }
        "ProviderReplayTranscript" => {
            round_trip::<crate::orchestration::ProviderReplayTranscript>(value)
        }
        "ProviderReplayTranscriptHeader" => {
            round_trip::<crate::orchestration::ProviderReplayTranscriptHeader>(value)
        }
        "ProviderRequestKind" => round_trip::<crate::orchestration::ProviderRequestKind>(value),
        "ProviderSessionId" => round_trip::<crate::orchestration::ProviderSessionId>(value),
        "ProviderThreadId" => round_trip::<crate::orchestration::ProviderThreadId>(value),
        "ProviderTurnId" => round_trip::<crate::orchestration::ProviderTurnId>(value),
        "ProviderUserInputAnswers" => {
            round_trip::<crate::orchestration::ProviderUserInputAnswers>(value)
        }
        "PullRequestActor" => round_trip::<crate::orchestration::PullRequestActor>(value),
        "PullRequestChecksState" => {
            round_trip::<crate::orchestration::PullRequestChecksState>(value)
        }
        "PullRequestContextMetadata" => {
            round_trip::<crate::orchestration::PullRequestContextMetadata>(value)
        }
        "PullRequestMergeability" => {
            round_trip::<crate::orchestration::PullRequestMergeability>(value)
        }
        "PullRequestReviewDecision" => {
            round_trip::<crate::orchestration::PullRequestReviewDecision>(value)
        }
        "PullRequestState" => round_trip::<crate::orchestration::PullRequestState>(value),
        "RawEventId" => round_trip::<crate::orchestration::RawEventId>(value),
        "RepositoryIdentity" => round_trip::<crate::orchestration::RepositoryIdentity>(value),
        "RepositoryIdentityLocator" => {
            round_trip::<crate::orchestration::RepositoryIdentityLocator>(value)
        }
        "ReviewCommentContextRecord" => {
            round_trip::<crate::orchestration::ReviewCommentContextRecord>(value)
        }
        "RunAttemptId" => round_trip::<crate::orchestration::RunAttemptId>(value),
        "RunId" => round_trip::<crate::orchestration::RunId>(value),
        "RunScheduledTaskNowError" => {
            round_trip::<crate::orchestration_mcp::RunScheduledTaskNowError>(value)
        }
        "RunScheduledTaskNowInput" => {
            round_trip::<crate::orchestration_mcp::RunScheduledTaskNowInput>(value)
        }
        "RunScheduledTaskNowResult" => {
            round_trip::<crate::orchestration_mcp::RunScheduledTaskNowResult>(value)
        }
        "RuntimeMode" => round_trip::<crate::orchestration::RuntimeMode>(value),
        "RuntimeRequestId" => round_trip::<crate::orchestration::RuntimeRequestId>(value),
        "ScheduleTaskError" => round_trip::<crate::orchestration_mcp::ScheduleTaskError>(value),
        "ScheduleTaskInput" => round_trip::<crate::orchestration_mcp::ScheduleTaskInput>(value),
        "ScheduleTaskResult" => round_trip::<crate::orchestration_mcp::ScheduleTaskResult>(value),
        "ScheduledTask" => round_trip::<crate::orchestration::ScheduledTask>(value),
        "ScheduledTaskDeleteInput" => {
            round_trip::<crate::orchestration::ScheduledTaskDeleteInput>(value)
        }
        "ScheduledTaskDeleteResult" => {
            round_trip::<crate::orchestration::ScheduledTaskDeleteResult>(value)
        }
        "ScheduledTaskError" => round_trip::<crate::orchestration::ScheduledTaskError>(value),
        "ScheduledTaskId" => round_trip::<crate::orchestration::ScheduledTaskId>(value),
        "ScheduledTaskListInput" => {
            round_trip::<crate::orchestration::ScheduledTaskListInput>(value)
        }
        "ScheduledTaskListResult" => {
            round_trip::<crate::orchestration::ScheduledTaskListResult>(value)
        }
        "ScheduledTaskMutationResult" => {
            round_trip::<crate::orchestration::ScheduledTaskMutationResult>(value)
        }
        "ScheduledTaskRunNowInput" => {
            round_trip::<crate::orchestration::ScheduledTaskRunNowInput>(value)
        }
        "ScheduledTaskRunNowResult" => {
            round_trip::<crate::orchestration::ScheduledTaskRunNowResult>(value)
        }
        "ScheduledTaskRunStatus" => {
            round_trip::<crate::orchestration::ScheduledTaskRunStatus>(value)
        }
        "ScheduledTaskSchedule" => round_trip::<crate::orchestration::ScheduledTaskSchedule>(value),
        "ScheduledTaskSetEnabledInput" => {
            round_trip::<crate::orchestration::ScheduledTaskSetEnabledInput>(value)
        }
        "ScheduledTaskUpsertInput" => {
            round_trip::<crate::orchestration::ScheduledTaskUpsertInput>(value)
        }
        "ScheduledTaskUpsertSchedule" => {
            round_trip::<crate::orchestration::ScheduledTaskUpsertSchedule>(value)
        }
        "SelectProviderOptionDescriptor" => {
            round_trip::<crate::provider_instance::SelectProviderOptionDescriptor>(value)
        }
        "SkillContextRecord" => round_trip::<crate::orchestration::SkillContextRecord>(value),
        "SnapShotAccessibility" => round_trip::<crate::orchestration::SnapShotAccessibility>(value),
        "SnapShotAccessibilityNode" => {
            round_trip::<crate::orchestration::SnapShotAccessibilityNode>(value)
        }
        "SnapShotSource" => round_trip::<crate::orchestration::SnapShotSource>(value),
        "SourceControlCloneProtocol" => {
            round_trip::<crate::orchestration::SourceControlCloneProtocol>(value)
        }
        "SourceControlCloneRepositoryResult" => {
            round_trip::<crate::orchestration::SourceControlCloneRepositoryResult>(value)
        }
        "SourceControlProviderKind" => {
            round_trip::<crate::orchestration::SourceControlProviderKind>(value)
        }
        "SourceControlRepositoryInfo" => {
            round_trip::<crate::orchestration::SourceControlRepositoryInfo>(value)
        }
        "T3AttachmentDiscardError" => {
            round_trip::<crate::orchestration_mcp::T3AttachmentDiscardError>(value)
        }
        "T3AttachmentDiscardInput" => {
            round_trip::<crate::orchestration_mcp::T3AttachmentDiscardInput>(value)
        }
        "T3AttachmentDiscardResult" => {
            round_trip::<crate::orchestration_mcp::T3AttachmentDiscardResult>(value)
        }
        "T3AttachmentPrepareUploadError" => {
            round_trip::<crate::orchestration_mcp::T3AttachmentPrepareUploadError>(value)
        }
        "T3AttachmentPrepareUploadInput" => {
            round_trip::<crate::orchestration_mcp::T3AttachmentPrepareUploadInput>(value)
        }
        "T3AttachmentPrepareUploadResult" => {
            round_trip::<crate::orchestration_mcp::T3AttachmentPrepareUploadResult>(value)
        }
        "T3EnvironmentPreferencesUpdateError" => {
            round_trip::<crate::orchestration_mcp::T3EnvironmentPreferencesUpdateError>(value)
        }
        "T3EnvironmentPreferencesUpdateInput" => {
            round_trip::<crate::orchestration_mcp::T3EnvironmentPreferencesUpdateInput>(value)
        }
        "T3EnvironmentPreferencesUpdateResult" => {
            round_trip::<crate::orchestration_mcp::T3EnvironmentPreferencesUpdateResult>(value)
        }
        "T3EnvironmentReadError" => {
            round_trip::<crate::orchestration_mcp::T3EnvironmentReadError>(value)
        }
        "T3EnvironmentReadInput" => {
            round_trip::<crate::orchestration_mcp::T3EnvironmentReadInput>(value)
        }
        "T3EnvironmentReadResult" => {
            round_trip::<crate::orchestration_mcp::T3EnvironmentReadResult>(value)
        }
        "T3PendingRequestListError" => {
            round_trip::<crate::orchestration_mcp::T3PendingRequestListError>(value)
        }
        "T3PendingRequestListInput" => {
            round_trip::<crate::orchestration_mcp::T3PendingRequestListInput>(value)
        }
        "T3PendingRequestListResult" => {
            round_trip::<crate::orchestration_mcp::T3PendingRequestListResult>(value)
        }
        "T3PendingRequestReadError" => {
            round_trip::<crate::orchestration_mcp::T3PendingRequestReadError>(value)
        }
        "T3PendingRequestReadInput" => {
            round_trip::<crate::orchestration_mcp::T3PendingRequestReadInput>(value)
        }
        "T3PendingRequestReadResult" => {
            round_trip::<crate::orchestration_mcp::T3PendingRequestReadResult>(value)
        }
        "T3PendingRequestRespondError" => {
            round_trip::<crate::orchestration_mcp::T3PendingRequestRespondError>(value)
        }
        "T3PendingRequestRespondInput" => {
            round_trip::<crate::orchestration_mcp::T3PendingRequestRespondInput>(value)
        }
        "T3PendingRequestRespondResult" => {
            round_trip::<crate::orchestration_mcp::T3PendingRequestRespondResult>(value)
        }
        "T3PreviewCloseError" => round_trip::<crate::orchestration_mcp::T3PreviewCloseError>(value),
        "T3PreviewCloseInput" => round_trip::<crate::orchestration_mcp::T3PreviewCloseInput>(value),
        "T3PreviewCloseResult" => {
            round_trip::<crate::orchestration_mcp::T3PreviewCloseResult>(value)
        }
        "T3PreviewListError" => round_trip::<crate::orchestration_mcp::T3PreviewListError>(value),
        "T3PreviewListInput" => round_trip::<crate::orchestration_mcp::T3PreviewListInput>(value),
        "T3PreviewListResult" => round_trip::<crate::orchestration_mcp::T3PreviewListResult>(value),
        "T3ProjectCloneError" => round_trip::<crate::orchestration_mcp::T3ProjectCloneError>(value),
        "T3ProjectCloneInput" => round_trip::<crate::orchestration_mcp::T3ProjectCloneInput>(value),
        "T3ProjectCloneResult" => {
            round_trip::<crate::orchestration_mcp::T3ProjectCloneResult>(value)
        }
        "T3ProjectCreateError" => {
            round_trip::<crate::orchestration_mcp::T3ProjectCreateError>(value)
        }
        "T3ProjectCreateInput" => {
            round_trip::<crate::orchestration_mcp::T3ProjectCreateInput>(value)
        }
        "T3ProjectCreateResult" => {
            round_trip::<crate::orchestration_mcp::T3ProjectCreateResult>(value)
        }
        "T3ProjectDeleteError" => {
            round_trip::<crate::orchestration_mcp::T3ProjectDeleteError>(value)
        }
        "T3ProjectDeleteInput" => {
            round_trip::<crate::orchestration_mcp::T3ProjectDeleteInput>(value)
        }
        "T3ProjectDeleteResult" => {
            round_trip::<crate::orchestration_mcp::T3ProjectDeleteResult>(value)
        }
        "T3ProjectListError" => round_trip::<crate::orchestration_mcp::T3ProjectListError>(value),
        "T3ProjectListInput" => round_trip::<crate::orchestration_mcp::T3ProjectListInput>(value),
        "T3ProjectListResult" => round_trip::<crate::orchestration_mcp::T3ProjectListResult>(value),
        "T3ProjectReadError" => round_trip::<crate::orchestration_mcp::T3ProjectReadError>(value),
        "T3ProjectReadInput" => round_trip::<crate::orchestration_mcp::T3ProjectReadInput>(value),
        "T3ProjectReadResult" => round_trip::<crate::orchestration_mcp::T3ProjectReadResult>(value),
        "T3ProjectUpdateError" => {
            round_trip::<crate::orchestration_mcp::T3ProjectUpdateError>(value)
        }
        "T3ProjectUpdateInput" => {
            round_trip::<crate::orchestration_mcp::T3ProjectUpdateInput>(value)
        }
        "T3ProjectUpdateResult" => {
            round_trip::<crate::orchestration_mcp::T3ProjectUpdateResult>(value)
        }
        "T3QueueCancelError" => round_trip::<crate::orchestration_mcp::T3QueueCancelError>(value),
        "T3QueueCancelInput" => round_trip::<crate::orchestration_mcp::T3QueueCancelInput>(value),
        "T3QueueCancelResult" => round_trip::<crate::orchestration_mcp::T3QueueCancelResult>(value),
        "T3QueueEditError" => round_trip::<crate::orchestration_mcp::T3QueueEditError>(value),
        "T3QueueEditInput" => round_trip::<crate::orchestration_mcp::T3QueueEditInput>(value),
        "T3QueueEditResult" => round_trip::<crate::orchestration_mcp::T3QueueEditResult>(value),
        "T3QueueListError" => round_trip::<crate::orchestration_mcp::T3QueueListError>(value),
        "T3QueueListInput" => round_trip::<crate::orchestration_mcp::T3QueueListInput>(value),
        "T3QueueListResult" => round_trip::<crate::orchestration_mcp::T3QueueListResult>(value),
        "T3QueuePromoteToSteerError" => {
            round_trip::<crate::orchestration_mcp::T3QueuePromoteToSteerError>(value)
        }
        "T3QueuePromoteToSteerInput" => {
            round_trip::<crate::orchestration_mcp::T3QueuePromoteToSteerInput>(value)
        }
        "T3QueuePromoteToSteerResult" => {
            round_trip::<crate::orchestration_mcp::T3QueuePromoteToSteerResult>(value)
        }
        "T3QueueReadError" => round_trip::<crate::orchestration_mcp::T3QueueReadError>(value),
        "T3QueueReadInput" => round_trip::<crate::orchestration_mcp::T3QueueReadInput>(value),
        "T3QueueReadResult" => round_trip::<crate::orchestration_mcp::T3QueueReadResult>(value),
        "T3QueueReorderError" => round_trip::<crate::orchestration_mcp::T3QueueReorderError>(value),
        "T3QueueReorderInput" => round_trip::<crate::orchestration_mcp::T3QueueReorderInput>(value),
        "T3QueueReorderResult" => {
            round_trip::<crate::orchestration_mcp::T3QueueReorderResult>(value)
        }
        "T3ThreadConfigurationError" => {
            round_trip::<crate::orchestration_mcp::T3ThreadConfigurationError>(value)
        }
        "T3ThreadConfigurationInput" => {
            round_trip::<crate::orchestration_mcp::T3ThreadConfigurationInput>(value)
        }
        "T3ThreadConfigurationResult" => {
            round_trip::<crate::orchestration_mcp::T3ThreadConfigurationResult>(value)
        }
        "T3ThreadConfigureError" => {
            round_trip::<crate::orchestration_mcp::T3ThreadConfigureError>(value)
        }
        "T3ThreadConfigureInput" => {
            round_trip::<crate::orchestration_mcp::T3ThreadConfigureInput>(value)
        }
        "T3ThreadConfigureResult" => {
            round_trip::<crate::orchestration_mcp::T3ThreadConfigureResult>(value)
        }
        "T3ThreadForkError" => round_trip::<crate::orchestration_mcp::T3ThreadForkError>(value),
        "T3ThreadForkInput" => round_trip::<crate::orchestration_mcp::T3ThreadForkInput>(value),
        "T3ThreadForkResult" => round_trip::<crate::orchestration_mcp::T3ThreadForkResult>(value),
        "T3ThreadInterruptError" => {
            round_trip::<crate::orchestration_mcp::T3ThreadInterruptError>(value)
        }
        "T3ThreadInterruptInput" => {
            round_trip::<crate::orchestration_mcp::T3ThreadInterruptInput>(value)
        }
        "T3ThreadInterruptResult" => {
            round_trip::<crate::orchestration_mcp::T3ThreadInterruptResult>(value)
        }
        "T3ThreadLaunchError" => round_trip::<crate::orchestration_mcp::T3ThreadLaunchError>(value),
        "T3ThreadLaunchInput" => round_trip::<crate::orchestration_mcp::T3ThreadLaunchInput>(value),
        "T3ThreadLaunchResult" => {
            round_trip::<crate::orchestration_mcp::T3ThreadLaunchResult>(value)
        }
        "T3ThreadListError" => round_trip::<crate::orchestration_mcp::T3ThreadListError>(value),
        "T3ThreadListInput" => round_trip::<crate::orchestration_mcp::T3ThreadListInput>(value),
        "T3ThreadListResult" => round_trip::<crate::orchestration_mcp::T3ThreadListResult>(value),
        "T3ThreadMergeBackError" => {
            round_trip::<crate::orchestration_mcp::T3ThreadMergeBackError>(value)
        }
        "T3ThreadMergeBackInput" => {
            round_trip::<crate::orchestration_mcp::T3ThreadMergeBackInput>(value)
        }
        "T3ThreadMergeBackResult" => {
            round_trip::<crate::orchestration_mcp::T3ThreadMergeBackResult>(value)
        }
        "T3ThreadOrganizeError" => {
            round_trip::<crate::orchestration_mcp::T3ThreadOrganizeError>(value)
        }
        "T3ThreadOrganizeInput" => {
            round_trip::<crate::orchestration_mcp::T3ThreadOrganizeInput>(value)
        }
        "T3ThreadOrganizeResult" => {
            round_trip::<crate::orchestration_mcp::T3ThreadOrganizeResult>(value)
        }
        "T3ThreadReadError" => round_trip::<crate::orchestration_mcp::T3ThreadReadError>(value),
        "T3ThreadReadInput" => round_trip::<crate::orchestration_mcp::T3ThreadReadInput>(value),
        "T3ThreadReadResult" => round_trip::<crate::orchestration_mcp::T3ThreadReadResult>(value),
        "T3ThreadSearchError" => round_trip::<crate::orchestration_mcp::T3ThreadSearchError>(value),
        "T3ThreadSearchInput" => round_trip::<crate::orchestration_mcp::T3ThreadSearchInput>(value),
        "T3ThreadSearchResult" => {
            round_trip::<crate::orchestration_mcp::T3ThreadSearchResult>(value)
        }
        "T3ThreadSendAttachmentsError" => {
            round_trip::<crate::orchestration_mcp::T3ThreadSendAttachmentsError>(value)
        }
        "T3ThreadSendAttachmentsInput" => {
            round_trip::<crate::orchestration_mcp::T3ThreadSendAttachmentsInput>(value)
        }
        "T3ThreadSendAttachmentsResult" => {
            round_trip::<crate::orchestration_mcp::T3ThreadSendAttachmentsResult>(value)
        }
        "T3ThreadSendError" => round_trip::<crate::orchestration_mcp::T3ThreadSendError>(value),
        "T3ThreadSendInput" => round_trip::<crate::orchestration_mcp::T3ThreadSendInput>(value),
        "T3ThreadSendResult" => round_trip::<crate::orchestration_mcp::T3ThreadSendResult>(value),
        "T3ThreadTransfersError" => {
            round_trip::<crate::orchestration_mcp::T3ThreadTransfersError>(value)
        }
        "T3ThreadTransfersInput" => {
            round_trip::<crate::orchestration_mcp::T3ThreadTransfersInput>(value)
        }
        "T3ThreadTransfersResult" => {
            round_trip::<crate::orchestration_mcp::T3ThreadTransfersResult>(value)
        }
        "T3ThreadUpdateError" => round_trip::<crate::orchestration_mcp::T3ThreadUpdateError>(value),
        "T3ThreadUpdateInput" => round_trip::<crate::orchestration_mcp::T3ThreadUpdateInput>(value),
        "T3ThreadUpdateResult" => {
            round_trip::<crate::orchestration_mcp::T3ThreadUpdateResult>(value)
        }
        "T3ThreadWaitError" => round_trip::<crate::orchestration_mcp::T3ThreadWaitError>(value),
        "T3ThreadWaitInput" => round_trip::<crate::orchestration_mcp::T3ThreadWaitInput>(value),
        "T3ThreadWaitResult" => round_trip::<crate::orchestration_mcp::T3ThreadWaitResult>(value),
        "T3WorktreeHandoffError" => {
            round_trip::<crate::orchestration_mcp::T3WorktreeHandoffError>(value)
        }
        "T3WorktreeHandoffInput" => {
            round_trip::<crate::orchestration_mcp::T3WorktreeHandoffInput>(value)
        }
        "T3WorktreeHandoffResult" => {
            round_trip::<crate::orchestration_mcp::T3WorktreeHandoffResult>(value)
        }
        "T3WorktreeListError" => round_trip::<crate::orchestration_mcp::T3WorktreeListError>(value),
        "T3WorktreeListInput" => round_trip::<crate::orchestration_mcp::T3WorktreeListInput>(value),
        "T3WorktreeListResult" => {
            round_trip::<crate::orchestration_mcp::T3WorktreeListResult>(value)
        }
        "T3WorktreeStatusError" => {
            round_trip::<crate::orchestration_mcp::T3WorktreeStatusError>(value)
        }
        "T3WorktreeStatusInput" => {
            round_trip::<crate::orchestration_mcp::T3WorktreeStatusInput>(value)
        }
        "T3WorktreeStatusResult" => {
            round_trip::<crate::orchestration_mcp::T3WorktreeStatusResult>(value)
        }
        "TaskCancelError" => round_trip::<crate::orchestration_mcp::TaskCancelError>(value),
        "TaskCancelInput" => round_trip::<crate::orchestration_mcp::TaskCancelInput>(value),
        "TaskCancelResult" => round_trip::<crate::orchestration_mcp::TaskCancelResult>(value),
        "TaskStatusError" => round_trip::<crate::orchestration_mcp::TaskStatusError>(value),
        "TaskStatusInput" => round_trip::<crate::orchestration_mcp::TaskStatusInput>(value),
        "TaskStatusResult" => round_trip::<crate::orchestration_mcp::TaskStatusResult>(value),
        "TerminalContextRecord" => round_trip::<crate::orchestration::TerminalContextRecord>(value),
        "ThreadContextRecord" => round_trip::<crate::orchestration::ThreadContextRecord>(value),
        "ThreadEnvMode" => round_trip::<crate::orchestration::ThreadEnvMode>(value),
        "ThreadId" => round_trip::<crate::orchestration::ThreadId>(value),
        "ThreadLinkedPullRequest" => {
            round_trip::<crate::orchestration::ThreadLinkedPullRequest>(value)
        }
        "ThreadMetadataMcpAction" => {
            round_trip::<crate::orchestration_mcp::ThreadMetadataMcpAction>(value)
        }
        "ThreadMetadataMcpUpdateResult" => {
            round_trip::<crate::orchestration_mcp::ThreadMetadataMcpUpdateResult>(value)
        }
        "ThreadPullRequestKey" => round_trip::<crate::orchestration::ThreadPullRequestKey>(value),
        "ThreadPullRequestLink" => round_trip::<crate::orchestration::ThreadPullRequestLink>(value),
        "ThreadPullRequestLinkSource" => {
            round_trip::<crate::orchestration::ThreadPullRequestLinkSource>(value)
        }
        "ThreadPullRequestSnapshot" => {
            round_trip::<crate::orchestration::ThreadPullRequestSnapshot>(value)
        }
        "ThreadPullRequestStack" => {
            round_trip::<crate::orchestration::ThreadPullRequestStack>(value)
        }
        "ThreadPullRequestStackLayer" => {
            round_trip::<crate::orchestration::ThreadPullRequestStackLayer>(value)
        }
        "ThreadPullRequestWatch" => {
            round_trip::<crate::orchestration::ThreadPullRequestWatch>(value)
        }
        "ThreadTitleRegeneration" => {
            round_trip::<crate::orchestration::ThreadTitleRegeneration>(value)
        }
        "ThreadTokenUsageSnapshot" => {
            round_trip::<crate::orchestration::ThreadTokenUsageSnapshot>(value)
        }
        "ToolActivityIcon" => round_trip::<crate::orchestration::ToolActivityIcon>(value),
        "ToolActivityNativeAppReference" => {
            round_trip::<crate::orchestration::ToolActivityNativeAppReference>(value)
        }
        "ToolActivitySource" => round_trip::<crate::orchestration::ToolActivitySource>(value),
        "ToolActivitySurface" => round_trip::<crate::orchestration::ToolActivitySurface>(value),
        "ToolFrameworkAiError" => {
            round_trip::<crate::orchestration_mcp::ToolFrameworkAiError>(value)
        }
        "ToolFrameworkExecutionFailure" => {
            round_trip::<crate::orchestration_mcp::ToolFrameworkExecutionFailure>(value)
        }
        "TrimmedNonEmptyString" => round_trip::<crate::orchestration::TrimmedNonEmptyString>(value),
        "TurnItemId" => round_trip::<crate::orchestration::TurnItemId>(value),
        "TurnTokenUsage" => round_trip::<crate::orchestration::TurnTokenUsage>(value),
        "UnknownContextRecord" => round_trip::<crate::orchestration::UnknownContextRecord>(value),
        "UnlinkPullRequestError" => {
            round_trip::<crate::orchestration_mcp::UnlinkPullRequestError>(value)
        }
        "UnlinkPullRequestInput" => {
            round_trip::<crate::orchestration_mcp::UnlinkPullRequestInput>(value)
        }
        "UnlinkPullRequestResult" => {
            round_trip::<crate::orchestration_mcp::UnlinkPullRequestResult>(value)
        }
        "UnwatchPullRequestError" => {
            round_trip::<crate::orchestration_mcp::UnwatchPullRequestError>(value)
        }
        "UnwatchPullRequestInput" => {
            round_trip::<crate::orchestration_mcp::UnwatchPullRequestInput>(value)
        }
        "UnwatchPullRequestResult" => {
            round_trip::<crate::orchestration_mcp::UnwatchPullRequestResult>(value)
        }
        "UpdateScheduledTaskError" => {
            round_trip::<crate::orchestration_mcp::UpdateScheduledTaskError>(value)
        }
        "UpdateScheduledTaskInput" => {
            round_trip::<crate::orchestration_mcp::UpdateScheduledTaskInput>(value)
        }
        "UpdateScheduledTaskResult" => {
            round_trip::<crate::orchestration_mcp::UpdateScheduledTaskResult>(value)
        }
        "UserInputAttachmentAnswerPayload" => {
            round_trip::<crate::orchestration::UserInputAttachmentAnswerPayload>(value)
        }
        "UserInputAttachments" => round_trip::<crate::orchestration::UserInputAttachments>(value),
        "VcsListRefsResult" => round_trip::<crate::orchestration::VcsListRefsResult>(value),
        "VcsRef" => round_trip::<crate::orchestration::VcsRef>(value),
        "WatchPullRequestError" => {
            round_trip::<crate::orchestration_mcp::WatchPullRequestError>(value)
        }
        "WatchPullRequestInput" => {
            round_trip::<crate::orchestration_mcp::WatchPullRequestInput>(value)
        }
        "WatchPullRequestResult" => {
            round_trip::<crate::orchestration_mcp::WatchPullRequestResult>(value)
        }
        "WorktreeMcpContinuationStatus" => {
            round_trip::<crate::orchestration_mcp::WorktreeMcpContinuationStatus>(value)
        }
        "WorktreeMcpFailure" => round_trip::<crate::orchestration_mcp::WorktreeMcpFailure>(value),
        "WorktreeMcpHandoffResult" => {
            round_trip::<crate::orchestration_mcp::WorktreeMcpHandoffResult>(value)
        }
        "WorktreeMcpSetupScriptStatus" => {
            round_trip::<crate::orchestration_mcp::WorktreeMcpSetupScriptStatus>(value)
        }
        "WorktreeMcpStatusResult" => {
            round_trip::<crate::orchestration_mcp::WorktreeMcpStatusResult>(value)
        }
        _ => panic!("unmapped oracle contract: {name}"),
    }
}

// Copyright 2021 Datafuse Labs
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::fmt::Display;
use std::fmt::Formatter;
use std::sync::Arc;

use databend_common_ast::ast::ExplainKind;
use databend_common_catalog::query_kind::QueryKind;
use databend_common_expression::DataField;
use databend_common_expression::DataSchema;
use databend_common_expression::DataSchemaRef;
use databend_common_expression::DataSchemaRefExt;
use databend_common_expression::types::DataType;
use educe::Educe;

pub trait PlanQueryBindContext: Clone {
    fn output_schema(&self) -> DataSchemaRef;
}

pub trait PlanQuerySExpr: Clone {
    fn has_merge_exchange(&self) -> bool;
    fn first_child(&self) -> Option<Self>;
}

#[derive(Educe)]
#[educe(
    Clone(bound = false, attrs = "#[recursive::recursive]"),
    Debug(bound = false, attrs = "#[recursive::recursive]")
)]
pub enum GenericPlan<
    SExpr: Clone + std::fmt::Debug,
    Metadata: Clone + std::fmt::Debug,
    BindContext: Clone + std::fmt::Debug,
    ExplainConfig: Clone + std::fmt::Debug,
    RewriteKind: Clone + std::fmt::Debug,
    ScalarExpr: Clone + std::fmt::Debug,
    ReclusterPlan: Clone + std::fmt::Debug,
>
{
    Query {
        s_expr: Box<SExpr>,
        metadata: Metadata,
        bind_context: Box<BindContext>,
        rewrite_kind: Option<RewriteKind>,
        formatted_ast: Option<String>,
        ignore_result: bool,
    },

    Explain {
        kind: ExplainKind,
        config: ExplainConfig,
        plan: Box<Self>,
    },
    ExplainAst {
        formatted_string: String,
    },
    ExplainSyntax {
        formatted_sql: String,
    },
    ExplainAnalyze {
        partial: bool,
        graphical: bool,
        plan: Box<Self>,
    },
    ExplainPerf {
        sql: String,
        event_groups: Vec<Vec<String>>,
    },
    ReportIssue(String),

    ShowCreateCatalog(Box<crate::ShowCreateCatalogPlan>),
    CreateCatalog(Box<crate::CreateCatalogPlan>),
    DropCatalog(Box<crate::DropCatalogPlan>),
    UseCatalog(Box<crate::UseCatalogPlan>),

    ShowOnlineNodes,
    ShowWarehouses,
    UseWarehouse(Box<crate::UseWarehousePlan>),
    CreateWarehouse(Box<crate::CreateWarehousePlan>),
    DropWarehouse(Box<crate::DropWarehousePlan>),
    ResumeWarehouse(Box<crate::ResumeWarehousePlan>),
    SuspendWarehouse(Box<crate::SuspendWarehousePlan>),
    RenameWarehouse(Box<crate::RenameWarehousePlan>),
    InspectWarehouse(Box<crate::InspectWarehousePlan>),
    AddWarehouseCluster(Box<crate::AddWarehouseClusterPlan>),
    DropWarehouseCluster(Box<crate::DropWarehouseClusterPlan>),
    RenameWarehouseCluster(Box<crate::RenameWarehouseClusterPlan>),
    AssignWarehouseNodes(Box<crate::AssignWarehouseNodesPlan>),
    UnassignWarehouseNodes(Box<crate::UnassignWarehouseNodesPlan>),

    ShowWorkers,
    CreateWorker(Box<crate::CreateWorkerPlan>),
    AlterWorker(Box<crate::AlterWorkerPlan>),
    DropWorker(Box<crate::DropWorkerPlan>),

    ShowWorkloadGroups,
    CreateWorkloadGroup(Box<crate::CreateWorkloadGroupPlan>),
    DropWorkloadGroup(Box<crate::DropWorkloadGroupPlan>),
    RenameWorkloadGroup(Box<crate::RenameWorkloadGroupPlan>),
    SetWorkloadGroupQuotas(Box<crate::SetWorkloadGroupQuotasPlan>),
    UnsetWorkloadGroupQuotas(Box<crate::UnsetWorkloadGroupQuotasPlan>),

    ShowCreateDatabase(Box<crate::ShowCreateDatabasePlan>),
    CreateDatabase(Box<crate::CreateDatabasePlan>),
    DropDatabase(Box<crate::DropDatabasePlan>),
    UndropDatabase(Box<crate::UndropDatabasePlan>),
    RenameDatabase(Box<crate::RenameDatabasePlan>),
    UseDatabase(Box<crate::UseDatabasePlan>),
    RefreshDatabaseCache(Box<crate::RefreshDatabaseCachePlan>),
    AlterDatabase(Box<crate::AlterDatabasePlan>),

    ShowCreateTable(Box<crate::ShowCreateTablePlan>),
    DescribeTable(Box<crate::DescribeTablePlan>),
    CreateTable(Box<crate::GenericCreateTablePlan<Self>>),
    DropTable(Box<crate::DropTablePlan>),
    UndropTable(Box<crate::UndropTablePlan>),
    RenameTable(Box<crate::RenameTablePlan>),
    SwapTable(Box<crate::SwapTablePlan>),
    ModifyTableComment(Box<crate::ModifyTableCommentPlan>),
    RenameTableColumn(Box<crate::RenameTableColumnPlan>),
    AddTableColumn(Box<crate::AddTableColumnPlan>),
    DropTableColumn(Box<crate::DropTableColumnPlan>),
    ModifyTableColumn(Box<crate::ModifyTableColumnPlan>),
    AddTableConstraint(Box<crate::AddTableConstraintPlan>),
    DropTableConstraint(Box<crate::DropTableConstraintPlan>),
    AlterTableClusterKey(Box<crate::AlterTableClusterKeyPlan>),
    DropTableClusterKey(Box<crate::DropTableClusterKeyPlan>),
    ReclusterTable(Box<ReclusterPlan>),
    RevertTable(Box<crate::RevertTablePlan>),
    TruncateTable(Box<crate::TruncateTablePlan>),
    VacuumTable(Box<crate::VacuumTablePlan>),
    VacuumDropTable(Box<crate::VacuumDropTablePlan>),
    VacuumTemporaryFiles(Box<crate::VacuumTemporaryFilesPlan>),
    AnalyzeTable(Box<crate::AnalyzeTablePlan>),
    ExistsTable(Box<crate::ExistsTablePlan>),
    SetOptions(Box<crate::SetOptionsPlan>),
    UnsetOptions(Box<crate::UnsetOptionsPlan>),
    RefreshTableCache(Box<crate::RefreshTableCachePlan>),
    ModifyTableConnection(Box<crate::ModifyTableConnectionPlan>),
    AddTableRowAccessPolicy(Box<crate::AddTableRowAccessPolicyPlan>),
    DropTableRowAccessPolicy(Box<crate::DropTableRowAccessPolicyPlan>),
    DropAllTableRowAccessPolicies(Box<crate::DropAllTableRowAccessPoliciesPlan>),
    CreateTableBranch(Box<crate::CreateTableBranchPlan>),
    CreateTableTag(Box<crate::CreateTableTagPlan>),
    DropTableBranch(Box<crate::DropTableBranchPlan>),
    DropTableTag(Box<crate::DropTableTagPlan>),

    OptimizePurge(Box<crate::OptimizePurgePlan>),
    OptimizeCompactSegment(Box<crate::OptimizeCompactSegmentPlan>),
    OptimizeCompactBlock {
        s_expr: Box<SExpr>,
        need_purge: bool,
    },

    Insert(Box<crate::GenericInsert<Self>>),
    InsertMultiTable(Box<crate::GenericInsertMultiTable<Self, ScalarExpr, Metadata>>),
    Replace(Box<crate::GenericReplace<Self>>),
    DataMutation {
        s_expr: Box<SExpr>,
        schema: DataSchemaRef,
        metadata: Metadata,
    },

    CopyIntoTable(Box<crate::GenericCopyIntoTablePlan<Self>>),
    CopyIntoLocation(Box<crate::GenericCopyIntoLocationPlan<Self, ScalarExpr>>),

    CreateView(Box<crate::CreateViewPlan>),
    AlterView(Box<crate::AlterViewPlan>),
    DropView(Box<crate::DropViewPlan>),
    DescribeView(Box<crate::DescribeViewPlan>),

    CreateStream(Box<crate::CreateStreamPlan>),
    DropStream(Box<crate::DropStreamPlan>),

    CreateIndex(Box<crate::CreateIndexPlan>),
    DropIndex(Box<crate::DropIndexPlan>),
    RefreshIndex(Box<crate::GenericRefreshIndexPlan<Self>>),
    CreateTableIndex(Box<crate::CreateTableIndexPlan>),
    DropTableIndex(Box<crate::DropTableIndexPlan>),
    RefreshTableIndex(Box<crate::RefreshTableIndexPlan>),

    RefreshVirtualColumn(Box<crate::RefreshVirtualColumnPlan>),
    VacuumVirtualColumn(Box<crate::VacuumVirtualColumnPlan>),

    AlterUser(Box<crate::AlterUserPlan>),
    CreateUser(Box<crate::CreateUserPlan>),
    DropUser(Box<crate::DropUserPlan>),
    DescUser(Box<crate::DescUserPlan>),

    CreateUDF(Box<crate::CreateUDFPlan>),
    AlterUDF(Box<crate::AlterUDFPlan>),
    DropUDF(Box<crate::DropUDFPlan>),

    CreateRowAccessPolicy(Box<crate::CreateRowAccessPolicyPlan>),
    DropRowAccessPolicy(Box<crate::DropRowAccessPolicyPlan>),
    DescRowAccessPolicy(Box<crate::DescRowAccessPolicyPlan>),

    CreateRole(Box<crate::CreateRolePlan>),
    DropRole(Box<crate::DropRolePlan>),
    AlterRole(Box<crate::AlterRolePlan>),
    GrantRole(Box<crate::GrantRolePlan>),
    GrantPriv(Box<crate::GrantPrivilegePlan>),
    RevokePriv(Box<crate::RevokePrivilegePlan>),
    RevokeRole(Box<crate::RevokeRolePlan>),
    SetRole(Box<crate::SetRolePlan>),
    SetSecondaryRoles(Box<crate::SetSecondaryRolesPlan>),

    CreateFileFormat(Box<crate::CreateFileFormatPlan>),
    DropFileFormat(Box<crate::DropFileFormatPlan>),
    ShowFileFormats(Box<crate::ShowFileFormatsPlan>),

    CreateTag(Box<crate::CreateTagPlan>),
    DropTag(Box<crate::DropTagPlan>),
    SetObjectTags(Box<crate::SetObjectTagsPlan>),
    UnsetObjectTags(Box<crate::UnsetObjectTagsPlan>),

    CreateStage(Box<crate::CreateStagePlan>),
    AlterStage(Box<crate::AlterStagePlan>),
    DropStage(Box<crate::DropStagePlan>),
    RemoveStage(Box<crate::RemoveStagePlan>),

    CreateConnection(Box<crate::CreateConnectionPlan>),
    DescConnection(Box<crate::DescConnectionPlan>),
    DropConnection(Box<crate::DropConnectionPlan>),
    ShowConnections(Box<crate::ShowConnectionsPlan>),

    Presign(Box<crate::PresignPlan>),

    Set(Box<crate::GenericSetPlan<Self>>),
    Unset(Box<crate::UnsetPlan>),
    Kill(Box<crate::KillPlan>),
    SetPriority(Box<crate::SetPriorityPlan>),
    System(Box<crate::SystemPlan>),

    CreateDatamaskPolicy(Box<crate::CreateDatamaskPolicyPlan>),
    DropDatamaskPolicy(Box<crate::DropDatamaskPolicyPlan>),
    DescDatamaskPolicy(Box<crate::DescDatamaskPolicyPlan>),

    CreateNetworkPolicy(Box<crate::CreateNetworkPolicyPlan>),
    AlterNetworkPolicy(Box<crate::AlterNetworkPolicyPlan>),
    DropNetworkPolicy(Box<crate::DropNetworkPolicyPlan>),
    DescNetworkPolicy(Box<crate::DescNetworkPolicyPlan>),
    ShowNetworkPolicies(Box<crate::ShowNetworkPoliciesPlan>),

    CreatePasswordPolicy(Box<crate::CreatePasswordPolicyPlan>),
    AlterPasswordPolicy(Box<crate::AlterPasswordPolicyPlan>),
    DropPasswordPolicy(Box<crate::DropPasswordPolicyPlan>),
    DescPasswordPolicy(Box<crate::DescPasswordPolicyPlan>),

    CreateTask(Box<crate::CreateTaskPlan>),
    AlterTask(Box<crate::AlterTaskPlan>),
    DropTask(Box<crate::DropTaskPlan>),
    DescribeTask(Box<crate::DescribeTaskPlan>),
    ShowTasks(Box<crate::ShowTasksPlan>),
    ExecuteTask(Box<crate::ExecuteTaskPlan>),

    CreateDynamicTable(Box<crate::CreateDynamicTablePlan>),

    Begin,
    Commit,
    Abort,

    CreateNotification(Box<crate::CreateNotificationPlan>),
    AlterNotification(Box<crate::AlterNotificationPlan>),
    DropNotification(Box<crate::DropNotificationPlan>),
    DescNotification(Box<crate::DescNotificationPlan>),

    ExecuteImmediate(Box<crate::ExecuteImmediatePlan>),
    DropProcedure(Box<crate::DropProcedurePlan>),
    DescProcedure(Box<crate::DescProcedurePlan>),
    CreateProcedure(Box<crate::CreateProcedurePlan>),
    CallProcedure(Box<crate::CallProcedurePlan>),

    CreateSequence(Box<crate::CreateSequencePlan>),
    DropSequence(Box<crate::DropSequencePlan>),
    DescSequence(Box<crate::DescSequencePlan>),

    CreateDictionary(Box<crate::CreateDictionaryPlan>),
    DropDictionary(Box<crate::DropDictionaryPlan>),
    ShowCreateDictionary(Box<crate::ShowCreateDictionaryPlan>),
    RenameDictionary(Box<crate::RenameDictionaryPlan>),
}

#[derive(Clone, Debug)]
pub enum RewriteKind {
    ShowSettings,
    ShowVariables,
    ShowMetrics,
    ShowProcessList,
    ShowEngines,
    ShowIndexes,
    ShowLocks,
    ShowCatalogs,
    ShowDatabases,
    ShowDropDatabases,
    ShowTables(String, String),
    ShowColumns(String, String, String),
    ShowTablesStatus,
    ShowVirtualColumns,
    ShowDictionaries(String),
    ShowStatistics,
    ShowStreams(String),
    ShowTags,
    ShowFunctions,
    ShowUserFunctions,
    ShowTableFunctions,
    ShowUsers,
    ShowStages,
    DescribeStage,
    ListStage,
    ShowRoles,
    ShowPasswordPolicies,
    ShowGrants,
    Call,
    ShowProcedures,
    ShowSequences,
}

impl<
    SExpr: Clone + std::fmt::Debug,
    Metadata: Clone + std::fmt::Debug,
    BindContext: Clone + std::fmt::Debug,
    ExplainConfig: Clone + std::fmt::Debug,
    RewriteKindT: Clone + std::fmt::Debug,
    ScalarExpr: Clone + std::fmt::Debug,
    ReclusterPlan: Clone + std::fmt::Debug,
>
    GenericPlan<SExpr, Metadata, BindContext, ExplainConfig, RewriteKindT, ScalarExpr, ReclusterPlan>
{
    pub fn kind(&self) -> QueryKind {
        match self {
            Self::Query { .. } => QueryKind::Query,
            Self::CopyIntoTable(copy_plan) => match copy_plan.write_mode {
                crate::CopyIntoTableMode::Insert { .. } => QueryKind::Insert,
                _ => QueryKind::CopyIntoTable,
            },
            Self::Explain { .. }
            | Self::ExplainAnalyze { .. }
            | Self::ExplainAst { .. }
            | Self::ExplainSyntax { .. } => QueryKind::Explain,
            Self::Insert(_) => QueryKind::Insert,
            Self::Replace(_)
            | Self::DataMutation { .. }
            | Self::OptimizePurge(_)
            | Self::OptimizeCompactSegment(_)
            | Self::OptimizeCompactBlock { .. } => QueryKind::Update,
            _ => QueryKind::Other,
        }
    }

    pub fn schema(&self) -> DataSchemaRef
    where BindContext: PlanQueryBindContext
    {
        match self {
            Self::Query { bind_context, .. } => bind_context.output_schema(),
            Self::Explain { .. }
            | Self::ExplainAst { .. }
            | Self::ExplainSyntax { .. }
            | Self::ExplainAnalyze { .. }
            | Self::ExplainPerf { .. } => {
                DataSchemaRefExt::create(vec![DataField::new("explain", DataType::String)])
            }
            Self::DataMutation { schema, .. } => schema.clone(),
            Self::ShowCreateCatalog(plan) => plan.schema(),
            Self::ShowCreateDatabase(plan) => plan.schema(),
            Self::ShowCreateDictionary(plan) => plan.schema(),
            Self::ShowCreateTable(plan) => plan.schema(),
            Self::DescribeTable(plan) => plan.schema(),
            Self::VacuumTable(plan) => plan.schema(),
            Self::VacuumDropTable(plan) => plan.schema(),
            Self::VacuumTemporaryFiles(plan) => plan.schema(),
            Self::ExistsTable(plan) => plan.schema(),
            Self::DescribeView(plan) => plan.schema(),
            Self::ShowFileFormats(plan) => plan.schema(),
            Self::Replace(plan) => plan.schema(),
            Self::Presign(plan) => plan.schema(),
            Self::CreateDatamaskPolicy(plan) => plan.schema(),
            Self::DropDatamaskPolicy(plan) => plan.schema(),
            Self::DescDatamaskPolicy(plan) => plan.schema(),
            Self::DescNetworkPolicy(plan) => plan.schema(),
            Self::ShowNetworkPolicies(plan) => plan.schema(),
            Self::DescPasswordPolicy(plan) => plan.schema(),
            Self::CopyIntoTable(plan) => plan.schema(),
            Self::CopyIntoLocation(plan) => plan.schema(),
            Self::CreateTask(plan) => plan.schema(),
            Self::DescribeTask(plan) => plan.schema(),
            Self::RefreshVirtualColumn(plan) => plan.schema(),
            Self::VacuumVirtualColumn(plan) => plan.schema(),
            Self::ShowTasks(plan) => plan.schema(),
            Self::ExecuteTask(plan) => plan.schema(),
            Self::DescRowAccessPolicy(plan) => plan.schema(),
            Self::DescNotification(plan) => plan.schema(),
            Self::DescConnection(plan) => plan.schema(),
            Self::ShowConnections(plan) => plan.schema(),
            Self::ExecuteImmediate(plan) => plan.schema(),
            Self::CallProcedure(plan) => plan.schema(),
            Self::InsertMultiTable(plan) => plan.schema(),
            Self::DescUser(plan) => plan.schema(),
            Self::Insert(plan) => plan.schema(),
            Self::InspectWarehouse(plan) => plan.schema(),
            Self::ShowWarehouses => DataSchemaRefExt::create(vec![
                DataField::new("warehouse", DataType::String),
                DataField::new("type", DataType::String),
                DataField::new("status", DataType::String),
            ]),
            Self::ShowWorkers => crate::worker_schema(),
            Self::ShowOnlineNodes => DataSchemaRefExt::create(vec![
                DataField::new("id", DataType::String),
                DataField::new("type", DataType::String),
                DataField::new("node_group", DataType::String),
                DataField::new("warehouse", DataType::String),
                DataField::new("cluster", DataType::String),
                DataField::new("version", DataType::String),
            ]),
            Self::DescProcedure(plan) => plan.schema(),
            Self::ShowWorkloadGroups => DataSchemaRefExt::create(vec![
                DataField::new("name", DataType::String),
                DataField::new("cpu_quota", DataType::String),
                DataField::new("memory_quota", DataType::String),
                DataField::new("query_timeout", DataType::String),
                DataField::new("max_concurrency", DataType::String),
                DataField::new("query_queued_timeout", DataType::String),
            ]),
            Self::DescSequence(plan) => plan.schema(),
            Self::ReportIssue(..) => {
                DataSchemaRefExt::create(vec![DataField::new("summary", DataType::String)])
            }
            _ => Arc::new(DataSchema::empty()),
        }
    }

    pub fn has_result_set(&self) -> bool
    where BindContext: PlanQueryBindContext
    {
        !self.schema().fields().is_empty()
    }

    pub fn is_dynamic_schema(&self) -> bool {
        matches!(self, Self::ExecuteImmediate(..) | Self::CallProcedure(..))
    }

    pub fn remove_exchange_for_select(&self) -> Self
    where
        SExpr: PlanQuerySExpr,
        Metadata: Clone,
        BindContext: Clone,
        RewriteKindT: Clone,
    {
        if let Self::Query {
            s_expr,
            metadata,
            bind_context,
            rewrite_kind,
            formatted_ast,
            ignore_result,
        } = self
        {
            if s_expr.has_merge_exchange() {
                if let Some(s_expr) = s_expr.first_child() {
                    return Self::Query {
                        s_expr: Box::new(s_expr),
                        metadata: metadata.clone(),
                        bind_context: bind_context.clone(),
                        rewrite_kind: rewrite_kind.clone(),
                        formatted_ast: formatted_ast.clone(),
                        ignore_result: *ignore_result,
                    };
                }
            }
        }
        self.clone()
    }

    pub fn bind_context(&self) -> Option<BindContext>
    where BindContext: Clone
    {
        if let Self::Query { bind_context, .. } = self {
            Some(*bind_context.clone())
        } else {
            None
        }
    }

    pub fn replace_query_s_expr(&self, s_expr: SExpr) -> Self
    where
        Metadata: Clone,
        BindContext: Clone,
        RewriteKindT: Clone,
    {
        let Self::Query {
            metadata,
            bind_context,
            rewrite_kind,
            formatted_ast,
            ignore_result,
            ..
        } = self
        else {
            unreachable!()
        };

        Self::Query {
            s_expr: Box::new(s_expr),
            metadata: metadata.clone(),
            bind_context: bind_context.clone(),
            rewrite_kind: rewrite_kind.clone(),
            formatted_ast: formatted_ast.clone(),
            ignore_result: *ignore_result,
        }
    }
}

impl<
    SExpr: Clone + std::fmt::Debug,
    Metadata: Clone + std::fmt::Debug,
    BindContext: Clone + std::fmt::Debug,
    ExplainConfig: Clone + std::fmt::Debug,
    RewriteKindT: Clone + std::fmt::Debug,
    ScalarExpr: Clone + std::fmt::Debug,
    ReclusterPlan: Clone + std::fmt::Debug,
> Display
    for GenericPlan<SExpr, Metadata, BindContext, ExplainConfig, RewriteKindT, ScalarExpr, ReclusterPlan>
{
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", self.kind())
    }
}

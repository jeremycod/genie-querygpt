// ============================================================================
// Request Types
// ============================================================================

export interface QueryRequest {
  prompt: string;
  auto_approve?: boolean;
  max_attempts?: number;
  session_id?: string;
  execute_preview?: boolean;
  preview_limit?: number;
}

export interface ExecuteRequest {
  sql: string;
  mode: ExecutionMode;
  limit?: number;
}

export interface ExportRequest {
  sql: string;
  format: ExportFormat;
  session_id?: string;
}

export type ExecutionMode =
  | { preview: { limit: number } }
  | 'export';

export type ExportFormat = 'csv' | 'json';

// ============================================================================
// Response Types
// ============================================================================

export type QueryResponse =
  | QueryResponseSuccess
  | QueryResponsePendingConfirmation
  | QueryResponseCompilationFailed
  | QueryResponsePlannerFailed
  | QueryResponseRetryLimitExceeded;

export interface QueryResponseSuccess {
  status: 'success';
  sql: string;
  plan: IntermediatePlan;
  rationale?: string;
  assumptions?: string[];
  trace?: PlannerTrace;
  preview_data?: PreviewData;
  pipeline?: Pipeline;
}

export interface QueryResponsePendingConfirmation {
  status: 'pending_confirmation';
  session_id: string;
  draft: ReportSpecDraft;
  diffs: SpecDiff[];
  attempt: number;
}

export interface QueryResponseCompilationFailed {
  status: 'compilation_failed';
  diagnostics: CompilerDiagnostics;
  draft?: ReportSpecDraft;
}

export interface QueryResponsePlannerFailed {
  status: 'planner_failed';
  error: PlannerErrorResponse;
}

export interface QueryResponseRetryLimitExceeded {
  status: 'retry_limit_exceeded';
  diagnostics: CompilerDiagnostics;
  draft: ReportSpecDraft;
  attempts: number;
}

// ============================================================================
// Preview Data and Execution Results
// ============================================================================

export interface PreviewData {
  columns: ColumnInfo[];
  rows: any[][];
  total_rows: number;
  total_matching_rows?: number;
  execution_time_ms: number;
}

export interface ColumnInfo {
  name: string;
  pg_type: string;
}

// ============================================================================
// Pipeline Types (for Code Tab visualization)
// ============================================================================

export interface Pipeline {
  stages: PipelineStage[];
}

export interface PipelineStage {
  name: string;
  status: 'success' | 'failed' | 'skipped';
  timestamp: string;
  duration_ms?: number;
  output: any;
}

// ============================================================================
// Supporting Types
// ============================================================================

export interface IntermediatePlan {
  workspace: string;
  tables: TableRef[];
  joins: Join[];
  projections: Projection[];
  filters: Filter[];
  order_by: OrderBy[];
  limit?: number;
  offset?: number;
}

export interface ReportSpec {
  version: number;
  workspace: string;
  select: SelectItem[];
  filters: SpecFilter[];
  order_by: SpecOrderBy[];
  mode: Mode;
  pagination?: PaginationSpec;
}

export interface SelectItem {
  field: string;
  alias?: string;
}

export interface SpecFilter {
  field: string;
  op: string;
  value: any;
}

export interface SpecOrderBy {
  field: string;
  direction: 'asc' | 'desc';
}

export type Mode = 'preview' | 'export';

export interface PaginationSpec {
  limit?: number;
  offset?: number;
}

export interface TableRef {
  name: string;
  alias: string;
}

export interface Join {
  table: string;
  alias: string;
  condition: string;
  join_type: 'inner' | 'left' | 'right' | 'full';
}

export interface Projection {
  field: string;
  expression: string;
  alias?: string;
}

export interface Filter {
  expression: string;
}

export interface OrderBy {
  expression: string;
  direction: 'asc' | 'desc';
}

export interface ReportSpecDraft {
  spec: any; // ReportSpec from backend
  rationale?: string;
  assumptions: string[];
}

export interface SpecDiff {
  path: string;
  old_value?: any;
  new_value?: any;
  change_type: 'added' | 'removed' | 'modified';
}

export interface CompilerDiagnostics {
  errors: Diagnostic[];
  warnings: Diagnostic[];
}

export interface Diagnostic {
  code: string;
  message: string;
  spans: string[];
  details?: Record<string, any>;
  help?: string[];
}

export interface PlannerTrace {
  model?: string;
  attempts: number;
  revisions_occurred: boolean;
  final_status: string;
}

export interface PlannerErrorResponse {
  error_type: string;
  message: string;
  retry_after?: number;
  help?: string[];
}

// ============================================================================
// Error Response
// ============================================================================

export interface ErrorResponse {
  error: string;
  details?: string;
}

// ============================================================================
// Frontend-specific Types
// ============================================================================

export interface ConversationMessage {
  id: string;
  type: 'user' | 'assistant';
  content: string;
  timestamp: Date;
  sql?: string;
  preview_data?: PreviewData;
  error?: string;
}

export interface QueryState {
  isLoading: boolean;
  error: string | null;
  response: QueryResponse | null;
}

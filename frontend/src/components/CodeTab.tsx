import { useState } from 'react';
import { ClipboardDocumentIcon, CheckIcon, ChevronDownIcon, ChevronRightIcon } from '@heroicons/react/24/outline';
import { PipelineVisualization } from './PipelineVisualization';
import type { QueryResponseSuccess } from '@/types/api';

interface CodeTabProps {
  response: QueryResponseSuccess | null;
  userPrompt: string;
}

/**
 * Code Tab Component
 *
 * Displays pipeline visualization and stage-by-stage output.
 */
export function CodeTab({ response, userPrompt }: CodeTabProps) {
  const [selectedStage, setSelectedStage] = useState<string>('sql');
  const [copiedSql, setCopiedSql] = useState(false);
  const [specJsonExpanded, setSpecJsonExpanded] = useState(false);
  const [planJsonExpanded, setPlanJsonExpanded] = useState(false);

  if (!response) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <div className="text-center max-w-md">
          <div className="w-16 h-16 bg-gray-100 rounded-full flex items-center justify-center mx-auto mb-4">
            <svg
              className="w-8 h-8 text-gray-400"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M10 20l4-16m4 4l4 4-4 4M6 16l-4-4 4-4"
              />
            </svg>
          </div>
          <h3 className="text-lg font-medium text-gray-900 mb-2">No query generated yet</h3>
          <p className="text-sm text-gray-600">
            Submit a query to see the processing pipeline and execution details.
          </p>
        </div>
      </div>
    );
  }

  const stages = [
    { id: 'prompt', label: 'Prompt', status: 'completed' as const },
    { id: 'spec', label: 'ReportSpec', status: 'completed' as const },
    { id: 'plan', label: 'IntermediatePlan', status: 'completed' as const },
    { id: 'sql', label: 'SQL', status: 'completed' as const },
  ];

  const copySql = async () => {
    await navigator.clipboard.writeText(response.sql);
    setCopiedSql(true);
    setTimeout(() => setCopiedSql(false), 2000);
  };

  return (
    <div className="flex-1 flex flex-col overflow-hidden">
      {/* Pipeline Visualization */}
      <PipelineVisualization
        stages={stages}
        selectedStage={selectedStage}
        onStageClick={setSelectedStage}
      />

      {/* Stage Details */}
      <div className="flex-1 overflow-y-auto p-6">
        {selectedStage === 'prompt' && (
          <StageOutput title="User Prompt" subtitle="Natural language input from user">
            <div className="card p-4">
              <p className="text-gray-900 whitespace-pre-wrap">
                {userPrompt || 'No prompt available'}
              </p>
            </div>
          </StageOutput>
        )}

        {selectedStage === 'spec' && (
          <StageOutput
            title="Report Specification"
            subtitle="High-level query specification from LLM"
          >
            <div className="space-y-4">
              {/* Full ReportSpec as JSON - Collapsible */}
              <div className="card p-4">
                <button
                  onClick={() => setSpecJsonExpanded(!specJsonExpanded)}
                  className="flex items-center space-x-2 w-full text-left hover:text-primary-600 transition-colors"
                >
                  {specJsonExpanded ? (
                    <ChevronDownIcon className="w-4 h-4" />
                  ) : (
                    <ChevronRightIcon className="w-4 h-4" />
                  )}
                  <h4 className="text-sm font-semibold text-gray-900">ReportSpec Structure (JSON)</h4>
                </button>
                {specJsonExpanded && (
                  <div className="code-block mt-3">
                    <pre className="whitespace-pre-wrap text-xs">
                      {JSON.stringify({
                        version: 1,
                        workspace: response.plan.workspace,
                        select: response.plan.projections.map(p => ({
                          field: p.field,
                          alias: p.alias || null
                        })),
                        filters: response.plan.filters.map(f => ({
                          expression: f.expression
                        })),
                        order_by: response.plan.order_by.map(o => ({
                          field: o.expression,
                          direction: o.direction
                        })),
                        mode: "preview",
                        pagination: response.plan.limit ? {
                          limit: response.plan.limit,
                          offset: response.plan.offset || null
                        } : null
                      }, null, 2)}
                    </pre>
                  </div>
                )}
              </div>

              {/* Workspace */}
              <div className="card p-4">
                <h4 className="text-sm font-semibold text-gray-900 mb-2">Workspace</h4>
                <code className="text-sm bg-gray-100 text-gray-900 px-2 py-1 rounded">
                  {response.plan.workspace}
                </code>
              </div>

              {/* Selected Fields */}
              {response.plan.projections.length > 0 && (
                <div className="card p-4">
                  <h4 className="text-sm font-semibold text-gray-900 mb-2">Selected Fields</h4>
                  <div className="flex flex-wrap gap-2">
                    {response.plan.projections.map((proj, idx) => (
                      <code
                        key={idx}
                        className="font-mono text-xs bg-blue-50 text-blue-700 px-2 py-1 rounded"
                      >
                        {proj.field}{proj.alias && ` as ${proj.alias}`}
                      </code>
                    ))}
                  </div>
                </div>
              )}

              {/* Filters */}
              {response.plan.filters && response.plan.filters.length > 0 && (
                <div className="card p-4">
                  <h4 className="text-sm font-semibold text-gray-900 mb-2">Filters</h4>
                  <div className="space-y-1">
                    {response.plan.filters.map((filter, idx) => (
                      <code
                        key={idx}
                        className="block font-mono text-xs bg-yellow-50 text-yellow-700 px-2 py-1 rounded"
                      >
                        {filter.expression}
                      </code>
                    ))}
                  </div>
                </div>
              )}
            </div>
          </StageOutput>
        )}

        {selectedStage === 'plan' && (
          <StageOutput
            title="Intermediate Plan"
            subtitle="Compiled execution plan with resolved tables and joins"
          >
            <div className="space-y-4">
              {/* Full IntermediatePlan as JSON - Collapsible */}
              <div className="card p-4">
                <button
                  onClick={() => setPlanJsonExpanded(!planJsonExpanded)}
                  className="flex items-center space-x-2 w-full text-left hover:text-primary-600 transition-colors"
                >
                  {planJsonExpanded ? (
                    <ChevronDownIcon className="w-4 h-4" />
                  ) : (
                    <ChevronRightIcon className="w-4 h-4" />
                  )}
                  <h4 className="text-sm font-semibold text-gray-900">IntermediatePlan Structure (JSON)</h4>
                </button>
                {planJsonExpanded && (
                  <div className="code-block mt-3">
                    <pre className="whitespace-pre-wrap text-xs">
                      {JSON.stringify(response.plan, null, 2)}
                    </pre>
                  </div>
                )}
              </div>

              {/* Workspace */}
              <div className="card p-4">
                <h4 className="text-sm font-semibold text-gray-900 mb-2">Workspace</h4>
                <code className="text-sm bg-gray-100 text-gray-900 px-2 py-1 rounded">
                  {response.plan.workspace}
                </code>
              </div>

              {/* Tables */}
              {response.plan.tables.length > 0 && (
                <div className="card p-4">
                  <h4 className="text-sm font-semibold text-gray-900 mb-2">Tables</h4>
                  <div className="space-y-2">
                    {response.plan.tables.map((table, idx) => (
                      <div key={idx} className="text-sm">
                        <code className="font-mono bg-gray-100 px-2 py-1 rounded">
                          {table.name}
                        </code>
                        {table.alias && (
                          <span className="text-gray-500 ml-2">as {table.alias}</span>
                        )}
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {/* Joins */}
              {response.plan.joins && response.plan.joins.length > 0 && (
                <div className="card p-4">
                  <h4 className="text-sm font-semibold text-gray-900 mb-3">Joins</h4>
                  <div className="space-y-3">
                    {response.plan.joins.map((join, idx) => (
                      <div key={idx} className="border-l-2 border-blue-300 pl-3">
                        <div className="text-sm">
                          <span className="font-semibold text-blue-700 uppercase">
                            {join.join_type} JOIN
                          </span>
                          <code className="ml-2 font-mono bg-gray-100 px-2 py-1 rounded">
                            {join.table}
                          </code>
                          {join.alias && (
                            <span className="text-gray-500 ml-1">as {join.alias}</span>
                          )}
                        </div>
                        <div className="text-xs text-gray-600 mt-1">ON {join.condition}</div>
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {/* Projections */}
              {response.plan.projections.length > 0 && (
                <div className="card p-4">
                  <h4 className="text-sm font-semibold text-gray-900 mb-2">Projections</h4>
                  <div className="space-y-1">
                    {response.plan.projections.map((proj, idx) => (
                      <div key={idx} className="text-sm">
                        <code className="font-mono bg-green-50 text-green-700 px-2 py-1 rounded">
                          {proj.expression}
                          {proj.alias && ` as ${proj.alias}`}
                        </code>
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {/* Filters */}
              {response.plan.filters && response.plan.filters.length > 0 && (
                <div className="card p-4">
                  <h4 className="text-sm font-semibold text-gray-900 mb-2">Filters</h4>
                  <div className="space-y-1">
                    {response.plan.filters.map((filter, idx) => (
                      <code
                        key={idx}
                        className="block font-mono text-xs bg-yellow-50 text-yellow-700 px-2 py-1 rounded"
                      >
                        {filter.expression}
                      </code>
                    ))}
                  </div>
                </div>
              )}

              {/* Order By */}
              {response.plan.order_by && response.plan.order_by.length > 0 && (
                <div className="card p-4">
                  <h4 className="text-sm font-semibold text-gray-900 mb-2">Order By</h4>
                  <div className="space-y-1">
                    {response.plan.order_by.map((order, idx) => (
                      <code
                        key={idx}
                        className="block font-mono text-sm bg-purple-50 text-purple-700 px-2 py-1 rounded"
                      >
                        {order.expression} {order.direction.toUpperCase()}
                      </code>
                    ))}
                  </div>
                </div>
              )}

              {/* Limit & Offset */}
              {(response.plan.limit !== undefined || response.plan.offset !== undefined) && (
                <div className="card p-4">
                  <h4 className="text-sm font-semibold text-gray-900 mb-2">Pagination</h4>
                  <div className="space-y-1 text-sm">
                    {response.plan.limit !== undefined && (
                      <div>
                        <span className="text-gray-600">Limit:</span>{' '}
                        <span className="font-medium">{response.plan.limit} rows</span>
                      </div>
                    )}
                    {response.plan.offset !== undefined && (
                      <div>
                        <span className="text-gray-600">Offset:</span>{' '}
                        <span className="font-medium">{response.plan.offset} rows</span>
                      </div>
                    )}
                  </div>
                </div>
              )}
            </div>
          </StageOutput>
        )}

        {selectedStage === 'sql' && (
          <StageOutput
            title="Generated SQL"
            subtitle="Final executable SQL query"
            action={
              <button
                onClick={copySql}
                className="flex items-center space-x-1 px-3 py-1.5 text-xs text-gray-600 hover:text-gray-900 border border-gray-300 rounded hover:bg-gray-50 transition-colors"
              >
                {copiedSql ? (
                  <>
                    <CheckIcon className="w-4 h-4 text-green-600" />
                    <span className="text-green-600">Copied!</span>
                  </>
                ) : (
                  <>
                    <ClipboardDocumentIcon className="w-4 h-4" />
                    <span>Copy</span>
                  </>
                )}
              </button>
            }
          >
            <div className="code-block">
              <pre className="whitespace-pre-wrap">{response.sql}</pre>
            </div>
          </StageOutput>
        )}
      </div>
    </div>
  );
}

/**
 * Stage Output Wrapper Component
 */
function StageOutput({
  title,
  subtitle,
  action,
  children,
}: {
  title: string;
  subtitle: string;
  action?: React.ReactNode;
  children: React.ReactNode;
}) {
  return (
    <div>
      <div className="flex items-center justify-between mb-4">
        <div>
          <h3 className="text-lg font-semibold text-gray-900">{title}</h3>
          <p className="text-sm text-gray-600 mt-0.5">{subtitle}</p>
        </div>
        {action}
      </div>
      {children}
    </div>
  );
}

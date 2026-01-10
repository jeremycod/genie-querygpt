import {
  ArrowDownTrayIcon,
} from '@heroicons/react/24/outline';
import type { PreviewData } from '@/types/api';

interface PreviewTabProps {
  data: PreviewData | null;
  isLoading: boolean;
  onExport?: (format: 'csv' | 'json') => void;
}

/**
 * Preview Tab Component
 *
 * Displays query results in a table with export action.
 * Preview always shows up to 10 rows.
 */
export function PreviewTab({
  data,
  isLoading,
  onExport,
}: PreviewTabProps) {

  if (isLoading) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <div className="text-center">
          <div className="inline-block w-12 h-12 border-4 border-primary-200 border-t-primary-600 rounded-full animate-spin" />
          <p className="mt-4 text-sm text-gray-600">Executing query...</p>
        </div>
      </div>
    );
  }

  if (!data) {
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
                d="M3 10h18M3 14h18m-9-4v8m-7 0h14a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z"
              />
            </svg>
          </div>
          <h3 className="text-lg font-medium text-gray-900 mb-2">No data to preview</h3>
          <p className="text-sm text-gray-600">
            Submit a query to see results here. Preview shows up to 1,000 rows.
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex-1 flex flex-col overflow-hidden">
      {/* Action Bar */}
      <div className="px-6 py-4 border-b border-gray-200 bg-gray-50 flex items-center justify-between">
        <div className="flex items-center space-x-4">
          {data.total_matching_rows !== undefined ? (
            <div className="text-sm text-gray-600">
              <span className="font-medium text-gray-900">{data.total_matching_rows.toLocaleString()}</span> total records
              <span className="text-gray-500 ml-1">(showing {data.total_rows} in preview)</span>
            </div>
          ) : (
            <div className="text-sm text-gray-600">
              <span className="font-medium text-gray-900">{data.total_rows}</span> rows in preview
            </div>
          )}
          <div className="text-sm text-gray-500">
            {data.execution_time_ms}ms
          </div>
        </div>

        <div className="flex items-center space-x-2">
          {/* Export Button */}
          {onExport && (
            <div className="relative group">
              <button
                className="flex items-center space-x-1 px-3 py-1.5 text-sm text-white bg-primary-600 rounded-md hover:bg-primary-700 transition-colors shadow-sm"
                title="Export full results"
              >
                <ArrowDownTrayIcon className="w-4 h-4" />
                <span>Export</span>
              </button>
              <div className="absolute right-0 mt-1 w-32 bg-white border border-gray-200 rounded-lg shadow-lg opacity-0 invisible group-hover:opacity-100 group-hover:visible transition-all z-10">
                <button
                  onClick={() => onExport('csv')}
                  className="w-full text-left px-4 py-2 text-sm text-gray-700 hover:bg-gray-50 rounded-t-lg"
                >
                  CSV
                </button>
                <button
                  onClick={() => onExport('json')}
                  className="w-full text-left px-4 py-2 text-sm text-gray-700 hover:bg-gray-50 rounded-b-lg"
                >
                  JSON
                </button>
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Data Table */}
      <div className="flex-1 overflow-auto bg-gray-50 p-6">
        <div className="bg-white rounded-lg border border-gray-200 shadow-sm overflow-x-auto">
          <table className="min-w-full divide-y divide-gray-200">
            <thead className="bg-gray-50 sticky top-0">
              <tr>
                {data.columns.map((col, idx) => (
                  <th
                    key={idx}
                    className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider"
                  >
                    <div className="flex flex-col">
                      <span>{col.name}</span>
                      <span className="text-[10px] text-gray-400 font-normal normal-case">
                        {col.pg_type}
                      </span>
                    </div>
                  </th>
                ))}
              </tr>
            </thead>
            <tbody className="bg-white divide-y divide-gray-200">
              {data.rows.map((row, rowIdx) => (
                <tr key={rowIdx} className="hover:bg-gray-50 transition-colors">
                  {row.map((cell, cellIdx) => (
                    <td
                      key={cellIdx}
                      className="px-6 py-4 text-sm text-gray-900 whitespace-nowrap"
                    >
                      <CellValue value={cell} />
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
}

/**
 * Cell Value Component
 *
 * Renders different data types appropriately.
 */
function CellValue({ value }: { value: any }) {
  if (value === null || value === undefined) {
    return <span className="text-gray-400 italic">null</span>;
  }

  if (typeof value === 'boolean') {
    return (
      <span className={value ? 'text-green-600' : 'text-red-600'}>
        {value ? 'true' : 'false'}
      </span>
    );
  }

  if (Array.isArray(value)) {
    return (
      <span className="text-blue-600 font-mono text-xs">
        [{value.join(', ')}]
      </span>
    );
  }

  if (typeof value === 'object') {
    return (
      <span className="text-purple-600 font-mono text-xs">
        {JSON.stringify(value)}
      </span>
    );
  }

  return <span>{String(value)}</span>;
}

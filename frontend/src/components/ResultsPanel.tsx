import { Tab } from '@headlessui/react';
import { TableCellsIcon, CodeBracketIcon } from '@heroicons/react/24/outline';
import { PreviewTab } from './PreviewTab';
import { CodeTab } from './CodeTab';
import type { PreviewData, QueryResponseSuccess } from '@/types/api';
import clsx from 'clsx';

interface ResultsPanelProps {
  previewData: PreviewData | null;
  response: QueryResponseSuccess | null;
  userPrompt: string;
  isLoading: boolean;
  onExport?: (format: 'csv' | 'json') => void;
}

/**
 * Results Panel Component
 *
 * Right side of the split layout with tabs for Preview and Code views.
 */
export function ResultsPanel({
  previewData,
  response,
  userPrompt,
  isLoading,
  onExport,
}: ResultsPanelProps) {
  return (
    <div className="h-full flex flex-col bg-white">
      <Tab.Group>
        <Tab.List className="flex space-x-1 bg-gray-100 p-1 border-b border-gray-200">
          <Tab
            className={({ selected }) =>
              clsx(
                'flex items-center space-x-2 px-4 py-2 text-sm font-medium rounded-lg transition-colors',
                selected
                  ? 'bg-white text-primary-700 shadow-sm'
                  : 'text-gray-600 hover:text-gray-900 hover:bg-gray-50'
              )
            }
          >
            <TableCellsIcon className="w-5 h-5" />
            <span>Preview</span>
            {previewData && (
              <span className="ml-2 px-2 py-0.5 bg-primary-100 text-primary-700 text-xs rounded-full">
                {previewData.total_rows}
              </span>
            )}
          </Tab>

          <Tab
            className={({ selected }) =>
              clsx(
                'flex items-center space-x-2 px-4 py-2 text-sm font-medium rounded-lg transition-colors',
                selected
                  ? 'bg-white text-primary-700 shadow-sm'
                  : 'text-gray-600 hover:text-gray-900 hover:bg-gray-50'
              )
            }
          >
            <CodeBracketIcon className="w-5 h-5" />
            <span>Code</span>
          </Tab>
        </Tab.List>

        <Tab.Panels className="flex-1 overflow-hidden">
          <Tab.Panel className="h-full flex flex-col">
            <PreviewTab
              data={previewData}
              isLoading={isLoading}
              onExport={onExport}
            />
          </Tab.Panel>

          <Tab.Panel className="h-full flex flex-col">
            <CodeTab response={response} userPrompt={userPrompt} />
          </Tab.Panel>
        </Tab.Panels>
      </Tab.Group>
    </div>
  );
}

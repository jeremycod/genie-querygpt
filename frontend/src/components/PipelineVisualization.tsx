import { CheckCircleIcon, ChevronRightIcon } from '@heroicons/react/24/solid';
import clsx from 'clsx';

interface PipelineStage {
  id: string;
  label: string;
  status: 'completed' | 'active' | 'pending';
}

interface PipelineVisualizationProps {
  stages: PipelineStage[];
  selectedStage: string | null;
  onStageClick: (stageId: string) => void;
}

/**
 * Pipeline Visualization Component
 *
 * Shows a horizontal flowchart of query processing stages.
 * Stages are clickable to show detailed output below.
 */
export function PipelineVisualization({
  stages,
  selectedStage,
  onStageClick,
}: PipelineVisualizationProps) {
  return (
    <div className="bg-gradient-to-r from-blue-50 to-indigo-50 p-6 border-b border-gray-200">
      <div className="flex items-center justify-center space-x-2">
        {stages.map((stage, index) => (
          <div key={stage.id} className="flex items-center">
            {/* Stage Box */}
            <button
              onClick={() => onStageClick(stage.id)}
              className={clsx(
                'relative px-4 py-3 rounded-lg border-2 transition-all duration-200 min-w-[140px]',
                'hover:shadow-md',
                selectedStage === stage.id
                  ? 'border-primary-600 bg-primary-600 text-white shadow-lg scale-105'
                  : stage.status === 'completed'
                  ? 'border-green-400 bg-white text-gray-900 hover:border-green-500'
                  : stage.status === 'active'
                  ? 'border-blue-400 bg-white text-gray-900 animate-pulse'
                  : 'border-gray-300 bg-gray-50 text-gray-500'
              )}
            >
              <div className="flex items-center justify-between">
                <span className="text-sm font-medium">{stage.label}</span>
                {stage.status === 'completed' && (
                  <CheckCircleIcon
                    className={clsx(
                      'w-5 h-5 ml-2',
                      selectedStage === stage.id ? 'text-white' : 'text-green-500'
                    )}
                  />
                )}
              </div>
            </button>

            {/* Arrow */}
            {index < stages.length - 1 && (
              <ChevronRightIcon className="w-6 h-6 mx-2 text-gray-400" />
            )}
          </div>
        ))}
      </div>

      {/* Legend */}
      <div className="mt-4 flex items-center justify-center space-x-6 text-xs text-gray-600">
        <div className="flex items-center space-x-1">
          <div className="w-3 h-3 rounded-full bg-green-400"></div>
          <span>Completed</span>
        </div>
        <div className="flex items-center space-x-1">
          <div className="w-3 h-3 rounded-full bg-blue-400 animate-pulse"></div>
          <span>Processing</span>
        </div>
        <div className="flex items-center space-x-1">
          <div className="w-3 h-3 rounded-full bg-gray-300"></div>
          <span>Pending</span>
        </div>
      </div>
    </div>
  );
}

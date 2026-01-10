import { ReactNode } from 'react';

interface LayoutProps {
  chatPanel: ReactNode;
  resultsPanel: ReactNode;
}

/**
 * Main layout component with split view
 *
 * Left side (40%): Chat panel with conversation history and input
 * Right side (60%): Results panel with tabs (Preview, Code)
 */
export function Layout({ chatPanel, resultsPanel }: LayoutProps) {
  return (
    <div className="flex h-screen bg-gray-50">
      {/* Chat Panel - Left Side (40%) */}
      <div className="w-2/5 border-r border-gray-200 bg-white flex flex-col">
        {chatPanel}
      </div>

      {/* Results Panel - Right Side (60%) */}
      <div className="w-3/5 flex flex-col">
        {resultsPanel}
      </div>
    </div>
  );
}

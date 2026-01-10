import { useState, useEffect } from 'react';
import { Layout } from './components/Layout';
import { ChatPanel } from './components/ChatPanel';
import { ResultsPanel } from './components/ResultsPanel';
import { useQuery, useExport } from './hooks/useQuery';
import type { ConversationMessage, QueryResponseSuccess } from './types/api';

/**
 * Main Application Component
 *
 * Manages application state and coordinates between ChatPanel and ResultsPanel.
 */
function App() {
  const [messages, setMessages] = useState<ConversationMessage[]>([]);
  const [currentResponse, setCurrentResponse] = useState<QueryResponseSuccess | null>(null);
  const [currentPrompt, setCurrentPrompt] = useState<string>('');

  const { isLoading, response, previewData, sql, submitQuery } = useQuery();
  const { exportData } = useExport();

  // Update messages when query response arrives
  useEffect(() => {
    if (response && response.status === 'success') {
      // Add assistant message with SQL
      const assistantMessage: ConversationMessage = {
        id: Date.now().toString(),
        type: 'assistant',
        content: response.rationale || 'Here\'s your query result:',
        timestamp: new Date(),
        sql: response.sql,
        preview_data: response.preview_data,
      };

      setMessages((prev) => [...prev, assistantMessage]);
      setCurrentResponse(response);
    } else if (response) {
      // Handle error responses
      let errorMessage = 'Query failed';

      if (response.status === 'compilation_failed') {
        errorMessage = 'SQL compilation failed: ' +
          response.diagnostics.errors.map(e => e.message).join(', ');
      } else if (response.status === 'planner_failed') {
        errorMessage = response.error.message;
      } else if (response.status === 'retry_limit_exceeded') {
        errorMessage = `Failed after ${response.attempts} attempts`;
      }

      const errorMsg: ConversationMessage = {
        id: Date.now().toString(),
        type: 'assistant',
        content: 'I encountered an error processing your query.',
        timestamp: new Date(),
        error: errorMessage,
      };

      setMessages((prev) => [...prev, errorMsg]);
    }
  }, [response]);

  const handleSubmit = async (prompt: string) => {
    // Add user message
    const userMessage: ConversationMessage = {
      id: Date.now().toString(),
      type: 'user',
      content: prompt,
      timestamp: new Date(),
    };

    setMessages((prev) => [...prev, userMessage]);
    setCurrentPrompt(prompt); // Save the current prompt for Code tab

    // Submit query with preview enabled
    await submitQuery(prompt, {
      execute_preview: true,
      preview_limit: 10,
    });
  };

  const handleExport = async (format: 'csv' | 'json') => {
    if (sql) {
      await exportData(sql, format);
    }
  };

  return (
    <Layout
      chatPanel={
        <ChatPanel
          messages={messages}
          isLoading={isLoading}
          onSubmit={handleSubmit}
        />
      }
      resultsPanel={
        <ResultsPanel
          previewData={previewData}
          response={currentResponse}
          userPrompt={currentPrompt}
          isLoading={isLoading}
          onExport={handleExport}
        />
      }
    />
  );
}

export default App;

import { useState, useCallback } from 'react';
import { apiClient } from '@/api/client';
import type {
  QueryRequest,
  QueryResponse,
  PreviewData,
} from '@/types/api';

interface UseQueryState {
  isLoading: boolean;
  error: string | null;
  response: QueryResponse | null;
  sql: string | null;
  previewData: PreviewData | null;
}

interface UseQueryReturn extends UseQueryState {
  submitQuery: (prompt: string, options?: Partial<QueryRequest>) => Promise<void>;
  reset: () => void;
  isSuccess: boolean;
}

/**
 * Hook for submitting natural language queries to the backend
 *
 * Manages loading state, error handling, and response data.
 * Automatically extracts SQL and preview data from successful responses.
 */
export function useQuery(): UseQueryReturn {
  const [state, setState] = useState<UseQueryState>({
    isLoading: false,
    error: null,
    response: null,
    sql: null,
    previewData: null,
  });

  const submitQuery = useCallback(async (
    prompt: string,
    options?: Partial<QueryRequest>
  ) => {
    setState({
      isLoading: true,
      error: null,
      response: null,
      sql: null,
      previewData: null,
    });

    try {
      const request: QueryRequest = {
        prompt,
        auto_approve: options?.auto_approve ?? true,
        execute_preview: options?.execute_preview ?? true,
        preview_limit: options?.preview_limit ?? 10,
        max_attempts: options?.max_attempts ?? 3,
        session_id: options?.session_id,
      };

      const response = await apiClient.query(request);

      // Extract SQL and preview data if successful
      let sql: string | null = null;
      let previewData: PreviewData | null = null;

      if (response.status === 'success') {
        sql = response.sql;
        previewData = response.preview_data || null;
      }

      setState({
        isLoading: false,
        error: null,
        response,
        sql,
        previewData,
      });
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Unknown error occurred';
      setState({
        isLoading: false,
        error: errorMessage,
        response: null,
        sql: null,
        previewData: null,
      });
    }
  }, []);

  const reset = useCallback(() => {
    setState({
      isLoading: false,
      error: null,
      response: null,
      sql: null,
      previewData: null,
    });
  }, []);

  const isSuccess = state.response?.status === 'success';

  return {
    ...state,
    submitQuery,
    reset,
    isSuccess,
  };
}

/**
 * Hook for executing SQL directly (without query generation)
 *
 * Useful for refreshing preview data or re-executing with different limits.
 */
export function useExecute() {
  const [state, setState] = useState<{
    isLoading: boolean;
    error: string | null;
    data: PreviewData | null;
  }>({
    isLoading: false,
    error: null,
    data: null,
  });

  const execute = useCallback(async (sql: string, limit: number = 10) => {
    setState({ isLoading: true, error: null, data: null });

    try {
      const data = await apiClient.execute({
        sql,
        mode: { preview: { limit } },
        limit,
      });

      setState({ isLoading: false, error: null, data });
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Unknown error occurred';
      setState({ isLoading: false, error: errorMessage, data: null });
    }
  }, []);

  return { ...state, execute };
}

/**
 * Hook for exporting query results
 *
 * Handles file download in browser.
 */
export function useExport() {
  const [state, setState] = useState<{
    isLoading: boolean;
    error: string | null;
  }>({
    isLoading: false,
    error: null,
  });

  const exportData = useCallback(async (
    sql: string,
    format: 'csv' | 'json',
    sessionId?: string
  ) => {
    setState({ isLoading: true, error: null });

    try {
      await apiClient.exportAndDownload(sql, format, sessionId);
      setState({ isLoading: false, error: null });
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Unknown error occurred';
      setState({ isLoading: false, error: errorMessage });
    }
  }, []);

  return { ...state, exportData };
}

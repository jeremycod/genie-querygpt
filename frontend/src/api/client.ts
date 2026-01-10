import axios, { AxiosInstance, AxiosError } from 'axios';
import type {
  QueryRequest,
  QueryResponse,
  ExecuteRequest,
  PreviewData,
  ExportRequest,
  ExportFormat,
  ErrorResponse,
} from '@/types/api';

/**
 * API Client for QueryGPT Backend
 *
 * Handles all HTTP communication with the Rust backend server.
 * Uses axios with interceptors for error handling and logging.
 */
class ApiClient {
  private client: AxiosInstance;

  constructor(baseURL: string = import.meta.env.VITE_API_URL || 'http://localhost:8080') {
    this.client = axios.create({
      baseURL,
      headers: {
        'Content-Type': 'application/json',
      },
      timeout: 30000, // 30 seconds
      withCredentials: true, // Enable CORS credentials
    });

    // Request interceptor - log requests in development
    this.client.interceptors.request.use(
      (config) => {
        if (import.meta.env.DEV) {
          console.log(`[API] ${config.method?.toUpperCase()} ${config.url}`, config.data);
        }
        return config;
      },
      (error) => {
        console.error('[API] Request error:', error);
        return Promise.reject(error);
      }
    );

    // Response interceptor - handle errors
    this.client.interceptors.response.use(
      (response) => {
        if (import.meta.env.DEV) {
          console.log(`[API] Response:`, response.data);
        }
        return response;
      },
      (error: AxiosError<ErrorResponse>) => {
        // Handle different error scenarios
        if (error.response) {
          // Server responded with error status
          const errorMessage = error.response.data?.error || error.message;
          console.error('[API] Server error:', error.response.status, errorMessage);
          return Promise.reject(new Error(errorMessage));
        } else if (error.request) {
          // Request made but no response
          console.error('[API] No response from server:', error.message);
          return Promise.reject(new Error('Server is not responding. Please check your connection.'));
        } else {
          // Something else went wrong
          console.error('[API] Request setup error:', error.message);
          return Promise.reject(error);
        }
      }
    );
  }

  /**
   * POST /query - Submit natural language query
   *
   * @param request Query request with prompt and options
   * @returns QueryResponse with SQL, plan, and optional preview data
   */
  async query(request: QueryRequest): Promise<QueryResponse> {
    const response = await this.client.post<QueryResponse>('/query', request);
    return response.data;
  }

  /**
   * POST /execute - Execute SQL directly
   *
   * @param request Execute request with SQL and mode
   * @returns PreviewData with query results
   */
  async execute(request: ExecuteRequest): Promise<PreviewData> {
    const response = await this.client.post<PreviewData>('/execute', request);
    return response.data;
  }

  /**
   * POST /export - Download query results as CSV or JSON
   *
   * @param request Export request with SQL and format
   * @returns Blob for file download
   */
  async export(request: ExportRequest): Promise<Blob> {
    const response = await this.client.post('/export', request, {
      responseType: 'blob',
    });
    return response.data;
  }

  /**
   * Helper method to trigger file download in browser
   *
   * @param blob File blob
   * @param filename Filename for download
   */
  downloadFile(blob: Blob, filename: string): void {
    const url = window.URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.href = url;
    link.download = filename;
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    window.URL.revokeObjectURL(url);
  }

  /**
   * Helper method to export and download query results
   *
   * @param sql SQL query to export
   * @param format Export format (csv or json)
   * @param sessionId Optional session ID
   */
  async exportAndDownload(
    sql: string,
    format: ExportFormat,
    sessionId?: string
  ): Promise<void> {
    const blob = await this.export({ sql, format, session_id: sessionId });
    const timestamp = new Date().toISOString().replace(/[:.]/g, '-').slice(0, 19);
    const filename = `export_${timestamp}.${format}`;
    this.downloadFile(blob, filename);
  }
}

// Export singleton instance
export const apiClient = new ApiClient();

// Export class for testing or custom instances
export default ApiClient;

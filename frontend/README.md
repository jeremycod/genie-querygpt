# QueryGPT Frontend

React + TypeScript + Vite frontend for QueryGPT natural language to SQL interface.

## Overview

This is the web UI for QueryGPT, allowing users to:
- Submit natural language queries about their data
- View generated SQL queries
- See preview data in a table
- Export results as CSV or JSON
- Inspect execution plans and query details

## Tech Stack

- **Framework:** React 18
- **Build Tool:** Vite 5
- **Language:** TypeScript (strict mode)
- **Styling:** TailwindCSS 3
- **HTTP Client:** Axios
- **State Management:** React hooks (useState, useEffect)
- **UI Components:** Headless UI
- **Icons:** Heroicons

## Project Structure

```
frontend/
├── src/
│   ├── components/         # React components
│   │   ├── Layout.tsx      # Main layout (split view)
│   │   ├── ChatPanel.tsx   # Left panel (conversation)
│   │   ├── ResultsPanel.tsx # Right panel (tabs container)
│   │   ├── PreviewTab.tsx  # Data table view
│   │   └── CodeTab.tsx     # SQL and plan view
│   ├── hooks/              # Custom React hooks
│   │   └── useQuery.ts     # Query submission hook
│   ├── api/                # API client
│   │   └── client.ts       # Axios client with interceptors
│   ├── types/              # TypeScript type definitions
│   │   └── api.ts          # API request/response types
│   ├── styles/             # Global styles
│   │   └── index.css       # Tailwind directives
│   ├── App.tsx             # Main application component
│   └── main.tsx            # Entry point
├── index.html              # HTML template
├── package.json            # Dependencies
├── tsconfig.json           # TypeScript config
├── vite.config.ts          # Vite config
├── tailwind.config.js      # Tailwind config
└── README.md               # This file
```

## Getting Started

### Prerequisites

- Node.js 18+
- npm or yarn
- Backend server running on http://localhost:8080

### Installation

1. Install dependencies:
   ```bash
   npm install
   ```

2. Create environment file:
   ```bash
   cp .env.example .env
   ```

3. Edit `.env` if your backend runs on a different URL:
   ```bash
   VITE_API_URL=http://localhost:8080
   ```

### Development

Start the development server:
```bash
npm run dev
```

The app will be available at http://localhost:5173

### Build for Production

```bash
npm run build
```

The built files will be in the `dist/` directory.

### Preview Production Build

```bash
npm run preview
```

## Features

### Chat Interface
- Natural language input for queries
- Conversation history
- Example queries for quick start
- Real-time loading states

### Preview Tab
- Data table with column types
- Adjustable row limits (10, 50, 100, 500, 1000)
- Refresh button to re-execute with new limit
- Export dropdown (CSV/JSON)
- Execution time display

### Code Tab
- Generated SQL with syntax highlighting
- Copy to clipboard button
- Query rationale and assumptions
- Execution plan details (tables, joins, filters)
- Trace information (model, attempts, revisions)

## API Integration

The frontend communicates with the backend via REST API:

### POST /query
Submit natural language query and get SQL + preview data.

```typescript
const response = await apiClient.query({
  prompt: "show active campaigns",
  auto_approve: true,
  execute_preview: true,
  preview_limit: 10,
});
```

### POST /execute
Execute SQL directly (for refreshing data).

```typescript
const data = await apiClient.execute({
  sql: "SELECT * FROM campaigns_latest LIMIT 10",
  mode: { preview: { limit: 10 } },
  limit: 10,
});
```

### POST /export
Download results as CSV or JSON.

```typescript
await apiClient.exportAndDownload(
  "SELECT * FROM campaigns_latest",
  "csv"
);
```

## Component Overview

### App.tsx
Main component that:
- Manages conversation state
- Coordinates between ChatPanel and ResultsPanel
- Handles query submission and responses
- Manages export and refresh actions

### ChatPanel.tsx
Left panel component:
- Displays message history
- Provides input for new queries
- Shows loading states
- Displays SQL in message bubbles

### ResultsPanel.tsx
Right panel component:
- Tabs for Preview and Code views
- Manages tab switching
- Passes data to child tabs

### PreviewTab.tsx
Data table component:
- Renders query results as table
- Action bar with refresh/export/modify
- Limit selector
- Type-aware cell rendering

### CodeTab.tsx
SQL and plan viewer:
- Formatted SQL display
- Copy to clipboard
- Rationale and assumptions
- Execution plan breakdown
- Trace information

## Hooks

### useQuery
Main hook for submitting queries:
```typescript
const {
  isLoading,
  error,
  response,
  sql,
  previewData,
  submitQuery,
  reset,
  isSuccess,
} = useQuery();
```

### useExecute
Hook for direct SQL execution:
```typescript
const { isLoading, error, data, execute } = useExecute();
```

### useExport
Hook for data export:
```typescript
const { isLoading, error, exportData } = useExport();
```

## Styling

Uses TailwindCSS with custom utilities defined in `src/styles/index.css`:

- `.btn-primary` - Primary button style
- `.btn-secondary` - Secondary button style
- `.input-field` - Text input style
- `.card` - Card container style
- `.code-block` - Code display style

## Environment Variables

- `VITE_API_URL` - Backend API base URL (default: http://localhost:8080)

## Browser Support

- Chrome/Edge (latest)
- Firefox (latest)
- Safari (latest)

## Development Notes

### Proxy Configuration
Vite dev server proxies `/api/*` requests to the backend, but the frontend uses absolute URLs to the backend directly. This allows for flexible deployment.

### Type Safety
All API types are strictly typed in `src/types/api.ts` matching the Rust backend types.

### Error Handling
Axios interceptors handle:
- Network errors
- HTTP error responses
- Timeout errors
- CORS issues

### CORS
Backend must have CORS enabled for `http://localhost:5173` during development.

## Troubleshooting

### Backend Connection Error
- Ensure backend is running on http://localhost:8080
- Check CORS is enabled in backend
- Verify `VITE_API_URL` in `.env`

### Build Errors
- Delete `node_modules` and run `npm install` again
- Clear Vite cache: `rm -rf node_modules/.vite`

### Type Errors
- Run `npm run build` to see all TypeScript errors
- Ensure types in `src/types/api.ts` match backend

## Future Enhancements (Phase 2+)

- [ ] Refine/modify query workflow
- [ ] Session history and persistence
- [ ] Pipeline visualization with node graph
- [ ] Syntax highlighting for SQL (Prism.js)
- [ ] Keyboard shortcuts
- [ ] Dark mode
- [ ] Mobile responsive layout
- [ ] Query templates/favorites

## License

Same as parent project.

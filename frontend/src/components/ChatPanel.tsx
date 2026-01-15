import { useState, useRef, useEffect, FormEvent } from 'react';
import { PaperAirplaneIcon } from '@heroicons/react/24/solid';
import { ChatBubbleLeftIcon, CodeBracketIcon } from '@heroicons/react/24/outline';
import type { ConversationMessage } from '@/types/api';

interface ChatPanelProps {
  messages: ConversationMessage[];
  isLoading: boolean;
  onSubmit: (prompt: string) => void;
}

/**
 * Chat Panel Component
 *
 * Displays conversation history and provides input for new queries.
 * Left side of the split layout.
 */
export function ChatPanel({ messages, isLoading, onSubmit }: ChatPanelProps) {
  const [input, setInput] = useState('');
  const messagesEndRef = useRef<HTMLDivElement>(null);

  // Auto-scroll to bottom when new messages arrive
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  const handleSubmit = (e: FormEvent) => {
    e.preventDefault();
    if (input.trim() && !isLoading) {
      onSubmit(input.trim());
      setInput('');
    }
  };

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="px-6 py-4 border-b border-gray-200">
        <h1 className="text-xl font-semibold text-gray-900">Genie+ QueryGPT</h1>
        <p className="text-sm text-gray-600 mt-1">
          Ask questions about your data in natural language
        </p>
      </div>

      {/* Messages Area */}
      <div className="flex-1 overflow-y-auto px-6 py-4 space-y-4">
        {messages.length === 0 && (
          <div className="flex flex-col items-center justify-center h-full text-center">
            <div className="w-16 h-16 bg-primary-100 rounded-full flex items-center justify-center mb-4">
              <ChatBubbleLeftIcon className="w-8 h-8 text-primary-600" />
            </div>
            <h3 className="text-lg font-medium text-gray-900 mb-2">
              Start a conversation
            </h3>
            <p className="text-sm text-gray-600 max-w-sm">
              Ask about your campaigns, offers, or any data. I'll generate SQL and show you the
              results.
            </p>
            <div className="mt-6 space-y-2 text-left">
              <p className="text-xs font-medium text-gray-500 uppercase tracking-wide">
                Example queries:
              </p>
              <button
                onClick={() => setInput('Export all live ESPN offers with start date in 2025. For each offer export offer id, offer name, offer start date, campaign id, campaign name')}
                className="block w-full text-left px-4 py-2 text-sm text-gray-700 bg-gray-50 rounded-lg hover:bg-gray-100 transition-colors"
              >
                  Export all live ESPN offers with start date in 2025. For each offer export offer id, offer name, offer start date, campaign id, campaign name
              </button>
              <button
                onClick={() => setInput('Find all retail offers in South Korea and Taiwan which have a price defined and show the prices for each product. Show offer id, offer name, product id, product name, price amount, currency. Show offer id, offer name, product id, product name, price amount, currency')}
                className="block w-full text-left px-4 py-2 text-sm text-gray-700 bg-gray-50 rounded-lg hover:bg-gray-100 transition-colors"
              >
                  Find all retail offers in South Korea and Taiwan which have a price defined and show the prices for each product. Show offer id, offer name, product id, product name, price amount, currency. Show offer id, offer name, product id, product name, price amount, currency
              </button>
              <button
                onClick={() => setInput('Find all offers for bundle 29 having offer phase discount and show id, name, phase id, discount id, duration lenght, duration unit, repeat count and discount amount')}
                className="block w-full text-left px-4 py-2 text-sm text-gray-700 bg-gray-50 rounded-lg hover:bg-gray-100 transition-colors"
              >
                  Find all offers for bundle 29 having offer phase discount and show id, name, phase id, discount id, duration lenght, duration unit, repeat count and discount amount
              </button>
            </div>
          </div>
        )}

        {messages.map((message) => (
          <MessageBubble key={message.id} message={message} />
        ))}

        {isLoading && (
          <div className="flex items-start space-x-3">
            <div className="w-8 h-8 bg-primary-100 rounded-full flex items-center justify-center flex-shrink-0">
              <ChatBubbleLeftIcon className="w-5 h-5 text-primary-600" />
            </div>
            <div className="flex-1">
              <div className="bg-gray-100 rounded-lg px-4 py-3">
                <div className="flex space-x-2">
                  <div className="w-2 h-2 bg-gray-400 rounded-full animate-bounce" />
                  <div className="w-2 h-2 bg-gray-400 rounded-full animate-bounce [animation-delay:0.2s]" />
                  <div className="w-2 h-2 bg-gray-400 rounded-full animate-bounce [animation-delay:0.4s]" />
                </div>
              </div>
            </div>
          </div>
        )}

        <div ref={messagesEndRef} />
      </div>

      {/* Input Area */}
      <div className="px-6 py-4 border-t border-gray-200 bg-white">
        <form onSubmit={handleSubmit} className="flex space-x-2">
          <input
            type="text"
            value={input}
            onChange={(e) => setInput(e.target.value)}
            placeholder="Ask a question about your data..."
            className="input-field flex-1"
            disabled={isLoading}
          />
          <button
            type="submit"
            disabled={isLoading || !input.trim()}
            className="btn-primary"
          >
            <PaperAirplaneIcon className="w-5 h-5" />
          </button>
        </form>
      </div>
    </div>
  );
}

/**
 * Message Bubble Component
 *
 * Displays individual messages in the conversation.
 */
function MessageBubble({ message }: { message: ConversationMessage }) {
  const isUser = message.type === 'user';

  return (
    <div className={`flex items-start space-x-3 ${isUser ? 'justify-end' : ''}`}>
      {!isUser && (
        <div className="w-8 h-8 bg-primary-100 rounded-full flex items-center justify-center flex-shrink-0">
          <ChatBubbleLeftIcon className="w-5 h-5 text-primary-600" />
        </div>
      )}

      <div className={`flex-1 ${isUser ? 'flex justify-end' : ''}`}>
        <div
          className={`rounded-lg px-4 py-3 max-w-[85%] ${
            isUser
              ? 'bg-primary-600 text-white'
              : 'bg-gray-100 text-gray-900'
          }`}
        >
          <p className="text-sm whitespace-pre-wrap">{message.content}</p>

          {message.error && (
            <div className="mt-2 p-2 bg-red-50 border border-red-200 rounded text-red-700 text-xs">
              {message.error}
            </div>
          )}

          {message.preview_data && (
            <div className="mt-3 p-3 bg-white border border-gray-200 rounded-lg">
              <div className="flex items-center justify-between mb-3">
                <h4 className="text-xs font-semibold text-gray-700 uppercase tracking-wide">
                  Query Executed Successfully
                </h4>
                <CodeBracketIcon className="w-4 h-4 text-gray-400" />
              </div>
              <div className="space-y-2">
                {message.preview_data.total_matching_rows !== undefined && (
                  <div className="flex items-center justify-between text-xs">
                    <span className="text-gray-600">Records Found:</span>
                    <span className="font-semibold text-green-600">
                      {message.preview_data.total_matching_rows.toLocaleString()}
                    </span>
                  </div>
                )}
                <div className="flex items-center justify-between text-xs">
                  <span className="text-gray-600">Preview Rows:</span>
                  <span className="font-semibold text-blue-600">
                    {message.preview_data.total_rows}
                  </span>
                </div>
                <div className="flex items-center justify-between text-xs">
                  <span className="text-gray-600">Columns:</span>
                  <span className="font-semibold text-purple-600">
                    {message.preview_data.columns.length}
                  </span>
                </div>
                <div className="flex items-center justify-between text-xs">
                  <span className="text-gray-600">Execution Time:</span>
                  <span className="font-semibold text-orange-600">
                    {message.preview_data.execution_time_ms}ms
                  </span>
                </div>
                {message.preview_data.columns.length > 0 && (
                  <div className="mt-2 pt-2 border-t border-gray-200">
                    <span className="text-xs text-gray-600 block mb-1">Fields:</span>
                    <div className="flex flex-wrap gap-1">
                      {message.preview_data.columns.slice(0, 5).map((col, idx) => (
                        <span
                          key={idx}
                          className="inline-block px-2 py-0.5 bg-blue-50 text-blue-700 text-[10px] rounded"
                        >
                          {col.name}
                        </span>
                      ))}
                      {message.preview_data.columns.length > 5 && (
                        <span className="inline-block px-2 py-0.5 bg-gray-100 text-gray-600 text-[10px] rounded">
                          +{message.preview_data.columns.length - 5} more
                        </span>
                      )}
                    </div>
                  </div>
                )}
              </div>
            </div>
          )}

          {message.timestamp && (
            <p className={`text-xs mt-2 ${isUser ? 'text-primary-200' : 'text-gray-500'}`}>
              {message.timestamp.toLocaleTimeString()}
            </p>
          )}
        </div>
      </div>

      {isUser && (
        <div className="w-8 h-8 bg-gray-200 rounded-full flex items-center justify-center flex-shrink-0">
          <span className="text-gray-600 text-sm font-medium">You</span>
        </div>
      )}
    </div>
  );
}

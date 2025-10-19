import { FormEvent, useEffect, useRef, useState } from "react";

type SessionState = "running" | "completed" | "failed";

type SessionEventKind = "started" | "completed" | "error";

type SessionEvent = {
  kind: SessionEventKind;
  message?: string;
  summary?: string;
  trace_available?: boolean;
};

type StartSessionResponse = {
  session_id: string;
  state: SessionState;
  message?: string;
};

type TraceResponse = {
  session_id: string;
  summary: string;
  trace_events: Array<{
    task_id: string;
    message: string;
    timestamp_ms: number;
  }>;
  explain_markdown?: string;
  explain_mermaid?: string;
  explain_graphviz?: string;
};

type SessionStatus = {
  session_id: string;
  state: SessionState;
  summary?: string;
  error?: string;
  trace_available: boolean;
};

const fetchSessionStatus = async (sessionId: string): Promise<SessionStatus> => {
  const response = await fetch(`/api/sessions/${sessionId}`);
  if (!response.ok) {
    throw new Error(`Failed to load session status (${response.status})`);
  }
  return response.json();
};

const fetchTrace = async (sessionId: string): Promise<TraceResponse> => {
  const response = await fetch(`/api/sessions/${sessionId}/trace`);
  if (!response.ok) {
    throw new Error(`Trace not available (${response.status})`);
  }
  return response.json();
};

const App = () => {
  const [query, setQuery] = useState("Compare battery manufacturers Q4 revenue growth");
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [sessionState, setSessionState] = useState<SessionState | null>(null);
  const [summary, setSummary] = useState<string | null>(null);
  const [traceMarkdown, setTraceMarkdown] = useState<string | null>(null);
  const [events, setEvents] = useState<SessionEvent[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const eventSourceRef = useRef<EventSource | null>(null);

  useEffect(() => {
    return () => {
      eventSourceRef.current?.close();
    };
  }, []);

  const appendEvent = (event: SessionEvent) => {
    setEvents((current) => [...current, event]);
  };

  const subscribeToEvents = (id: string) => {
    eventSourceRef.current?.close();
    const source = new EventSource(`/api/sessions/${id}/stream`);

    source.addEventListener("started", (evt) => {
      const payload: SessionEvent = JSON.parse((evt as MessageEvent).data);
      appendEvent(payload);
      setSessionState("running");
    });

    source.addEventListener("completed", async (evt) => {
      const payload: SessionEvent = JSON.parse((evt as MessageEvent).data);
      appendEvent(payload);
      setSessionState("completed");
      source.close();

      try {
        const trace = await fetchTrace(id);
        setSummary(trace.summary);
        setTraceMarkdown(trace.explain_markdown ?? null);
      } catch (traceError) {
        console.error(traceError);
      }
    });

    source.addEventListener("error", (evt) => {
      const payload = safeParseEvent(evt);
      appendEvent(payload);
      setSessionState("failed");
      setError(payload.message ?? "Session failed");
      source.close();
    });

    eventSourceRef.current = source;
  };

  const safeParseEvent = (evt: Event): SessionEvent => {
    if (evt instanceof MessageEvent && typeof evt.data === "string") {
      try {
        return JSON.parse(evt.data) as SessionEvent;
      } catch (parseError) {
        console.error("Failed to parse session event", parseError);
      }
    }
    return { kind: "error", message: "Unexpected event payload", trace_available: false };
  };

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setIsLoading(true);
    setError(null);
    setSummary(null);
    setTraceMarkdown(null);
    setEvents([]);

    try {
      const response = await fetch(`/api/sessions`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json"
        },
        body: JSON.stringify({ query })
      });

      if (!response.ok) {
        const text = await response.text();
        throw new Error(text || `Failed to start session (${response.status})`);
      }

      const data: StartSessionResponse = await response.json();
      setSessionId(data.session_id);
      setSessionState(data.state);
      appendEvent({ kind: "started", message: data.message ?? "Session started" });
      subscribeToEvents(data.session_id);

      try {
        const status = await fetchSessionStatus(data.session_id);
        if (status.summary) {
          setSummary(status.summary);
        }
      } catch (statusError) {
        console.warn(statusError);
      }
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setError(message);
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <div className="min-h-screen bg-slate-950">
      <header className="border-b border-slate-800 bg-slate-900/80 backdrop-blur">
        <div className="mx-auto flex max-w-6xl flex-col gap-1 px-6 py-6">
          <h1 className="text-2xl font-semibold text-white">
            DeepResearch GUI (v0.2 preview)
          </h1>
          <p className="text-sm text-slate-300">
            Launch multi-agent research sessions with at-a-glance progress and traces when runs
            finish.
          </p>
        </div>
      </header>

      <main className="mx-auto flex max-w-6xl flex-col gap-6 px-6 py-8">
        <section className="rounded-lg border border-slate-800 bg-slate-900/60 p-6 shadow-lg shadow-slate-900/30">
          <form className="flex flex-col gap-4" onSubmit={handleSubmit}>
            <label className="flex flex-col gap-2">
              <span className="text-sm font-medium text-slate-200">Research prompt</span>
              <textarea
                value={query}
                onChange={(evt) => setQuery(evt.target.value)}
                className="min-h-[120px] rounded-md border border-slate-700 bg-slate-950 px-4 py-3 text-base text-slate-100 outline-none transition focus:border-primary focus:ring-2 focus:ring-primary/50"
                placeholder="Describe the analysis you need"
                required
              />
            </label>
            <div className="flex items-center justify-between gap-4">
              <div className="text-sm text-slate-400">
                Sessions stream progress via SSE; results appear below once complete.
              </div>
              <button
                type="submit"
                disabled={isLoading}
                className="rounded-md bg-primary px-5 py-2 text-sm font-semibold text-white shadow transition hover:bg-primary-dark disabled:cursor-not-allowed disabled:bg-slate-700"
              >
                {isLoading ? "Running…" : "Run research"}
              </button>
            </div>
          </form>
        </section>

        {error && (
          <section className="rounded-lg border border-red-800/60 bg-red-900/20 px-6 py-4">
            <p className="text-sm text-red-200">{error}</p>
          </section>
        )}

        <section className="grid gap-6 lg:grid-cols-2">
          <div className="flex h-full flex-col gap-4 rounded-lg border border-slate-800 bg-slate-900/60 p-6">
            <header className="flex items-center justify-between">
              <h2 className="text-lg font-semibold text-white">Session Events</h2>
              {sessionState && (
                <span className="rounded-full border border-slate-700 bg-slate-800 px-3 py-1 text-xs font-medium uppercase tracking-wide text-slate-300">
                  {sessionState}
                </span>
              )}
            </header>
            <div className="space-y-3 overflow-y-auto pr-2">
              {events.length === 0 && (
                <p className="text-sm text-slate-500">No events yet. Submit a query to begin.</p>
              )}
              {events.map((event, index) => (
                <article
                  key={`${event.kind}-${index}`}
                  className="rounded-md border border-slate-800/70 bg-slate-900/70 px-4 py-3"
                >
                  <p className="text-sm font-semibold text-slate-200">{event.kind}</p>
                  {event.message && (
                    <p className="mt-1 text-sm text-slate-400">{event.message}</p>
                  )}
                  {event.summary && (
                    <p className="mt-2 text-sm text-slate-200">{event.summary}</p>
                  )}
                </article>
              ))}
            </div>
          </div>

          <div className="flex h-full flex-col gap-4 rounded-lg border border-slate-800 bg-slate-900/60 p-6">
            <h2 className="text-lg font-semibold text-white">Summary & Trace</h2>
            {summary ? (
              <div className="prose prose-invert max-w-none">
                <h3>Summary</h3>
                <p>{summary}</p>
                {traceMarkdown && (
                  <>
                    <h3>Trace</h3>
                    <pre className="overflow-x-auto whitespace-pre-wrap rounded-md bg-slate-950/80 p-4 text-sm text-slate-200">
                      {traceMarkdown}
                    </pre>
                  </>
                )}
              </div>
            ) : sessionState === "running" ? (
              <p className="text-sm text-slate-400">Running analysis…</p>
            ) : (
              <p className="text-sm text-slate-500">Run a session to view the summary.</p>
            )}
          </div>
        </section>

        {sessionId && (
          <section className="rounded-lg border border-slate-800 bg-slate-900/60 px-6 py-4 text-xs text-slate-400">
            <span className="font-semibold text-slate-300">Session ID:</span> {sessionId}
          </section>
        )}
      </main>
    </div>
  );
};

export default App;

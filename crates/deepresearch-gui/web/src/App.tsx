import { FormEvent, useEffect, useRef, useState } from "react";

type SessionState = "running" | "completed" | "failed";

type SessionEventKind = "started" | "completed" | "error";

type SessionEvent = {
  kind: SessionEventKind;
  message?: string;
  summary?: string;
  trace_available?: boolean;
};

type CapacitySnapshot = {
  max_concurrency: number;
  available_permits: number;
  running_sessions: number;
  total_sessions: number;
};

type StartSessionResponse = {
  session_id: string;
  state: SessionState;
  capacity: CapacitySnapshot;
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
  trace_summary: {
    steps: Array<{
      index: number;
      task_id: string;
      message: string;
    }>;
  };
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

type ListSessionsResponse = {
  sessions: SessionStatus[];
  capacity: CapacitySnapshot;
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

const fetchSessionsDirectory = async (): Promise<ListSessionsResponse> => {
  const response = await fetch(`/api/sessions`);
  if (!response.ok) {
    throw new Error(`Failed to list sessions (${response.status})`);
  }
  return response.json();
};

const App = () => {
  const [query, setQuery] = useState("Compare battery manufacturers Q4 revenue growth");
  const [sessionInput, setSessionInput] = useState("");
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [sessionState, setSessionState] = useState<SessionState | null>(null);
  const [summary, setSummary] = useState<string | null>(null);
  const [traceMarkdown, setTraceMarkdown] = useState<string | null>(null);
  const [traceEvents, setTraceEvents] = useState<TraceResponse["trace_events"] | null>(null);
  const [traceSteps, setTraceSteps] = useState<TraceResponse["trace_summary"]["steps"] | null>(null);
  const [events, setEvents] = useState<SessionEvent[]>([]);
  const [sessions, setSessions] = useState<SessionStatus[]>([]);
  const [capacity, setCapacity] = useState<CapacitySnapshot | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const eventSourceRef = useRef<EventSource | null>(null);

  useEffect(() => {
    refreshSessions();
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
        setTraceEvents(trace.trace_events);
        setTraceSteps(trace.trace_summary.steps);
      } catch (traceError) {
        console.error(traceError);
      } finally {
        refreshSessions();
      }
    });

    source.addEventListener("error", (evt) => {
      const payload = safeParseEvent(evt);
      appendEvent(payload);
      setSessionState("failed");
      setError(payload.message ?? "Session failed");
      source.close();
      refreshSessions();
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

  const refreshSessions = async () => {
    try {
      const data = await fetchSessionsDirectory();
      setSessions(data.sessions);
      setCapacity(data.capacity);
    } catch (err) {
      console.warn("Unable to refresh session directory", err);
    }
  };

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setIsLoading(true);
    setError(null);
    setSummary(null);
    setTraceMarkdown(null);
    setTraceEvents(null);
    setTraceSteps(null);
    setEvents([]);

    try {
      const response = await fetch(`/api/sessions`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json"
        },
        body: JSON.stringify({
          query,
          session_id: sessionInput.trim() ? sessionInput.trim() : undefined
        })
      });

      if (!response.ok) {
        const text = await response.text();
        throw new Error(text || `Failed to start session (${response.status})`);
      }

      const data: StartSessionResponse = await response.json();
      setSessionId(data.session_id);
      setSessionState(data.state);
      setCapacity(data.capacity);
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

      await refreshSessions();
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setError(message);
    } finally {
      setIsLoading(false);
    }
  };

  const handleSessionSelect = async (id: string) => {
    setSessionInput(id);
    setSessionId(id);
    setTraceEvents(null);
    setTraceSteps(null);
    setTraceMarkdown(null);
    setSummary(null);
    setEvents([]);
    setError(null);

    try {
      const status = await fetchSessionStatus(id);
      setSessionState(status.state);
      if (status.summary) {
        setSummary(status.summary);
      }
      if (status.trace_available) {
        const trace = await fetchTrace(id);
        setTraceEvents(trace.trace_events);
        setTraceSteps(trace.trace_summary.steps);
        setTraceMarkdown(trace.explain_markdown ?? null);
        setSummary(trace.summary);
      }
    } catch (err) {
      console.warn("Unable to hydrate session", err);
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
            <label className="flex flex-col gap-2">
              <span className="text-sm font-medium text-slate-200">Session ID (optional)</span>
              <input
                value={sessionInput}
                onChange={(evt) => setSessionInput(evt.target.value)}
                placeholder="Provide an existing ID to resume or namespace a run"
                className="rounded-md border border-slate-700 bg-slate-950 px-4 py-2 text-sm text-slate-100 outline-none transition focus:border-primary focus:ring-2 focus:ring-primary/40"
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

        {capacity && (
          <section className="grid gap-4 rounded-lg border border-slate-800 bg-slate-900/60 p-6 text-sm text-slate-300 sm:grid-cols-4">
            <div>
              <p className="text-xs uppercase text-slate-500">Max concurrency</p>
              <p className="text-xl font-semibold text-white">{capacity.max_concurrency}</p>
            </div>
            <div>
              <p className="text-xs uppercase text-slate-500">Available slots</p>
              <p className="text-xl font-semibold text-white">{capacity.available_permits}</p>
            </div>
            <div>
              <p className="text-xs uppercase text-slate-500">Running</p>
              <p className="text-xl font-semibold text-white">{capacity.running_sessions}</p>
            </div>
            <div>
              <p className="text-xs uppercase text-slate-500">Tracked sessions</p>
              <p className="text-xl font-semibold text-white">{capacity.total_sessions}</p>
            </div>
          </section>
        )}

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
                <p className="text-sm text-slate-500">No events yet. Submit or select a session to begin.</p>
              )}
              {events.map((event, index) => (
                <article
                  key={`${event.kind}-${index}-${event.message ?? ""}`}
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
            <div className="flex items-center justify-between">
              <h2 className="text-lg font-semibold text-white">Summary & Trace</h2>
              <button
                type="button"
                onClick={refreshSessions}
                className="text-xs font-medium text-primary hover:text-primary-dark"
              >
                Refresh directory
              </button>
            </div>
            {summary ? (
              <div className="flex flex-col gap-4">
                <div className="prose prose-invert max-w-none">
                  <h3>Summary</h3>
                  <p>{summary}</p>
                </div>
                {traceMarkdown && (
                  <details className="rounded-md border border-slate-800 bg-slate-950/70">
                    <summary className="cursor-pointer px-4 py-2 text-sm font-medium text-slate-200">
                      Trace Markdown
                    </summary>
                    <pre className="overflow-x-auto whitespace-pre-wrap px-4 py-3 text-sm text-slate-200">
                      {traceMarkdown}
                    </pre>
                  </details>
                )}
                {traceSteps && traceSteps.length > 0 && (
                  <details className="rounded-md border border-slate-800 bg-slate-950/70">
                    <summary className="cursor-pointer px-4 py-2 text-sm font-medium text-slate-200">
                      Reasoning Steps
                    </summary>
                    <div className="space-y-2 px-4 py-3 text-sm text-slate-200">
                      {traceSteps.map((step) => (
                        <div key={step.index} className="border-b border-slate-800 pb-2 last:border-b-0">
                          <p className="font-semibold text-primary">{step.index}. {step.task_id}</p>
                          <p className="text-slate-300">{step.message}</p>
                        </div>
                      ))}
                    </div>
                  </details>
                )}
                {traceEvents && traceEvents.length > 0 && (
                  <details className="rounded-md border border-slate-800 bg-slate-950/70">
                    <summary className="cursor-pointer px-4 py-2 text-sm font-medium text-slate-200">
                      Raw Trace Events
                    </summary>
                    <pre className="overflow-x-auto whitespace-pre-wrap px-4 py-3 text-xs text-slate-300">
                      {JSON.stringify(traceEvents, null, 2)}
                    </pre>
                  </details>
                )}
              </div>
            ) : sessionState === "running" ? (
              <p className="text-sm text-slate-400">Running analysis…</p>
            ) : (
              <p className="text-sm text-slate-500">Run or select a session to view the summary.</p>
            )}
          </div>
        </section>

        <section className="rounded-lg border border-slate-800 bg-slate-900/60 p-6">
          <header className="mb-4 flex items-center justify-between">
            <div>
              <h2 className="text-lg font-semibold text-white">Session Directory</h2>
              <p className="text-xs text-slate-400">
                Resume, inspect, or continue existing sessions (namespaced IDs supported).
              </p>
            </div>
            <button
              type="button"
              onClick={refreshSessions}
              className="rounded-md border border-slate-700 px-3 py-1 text-xs font-medium text-slate-200 transition hover:border-primary hover:text-primary"
            >
              Refresh
            </button>
          </header>
          <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
            {sessions.length === 0 && (
              <p className="text-sm text-slate-500">No sessions recorded yet.</p>
            )}
            {sessions.map((session) => (
              <button
                key={session.session_id}
                type="button"
                onClick={() => handleSessionSelect(session.session_id)}
                className="flex flex-col gap-1 rounded-md border border-slate-800 bg-slate-900/80 px-4 py-3 text-left text-sm text-slate-200 transition hover:border-primary hover:text-white"
              >
                <span className="font-semibold text-primary">{session.session_id}</span>
                <span className="text-xs uppercase text-slate-500">{session.state}</span>
                {session.summary && (
                  <span className="text-xs text-slate-400 line-clamp-2">{session.summary}</span>
                )}
                {session.error && (
                  <span className="text-xs text-red-400">{session.error}</span>
                )}
              </button>
            ))}
          </div>
        </section>

        {(sessionId || sessionInput.trim()) && (
          <section className="rounded-lg border border-slate-800 bg-slate-900/60 px-6 py-4 text-xs text-slate-400">
            <span className="font-semibold text-slate-300">Session ID:</span> {sessionId ?? sessionInput}
          </section>
        )}
      </main>
    </div>
  );
};

export default App;

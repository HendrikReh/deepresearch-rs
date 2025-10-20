import { ChangeEvent, FormEvent, useEffect, useMemo, useRef, useState } from "react";

type SessionState = "running" | "completed" | "failed";

type SessionEventKind = "started" | "completed" | "error";

type SessionEvent = {
  kind: SessionEventKind;
  message?: string;
  summary?: string;
  trace_available?: boolean;
  requires_manual?: boolean;
};

type TraceEventRecord = {
  task_id: string;
  message: string;
  timestamp_ms: number;
};

type TraceStep = {
  index: number;
  task_id: string;
  message: string;
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
  trace_events: TraceEventRecord[];
  trace_summary: {
    steps: TraceStep[];
  };
  timeline: TimelinePoint[];
  task_metrics: TaskMetric[];
  artifacts: TraceArtifacts;
  requires_manual: boolean;
  fact_check?: FactCheckSnapshot;
  critic?: CriticSnapshot;
  trace_path?: string;
};

type SessionStatus = {
  session_id: string;
  state: SessionState;
  summary?: string;
  error?: string;
  trace_available: boolean;
  requires_manual: boolean;
};

type ListSessionsResponse = {
  sessions: SessionStatus[];
  capacity: CapacitySnapshot;
};

type TimelinePoint = {
  step_index: number;
  task_id: string;
  message: string;
  timestamp_ms: number;
  offset_ms: number;
  duration_ms?: number;
};

type TaskMetric = {
  task_id: string;
  occurrences: number;
  total_duration_ms?: number;
  average_duration_ms?: number;
};

type FactCheckSnapshot = {
  confidence: number;
  passed: boolean;
  verified_sources: string[];
};

type CriticSnapshot = {
  confident: boolean;
};

type TraceArtifacts = {
  markdown?: string;
  mermaid?: string;
  graphviz?: string;
};

type ArtifactVisibilityKey = "timeline" | "metrics" | "markdown" | "mermaid" | "graphviz";

const formatMilliseconds = (value?: number) => {
  if (value === undefined || value === null) {
    return "—";
  }
  if (value < 1000) {
    return `${value} ms`;
  }
  if (value < 60_000) {
    return `${(value / 1000).toFixed(2)} s`;
  }
  return `${(value / 60_000).toFixed(2)} min`;
};

const formatDelta = (value?: number) => {
  if (value === undefined || value === null) {
    return "—";
  }
  const absolute = Math.abs(value);
  const sign = value > 0 ? "+" : value < 0 ? "-" : "";
  return `${sign}${formatMilliseconds(absolute)}`;
};

const formatTimestamp = (value: number) => {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value.toString();
  }
  const base = date.toLocaleTimeString([], {
    hour12: false,
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit"
  });
  const ms = String(value % 1000).padStart(3, "0");
  return `${base}.${ms}`;
};

const ReasoningGraph = ({ timeline }: { timeline: TimelinePoint[] }) => (
  <div className="overflow-x-auto rounded-md border border-slate-800 bg-slate-950/70 p-4">
    <div className="flex flex-col gap-6">
      {timeline.map((point, index) => (
        <div
          key={`${point.step_index}-${point.task_id}`}
          className="relative flex items-start gap-3 md:items-center"
        >
          <div className="flex flex-col items-center">
            <span className="flex h-10 w-10 items-center justify-center rounded-full border-2 border-primary bg-slate-950 text-sm font-semibold text-primary">
              {point.step_index}
            </span>
            {index < timeline.length - 1 && (
              <span className="mt-2 hidden h-full w-px flex-1 bg-slate-700 md:block" />
            )}
          </div>
          <div className="flex-1 rounded-md border border-slate-800 bg-slate-950/80 p-3 text-sm text-slate-200 shadow-inner shadow-slate-900/40">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <p className="font-semibold text-primary">{point.task_id}</p>
              <span className="text-xs text-slate-400">
                {formatTimestamp(point.timestamp_ms)}
              </span>
            </div>
            <p className="mt-2 text-slate-300">{point.message}</p>
            <div className="mt-3 flex flex-wrap items-center gap-3 text-xs text-slate-400">
              <span>Offset: {formatMilliseconds(point.offset_ms)}</span>
              <span>Duration: {formatMilliseconds(point.duration_ms)}</span>
            </div>
          </div>
          {index < timeline.length - 1 && (
            <div className="absolute bottom-[-24px] left-5 hidden h-12 w-px bg-slate-700 md:block" />
          )}
        </div>
      ))}
    </div>
  </div>
);

const TimelineTable = ({ timeline, extent }: { timeline: TimelinePoint[]; extent: number }) => (
  <div className="rounded-md border border-slate-800 bg-slate-950/70">
    <div className="overflow-x-auto">
      <table className="min-w-full divide-y divide-slate-800 text-xs text-slate-300">
        <thead className="bg-slate-900/70 text-slate-400">
          <tr>
            <th className="px-3 py-2 text-left font-semibold uppercase tracking-wide">Step</th>
            <th className="px-3 py-2 text-left font-semibold uppercase tracking-wide">Task</th>
            <th className="px-3 py-2 text-left font-semibold uppercase tracking-wide">Message</th>
            <th className="px-3 py-2 text-left font-semibold uppercase tracking-wide">Start</th>
            <th className="px-3 py-2 text-left font-semibold uppercase tracking-wide">Offset</th>
            <th className="px-3 py-2 text-left font-semibold uppercase tracking-wide">Duration</th>
          </tr>
        </thead>
        <tbody className="divide-y divide-slate-800">
          {timeline.map((point) => {
            const progress =
              extent > 0 ? ((point.duration_ms ?? 0) / extent) * 100 : point.duration_ms ? 100 : 0;
            return (
              <tr key={`${point.step_index}-${point.task_id}`} className="hover:bg-slate-900/80">
                <td className="px-3 py-2 font-semibold text-primary">{point.step_index}</td>
                <td className="px-3 py-2 font-mono text-slate-200">{point.task_id}</td>
                <td className="px-3 py-2 text-slate-300">{point.message}</td>
                <td className="px-3 py-2 text-slate-400">{formatTimestamp(point.timestamp_ms)}</td>
                <td className="px-3 py-2 text-slate-400">{formatMilliseconds(point.offset_ms)}</td>
                <td className="px-3 py-2 text-slate-400">
                  <div className="flex items-center gap-2">
                    <div className="relative h-2 flex-1 overflow-hidden rounded bg-slate-800">
                      <span
                        className="absolute left-0 top-0 h-full rounded bg-primary transition-[width]"
                        style={{ width: `${progress}%` }}
                      />
                    </div>
                    <span>{formatMilliseconds(point.duration_ms)}</span>
                  </div>
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  </div>
);

const MetricsTable = ({ metrics }: { metrics: TaskMetric[] }) => (
  <div className="rounded-md border border-slate-800 bg-slate-950/70">
    <div className="overflow-x-auto">
      <table className="min-w-full divide-y divide-slate-800 text-xs text-slate-300">
        <thead className="bg-slate-900/70 text-slate-400">
          <tr>
            <th className="px-3 py-2 text-left font-semibold uppercase tracking-wide">Task</th>
            <th className="px-3 py-2 text-left font-semibold uppercase tracking-wide">Occurrences</th>
            <th className="px-3 py-2 text-left font-semibold uppercase tracking-wide">Total Duration</th>
            <th className="px-3 py-2 text-left font-semibold uppercase tracking-wide">Avg Duration</th>
          </tr>
        </thead>
        <tbody className="divide-y divide-slate-800">
          {metrics.map((metric) => (
            <tr key={metric.task_id} className="hover:bg-slate-900/80">
              <td className="px-3 py-2 font-mono text-slate-200">{metric.task_id}</td>
              <td className="px-3 py-2 text-slate-300">{metric.occurrences}</td>
              <td className="px-3 py-2 text-slate-400">{formatMilliseconds(metric.total_duration_ms)}</td>
              <td className="px-3 py-2 text-slate-400">{formatMilliseconds(metric.average_duration_ms)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  </div>
);

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
  const [traceArtifacts, setTraceArtifacts] = useState<TraceArtifacts | null>(null);
  const [traceEvents, setTraceEvents] = useState<TraceEventRecord[] | null>(null);
  const [traceSteps, setTraceSteps] = useState<TraceStep[] | null>(null);
  const [traceTimeline, setTraceTimeline] = useState<TimelinePoint[] | null>(null);
  const [taskMetrics, setTaskMetrics] = useState<TaskMetric[] | null>(null);
  const [factCheckSnapshot, setFactCheckSnapshot] = useState<FactCheckSnapshot | null>(null);
  const [criticSnapshot, setCriticSnapshot] = useState<CriticSnapshot | null>(null);
  const [requiresManual, setRequiresManual] = useState<boolean>(false);
  const [artifactVisibility, setArtifactVisibility] = useState<Record<ArtifactVisibilityKey, boolean>>({
    timeline: true,
    metrics: true,
    markdown: false,
    mermaid: true,
    graphviz: false
  });
  const [comparisonId, setComparisonId] = useState<string | null>(null);
  const [comparisonSummary, setComparisonSummary] = useState<string | null>(null);
  const [comparisonMetrics, setComparisonMetrics] = useState<TaskMetric[] | null>(null);
  const [comparisonTimeline, setComparisonTimeline] = useState<TimelinePoint[] | null>(null);
  const [comparisonError, setComparisonError] = useState<string | null>(null);
  const [events, setEvents] = useState<SessionEvent[]>([]);
  const [sessions, setSessions] = useState<SessionStatus[]>([]);
  const [capacity, setCapacity] = useState<CapacitySnapshot | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const eventSourceRef = useRef<EventSource | null>(null);

  const timelineExtent = useMemo(() => {
    if (!traceTimeline || traceTimeline.length === 0) {
      return 0;
    }
    return traceTimeline.reduce((acc, point) => {
      const end = point.offset_ms + (point.duration_ms ?? 0);
      return end > acc ? end : acc;
    }, 0);
  }, [traceTimeline]);

  const comparisonExtent = useMemo(() => {
    if (!comparisonTimeline || comparisonTimeline.length === 0) {
      return 0;
    }
    return comparisonTimeline.reduce((acc, point) => {
      const end = point.offset_ms + (point.duration_ms ?? 0);
      return end > acc ? end : acc;
    }, 0);
  }, [comparisonTimeline]);

  const toggleArtifact = (key: ArtifactVisibilityKey) => {
    setArtifactVisibility((current) => ({
      ...current,
      [key]: !current[key]
    }));
  };

  const sessionSlug = useMemo(() => {
    const base = sessionId ?? "session";
    return base.replace(/[^a-zA-Z0-9_-]+/g, "-");
  }, [sessionId]);

  const metricsComparison = useMemo(() => {
    if (!taskMetrics && !comparisonMetrics) {
      return [];
    }
    const currentMap = new Map<string, TaskMetric>(
      (taskMetrics ?? []).map((metric) => [metric.task_id, metric])
    );
    const comparisonMap = new Map<string, TaskMetric>(
      (comparisonMetrics ?? []).map((metric) => [metric.task_id, metric])
    );
    const allKeys = Array.from(
      new Set<string>([...currentMap.keys(), ...comparisonMap.keys()])
    ).sort((a, b) => a.localeCompare(b));

    return allKeys.map((taskId) => {
      const current = currentMap.get(taskId) ?? null;
      const prev = comparisonMap.get(taskId) ?? null;
      const delta =
        current?.average_duration_ms !== undefined && prev?.average_duration_ms !== undefined
          ? current.average_duration_ms - prev.average_duration_ms
          : undefined;
      return { task_id: taskId, current, prev, delta };
    });
  }, [taskMetrics, comparisonMetrics]);

  const resetTraceState = () => {
    setSummary(null);
    setTraceArtifacts(null);
    setTraceEvents(null);
    setTraceSteps(null);
    setTraceTimeline(null);
    setTaskMetrics(null);
    setFactCheckSnapshot(null);
    setCriticSnapshot(null);
    setRequiresManual(false);
    setArtifactVisibility({
      timeline: true,
      metrics: true,
      markdown: false,
      mermaid: true,
      graphviz: false
    });
  };

  const resetComparison = () => {
    setComparisonId(null);
    setComparisonSummary(null);
    setComparisonMetrics(null);
    setComparisonTimeline(null);
    setComparisonError(null);
  };

  const downloadArtifact = (filename: string, content: string) => {
    const blob = new Blob([content], { type: "text/plain;charset=utf-8" });
    const href = URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = href;
    anchor.download = filename;
    document.body.appendChild(anchor);
    anchor.click();
    document.body.removeChild(anchor);
    URL.revokeObjectURL(href);
  };

  const toggleOptions: Array<{ key: ArtifactVisibilityKey; label: string; disabled: boolean }> = [
    {
      key: "timeline",
      label: "Timeline",
      disabled: !traceTimeline || traceTimeline.length === 0
    },
    {
      key: "metrics",
      label: "Metrics",
      disabled: !taskMetrics || taskMetrics.length === 0
    },
    {
      key: "markdown",
      label: "Markdown",
      disabled: !traceArtifacts?.markdown
    },
    {
      key: "mermaid",
      label: "Graph",
      disabled: !traceTimeline || traceTimeline.length === 0
    },
    {
      key: "graphviz",
      label: "Graphviz",
      disabled: !traceArtifacts?.graphviz
    }
  ];

  const downloadOptions: Array<{
    key: "markdown" | "mermaid" | "graphviz";
    label: string;
    extension: string;
    content?: string;
  }> = [
    { key: "markdown", label: "Markdown", extension: "md", content: traceArtifacts?.markdown },
    { key: "mermaid", label: "Mermaid", extension: "mmd", content: traceArtifacts?.mermaid },
    { key: "graphviz", label: "Graphviz", extension: "gv", content: traceArtifacts?.graphviz }
  ];

  const comparableSessions = useMemo(
    () => sessions.filter((session) => session.trace_available),
    [sessions]
  );

  useEffect(() => {
    refreshSessions();
    return () => {
      eventSourceRef.current?.close();
    };
  }, []);

  const appendEvent = (event: SessionEvent) => {
    setEvents((current) => [...current, event]);
    if (typeof event.requires_manual === "boolean") {
      setRequiresManual(event.requires_manual);
    }
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
        setTraceEvents(trace.trace_events);
        setTraceSteps(trace.trace_summary.steps);
        setTraceArtifacts(trace.artifacts ?? null);
        setTraceTimeline(trace.timeline);
        setTaskMetrics(trace.task_metrics);
        setFactCheckSnapshot(trace.fact_check ?? null);
        setCriticSnapshot(trace.critic ?? null);
        setRequiresManual(trace.requires_manual);
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
      if (payload.message) {
        setError(payload.message);
      } else {
        setError("Session failed");
      }
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
    resetTraceState();
    setEvents([]);
    resetComparison();

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
    resetTraceState();
    setEvents([]);
    setError(null);

    try {
      const status = await fetchSessionStatus(id);
      setSessionState(status.state);
      setRequiresManual(status.requires_manual ?? false);
      if (status.summary) {
        setSummary(status.summary);
      }
      if (status.trace_available) {
        const trace = await fetchTrace(id);
        setTraceEvents(trace.trace_events);
        setTraceSteps(trace.trace_summary.steps);
        setTraceTimeline(trace.timeline);
        setTaskMetrics(trace.task_metrics);
        setTraceArtifacts(trace.artifacts ?? null);
        setFactCheckSnapshot(trace.fact_check ?? null);
        setCriticSnapshot(trace.critic ?? null);
        setRequiresManual(trace.requires_manual);
        setSummary(trace.summary);
      }
    } catch (err) {
      console.warn("Unable to hydrate session", err);
    }
  };

  const handleComparisonSelect = async (event: ChangeEvent<HTMLSelectElement>) => {
    const selected = event.target.value;
    if (!selected) {
      resetComparison();
      return;
    }

    if (selected === sessionId) {
      setComparisonId(selected);
      setComparisonSummary(summary);
      setComparisonMetrics(taskMetrics);
      setComparisonTimeline(traceTimeline);
      setComparisonError(null);
      return;
    }

    if (selected === comparisonId) {
      return;
    }

    setComparisonError(null);
    try {
      const trace = await fetchTrace(selected);
      setComparisonId(selected);
      setComparisonSummary(trace.summary);
      setComparisonMetrics(trace.task_metrics);
      setComparisonTimeline(trace.timeline);
    } catch (err) {
      console.error("Failed to load comparison session", err);
      setComparisonError("Unable to load comparison session. Please try again.");
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
                  {typeof event.requires_manual === "boolean" && (
                    <span
                      className={`mt-2 inline-flex items-center rounded-full px-3 py-1 text-xs font-semibold ${
                        event.requires_manual
                          ? "bg-amber-900/40 text-amber-200"
                          : "bg-emerald-900/40 text-emerald-200"
                      }`}
                    >
                      {event.requires_manual ? "Manual review required" : "Automated pass"}
                    </span>
                  )}
                </article>
              ))}
            </div>
          </div>

          <div className="flex h-full flex-col gap-4 rounded-lg border border-slate-800 bg-slate-900/60 p-6">
            <div className="flex items-center justify-between">
              <h2 className="text-lg font-semibold text-white">Summary & Explainability</h2>
              <button
                type="button"
                onClick={refreshSessions}
                className="text-xs font-medium text-primary hover:text-primary-dark"
              >
                Refresh directory
              </button>
            </div>
            {requiresManual && (
              <div className="rounded-md border border-amber-700/60 bg-amber-900/20 px-4 py-3 text-sm text-amber-200">
                Manual review required — critic flagged this session for follow-up.
              </div>
            )}
            {summary ? (
              <div className="flex flex-col gap-4">
                <div className="prose prose-invert max-w-none">
                  <h3>Summary</h3>
                  <p>{summary}</p>
                </div>

                <div className="grid gap-3 md:grid-cols-2">
                  {factCheckSnapshot && (
                    <div className="rounded-md border border-slate-800 bg-slate-950/70 p-4 text-sm text-slate-200">
                      <header className="flex items-center justify-between text-xs font-semibold uppercase tracking-wide text-slate-400">
                        <span>Fact Check</span>
                        <span
                          className={
                            factCheckSnapshot.passed ? "text-emerald-300" : "text-amber-300"
                          }
                        >
                          {factCheckSnapshot.passed ? "Passed" : "Manual Review"}
                        </span>
                      </header>
                      <p className="mt-2 text-sm text-slate-300">
                        Confidence: {(factCheckSnapshot.confidence * 100).toFixed(1)}%
                      </p>
                      {factCheckSnapshot.verified_sources.length > 0 && (
                        <div className="mt-2 space-y-1 text-xs text-slate-400">
                          <p className="font-semibold uppercase tracking-wide text-slate-500">
                            Verified sources
                          </p>
                          <ul className="list-disc space-y-1 pl-5">
                            {factCheckSnapshot.verified_sources.map((source) => (
                              <li key={source} className="break-all">
                                {source}
                              </li>
                            ))}
                          </ul>
                        </div>
                      )}
                    </div>
                  )}
                  {criticSnapshot && (
                    <div className="rounded-md border border-slate-800 bg-slate-950/70 p-4 text-sm text-slate-200">
                      <header className="flex items-center justify-between text-xs font-semibold uppercase tracking-wide text-slate-400">
                        <span>Critic Verdict</span>
                        <span
                          className={
                            criticSnapshot.confident ? "text-emerald-300" : "text-amber-300"
                          }
                        >
                          {criticSnapshot.confident ? "Confident" : "Needs review"}
                        </span>
                      </header>
                      <p className="mt-2 text-sm text-slate-300">
                        {criticSnapshot.confident
                          ? "Automated checks passed"
                          : "Critic requested additional review."}
                      </p>
                    </div>
                  )}
                </div>

                <div className="flex flex-wrap items-center gap-2">
                  {toggleOptions.map(({ key, label, disabled }) => (
                    <button
                      key={key}
                      type="button"
                      disabled={disabled}
                      onClick={() => toggleArtifact(key)}
                      className={`rounded-md border px-3 py-1 text-xs font-semibold transition ${
                        artifactVisibility[key]
                          ? "border-primary bg-primary text-white"
                          : "border-slate-700 text-slate-300 hover:border-primary hover:text-primary"
                      } disabled:cursor-not-allowed disabled:border-slate-800 disabled:text-slate-600`}
                    >
                      {artifactVisibility[key] ? "Hide" : "Show"} {label}
                    </button>
                  ))}
                </div>

                <div className="flex flex-wrap items-center gap-2">
                  {downloadOptions.map(({ key, label, extension, content }) => (
                    <button
                      key={key}
                      type="button"
                      disabled={!content}
                      onClick={() => {
                        if (content) {
                          downloadArtifact(`${sessionSlug}.${extension}`, content);
                        }
                      }}
                      className="rounded-md border border-slate-700 px-3 py-1 text-xs font-medium text-slate-200 transition hover:border-primary hover:text-primary disabled:cursor-not-allowed disabled:border-slate-800 disabled:text-slate-600"
                    >
                      {label}
                    </button>
                  ))}
                </div>

                {artifactVisibility.timeline && traceTimeline && traceTimeline.length > 0 && (
                  <TimelineTable timeline={traceTimeline} extent={timelineExtent} />
                )}

                {artifactVisibility.metrics && taskMetrics && taskMetrics.length > 0 && (
                  <MetricsTable metrics={taskMetrics} />
                )}

                {artifactVisibility.mermaid && traceTimeline && traceTimeline.length > 0 && (
                  <ReasoningGraph timeline={traceTimeline} />
                )}

                {artifactVisibility.markdown && traceArtifacts?.markdown && (
                  <div className="rounded-md border border-slate-800 bg-slate-950/70">
                    <header className="border-b border-slate-800 px-4 py-2 text-sm font-semibold text-slate-200">
                      Trace Markdown
                    </header>
                    <pre className="overflow-x-auto whitespace-pre-wrap px-4 py-3 text-sm text-slate-200">
                      {traceArtifacts.markdown}
                    </pre>
                  </div>
                )}

                {artifactVisibility.graphviz && traceArtifacts?.graphviz && (
                  <div className="rounded-md border border-slate-800 bg-slate-950/70">
                    <header className="border-b border-slate-800 px-4 py-2 text-sm font-semibold text-slate-200">
                      Trace Graphviz
                    </header>
                    <pre className="overflow-x-auto whitespace-pre px-4 py-3 text-xs text-slate-300">
                      {traceArtifacts.graphviz}
                    </pre>
                  </div>
                )}

                {traceSteps && traceSteps.length > 0 && (
                  <details className="rounded-md border border-slate-800 bg-slate-950/70">
                    <summary className="cursor-pointer px-4 py-2 text-sm font-medium text-slate-200">
                      Reasoning Steps
                    </summary>
                    <div className="space-y-2 px-4 py-3 text-sm text-slate-200">
                      {traceSteps.map((step) => (
                        <div key={step.index} className="border-b border-slate-800 pb-2 last:border-b-0">
                          <p className="font-semibold text-primary">
                            {step.index}. {step.task_id}
                          </p>
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
                {session.requires_manual && (
                  <span className="text-[10px] font-semibold uppercase tracking-wide text-amber-300">
                    Manual review
                  </span>
                )}
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

        <section className="rounded-lg border border-slate-800 bg-slate-900/60 p-6">
          <header className="mb-4 flex flex-col gap-1 md:flex-row md:items-center md:justify-between">
            <div>
              <h2 className="text-lg font-semibold text-white">Session Comparison</h2>
              <p className="text-xs text-slate-400">
                Diff metrics and timelines between runs to support QA and analyst reviews.
              </p>
            </div>
          </header>
          <div className="flex flex-col gap-4">
            <div className="flex flex-wrap items-center gap-2">
              <label className="text-xs font-semibold uppercase tracking-wide text-slate-400">
                Compare against
              </label>
              <select
                value={comparisonId ?? ""}
                onChange={handleComparisonSelect}
                className="rounded-md border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-slate-200 outline-none transition focus:border-primary focus:ring-2 focus:ring-primary/40"
              >
                <option value="">Select session…</option>
                {comparableSessions.map((session) => (
                  <option key={session.session_id} value={session.session_id}>
                    {session.session_id}
                  </option>
                ))}
              </select>
              <button
                type="button"
                onClick={resetComparison}
                disabled={!comparisonId}
                className="rounded-md border border-slate-700 px-3 py-1 text-xs font-medium text-slate-200 transition hover:border-primary hover:text-primary disabled:cursor-not-allowed disabled:border-slate-800 disabled:text-slate-600"
              >
                Clear
              </button>
            </div>

            {comparisonError && (
              <div className="rounded-md border border-red-800/60 bg-red-900/20 px-4 py-2 text-sm text-red-200">
                {comparisonError}
              </div>
            )}

            {comparisonSummary && (
              <div className="grid gap-3 md:grid-cols-2">
                <div className="rounded-md border border-slate-800 bg-slate-950/70 p-4">
                  <h3 className="text-sm font-semibold uppercase tracking-wide text-slate-400">
                    Current Session
                  </h3>
                  <p className="mt-2 text-sm text-slate-300">
                    {summary ?? "No summary captured yet."}
                  </p>
                </div>
                <div className="rounded-md border border-slate-800 bg-slate-950/70 p-4">
                  <h3 className="text-sm font-semibold uppercase tracking-wide text-slate-400">
                    {comparisonId === sessionId ? "Same Session" : `Comparison (${comparisonId})`}
                  </h3>
                  <p className="mt-2 text-sm text-slate-300">{comparisonSummary}</p>
                </div>
              </div>
            )}

            {metricsComparison.length > 0 && (
              <div className="rounded-md border border-slate-800 bg-slate-950/70">
                <div className="overflow-x-auto">
                  <table className="min-w-full divide-y divide-slate-800 text-xs text-slate-300">
                    <thead className="bg-slate-900/70 text-slate-400">
                      <tr>
                        <th className="px-3 py-2 text-left font-semibold uppercase tracking-wide">Task</th>
                        <th className="px-3 py-2 text-left font-semibold uppercase tracking-wide">
                          Avg (Current)
                        </th>
                        <th className="px-3 py-2 text-left font-semibold uppercase tracking-wide">
                          Avg (Comparison)
                        </th>
                        <th className="px-3 py-2 text-left font-semibold uppercase tracking-wide">Delta</th>
                      </tr>
                    </thead>
                    <tbody className="divide-y divide-slate-800">
                      {metricsComparison.map(({ task_id: taskId, current, prev, delta }) => (
                        <tr key={taskId} className="hover:bg-slate-900/80">
                          <td className="px-3 py-2 font-mono text-slate-200">{taskId}</td>
                          <td className="px-3 py-2 text-slate-300">
                            {formatMilliseconds(current?.average_duration_ms)}
                          </td>
                          <td className="px-3 py-2 text-slate-300">
                            {formatMilliseconds(prev?.average_duration_ms)}
                          </td>
                          <td
                            className={`px-3 py-2 font-semibold ${
                              delta === undefined
                                ? "text-slate-400"
                                : delta > 0
                                  ? "text-amber-300"
                                  : delta < 0
                                    ? "text-emerald-300"
                                    : "text-slate-300"
                            }`}
                          >
                            {formatDelta(delta)}
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </div>
            )}

            {comparisonMetrics && comparisonMetrics.length > 0 && (
              <details className="rounded-md border border-slate-800 bg-slate-950/70">
                <summary className="cursor-pointer px-4 py-2 text-sm font-medium text-slate-200">
                  Comparison Metrics Detail
                </summary>
                <div className="px-4 py-3">
                  <MetricsTable metrics={comparisonMetrics} />
                </div>
              </details>
            )}

            {comparisonTimeline && comparisonTimeline.length > 0 && (
              <details className="rounded-md border border-slate-800 bg-slate-950/70">
                <summary className="cursor-pointer px-4 py-2 text-sm font-medium text-slate-200">
                  Comparison Timeline
                </summary>
                <div className="px-4 py-3">
                  <TimelineTable timeline={comparisonTimeline} extent={comparisonExtent} />
                </div>
              </details>
            )}
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

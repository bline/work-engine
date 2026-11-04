import { useCallback, useEffect, useRef, useState } from "react";
import { useStytchUser } from "@stytch/react";
import { useApi } from "./approvals.jsx";
import { SignIn } from "./auth.jsx";
import { readTaskProjection } from "./task-projection.js";
import { TaskHistory } from "./task-history.jsx";
import { Icon, LoadingState, Notice, PageHeader, formatDate } from "./ui.jsx";

import { readViewUrl, updateViewUrl } from "./view-url.js";
import { label, group, freshness, checkedQueue } from "./work-dashboard-data.js";

export function WorkDashboard() {
  const { user, isInitialized } = useStytchUser();
  const api = useApi();
  const [snapshot, setSnapshot] = useState(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const readState = () => {
    const params = readViewUrl().searchParams;
    const status = params.get("status");
    return { filter: ["all", "active", "held", "closed"].includes(status) ? status : "all", search: params.get("q") || "", task: params.get("task") };
  };
  const [view, setView] = useState(readState);
  const { filter, search, task: selected } = view;
  const changeView = (changes, replace = false) => { updateViewUrl(changes, replace); setView(readState()); };
  useEffect(() => { const restore = () => setView(readState()); window.addEventListener("popstate", restore); return () => window.removeEventListener("popstate", restore); }, []);
  const [lastSuccess, setLastSuccess] = useState(null);
  const [network, setNetwork] = useState(() => ({ visible: !document.hidden, online: navigator.onLine !== false }));
  const [now, setNow] = useState(Date.now());
  const generation = useRef(0);
  const controller = useRef(null);
  const inFlight = useRef(false);
  const userId = user?.user_id;
  const owner = useRef(userId);
  const ownerEpoch = useRef(0);
  if (owner.current !== userId) { owner.current = userId; ownerEpoch.current++; }
  const load = useCallback(async () => {
    if (inFlight.current) return;
    inFlight.current = true;
    const sequence = ++generation.current;
    const epoch = ownerEpoch.current;
    const abort = new AbortController(); controller.current = abort;
    setLoading(true); setError("");
    try {
      const data = await api("/api/requests", { signal: abort.signal });
      const [projection, queue] = await Promise.all([readTaskProjection(data.task_projection), checkedQueue(data.requests)]);
      if (generation.current === sequence && ownerEpoch.current === epoch) { setSnapshot({ projection, queue, pushedAt: data.pushed_at, owner: userId, epoch }); setNow(Date.now()); setLastSuccess(Date.now()); }
    } catch (e) {
      if (generation.current === sequence && e.name !== "AbortError") setError(e.message);
    } finally { if (generation.current === sequence) { inFlight.current = false; setLoading(false); } }
  }, [api, userId]);
  useEffect(() => {
    setSnapshot(null); setError(""); setLastSuccess(null); setLoading(false); inFlight.current = false;
    if (userId && !document.hidden && navigator.onLine !== false) load();
    return () => { generation.current++; controller.current?.abort(); inFlight.current = false; };
  }, [userId, load]);
  useEffect(() => {
    const sync = () => setNetwork({ visible: !document.hidden, online: navigator.onLine !== false });
    document.addEventListener("visibilitychange", sync); window.addEventListener("online", sync); window.addEventListener("offline", sync);
    return () => { document.removeEventListener("visibilitychange", sync); window.removeEventListener("online", sync); window.removeEventListener("offline", sync); };
  }, []);
  const wasRunnable = useRef(true);
  useEffect(() => {
    const runnable = Boolean(userId && network.visible && network.online);
    if (!runnable) { if (inFlight.current) { generation.current++; controller.current?.abort(); inFlight.current = false; setLoading(false); } }
    else if (!wasRunnable.current) load();
    wasRunnable.current = runnable;
    if (!runnable) return;
    const timer = setInterval(load, 60000);
    return () => clearInterval(timer);
  }, [userId, network.visible, network.online, load]);
  useEffect(() => { const timer = setInterval(() => setNow(Date.now()), 30000); return () => clearInterval(timer); }, []);
  if (!isInitialized) return <LoadingState />;
  if (!user) return <SignIn purpose="Sign in to view your work." />;
  const current = snapshot?.owner === userId && snapshot?.epoch === ownerEpoch.current ? snapshot : null;
  const projection = current?.projection;
  const tasks = projection?.tasks || [];
  const visible = tasks.filter((entry) => (filter === "all" || group(entry) === filter) &&
    `${entry.request.data.intent.objective} ${entry.task.id} ${entry.admission?.status || ""}`.toLowerCase().includes(search.toLowerCase()));
  const counts = { active: 0, held: 0, closed: 0 };
  tasks.forEach((entry) => counts[group(entry)]++);
  // The timer refreshes the local display cue; it does not change the snapshot.
  void now;
  const stale = current && (!freshness(current.pushedAt) || (projection?.available && !freshness(projection.observed_at)));
  return <div className="work-dashboard">
    <PageHeader eyebrow="YOUR WORKSPACE / WORK" title="Work at a glance." description="A read-only view of the latest reported task observations." action={<button className="button button-secondary" type="button" onClick={load} disabled={loading}><Icon name="refresh" size={16} />{loading ? "Refreshing…" : "Refresh"}</button>} />
    {!current && !loading && (!network.online || !network.visible) && <Notice>Automatic refresh paused: {!network.online ? "offline" : "page hidden"}. Use Refresh when available.</Notice>}
    {loading && !current && <LoadingState label="Loading work observations…" />}
    {error && <Notice tone="error">Couldn’t refresh work observations. {error}{current && " Showing the last successful snapshot."}</Notice>}
    {current && <>
      {stale && <Notice tone="warning">These observations may be stale or have an unusable timestamp. Five minutes is a local display cue, not an expiry policy.</Notice>}
      <div className="work-source">{loading ? "Refreshing observations" : !network.online ? "Automatic refresh paused: offline" : !network.visible ? "Automatic refresh paused: page hidden" : "Automatic refresh every 60 seconds"}{lastSuccess && <> · Last successful refresh {formatDate(new Date(lastSuccess).toISOString())}</>}</div>
      <div className="work-source">Push observed {formatDate(current.pushedAt)}{projection?.available && <> · Source observed {formatDate(projection.observed_at)} · revision {projection.source_revision}</>}</div>
      <div className="work-summary" aria-label="Reported observation summary">
        {[['Active', counts.active], ['Held / uncertain', counts.held], ['Reported closed', counts.closed]].map(([title, count]) => <div className="panel work-stat" key={title}><span>{title}</span><strong>{projection?.available ? count : "—"}</strong><small>task observations</small></div>)}
        <a className="panel work-stat work-stat-link" href="/approve"><span>Open requests</span><strong>{current.queue.dropped ? `${current.queue.count}+` : current.queue.count}</strong><small>Review decisions →</small></a>
      </div>
      {current.queue.dropped > 0 && <Notice tone="warning">Some requests could not be verified. The open request count is incomplete; visit Approvals to review available requests.</Notice>}
      {selected !== null && !projection?.available && <Notice tone="warning">The requested task cannot be resolved without an available task snapshot. <button type="button" onClick={() => changeView({ task: null })}>Clear selection</button></Notice>}
      {!projection && <Notice>No task projection was supplied with this authenticated snapshot.</Notice>}
      {projection && !projection.available && <Notice tone="warning">Task observations are unavailable or invalid. No empty snapshot has been established.</Notice>}
      {projection?.issues?.length > 0 && <Notice tone="warning"><strong>Some task observations are unavailable.</strong><ul>{projection.issues.map((issue, i) => <li key={i}>{issue}</li>)}</ul>Counts only include validated observations.</Notice>}
      {projection?.available && <section className="work-list" aria-labelledby="work-list-heading">
        <div className="section-heading"><div><h2 id="work-list-heading">Task observations</h2><p className="muted">Status is reported by the source, not a guarantee of admission or completion.</p></div></div>
        <div className="work-controls"><label>Search tasks<input type="search" value={search} onChange={(e) => changeView({ q: e.target.value }, true)} placeholder="Search objective or task ID" /></label><label>Status<select value={filter} onChange={(e) => changeView({ status: e.target.value === "all" ? null : e.target.value })}><option value="all">All observations</option><option value="active">Active</option><option value="held">Held / uncertain</option><option value="closed">Reported closed</option></select></label></div>
        {selected !== null && !visible.some((entry) => entry.task.id === selected) && <Notice tone="warning">{tasks.some((entry) => entry.task.id === selected) ? "The selected task is hidden by the current filters." : "The requested task is unavailable in this snapshot."} <button type="button" onClick={() => changeView({ task: null, status: null, q: null })}>Clear selection and filters</button></Notice>}
        {tasks.length === 0 && projection.issues.length === 0 ? <p className="work-empty">No task observations in this validated snapshot.</p> : visible.length === 0 ? <p className="work-empty">No validated observations match these filters.</p> : visible.map((entry) => {
          const { task, request, admission, result } = entry;
          const status = !admission ? "Admission not observed" : admission.authority_revision === null && !["held", "rejected"].includes(admission.status) ? "Admission not established" : label(admission.status);
          return <details className="panel work-task" key={task.id} open={selected === task.id}><summary onClick={(e) => { e.preventDefault(); changeView({ task: selected === task.id ? null : task.id }); }}><span className="work-task-title"><strong>{request.data.intent.objective}</strong><small>{task.id} · {task.revision}</small></span><span className="badge badge-amber">{status}</span></summary>
            <div className="work-task-content"><a href={(() => { const url = readViewUrl(); url.searchParams.set("task", task.id); return url.pathname + url.search + url.hash; })()}>Link to this task</a><div className="work-task-facts"><div><span>Reported status</span><strong>{label(admission?.status)}</strong></div><div><span>Task allowance</span><strong>{result?.accounting.limit ?? "Unknown"} API-equivalent USD</strong></div><div><span>Observed usage</span><strong>{result?.accounting.observed_usage ?? "Unknown"}</strong></div><div><span>Usage coverage</span><strong>{label(result?.accounting.coverage)}</strong></div></div>
              <TaskHistory projection={{ ...projection, tasks: [entry], issues: [] }} />
            </div></details>;
        })}
      </section>}
    </>}
  </div>;
}

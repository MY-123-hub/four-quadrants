import { useEffect, useId, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import type { Task } from '../domain/tasks';
import { useTasks } from '../storage/taskStore';
import { actOnMindmap, type MindmapAction } from '../mindmaps/actions';
import { Close, Mindmap } from './Icons';

export function MindmapButton({ task }: { task?: Task }) {
  const button = useRef<HTMLButtonElement>(null);
  const { mindmaps } = useTasks();
  const linked = !!task && !!mindmaps[task.id];
  const [show, setShow] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const act = async (action: MindmapAction) => {
    if (!task) return;
    setBusy(true); setError(null);
    try { if (await actOnMindmap(task.id, action)) setShow(false); }
    catch (reason) { setError(String(reason instanceof Error ? reason.message : reason)); setShow(true); }
    finally { setBusy(false); }
  };
  return <>
    <button ref={button} className={`mindmap-button${linked ? ' is-linked' : ''}`} aria-label={task ? `思维导图 ${task.title}` : '思维导图'} title={task ? `${linked ? '打开思维导图' : '新建或关联思维导图'}：${task.title}` : '请先选中任务'} aria-haspopup="dialog" disabled={busy || !task} aria-busy={busy} onClick={() => { if (linked) void act('open'); else { setError(null); setShow(true); } }}><Mindmap /></button>
    {show && task && createPortal(<MindmapChoices title={task.title} linked={linked} busy={busy} error={error} act={act} close={() => { setShow(false); requestAnimationFrame(() => button.current?.focus()); }} />, document.body)}
  </>;
}
function MindmapChoices({ title, linked, busy, error, act, close }: { title: string; linked: boolean; busy: boolean; error: string | null; act: (action: MindmapAction) => Promise<void>; close: () => void }) {
  const dialog = useRef<HTMLDialogElement>(null);
  const titleId = useId();
  useEffect(() => { const element = dialog.current!; element.showModal(); return () => element.close(); }, []);
  return <dialog ref={dialog} className="mindmap-dialog" aria-labelledby={titleId} aria-busy={busy} onCancel={event => { event.preventDefault(); if (!busy) close(); }} onClick={event => { if (event.target === event.currentTarget && !busy) close(); }}>
    <div className="mindmap-dialog-content">
      <header><h2 id={titleId}>思维导图</h2><button className="icon-button" aria-label="关闭导图选项" disabled={busy} onClick={close}><Close /></button></header>
      <p className="mindmap-task-name">{title}</p>
      {error && <p className="mindmap-error" role="alert">{error}</p>}
      <div className="mindmap-choices">
        <button disabled={busy} onClick={() => void act(linked ? 'open' : 'create')}>{linked ? '再次打开导图' : '新建思维导图'}</button>
        <button disabled={busy} onClick={() => void act('associate')}>{linked ? '重新关联已有导图' : '关联已有导图'}</button>
      </div>
      {busy && <p className="mindmap-status" role="status">正在打开…</p>}
    </div>
  </dialog>;
}

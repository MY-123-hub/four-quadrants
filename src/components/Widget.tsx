import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { quadrants, type QuadrantId, type Task } from '../domain/tasks';
import { taskStore, useTasks } from '../storage/taskStore';
import { native } from '../storage/repository';
import { hideCurrentWindow, showMain, useWindowLifecycle } from '../windows/lifecycle';
import { Check, Close, Matrix, OpenWindow, Pin, Plus, Trash } from './Icons';
import { TaskEditor } from './TaskEditor';

function WidgetTask({ task }: { task: Task }) {
  const [editing, setEditing] = useState(false);
  const update = (patch: Partial<Task>) => taskStore.change(tasks => tasks.map(t => t.id === task.id ? { ...t, ...patch } : t));
  return <li className={`widget-task${task.completed ? ' completed' : ''}${editing ? ' editing' : ''}`} data-task-id={task.id}>
    <button className="widget-check" role="checkbox" aria-checked={task.completed} aria-label={`${task.completed ? '取消完成' : '完成'} ${task.title}`} onClick={() => update({ completed: !task.completed })}><span>{task.completed && <Check />}</span></button>
    {editing ? <TaskEditor initial={task.title} label="编辑任务名称" onSave={title => { update({ title }); setEditing(false); }} onCancel={() => setEditing(false)} /> : <button className="widget-task-title" title={task.title} onClick={() => setEditing(true)}>{task.title}</button>}
    <button className="widget-delete" aria-label={`删除 ${task.title}`} onClick={() => taskStore.change(tasks => tasks.filter(t => t.id !== task.id))}><Trash /></button>
  </li>;
}
function WidgetQuadrant({ id, title, tasks }: { id: QuadrantId; title: string; tasks: Task[] }) {
  const [adding, setAdding] = useState(false);
  const scroll = useRef<HTMLDivElement>(null);
  const ordered = [...tasks.filter(t => !t.completed), ...tasks.filter(t => t.completed)];
  useEffect(() => { if (adding) scroll.current?.scrollTo({ top: 0, behavior: 'instant' }); }, [adding]);
  return <section className={`widget-quadrant widget-${id}`} data-quadrant={id} aria-labelledby={`widget-title-${id}`}>
    <header className="widget-quadrant-header"><h2 id={`widget-title-${id}`}><span className="widget-mark" />{title}</h2><span className="widget-count" aria-label={`${tasks.filter(t => !t.completed).length} 件未完成任务`}>{tasks.filter(t => !t.completed).length}</span></header>
    <div className="widget-scroll" ref={scroll}><ul>
      {adding && <li className="widget-task widget-new"><span className="widget-check"><span /></span><TaskEditor label={`新任务名称：${title}`} onSave={name => { taskStore.change(all => [...all, { id: crypto.randomUUID(), title: name, completed: false, quadrant: id }]); setAdding(false); }} onCancel={() => setAdding(false)} /></li>}
      {ordered.map(task => <WidgetTask key={task.id} task={task} />)}
    </ul>{!tasks.length && !adding && <span className="widget-empty">暂无任务</span>}</div>
    <button className="widget-add" aria-label={`添加任务到${title}`} title="添加任务" onClick={() => setAdding(true)} disabled={adding}><Plus /></button>
  </section>;
}
export function Widget() {
  const { tasks, ready, error } = useTasks();
  const [pinned, setPinned] = useState(false);
  const [busy, setBusy] = useState(false);
  const [windowError, setWindowError] = useState<string | null>(null);
  useWindowLifecycle();
  useEffect(() => { if (native) void invoke<{ pinned: boolean }>('widget_preferences').then(preferences => setPinned(preferences.pinned)).catch(() => setWindowError('无法读取窗口状态')); }, []);
  const act = (action: () => Promise<unknown>) => { setBusy(true); void action().then(() => setWindowError(null)).catch(() => setWindowError('窗口操作未成功，请重试')).finally(() => setBusy(false)); };
  return <main className={`widget-shell${pinned ? ' is-pinned' : ''}`}>
    <header className="widget-titlebar" data-tauri-drag-region>
      <span className="widget-brand" data-tauri-drag-region><Matrix /><span data-tauri-drag-region>四象限</span></span>
      <div className="widget-actions">
        <button className="widget-action widget-pin" aria-label="始终置顶" aria-pressed={pinned} title={pinned ? '取消置顶' : '始终置顶'} disabled={busy} onClick={() => act(async () => { if (native) await invoke('set_widget_pinned', { pinned: !pinned }); setPinned(!pinned); })}><Pin /></button>
        <button className="widget-action" aria-label="打开主窗口" title="打开主窗口" disabled={busy} onClick={() => act(showMain)}><OpenWindow /></button>
        <button className="widget-action" aria-label="隐藏小组件" title="隐藏小组件" disabled={busy} onClick={() => act(hideCurrentWindow)}><Close /></button>
      </div>
    </header>
    {(error || windowError) && <div className="widget-error" role="alert"><span>{error ?? windowError}</span>{error && <button onClick={taskStore.retry}>重试</button>}</div>}
    {ready ? <div className="widget-grid">{quadrants.map(q => <WidgetQuadrant key={q.id} {...q} tasks={tasks.filter(t => t.quadrant === q.id)} />)}</div> : !error && <div className="loading-state" role="status">正在打开任务…</div>}
  </main>;
}

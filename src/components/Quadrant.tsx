import { useEffect, useRef, useState } from 'react';
import { useDroppable } from '@dnd-kit/core';
import { SortableContext, verticalListSortingStrategy } from '@dnd-kit/sortable';
import type { QuadrantId, Task } from '../domain/tasks';
import { taskStore } from '../storage/taskStore';
import { Plus } from './Icons';
import { TaskEditor } from './TaskEditor';
import { TaskRow } from './TaskRow';
interface Props { id: QuadrantId; title: string; tasks: Task[]; target: boolean }
export function Quadrant({ id, title, tasks, target }: Props) {
  const [adding, setAdding] = useState(false);
  const scroll = useRef<HTMLDivElement>(null);
  const { setNodeRef } = useDroppable({ id, data: { quadrant: id } });
  useEffect(() => { if (adding) scroll.current?.scrollTo({ top: scroll.current.scrollHeight, behavior: matchMedia('(prefers-reduced-motion: reduce)').matches ? 'instant' : 'smooth' }); }, [adding]);
  return <section ref={setNodeRef} className={`quadrant quadrant-${id}${target ? ' drop-target' : ''}`} aria-labelledby={`title-${id}`} data-quadrant={id}>
    <header className="quadrant-header"><h2 id={`title-${id}`}><span className="quadrant-mark" />{title}</h2><button className="add-task icon-button" aria-label={`添加任务到${title}`} onClick={() => setAdding(true)}><Plus /></button></header>
    <div className="task-scroll" ref={scroll}>
      <SortableContext items={tasks.map(t => t.id)} strategy={verticalListSortingStrategy}>
        <ul className="task-list">{tasks.map(task => <TaskRow key={task.id} task={task} />)}
          {adding && <li className="task-row new-task"><span className="drag-spacer" /><span className="checkbox placeholder-checkbox" /><TaskEditor label={`新任务名称：${title}`} onSave={name => { taskStore.change(all => [...all, { id: crypto.randomUUID(), title: name, completed: false, quadrant: id }]); setAdding(false); }} onCancel={() => setAdding(false)} /><span className="editor-hint" aria-hidden="true">↵</span></li>}
        </ul>
      </SortableContext>
      {!tasks.length && !adding && <button className="empty-state" onClick={() => setAdding(true)}>点击 + 添加任务</button>}
    </div>
  </section>;
}

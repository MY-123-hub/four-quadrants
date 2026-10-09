import { useState } from 'react';
import { useSortable } from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import type { Task } from '../domain/tasks';
import { taskStore } from '../storage/taskStore';
import { TaskEditor } from './TaskEditor';
import { Check, Grip, Trash } from './Icons';
export function TaskRow({ task }: { task: Task }) {
  const [editing, setEditing] = useState(false);
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({ id: task.id, disabled: editing, data: { quadrant: task.quadrant } });
  const update = (patch: Partial<Task>) => taskStore.change(tasks => tasks.map(t => t.id === task.id ? { ...t, ...patch } : t));
  return <li ref={setNodeRef} className={`task-row${task.completed ? ' completed' : ''}${isDragging ? ' dragging' : ''}${editing ? ' editing' : ''}`} style={{ transform: CSS.Transform.toString(transform), transition }} data-task-id={task.id}>
    <button className="drag-handle" aria-label={`拖动 ${task.title}`} {...attributes} {...listeners}><Grip /></button>
    <button className="checkbox" role="checkbox" aria-checked={task.completed} aria-label={`${task.completed ? '取消完成' : '完成'} ${task.title}`} onClick={() => update({ completed: !task.completed })}>{task.completed && <Check />}</button>
    {editing ? <TaskEditor initial={task.title} label="编辑任务名称" onSave={title => { update({ title }); setEditing(false); }} onCancel={() => setEditing(false)} /> : <button className="task-title" onClick={() => setEditing(true)} title="点击编辑">{task.title}</button>}
    <button className="delete-task icon-button" aria-label={`删除 ${task.title}`} onClick={() => taskStore.change(tasks => tasks.filter(t => t.id !== task.id))}><Trash /></button>
  </li>;
}

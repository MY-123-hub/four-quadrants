import { quadrants, type QuadrantId, type Task } from './tasks';

export type TaskChange =
  | { kind: 'add'; task: Task }
  | { kind: 'patch'; id: string; title?: string; completed?: boolean; quadrant?: QuadrantId }
  | { kind: 'remove'; id: string }
  | { kind: 'move'; id: string; quadrant: QuadrantId; before: string | null };
export interface TaskSnapshot { revision: number; tasks: Task[] }

// UI updates express only the fields the person changed, never a stale document.
export function changesBetween(previous: Task[], next: Task[]): TaskChange[] {
  const changes: TaskChange[] = [];
  for (const task of previous) if (!next.some(t => t.id === task.id)) changes.push({ kind: 'remove', id: task.id });
  for (const task of next) {
    const old = previous.find(t => t.id === task.id);
    if (!old) { changes.push({ kind: 'add', task }); continue; }
    const patch: Extract<TaskChange, { kind: 'patch' }> = { kind: 'patch', id: task.id };
    if (task.title !== old.title) patch.title = task.title;
    if (task.completed !== old.completed) patch.completed = task.completed;
    if (task.quadrant !== old.quadrant) patch.quadrant = task.quadrant;
    if (Object.keys(patch).length > 2) changes.push(patch);
  }
  return changes;
}

export function applyChanges(tasks: Task[], changes: TaskChange[]): Task[] {
  let result = tasks;
  for (const change of changes) {
    if (change.kind === 'add') {
      if (!result.some(t => t.id === change.task.id)) result = [...result, change.task];
    } else if (change.kind === 'remove') result = result.filter(t => t.id !== change.id);
    else if (change.kind === 'patch') result = result.map(t => t.id === change.id ? { ...t, ...(change.title !== undefined && { title: change.title }), ...(change.completed !== undefined && { completed: change.completed }), ...(change.quadrant !== undefined && { quadrant: change.quadrant }) } : t);
    else {
      const task = result.find(t => t.id === change.id);
      if (!task || change.id === change.before) continue;
      const target = result.filter(t => t.quadrant === change.quadrant && t.id !== change.id);
      const index = target.findIndex(t => t.id === change.before);
      target.splice(index < 0 ? target.length : index, 0, { ...task, quadrant: change.quadrant });
      result = quadrants.flatMap(q => q.id === change.quadrant ? target : result.filter(t => t.quadrant === q.id && t.id !== change.id));
    }
  }
  return result;
}

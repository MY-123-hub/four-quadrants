export const quadrants = [
  { id: 'do', title: '重要且紧急' },
  { id: 'plan', title: '重要不紧急' },
  { id: 'delegate', title: '紧急不重要' },
  { id: 'eliminate', title: '不重要不紧急' },
] as const;
export type QuadrantId = (typeof quadrants)[number]['id'];
export interface Task { id: string; title: string; completed: boolean; quadrant: QuadrantId }
export const MAX_TITLE_LENGTH = 1000;
export function cleanTitle(title: string): string { return title.replace(/\s+/g, ' ').trim().slice(0, MAX_TITLE_LENGTH); }
export function validateTasks(value: unknown): Task[] {
  if (!Array.isArray(value)) throw new Error('任务文件格式不正确');
  const ids = new Set<string>();
  for (const task of value) {
    if (!task || typeof task.id !== 'string' || !task.id || ids.has(task.id) || typeof task.title !== 'string' || !task.title.trim() || task.title.length > MAX_TITLE_LENGTH || typeof task.completed !== 'boolean' || !quadrants.some(q => q.id === task.quadrant)) throw new Error('任务文件内容不完整');
    ids.add(task.id);
  }
  return value as Task[];
}
export function moveTask(tasks: Task[], id: string, quadrant: QuadrantId, overId?: string): Task[] {
  const task = tasks.find(t => t.id === id);
  if (!task || id === overId) return tasks;
  const current = tasks.filter(t => t.quadrant === task.quadrant);
  const target = tasks.filter(t => t.quadrant === quadrant && t.id !== id);
  let index = overId ? target.findIndex(t => t.id === overId) : target.length;
  if (index < 0) index = target.length;
  // Downward movement within a list takes the hovered row's original position.
  if (task.quadrant === quadrant && overId && current.findIndex(t => t.id === id) < current.findIndex(t => t.id === overId)) index += 1;
  target.splice(index, 0, { ...task, quadrant });
  return quadrants.flatMap(q => q.id === quadrant ? target : tasks.filter(t => t.quadrant === q.id && t.id !== id));
}

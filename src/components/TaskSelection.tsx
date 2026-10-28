import { createContext, useContext, useEffect, useMemo, useState, type ReactNode } from 'react';
import type { Task } from '../domain/tasks';
import { useTasks } from '../storage/taskStore';

type Selection = { selectedTask: Task | undefined; selectedId: string | null; selectTask: (id: string) => void };
const TaskSelection = createContext<Selection | null>(null);

// UI-only state: selecting a task never writes to SQLite.
export function TaskSelectionProvider({ children }: { children: ReactNode }) {
  const { tasks, ready } = useTasks();
  const [id, setId] = useState<string | null>(null);
  const selectedTask = tasks.find(task => task.id === id);
  useEffect(() => { if (ready && id && !selectedTask) setId(null); }, [ready, id, selectedTask]);
  const value = useMemo(() => ({ selectedTask, selectedId: selectedTask?.id ?? null, selectTask: setId }), [selectedTask]);
  return <TaskSelection.Provider value={value}>{children}</TaskSelection.Provider>;
}
export function useTaskSelection() {
  const selection = useContext(TaskSelection);
  if (!selection) throw new Error('TaskSelectionProvider is required');
  return selection;
}

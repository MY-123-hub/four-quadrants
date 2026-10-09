import { useState } from 'react';
import { DndContext, DragOverlay, PointerSensor, KeyboardSensor, useSensor, useSensors, pointerWithin, closestCenter, type CollisionDetection, type DragEndEvent } from '@dnd-kit/core';
import { sortableKeyboardCoordinates } from '@dnd-kit/sortable';
import { quadrants, type QuadrantId } from './domain/tasks';
import { taskStore, useTasks } from './storage/taskStore';
import { useWindowLifecycle, showWidget } from './windows/lifecycle';
import { Matrix } from './components/Icons';
import { Quadrant } from './components/Quadrant';
const collision: CollisionDetection = args => {
  const hits = pointerWithin(args);
  return hits.length ? (hits.some(h => !quadrants.some(q => q.id === h.id)) ? hits.filter(h => !quadrants.some(q => q.id === h.id)) : hits) : args.pointerCoordinates ? [] : closestCenter(args);
};
export default function App() {
  const { tasks, ready, error } = useTasks();
  const [active, setActive] = useState<string | null>(null);
  const [target, setTarget] = useState<QuadrantId | null>(null);
  const [windowError, setWindowError] = useState<string | null>(null);
  useWindowLifecycle();
  const sensors = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: 5 } }), useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates }));
  const dragEnd = ({ active, over }: DragEndEvent) => {
    if (over) {
      const quadrant = over.data.current?.quadrant as QuadrantId | undefined;
      if (quadrant) taskStore.move(String(active.id), quadrant, quadrants.some(q => q.id === over.id) ? undefined : String(over.id));
    }
    setActive(null); setTarget(null);
  };
  const dragged = tasks.find(t => t.id === active);
  return <main className="app-shell">
    <div className="window-top" data-tauri-drag-region><span data-tauri-drag-region>四象限</span><button className="widget-launch icon-button" aria-label="显示悬浮小组件" title="显示悬浮小组件" onClick={() => { void showWidget().then(() => setWindowError(null)).catch(() => setWindowError("无法打开小组件，请通过窗口菜单重试。")); }}><Matrix /></button></div>
    {windowError && <div className="storage-error" role="alert">{windowError}</div>}
    {error && <div className="storage-error" role="alert"><span>{error}</span><button onClick={taskStore.retry}>重试</button></div>}
    {ready ? <DndContext sensors={sensors} collisionDetection={collision} onDragStart={e => { setActive(String(e.active.id)); }} onDragOver={e => setTarget(e.over?.data.current?.quadrant ?? null)} onDragEnd={dragEnd} onDragCancel={() => { setActive(null); setTarget(null); }} accessibility={{ screenReaderInstructions: { draggable: '按空格开始拖动，用方向键移动，再按空格放下，按 Escape 取消。' }, announcements: { onDragStart: () => '已开始移动任务。', onDragOver: ({ over }) => over ? `移到${quadrants.find(q => q.id === over.data.current?.quadrant)?.title ?? '任务位置'}` : '移到象限内放下。', onDragEnd: () => '已放下任务。', onDragCancel: () => '已取消移动。' } }}>
      <div className="board">{quadrants.map(q => <Quadrant key={q.id} {...q} tasks={tasks.filter(t => t.quadrant === q.id)} target={target === q.id && active !== null} />)}</div>
      <DragOverlay dropAnimation={{ duration: 160, easing: 'ease-out' }}>{dragged && <div className="drag-preview"><span className="preview-checkbox" /><span>{dragged.title}</span></div>}</DragOverlay>
    </DndContext> : !error && <div className="loading-state" role="status">正在打开任务…</div>}
  </main>;
}

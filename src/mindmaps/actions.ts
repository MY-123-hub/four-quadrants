import { invoke } from '@tauri-apps/api/core';
import type { TaskSnapshot } from '../domain/changes';
import { native } from '../storage/repository';
import { taskStore } from '../storage/taskStore';

export type MindmapAction = 'open' | 'create' | 'associate';
export async function actOnMindmap(id: string, action: MindmapAction) {
  if (!native) throw new Error('请在四象限桌面应用中打开思维导图。');
  if (!await taskStore.flush()) throw new Error('任务尚未保存，请先重试保存。');
  return taskStore.trackOperation(async () => {
    const snapshot = await invoke<TaskSnapshot | null>('mindmap_action', { id, action });
    if (snapshot) taskStore.acceptSnapshot(snapshot);
    return snapshot !== null;
  });
}

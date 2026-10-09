import { useEffect, useRef, useState } from 'react';
import { cleanTitle, MAX_TITLE_LENGTH } from '../domain/tasks';
interface Props { initial?: string; label: string; onSave: (title: string) => void; onCancel: () => void }
export function TaskEditor({ initial = '', label, onSave, onCancel }: Props) {
  const [value, setValue] = useState(initial);
  const input = useRef<HTMLInputElement>(null);
  const finished = useRef(false);
  useEffect(() => { input.current?.focus(); if (initial) input.current?.select(); }, [initial]);
  const finish = (cancel = false) => {
    if (finished.current) return;
    finished.current = true;
    const title = cleanTitle(value);
    if (cancel || !title) onCancel(); else onSave(title);
  };
  return <input ref={input} className="task-editor" aria-label={label} placeholder="写下一件要做的事" maxLength={MAX_TITLE_LENGTH} value={value} onChange={e => setValue(e.target.value)} onBlur={() => finish()} onKeyDown={e => {
    if (e.nativeEvent.isComposing || e.nativeEvent.keyCode === 229) return;
    if (e.key === 'Enter') { e.preventDefault(); finish(); }
    if (e.key === 'Escape') { e.preventDefault(); finish(true); }
  }} />;
}

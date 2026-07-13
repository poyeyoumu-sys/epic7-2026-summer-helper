import { ActionIcon, Badge, Card, Group, ScrollArea, SegmentedControl, Text, Tooltip } from '@mantine/core';
import { IconTrash } from '@tabler/icons-react';
import { useMemo, useRef, useState, useEffect } from 'react';
import type { LogEntry } from '../types';

export function LogPanel({ logs, onClear }: { logs: LogEntry[]; onClear: () => void }) {
  const [filter, setFilter] = useState('ALL');
  const viewport = useRef<HTMLDivElement>(null);
  const visible = useMemo(
    () => filter === 'ALL' ? logs : logs.filter((item) => item.level.toUpperCase() === filter),
    [logs, filter],
  );

  useEffect(() => {
    viewport.current?.scrollTo({ top: viewport.current.scrollHeight, behavior: 'smooth' });
  }, [visible.length]);

  return (
    <Card padding="md" h="100%">
      <Group justify="space-between" mb="sm">
        <div>
          <Text fw={700}>运行日志</Text>
          <Text size="xs" c="dimmed">Rust 后端通过 Tauri 事件实时推送</Text>
        </div>
        <Group gap="xs">
          <SegmentedControl
            size="xs"
            value={filter}
            onChange={setFilter}
            data={['ALL', 'INFO', 'WARN', 'ERROR']}
          />
          <Tooltip label="清空日志"><ActionIcon variant="light" color="red" onClick={onClear}><IconTrash size={18} /></ActionIcon></Tooltip>
        </Group>
      </Group>
      <ScrollArea viewportRef={viewport} className="log-panel" type="always">
        {visible.length === 0 && <Text c="dimmed" ta="center" mt="xl">暂无日志</Text>}
        {visible.map((entry, index) => (
          <div className="log-line mono" key={`${entry.timestamp}-${index}`}>
            <span>{entry.timestamp}</span>
            <Badge size="xs" variant="light" color={entry.level === 'ERROR' ? 'red' : entry.level === 'WARN' ? 'orange' : 'blue'}>{entry.level}</Badge>
            <span>{entry.scope}</span>
            <span className={`log-${entry.level.toLowerCase()}`}>{entry.message}</span>
          </div>
        ))}
      </ScrollArea>
    </Card>
  );
}

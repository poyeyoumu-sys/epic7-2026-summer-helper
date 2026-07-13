import { Card, Grid, Group, Text, ThemeIcon } from '@mantine/core';
import { IconActivity, IconDeviceGamepad2, IconPhotoScan, IconRoute } from '@tabler/icons-react';
import type { RuntimeStatus } from '../types';

export function StatusCards({ status }: { status: RuntimeStatus }) {
  const cards = [
    { label: '运行状态', value: status.running ? '运行中' : '空闲', icon: IconActivity },
    { label: '当前设备', value: status.device || '未连接', icon: IconDeviceGamepad2 },
    { label: '截图后端', value: status.backend || '未初始化', icon: IconPhotoScan },
    { label: '当前阶段', value: status.phase || '就绪', icon: IconRoute },
  ];

  return (
    <Grid gutter="sm">
      {cards.map(({ label, value, icon: Icon }) => (
        <Grid.Col key={label} span={{ base: 12, sm: 6, lg: 3 }}>
          <Card padding="md">
            <Group justify="space-between" wrap="nowrap">
              <div>
                <Text size="xs" c="dimmed" fw={600}>{label}</Text>
                <Text className="status-value" truncate>{value}</Text>
              </div>
              <ThemeIcon variant="light" size={42} radius="md"><Icon size={22} /></ThemeIcon>
            </Group>
          </Card>
        </Grid.Col>
      ))}
    </Grid>
  );
}

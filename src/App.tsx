import {
  ActionIcon,
  AppShell,
  Badge,
  Button,
  Card,
  Checkbox,
  Divider,
  Grid,
  Group,
  NumberInput,
  Select,
  Stack,
  Switch,
  Text,
  TextInput,
  Title,
  Tooltip,
  useMantineColorScheme,
} from '@mantine/core';
import { notifications } from '@mantine/notifications';
import {
  IconBolt,
  IconCamera,
  IconDeviceDesktopSearch,
  IconMoon,
  IconPlayerPlay,
  IconPlayerStop,
  IconRefresh,
  IconRoute,
  IconSun,
  IconTestPipe,
} from '@tabler/icons-react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { LogPanel } from './components/LogPanel';
import { StatusCards } from './components/StatusCards';
import type { AppSettings, DeviceInfo, LogEntry, RunnerMode, RuntimeStatus } from './types';

const defaultStatus: RuntimeStatus = {
  running: false,
  connected: false,
  device: '',
  backend: '',
  phase: '就绪',
  pos: null,
  shield: null,
  boost: null,
  lucky: null,
  lucky_level: null,
  strategy: 'equipment_score',
};

export default function App() {
  const { colorScheme, setColorScheme } = useMantineColorScheme();
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [devices, setDevices] = useState<DeviceInfo[]>([]);
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [status, setStatus] = useState<RuntimeStatus>(defaultStatus);
  const [busy, setBusy] = useState(false);

  const saveSettings = useCallback(async (next: AppSettings) => {
    setSettings(next);
    await invoke('save_settings', { settings: next });
  }, []);

  useEffect(() => {
    if (!settings) return;
    const timer = window.setTimeout(() => {
      void invoke('save_settings', { settings }).catch((error) => {
        console.error('自动保存设置失败', error);
      });
    }, 350);
    return () => window.clearTimeout(timer);
  }, [settings]);

  useEffect(() => {
    void (async () => {
      try {
        const loaded = await invoke<AppSettings>('load_settings');
        setSettings(loaded);
        const current = await invoke<RuntimeStatus>('get_status');
        setStatus(current);
      } catch (error) {
        notifications.show({ color: 'red', title: '初始化失败', message: String(error) });
      }
    })();

    let unlistenLog: (() => void) | undefined;
    let unlistenStatus: (() => void) | undefined;
    void listen<LogEntry>('runner-log', (event) => setLogs((prev) => [...prev.slice(-1999), event.payload])).then((fn) => { unlistenLog = fn; });
    void listen<RuntimeStatus>('runner-status', (event) => setStatus(event.payload)).then((fn) => { unlistenStatus = fn; });
    return () => { unlistenLog?.(); unlistenStatus?.(); };
  }, []);

  const discover = async () => {
    setBusy(true);
    try {
      const result = await invoke<DeviceInfo[]>('discover_devices');
      setDevices(result);
      notifications.show({ color: result.length ? 'green' : 'orange', title: '设备搜索完成', message: result.length ? `找到 ${result.length} 个设备` : '未找到设备' });
    } catch (error) {
      notifications.show({ color: 'red', title: '设备搜索失败', message: String(error) });
    } finally { setBusy(false); }
  };

  const connect = async () => {
    if (!settings) return;
    setBusy(true);
    try {
      const result = await invoke<RuntimeStatus>('connect_device', { settings });
      setStatus(result);
      notifications.show({ color: 'green', title: '连接成功', message: `${result.device} · ${result.backend}` });
    } catch (error) {
      notifications.show({ color: 'red', title: '连接失败', message: String(error) });
    } finally { setBusy(false); }
  };

  const run = async (mode: RunnerMode) => {
    if (!settings) return;
    try {
      await saveSettings(settings);
      await invoke('start_runner', { mode, settings });
    } catch (error) {
      notifications.show({ color: 'red', title: '启动失败', message: String(error) });
    }
  };

  const takeScreenshot = async () => {
    try {
      const path = await invoke<string>('save_screenshot');
      notifications.show({ color: 'green', title: '截图已保存', message: path });
    } catch (error) {
      notifications.show({ color: 'red', title: '截图失败', message: String(error) });
    }
  };

  const strategyDescription = useMemo(() => settings?.strategy_mode === 'reward32_fixed'
    ? '固定执行“跑跑冲保保幸保冲幸幸”，优先到达 200 米。50/60 米保护位失败后补跑 10 米；100 米保护位仍有保护时原地重试。'
    : '沿用装备分动态规划。每次动作结算后按当前米数、资源和成功率重新计算路线。', [settings?.strategy_mode]);

  if (!settings) return <AppShell padding="xl"><Text>正在加载配置……</Text></AppShell>;

  return (
    <AppShell header={{ height: 76 }} padding="md">
      <AppShell.Header px="lg">
        <Group h="100%" justify="space-between">
          <div>
            <Group gap="xs"><IconBolt color="var(--mantine-color-blue-6)" /><Title order={2}>2026 夏活辅助</Title><Badge variant="light">Tauri + Rust + MAA</Badge></Group>
            <Text size="sm" c="dimmed">Mantine UI · EmulatorExtras 高速截图 · ADB 自动回退</Text>
          </div>
          <Tooltip label="切换明暗主题"><ActionIcon size="lg" variant="light" onClick={() => setColorScheme(colorScheme === 'dark' ? 'light' : 'dark')}>{colorScheme === 'dark' ? <IconSun /> : <IconMoon />}</ActionIcon></Tooltip>
        </Group>
      </AppShell.Header>

      <AppShell.Main>
        <Stack gap="md">
          <StatusCards status={status} />
          <Grid align="stretch">
            <Grid.Col span={{ base: 12, xl: 4 }}>
              <Stack>
                <Card padding="md">
                  <Group justify="space-between" mb="md"><div><Text fw={700}>设备与截图</Text><Text size="xs" c="dimmed">MAA 优先，失败时可回退普通 ADB</Text></div><IconDeviceDesktopSearch /></Group>
                  <Stack gap="sm">
                    <TextInput label="模拟器 Serial" value={settings.serial} onChange={(e) => setSettings({ ...settings, serial: e.currentTarget.value })} placeholder="auto / 127.0.0.1:16384" />
                    <Select label="截图后端" value={settings.capture_backend} data={[{ value: 'maa_emulator_extras', label: 'MAA EmulatorExtras（推荐）' }, { value: 'adb_screencap', label: '普通 ADB screencap' }]} onChange={(value) => value && setSettings({ ...settings, capture_backend: value as AppSettings['capture_backend'] })} />
                    <Switch label="MAA 初始化失败时自动回退 ADB" checked={settings.fallback_to_adb} onChange={(e) => setSettings({ ...settings, fallback_to_adb: e.currentTarget.checked })} />
                    <Group grow><Button loading={busy} variant="light" leftSection={<IconRefresh size={18} />} onClick={discover}>搜索设备</Button><Button loading={busy} leftSection={<IconBolt size={18} />} onClick={connect}>连接</Button></Group>
                    {devices.length > 0 && <Select label="搜索结果" placeholder="选择设备" data={devices.map((d) => ({ value: d.serial, label: `${d.name || d.serial}${d.supports_emulator_extras ? ' · EmulatorExtras' : ''}` }))} onChange={(value) => value && setSettings({ ...settings, serial: value })} />}
                    <Button variant="default" leftSection={<IconCamera size={18} />} onClick={takeScreenshot}>保存测试截图</Button>
                  </Stack>
                </Card>

                <Card padding="md">
                  <Group justify="space-between" mb="md"><div><Text fw={700}>决策模式</Text><Text size="xs" c="dimmed">设置实时保存到本地 JSON</Text></div><IconRoute /></Group>
                  <Select value={settings.strategy_mode} data={[{ value: 'equipment_score', label: '装备分优先' }, { value: 'reward32_fixed', label: '200 米优先' }]} onChange={(value) => value && void saveSettings({ ...settings, strategy_mode: value as AppSettings['strategy_mode'] })} />
                  <Text mt="sm" size="sm" c="dimmed" style={{ whiteSpace: 'pre-line' }}>{strategyDescription}</Text>
                </Card>
              </Stack>
            </Grid.Col>

            <Grid.Col span={{ base: 12, xl: 4 }}>
              <Stack>
                <Card padding="md">
                  <Text fw={700} mb="md">中途接管状态</Text>
                  <Grid>
                    <Grid.Col span={6}><NumberInput label="当前米数" min={0} max={200} step={10} suffix=" m" value={settings.manual_state.pos} onChange={(v) => setSettings({ ...settings, manual_state: { ...settings.manual_state, pos: Number(v) } })} /></Grid.Col>
                    <Grid.Col span={6}><NumberInput label="保护" min={0} max={4} value={settings.manual_state.shield} onChange={(v) => setSettings({ ...settings, manual_state: { ...settings.manual_state, shield: Number(v) } })} /></Grid.Col>
                    <Grid.Col span={6}><NumberInput label="助跑" min={0} max={2} value={settings.manual_state.boost} onChange={(v) => setSettings({ ...settings, manual_state: { ...settings.manual_state, boost: Number(v) } })} /></Grid.Col>
                    <Grid.Col span={6}><NumberInput label="幸运" min={0} max={2} value={settings.manual_state.lucky} onChange={(v) => setSettings({ ...settings, manual_state: { ...settings.manual_state, lucky: Number(v) } })} /></Grid.Col>
                  </Grid>
                </Card>

                <Card padding="md">
                  <Text fw={700} mb="md">运行控制</Text>
                  <Stack>
                    <Button variant="light" leftSection={<IconTestPipe size={18} />} disabled={status.running} onClick={() => run('recognition_test')}>识别测试</Button>
                    <Button leftSection={<IconPlayerPlay size={18} />} disabled={status.running} onClick={() => run('start_zero')}>从 0 米开始</Button>
                    <Button color="teal" leftSection={<IconPlayerPlay size={18} />} disabled={status.running} onClick={() => run('takeover_manual')}>中途接管</Button>
                    <Button color="red" variant="light" leftSection={<IconPlayerStop size={18} />} disabled={!status.running} onClick={() => invoke('stop_runner')}>停止当前流程</Button>
                  </Stack>
                </Card>

                <Card padding="md">
                  <Text fw={700}>实时内部状态</Text>
                  <Divider my="sm" />
                  <Grid>
                    {[['米数', status.pos == null ? '—' : `${status.pos}m`], ['幸运等级', status.lucky_level == null ? '—' : `LV${status.lucky_level}`], ['保护', status.shield ?? '—'], ['助跑', status.boost ?? '—'], ['幸运', status.lucky ?? '—']].map(([k, v]) => <Grid.Col key={String(k)} span={6}><Text size="xs" c="dimmed">{k}</Text><Text fw={700}>{v}</Text></Grid.Col>)}
                  </Grid>
                </Card>
              </Stack>
            </Grid.Col>

            <Grid.Col span={{ base: 12, xl: 4 }}>
              <LogPanel logs={logs} onClear={() => setLogs([])} />
            </Grid.Col>
          </Grid>
        </Stack>
      </AppShell.Main>
    </AppShell>
  );
}

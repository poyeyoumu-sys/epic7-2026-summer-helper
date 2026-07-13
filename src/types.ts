export type CaptureBackend = 'maa_emulator_extras' | 'adb_screencap';
export type StrategyMode = 'equipment_score' | 'reward32_fixed';
export type RunnerMode = 'recognition_test' | 'start_zero' | 'takeover_manual';

export interface RunnerOptions {
  action_step_timeout_ms: number;
  followup_step_timeout_ms: number;
  fast_poll_interval_ms: number;
  skill_select_delay_ms: number;
  skill_select_retry_delay_ms: number;
  skill_select_retry_limit: number;
  skill_cancel_threshold: number;
  skill_soft_confirm_threshold: number;
  unavailable_score_threshold: number;
  no_change_limit: number;
  post200_cutdown_threshold: number;
  post200_cutdown_confirm_hits: number;
  post200_cutdown_confirm_interval_ms: number;
  post200_cutdown_dismiss_timeout_ms: number;
}

export interface ManualState {
  pos: number;
  shield: number;
  boost: number;
  lucky: number;
}

export interface AppSettings {
  serial: string;
  capture_backend: CaptureBackend;
  fallback_to_adb: boolean;
  strategy_mode: StrategyMode;
  manual_state: ManualState;
  runner: RunnerOptions;
}

export interface DeviceInfo {
  serial: string;
  name: string;
  source: string;
  supports_emulator_extras: boolean;
}

export interface RuntimeStatus {
  running: boolean;
  connected: boolean;
  device: string;
  backend: string;
  phase: string;
  pos: number | null;
  shield: number | null;
  boost: number | null;
  lucky: number | null;
  lucky_level: number | null;
  strategy: StrategyMode;
}

export interface LogEntry {
  timestamp: string;
  level: string;
  scope: string;
  message: string;
}

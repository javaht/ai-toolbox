import { invoke } from '@tauri-apps/api/core';
import type {
  ClaudeDesktopCommonConfig,
  ClaudeDesktopCommonConfigInput,
  ClaudeDesktopPathInfo,
  ClaudeDesktopProvider,
  ClaudeDesktopProviderInput,
} from '@/types/claude';

export const getClaudeDesktopConfigPath = async (): Promise<string> => {
  return invoke<string>('get_claude_desktop_config_path');
};

export const getClaudeDesktopRootPathInfo = async (): Promise<ClaudeDesktopPathInfo> => {
  return invoke<ClaudeDesktopPathInfo>('get_claude_desktop_root_path_info');
};

export const getClaudeDesktopCommonConfig = async (): Promise<ClaudeDesktopCommonConfig | null> => {
  return invoke<ClaudeDesktopCommonConfig | null>('get_claude_desktop_common_config');
};

export const saveClaudeDesktopCommonConfig = async (
  input: ClaudeDesktopCommonConfigInput,
): Promise<void> => {
  await invoke('save_claude_desktop_common_config', { input });
};

export const listClaudeDesktopProviders = async (): Promise<ClaudeDesktopProvider[]> => {
  return invoke<ClaudeDesktopProvider[]>('list_claude_desktop_providers');
};

export const createClaudeDesktopProvider = async (
  provider: ClaudeDesktopProviderInput,
): Promise<ClaudeDesktopProvider> => {
  return invoke<ClaudeDesktopProvider>('create_claude_desktop_provider', { provider });
};

export const updateClaudeDesktopProvider = async (
  provider: ClaudeDesktopProvider,
): Promise<ClaudeDesktopProvider> => {
  return invoke<ClaudeDesktopProvider>('update_claude_desktop_provider', { provider });
};

export const deleteClaudeDesktopProvider = async (id: string): Promise<void> => {
  await invoke('delete_claude_desktop_provider', { id });
};

export const applyClaudeDesktopConfig = async (providerId: string): Promise<void> => {
  await invoke('apply_claude_desktop_config', { providerId });
};

export const toggleClaudeDesktopProviderDisabled = async (
  providerId: string,
  isDisabled: boolean,
): Promise<void> => {
  await invoke('toggle_claude_desktop_provider_disabled', { providerId, isDisabled });
};

export const reorderClaudeDesktopProviders = async (ids: string[]): Promise<void> => {
  await invoke('reorder_claude_desktop_providers', { ids });
};

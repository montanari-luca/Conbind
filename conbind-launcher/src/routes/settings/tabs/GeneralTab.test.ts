// @vitest-environment jsdom
import { render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';

vi.mock('@tauri-apps/api/event', () => ({
  emit: vi.fn().mockResolvedValue(undefined),
  listen: vi.fn().mockResolvedValue(() => {}),
}));

vi.mock('../../../built-in-features/shortcuts/shortcutService', () => ({
  shortcutService: { isConflict: vi.fn().mockResolvedValue(null) },
}));

vi.mock('../../../lib/ipc/commands', () => ({
  onboardingCommands: { reset: vi.fn().mockResolvedValue(undefined) },
}));

vi.mock('../../../services/feedback/feedbackService.svelte', () => ({
  feedbackService: { report: vi.fn() },
}));

vi.mock('../../../services/log/logService', () => ({
  logService: { debug: vi.fn(), info: vi.fn(), warn: vi.fn(), error: vi.fn() },
}));

import GeneralTab from './GeneralTab.svelte';
import type { SettingsHandler } from '../settingsHandlers.svelte';

function createHandler(): SettingsHandler {
  return {
    settings: { general: { startAtLogin: true, showDockIcon: false, showTrayIcon: true } },
    selectedModifier: 'Super',
    selectedKey: 'Space',
    isSaving: false,
    handleAutostartToggle: vi.fn(),
    handleTrayIconToggle: vi.fn(),
    handleDockIconToggle: vi.fn(),
  } as unknown as SettingsHandler;
}

describe('GeneralTab', () => {
  it('keeps startup and onboarding, and leaves appearance to its own page', () => {
    render(GeneralTab, { props: { handler: createHandler() } });

    expect(screen.getByText('Launch at Login')).toBeTruthy();
    expect(screen.getByText('Launcher Hotkey')).toBeTruthy();
    expect(screen.getByText('Onboarding')).toBeTruthy();
    expect(screen.queryByText('Interface Size')).toBeNull();
    expect(screen.queryByText('Theme')).toBeNull();
  });
});

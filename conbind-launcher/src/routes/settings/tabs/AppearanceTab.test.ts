// @vitest-environment jsdom
import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const {
  currentSettingsMock,
  emitMock,
  feedbackReportMock,
  logErrorMock,
  setUiScaleMock,
  updateSettingsMock,
} = vi.hoisted(() => ({
  currentSettingsMock: { appearance: { uiScale: 1.2 } },
  emitMock: vi.fn().mockResolvedValue(undefined),
  feedbackReportMock: vi.fn(),
  logErrorMock: vi.fn(),
  setUiScaleMock: vi.fn().mockResolvedValue(undefined),
  updateSettingsMock: vi.fn().mockResolvedValue(true),
}));

vi.mock('@tauri-apps/api/event', () => ({
  emit: emitMock,
  listen: vi.fn().mockResolvedValue(() => {}),
}));

vi.mock('../../../built-in-features/shortcuts/shortcutService', () => ({
  shortcutService: { isConflict: vi.fn().mockResolvedValue(null) },
}));

vi.mock('../../../lib/ipc/commands', () => ({
  discoverExtensions: vi.fn().mockResolvedValue([]),
  onboardingCommands: { reset: vi.fn().mockResolvedValue(undefined) },
}));

vi.mock('../../../lib/ipc/windowCommands', () => ({
  registerLauncherResetHook: vi.fn(),
  setUiScale: setUiScaleMock,
}));

vi.mock('../../../services/feedback/feedbackService.svelte', () => ({
  feedbackService: { report: feedbackReportMock },
}));

vi.mock('../../../services/launcher/launcherPlacementService.svelte', () => ({
  launcherPlacementService: {
    load: vi.fn().mockResolvedValue(undefined),
    placement: { monitor: 'cursor', snapEnabled: true },
    vertical: 'top',
    biasPercent: 20,
    isDragged: false,
    setMonitor: vi.fn(),
    setVertical: vi.fn(),
    setBias: vi.fn(),
    setSnapEnabled: vi.fn(),
    reset: vi.fn(),
  },
}));

vi.mock('../../../services/log/logService', () => ({
  logService: {
    debug: vi.fn(),
    info: vi.fn(),
    warn: vi.fn(),
    error: logErrorMock,
  },
}));

vi.mock('../../../services/settings/settingsService.svelte', () => ({
  settingsService: {
    currentSettings: currentSettingsMock,
    updateSettings: updateSettingsMock,
  },
}));

vi.mock('../../../services/theme/themeService', () => ({
  applyTheme: vi.fn(),
  removeTheme: vi.fn(),
}));

import AppearanceTab from './AppearanceTab.svelte';
import type { SettingsHandler } from '../settingsHandlers.svelte';

function createHandler(): SettingsHandler {
  return {
    settings: {
      general: {
        startAtLogin: false,
        showDockIcon: false,
        showTrayIcon: true,
      },
      appearance: { activeTheme: null },
    },
    selectedModifier: 'Super',
    selectedKey: 'Space',
    selectedTheme: 'system',
    selectedPalette: 'blue-mint',
    selectedLaunchView: 'default',
    isSaving: false,
    saveMessage: '',
    saveError: false,
    handleAutostartToggle: vi.fn(),
    handleTrayIconToggle: vi.fn(),
    handleDockIconToggle: vi.fn(),
    updateThemeSetting: vi.fn(),
    updatePaletteSetting: vi.fn(),
    updateLaunchView: vi.fn(),
  } as unknown as SettingsHandler;
}

describe('AppearanceTab palette', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('offers Blue Mint and Classic and reports the choice to the handler', async () => {
    const handler = createHandler();
    render(AppearanceTab, { props: { handler } });

    const blueMint = screen.getByRole('radio', { name: 'Blue Mint' });
    const classic = screen.getByRole('radio', { name: 'Classic' });
    expect(blueMint.getAttribute('aria-checked')).toBe('true');
    expect(classic.getAttribute('aria-checked')).toBe('false');

    await fireEvent.click(classic);
    expect(handler.updatePaletteSetting).toHaveBeenCalledWith('classic');
  });
});

describe('AppearanceTab interface size', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    currentSettingsMock.appearance.uiScale = 1.2;
  });

  it('shows the persisted scale and applies a selected scale to settings and the native window', async () => {
    render(AppearanceTab, { props: { handler: createHandler() } });

    expect(screen.getByText('Interface Size')).toBeTruthy();

    const select = screen.getByRole('combobox') as HTMLSelectElement;
    expect(select.value).toBe('1.2');
    expect((screen.getByRole('option', { name: 'Large' }) as HTMLOptionElement).value).toBe('1.1');

    await fireEvent.change(select, { target: { value: '1.1' } });

    await waitFor(() => {
      expect(updateSettingsMock).toHaveBeenCalledWith('appearance', { uiScale: 1.1 });
      expect(setUiScaleMock).toHaveBeenCalledWith(1.1);
    });
  });

  it('does not resize the native window when persisting the scale fails', async () => {
    updateSettingsMock.mockImplementationOnce(async (_section, values) => {
      Object.assign(currentSettingsMock.appearance, values);
      return false;
    });
    render(AppearanceTab, { props: { handler: createHandler() } });

    const select = screen.getByRole('combobox');
    await fireEvent.change(select, { target: { value: '1.1' } });

    await waitFor(() => {
      expect(updateSettingsMock).toHaveBeenCalledWith('appearance', { uiScale: 1.1 });
    });
    expect(setUiScaleMock).not.toHaveBeenCalled();
    expect(currentSettingsMock.appearance.uiScale).toBe(1.2);
    expect((select as HTMLSelectElement).value).toBe('1.2');
    expect(logErrorMock).toHaveBeenCalledWith('Failed to update interface size');
    expect(feedbackReportMock).toHaveBeenCalledWith(
      expect.objectContaining({
        severity: 'error',
        context: { message: 'Could not save the interface size' },
      }),
    );
  });
});

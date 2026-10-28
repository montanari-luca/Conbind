// @vitest-environment jsdom
import { fireEvent, render, screen } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(async () => () => {}),
}));

const mockOpenUrl = vi.fn(async (_url: string) => {});

vi.mock('@tauri-apps/plugin-opener', () => ({
  openUrl: (url: string) => mockOpenUrl(url),
}));

vi.mock('../../services/i18n', () => ({
  t: (key: string) => {
    if (key === 'settings.about.download_update') return 'Download';
    if (key === 'actions.title') return 'Actions';
    if (key === 'common.show_more') return 'Show More';
    return key;
  },
}));

vi.mock('../../services/action/actionService.svelte', () => ({
  actionService: {
    filteredActions: [],
  },
}));

vi.mock('../../services/extension/viewManager.svelte', () => ({
  viewManager: {
    activeView: null,
    activeViewPrimaryActionLabel: null,
  },
}));

vi.mock('../../services/extension/extensionManager.svelte', () => ({
  default: {
    getManifestById: () => null,
  },
}));

vi.mock('../../services/feedback/feedbackService.svelte', () => ({
  feedbackService: {
    current: null,
  },
}));

import { appUpdateState } from '../../services/update/appUpdateStore.svelte';
import { viewManager } from '../../services/extension/viewManager.svelte';
import BottomActionBar from './BottomActionBar.svelte';

describe('BottomActionBar - Update Ready Pill', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    appUpdateState.phase = 'idle';
    appUpdateState.pendingVersion = null;
    appUpdateState.downloadUrl = null;
  });

  it('does not render update-ready pill when phase is idle', () => {
    render(BottomActionBar, {
      props: {
        isActionListOpen: false,
        onactionListToggled: () => {},
        onactionListClosed: () => {},
      },
    });

    expect(screen.queryByTitle('Download')).toBeNull();
  });

  it('renders update pill with version and opens the download link on click when phase is ready', async () => {
    appUpdateState.phase = 'ready';
    appUpdateState.pendingVersion = '0.1.2';
    appUpdateState.downloadUrl = 'https://example.test/conbind.dmg';

    render(BottomActionBar, {
      props: {
        isActionListOpen: false,
        onactionListToggled: () => {},
        onactionListClosed: () => {},
      },
    });

    const pills = screen.getAllByTitle('Download');
    expect(pills.length).toBeGreaterThanOrEqual(1);
    expect(pills[0].textContent).toContain('Update 0.1.2 available');

    await fireEvent.click(pills[0]);
    expect(mockOpenUrl).toHaveBeenCalledWith('https://example.test/conbind.dmg');
  });

  it('suppresses search selectedItem action when an extension view is active without activeViewPrimaryActionLabel', () => {
    (viewManager as any).activeView = 'create-extension/BuildProgressView';
    (viewManager as any).activeViewPrimaryActionLabel = null;

    const dummyItem: any = {
      id: 'cmd-1',
      name: 'Build Extension with AI',
      type: 'command',
    };

    render(BottomActionBar, {
      props: {
        selectedItem: dummyItem,
        isActionListOpen: false,
        onactionListToggled: () => {},
        onactionListClosed: () => {},
      },
    });

    expect(screen.queryByText('Run')).toBeNull();
  });
});

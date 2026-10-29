// @vitest-environment jsdom
import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import SettingsSidebar from './SettingsSidebar.svelte';

vi.mock('@tauri-apps/api/event', () => ({
  emit: vi.fn().mockResolvedValue(undefined),
  listen: vi.fn().mockResolvedValue(() => {}),
}));

vi.mock('../../lib/developmentBuild', () => ({ developmentBuildIndicator: null }));

const groups = [
  {
    id: 'basics',
    title: 'Basics',
    items: [
      { id: 'general', label: 'General', icon: 'settings' },
      { id: 'appearance', label: 'Appearance', icon: 'palette' },
    ],
  },
  { id: 'empty', title: 'Hidden group', items: [] },
  {
    id: 'system',
    title: 'System',
    items: [{ id: 'about', label: 'About', icon: 'info' }],
  },
];

function setup(overrides: Record<string, unknown> = {}) {
  return render(SettingsSidebar, {
    props: { groups, activeTab: 'general', query: '', results: [], onpick: vi.fn(), ...overrides },
  });
}

describe('SettingsSidebar', () => {
  it('renders a vertical tablist with the active tab selected', () => {
    setup({ activeTab: 'appearance' });
    expect(screen.getByRole('tablist').getAttribute('aria-orientation')).toBe('vertical');
    expect(screen.getByRole('tab', { name: 'Appearance' }).getAttribute('aria-selected')).toBe(
      'true',
    );
    expect(screen.getByRole('tab', { name: 'General' }).getAttribute('aria-selected')).toBe(
      'false',
    );
  });

  it('skips the title of an empty group', () => {
    setup();
    expect(screen.getByText('Basics')).toBeTruthy();
    expect(screen.queryByText('Hidden group')).toBeNull();
  });

  it('only the active tab is in the tab order', () => {
    setup({ activeTab: 'general' });
    expect(screen.getByRole('tab', { name: 'General' }).getAttribute('tabindex')).toBe('0');
    expect(screen.getByRole('tab', { name: 'About' }).getAttribute('tabindex')).toBe('-1');
  });

  it('moves between tabs with the arrow keys, across groups', async () => {
    setup({ activeTab: 'appearance' });
    const tablist = screen.getByRole('tablist');
    await fireEvent.keyDown(tablist, { key: 'ArrowDown' });
    expect(screen.getByRole('tab', { name: 'About' }).getAttribute('aria-selected')).toBe('true');
    await fireEvent.keyDown(tablist, { key: 'Home' });
    expect(screen.getByRole('tab', { name: 'General' }).getAttribute('aria-selected')).toBe('true');
    await fireEvent.keyDown(tablist, { key: 'ArrowUp' });
    expect(screen.getByRole('tab', { name: 'General' }).getAttribute('aria-selected')).toBe('true');
  });

  it('selects a tab on click', async () => {
    setup();
    await fireEvent.click(screen.getByRole('tab', { name: 'About' }));
    expect(screen.getByRole('tab', { name: 'About' }).getAttribute('aria-selected')).toBe('true');
  });

  it('swaps the nav for search results while a query is typed', () => {
    setup({
      query: 'dock',
      results: [{ key: '0', label: 'Show icon in Dock', description: '', pageLabel: 'General' }],
    });
    expect(screen.queryByRole('tablist')).toBeNull();
    expect(screen.getByRole('option', { name: /Show icon in Dock/ })).toBeTruthy();
  });

  it('shows an empty state when nothing matches', () => {
    setup({ query: 'zzzz', results: [] });
    expect(screen.getByText('No matching settings')).toBeTruthy();
  });

  it('picks a result with the pointer and with Enter', async () => {
    const onpick = vi.fn();
    setup({
      query: 'a',
      onpick,
      results: [
        { key: '0', label: 'First', description: '', pageLabel: 'General' },
        { key: '1', label: 'Second', description: '', pageLabel: 'About' },
      ],
    });
    await fireEvent.click(screen.getByRole('option', { name: /Second/ }));
    expect(onpick).toHaveBeenLastCalledWith('1');

    const input = screen.getByRole('combobox');
    await fireEvent.keyDown(input, { key: 'ArrowDown' });
    await fireEvent.keyDown(input, { key: 'Enter' });
    expect(onpick).toHaveBeenLastCalledWith('1');
  });

  it('clears the query with Escape', async () => {
    setup({ query: 'dock' });
    const input = screen.getByRole('combobox') as HTMLInputElement;
    await fireEvent.keyDown(input, { key: 'Escape' });
    expect(input.value).toBe('');
  });
});

import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';

import type { ItemSummary } from '../../api/types';
import { mockCommands, shellError, type CommandHandlers } from '../../test/ipc';
import { ItemsScreen } from './ItemsScreen';

const ITEMS: ItemSummary[] = [
  { id: 'a'.repeat(32), kind: 'password', title: 'GitHub', createdAtMs: 0, updatedAtMs: 0 },
  { id: 'b'.repeat(32), kind: 'note', title: 'Diary', createdAtMs: 0, updatedAtMs: 0 },
  { id: 'c'.repeat(32), kind: 'secret', title: 'GitHub token', createdAtMs: 0, updatedAtMs: 0 },
];

function renderItems(handlers: CommandHandlers = {}, onLocked = vi.fn()) {
  const calls = mockCommands({ list_items: () => ITEMS, ...handlers });
  render(<ItemsScreen onLocked={onLocked} />);
  return { calls, onLocked, user: userEvent.setup() };
}

function listedTitles(): string[] {
  const list = screen.queryByRole('list');
  if (list === null) return [];
  return within(list)
    .getAllByRole('button')
    .map((button) => button.querySelector('.item-list__title')?.textContent ?? '');
}

describe('ItemsScreen: list', () => {
  it('lists every item sorted by title with its type', async () => {
    renderItems();
    expect(await screen.findByRole('button', { name: /Diary/ })).toHaveTextContent('Note');
    expect(listedTitles()).toEqual(['Diary', 'GitHub', 'GitHub token']);
    expect(screen.getByText('3 items')).toBeInTheDocument();
  });

  it('searches by title', async () => {
    const { user } = renderItems();
    await screen.findByRole('button', { name: /Diary/ });
    await user.type(screen.getByRole('searchbox', { name: 'Search by title' }), 'git');
    expect(listedTitles()).toEqual(['GitHub', 'GitHub token']);
    expect(screen.getByText('2 of 3 items')).toBeInTheDocument();

    await user.clear(screen.getByRole('searchbox', { name: 'Search by title' }));
    await user.type(screen.getByRole('searchbox', { name: 'Search by title' }), 'nope');
    expect(screen.getByText('No items match your search.')).toBeInTheDocument();
  });

  it('filters by type', async () => {
    const { user } = renderItems();
    await screen.findByRole('button', { name: /Diary/ });
    await user.selectOptions(screen.getByRole('combobox', { name: 'Type' }), 'Secrets');
    expect(listedTitles()).toEqual(['GitHub token']);
    await user.type(screen.getByRole('searchbox', { name: 'Search by title' }), 'diary');
    expect(listedTitles()).toEqual([]);
  });

  it('explains an empty vault', async () => {
    renderItems({ list_items: () => [] });
    expect(await screen.findByText(/Your vault is empty/)).toBeInTheDocument();
    expect(screen.getByText('0 items')).toBeInTheDocument();
  });

  it('reports a failure to load and retries', async () => {
    let attempts = 0;
    const { user } = renderItems({
      list_items: () => {
        attempts += 1;
        if (attempts === 1) throw shellError('corrupted');
        return ITEMS;
      },
    });
    expect(await screen.findByRole('alert')).toHaveTextContent('damaged');
    await user.click(screen.getByRole('button', { name: 'Try again' }));
    expect(await screen.findByRole('button', { name: /Diary/ })).toBeInTheDocument();
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });

  it('returns to the unlock screen if the vault is locked', async () => {
    const { onLocked } = renderItems({
      list_items: () => {
        throw shellError('locked');
      },
    });
    await vi.waitFor(() => {
      expect(onLocked).toHaveBeenCalled();
    });
  });
});

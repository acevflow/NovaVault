import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';

import type { ItemDetails, ItemSummary } from '../../api/types';
import { mockCommands, type CommandArgs } from '../../test/ipc';
import { ItemsScreen } from './ItemsScreen';

const GITHUB_ID = 'a'.repeat(32);
const NEW_ID = 'f'.repeat(32);

/** A fake vault: one password item, plus whatever the test creates. */
function setUp() {
  const details = new Map<string, ItemDetails>([
    [
      GITHUB_ID,
      {
        id: GITHUB_ID,
        kind: 'password',
        title: 'GitHub',
        username: 'alice',
        url: 'https://github.com',
        hasPassword: true,
        hasNotes: false,
        createdAtMs: 0,
        updatedAtMs: Date.UTC(2026, 0, 2),
      },
    ],
  ]);
  const summaries = (): ItemSummary[] =>
    [...details.values()].map(({ id, kind, title, createdAtMs, updatedAtMs }) => ({
      id,
      kind,
      title,
      createdAtMs,
      updatedAtMs,
    }));
  const calls = mockCommands({
    list_items: summaries,
    get_item: ({ id }: CommandArgs) => details.get(id as string),
    reveal_item_field: () => ({ value: 'hunter2' }),
    copy_item_field: () => ({ clearAfterSeconds: 60 }),
    create_item: ({ item }: CommandArgs) => {
      const input = item as { kind: 'note'; title: string; body: string };
      details.set(NEW_ID, {
        id: NEW_ID,
        kind: 'note',
        title: input.title,
        hasBody: input.body !== '',
        createdAtMs: 0,
        updatedAtMs: 0,
      });
      return NEW_ID;
    },
    update_item: ({ id, item }: CommandArgs) => {
      const current = details.get(id as string);
      const update = item as { title: string; username: string };
      if (current?.kind === 'password') {
        details.set(current.id, { ...current, title: update.title, username: update.username });
      }
      return null;
    },
    delete_item: ({ id }: CommandArgs) => {
      details.delete(id as string);
      return null;
    },
  });
  render(<ItemsScreen onLocked={vi.fn()} />);
  return { calls, user: userEvent.setup() };
}

function argsOf(calls: { command: string; args: CommandArgs }[], command: string) {
  return calls.filter((call) => call.command === command).map((call) => call.args);
}

async function openGitHub(user: ReturnType<typeof userEvent.setup>) {
  await user.click(await screen.findByRole('button', { name: /^GitHub/ }));
  return within(await screen.findByRole('region', { name: 'Item details' }));
}

describe('ItemsScreen: view', () => {
  it('shows plain fields and keeps secret fields hidden', async () => {
    const { calls, user } = setUp();
    const details = await openGitHub(user);
    expect(await details.findByRole('heading', { level: 2, name: 'GitHub' })).toHaveFocus();
    expect(details.getByText('alice')).toBeInTheDocument();
    expect(details.getByText('https://github.com')).toBeInTheDocument();
    expect(details.getByText('Hidden')).toBeInTheDocument();
    expect(details.queryByText('hunter2')).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: /^GitHub/ })).toHaveAttribute('aria-current', 'true');
    expect(argsOf(calls, 'get_item')).toEqual([{ id: GITHUB_ID }]);
    expect(argsOf(calls, 'reveal_item_field')).toEqual([]);
  });
});

describe('ItemsScreen: copy', () => {
  it('copies plain and secret fields through the shell', async () => {
    const { calls, user } = setUp();
    const details = await openGitHub(user);
    await user.click(await details.findByRole('button', { name: 'Copy username' }));
    expect(
      await details.findByText('Copied. The clipboard clears in 1 minute.'),
    ).toBeInTheDocument();
    await user.click(details.getByRole('button', { name: 'Copy password' }));
    expect(argsOf(calls, 'copy_item_field')).toEqual([
      { id: GITHUB_ID, field: 'username' },
      { id: GITHUB_ID, field: 'password' },
    ]);
    expect(argsOf(calls, 'reveal_item_field')).toEqual([]);
  });
});

describe('ItemsScreen: create', () => {
  it('creates a note and shows it', async () => {
    const { calls, user } = setUp();
    await user.click(await screen.findByRole('button', { name: 'New note' }));
    const form = within(screen.getByRole('region', { name: 'New note' }));
    await user.type(form.getByLabelText('Title'), 'Diary');
    await user.type(form.getByLabelText('Text'), 'dear diary');
    await user.click(form.getByRole('button', { name: 'Save' }));

    expect(await screen.findByText('Item added.')).toBeInTheDocument();
    expect(argsOf(calls, 'create_item')).toEqual([
      { item: { kind: 'note', title: 'Diary', body: 'dear diary' } },
    ]);
    expect(await screen.findByRole('heading', { level: 2, name: 'Diary' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /^Diary/ })).toHaveAttribute('aria-current', 'true');
  });

  it('sends every field of a new password', async () => {
    const { calls, user } = setUp();
    await user.click(await screen.findByRole('button', { name: 'New password' }));
    const form = within(screen.getByRole('region', { name: 'New password' }));
    await user.type(form.getByLabelText('Title'), 'Bank');
    await user.type(form.getByLabelText('Username'), 'bob');
    await user.type(form.getByLabelText('Website'), 'https://bank.example');
    await user.type(form.getByLabelText('Password'), 's3cret');
    await user.type(form.getByLabelText('Notes'), 'PIN elsewhere');
    await user.click(form.getByRole('button', { name: 'Save' }));
    await screen.findByText('Item added.');
    expect(argsOf(calls, 'create_item')).toEqual([
      {
        item: {
          kind: 'password',
          title: 'Bank',
          username: 'bob',
          url: 'https://bank.example',
          password: 's3cret',
          notes: 'PIN elsewhere',
        },
      },
    ]);
  });

  it('requires a title', async () => {
    const { calls, user } = setUp();
    await user.click(await screen.findByRole('button', { name: 'New secret' }));
    const form = within(screen.getByRole('region', { name: 'New secret' }));
    await user.type(form.getByLabelText('Value'), 'sk-123');
    await user.click(form.getByRole('button', { name: 'Save' }));
    expect(form.getByText('Enter a title.')).toBeInTheDocument();
    expect(form.getByLabelText('Title')).toHaveFocus();
    expect(argsOf(calls, 'create_item')).toEqual([]);
  });

  it('can be cancelled', async () => {
    const { user } = setUp();
    await user.click(await screen.findByRole('button', { name: 'New note' }));
    await user.click(screen.getByRole('button', { name: 'Cancel' }));
    expect(screen.queryByRole('region', { name: 'New note' })).not.toBeInTheDocument();
  });
});

describe('ItemsScreen: edit', () => {
  it('keeps secret fields that were not touched', async () => {
    const { calls, user } = setUp();
    const details = await openGitHub(user);
    await user.click(await details.findByRole('button', { name: 'Edit' }));
    const form = within(screen.getByRole('region', { name: 'Edit “GitHub”' }));
    expect(form.getByLabelText('Password')).toHaveValue('');
    expect(form.getByText('Leave empty to keep the current password.')).toBeInTheDocument();
    // Notes are empty, so there is nothing to keep or reveal.
    expect(form.queryByRole('button', { name: 'Reveal current notes' })).not.toBeInTheDocument();

    await user.clear(form.getByLabelText('Username'));
    await user.type(form.getByLabelText('Username'), 'alice2');
    await user.click(form.getByRole('button', { name: 'Save' }));

    expect(await screen.findByText('Changes saved.')).toBeInTheDocument();
    expect(argsOf(calls, 'update_item')).toEqual([
      {
        id: GITHUB_ID,
        item: {
          kind: 'password',
          title: 'GitHub',
          username: 'alice2',
          url: 'https://github.com',
          password: null,
          notes: null,
        },
      },
    ]);
    expect(argsOf(calls, 'reveal_item_field')).toEqual([]);
    expect(await screen.findByText('alice2')).toBeInTheDocument();
  });

  it('loads the current secret on request, so it can be cleared', async () => {
    const { calls, user } = setUp();
    const details = await openGitHub(user);
    await user.click(await details.findByRole('button', { name: 'Edit' }));
    const form = within(screen.getByRole('region', { name: 'Edit “GitHub”' }));
    await user.click(form.getByRole('button', { name: 'Reveal current password' }));
    expect(await form.findByDisplayValue('hunter2')).toBeInTheDocument();
    await user.clear(form.getByLabelText('Password'));
    await user.click(form.getByRole('button', { name: 'Save' }));
    await screen.findByText('Changes saved.');
    expect(argsOf(calls, 'update_item')[0]?.item).toMatchObject({ password: '', notes: null });
  });
});

describe('ItemsScreen: delete', () => {
  it('asks for confirmation and can be cancelled', async () => {
    const { calls, user } = setUp();
    const details = await openGitHub(user);
    await user.click(await details.findByRole('button', { name: 'Delete' }));
    const dialog = screen.getByRole('alertdialog', { name: 'Delete this item?' });
    expect(dialog).toHaveAccessibleDescription(/“GitHub” will be deleted/);
    expect(within(dialog).getByRole('button', { name: 'Cancel' })).toHaveFocus();

    await user.click(within(dialog).getByRole('button', { name: 'Cancel' }));
    expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument();

    await user.click(details.getByRole('button', { name: 'Delete' }));
    await user.keyboard('{Escape}');
    expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument();
    expect(argsOf(calls, 'delete_item')).toEqual([]);
  });

  it('deletes after confirmation', async () => {
    const { calls, user } = setUp();
    const details = await openGitHub(user);
    await user.click(await details.findByRole('button', { name: 'Delete' }));
    const dialog = screen.getByRole('alertdialog');
    await user.click(within(dialog).getByRole('button', { name: 'Delete' }));

    expect(await screen.findByText('Item deleted.')).toBeInTheDocument();
    expect(argsOf(calls, 'delete_item')).toEqual([{ id: GITHUB_ID }]);
    expect(await screen.findByText(/Your vault is empty/)).toBeInTheDocument();
  });
});

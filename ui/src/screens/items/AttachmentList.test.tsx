import type { Channel } from '@tauri-apps/api/core';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';

import type { Attachment, TransferProgress } from '../../api/types';
import { mockCommands, shellError, type CommandArgs } from '../../test/ipc';
import { AttachmentList } from './AttachmentList';

const ITEM_ID = 'a'.repeat(32);

const SCAN: Attachment = {
  id: 'b'.repeat(32),
  name: 'passport scan.pdf',
  size: 1536,
  addedAtMs: Date.UTC(2026, 0, 2),
};

const PHOTO: Attachment = {
  id: 'c'.repeat(32),
  name: 'photo.jpg',
  size: 3 * 1024 * 1024,
  addedAtMs: Date.UTC(2026, 0, 3),
};

/** A promise and the function that resolves it, to hold a command open. */
function deferred<T>() {
  let resolve: (value: T) => void = () => undefined;
  const promise = new Promise<T>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}

/** Sends one progress message the way the shell does, over the channel. */
function sendProgress(args: CommandArgs, progress: TransferProgress) {
  (args.onProgress as Channel<TransferProgress>).onmessage(progress);
}

function renderList(attachments: readonly Attachment[] = [SCAN]) {
  const onChange = vi.fn<(attachments: readonly Attachment[]) => void>();
  const onLocked = vi.fn<() => void>();
  render(
    <AttachmentList
      itemId={ITEM_ID}
      attachments={attachments}
      onChange={onChange}
      onLocked={onLocked}
    />,
  );
  return { onChange, onLocked, user: userEvent.setup() };
}

describe('AttachmentList', () => {
  it('lists files with their size and actions', () => {
    renderList([SCAN, PHOTO]);
    expect(screen.getByRole('heading', { name: 'Attachments' })).toBeInTheDocument();
    expect(screen.getByText('passport scan.pdf')).toBeInTheDocument();
    expect(screen.getByText(/1\.5 KB · added/)).toBeInTheDocument();
    expect(screen.getByText(/3\.0 MB · added/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Export photo.jpg' })).toBeEnabled();
    expect(screen.getByRole('button', { name: 'Remove photo.jpg' })).toBeEnabled();
  });

  it('says when there are no files', () => {
    renderList([]);
    expect(screen.getByText('No files attached.')).toBeInTheDocument();
  });

  it('attaches a file and shows its progress', async () => {
    const result = deferred<Attachment>();
    let received: CommandArgs = {};
    const calls = mockCommands({
      add_attachment: (args) => {
        received = args;
        return result.promise;
      },
    });
    const { onChange, user } = renderList();

    await user.click(screen.getByRole('button', { name: 'Attach file' }));
    // While the dialog is open: an indeterminate bar, other actions off.
    expect(screen.getByText('Choose a file in the dialog…')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Export passport scan.pdf' })).toBeDisabled();
    expect(calls[0]?.args.itemId).toBe(ITEM_ID);

    sendProgress(received, { done: 512 * 1024, total: 1024 * 1024 });
    const bar = await screen.findByRole('progressbar', { name: 'Attaching file' });
    expect(bar).toHaveAttribute('value', '50');
    expect(screen.getByText('50% · 512 KB of 1.0 MB')).toBeInTheDocument();

    result.resolve(PHOTO);
    expect(await screen.findByText('Attached “photo.jpg”.')).toBeInTheDocument();
    expect(onChange).toHaveBeenCalledWith([SCAN, PHOTO]);
    expect(screen.queryByRole('progressbar')).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Attach file' })).toBeEnabled();
  });

  it('does nothing when the dialog is closed', async () => {
    mockCommands({ add_attachment: () => null });
    const { onChange, user } = renderList();
    await user.click(screen.getByRole('button', { name: 'Attach file' }));
    await waitFor(() => {
      expect(screen.getByRole('button', { name: 'Attach file' })).toBeEnabled();
    });
    expect(onChange).not.toHaveBeenCalled();
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
    expect(screen.queryByRole('status')).not.toBeInTheDocument();
  });

  it('exports a file', async () => {
    const calls = mockCommands({ export_attachment: () => true });
    const { user } = renderList();
    await user.click(screen.getByRole('button', { name: 'Export passport scan.pdf' }));
    expect(await screen.findByText('Exported “passport scan.pdf”.')).toBeInTheDocument();
    expect(calls[0]?.command).toBe('export_attachment');
    expect(calls[0]?.args).toMatchObject({ itemId: ITEM_ID, attachmentId: SCAN.id });
  });

  it('removes a file after confirmation', async () => {
    const calls = mockCommands({ delete_attachment: () => null });
    const { onChange, user } = renderList([SCAN, PHOTO]);

    await user.click(screen.getByRole('button', { name: 'Remove passport scan.pdf' }));
    const dialog = screen.getByRole('alertdialog', { name: 'Remove this attachment?' });
    expect(dialog).toHaveTextContent('“passport scan.pdf” will be removed from this item');
    expect(calls).toHaveLength(0);

    await user.click(screen.getByRole('button', { name: 'Remove' }));
    expect(await screen.findByText('Removed “passport scan.pdf”.')).toBeInTheDocument();
    expect(calls[0]?.args).toEqual({ itemId: ITEM_ID, attachmentId: SCAN.id });
    expect(onChange).toHaveBeenCalledWith([PHOTO]);
  });

  it('keeps the file when removal is cancelled', async () => {
    const calls = mockCommands({});
    const { user } = renderList();
    await user.click(screen.getByRole('button', { name: 'Remove passport scan.pdf' }));
    await user.click(screen.getByRole('button', { name: 'Cancel' }));
    expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument();
    expect(calls).toHaveLength(0);
  });

  it('reports a locked vault to its parent', async () => {
    mockCommands({
      add_attachment: () => {
        throw shellError('locked');
      },
    });
    const { onLocked, user } = renderList();
    await user.click(screen.getByRole('button', { name: 'Attach file' }));
    await waitFor(() => {
      expect(onLocked).toHaveBeenCalledOnce();
    });
  });

  it('shows errors', async () => {
    mockCommands({
      export_attachment: () => {
        throw shellError('corrupted');
      },
    });
    const { user } = renderList();
    await user.click(screen.getByRole('button', { name: 'Export passport scan.pdf' }));
    expect(await screen.findByRole('alert')).toHaveTextContent(
      'The vault data is damaged and cannot be read.',
    );
  });
});

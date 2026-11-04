import { describe, expect, it } from 'vitest';

import { mockCommands, shellError, TEST_RECOVERY_CODE } from '../test/ipc';
import {
  changeMasterPassword,
  createItem,
  createVault,
  deleteItem,
  getItem,
  getSettings,
  listItems,
  regenerateRecoveryCode,
  revealItemField,
  setAutoLockTimeout,
  unlock,
  updateItem,
  vaultStatus,
} from './commands';
import { CommandError, errorMessage, isCommandError, toCommandError } from './errors';

describe('commands', () => {
  it('passes camelCase arguments and unwraps responses', async () => {
    const calls = mockCommands({
      vault_status: () => ({ exists: true, unlocked: false }),
      create_vault: () => ({ recoveryCode: TEST_RECOVERY_CODE }),
      unlock: () => null,
      change_master_password: () => null,
      regenerate_recovery_code: () => ({ recoveryCode: TEST_RECOVERY_CODE }),
      get_settings: () => ({ autoLockMinutes: 5 }),
      set_auto_lock_timeout: () => null,
    });
    const current = { method: 'password', secret: 'old' } as const;

    await expect(vaultStatus()).resolves.toEqual({ exists: true, unlocked: false });
    await expect(createVault('pw')).resolves.toBe(TEST_RECOVERY_CODE);
    await unlock({ method: 'recoveryCode', secret: 'code' });
    await changeMasterPassword(current, 'new');
    await expect(regenerateRecoveryCode(current)).resolves.toBe(TEST_RECOVERY_CODE);
    await expect(getSettings()).resolves.toEqual({ autoLockMinutes: 5 });
    await setAutoLockTimeout(15);

    expect(calls.map(({ command, args }) => [command, args])).toEqual([
      ['vault_status', {}],
      ['create_vault', { password: 'pw' }],
      ['unlock', { credential: { method: 'recoveryCode', secret: 'code' } }],
      ['change_master_password', { currentCredential: current, newPassword: 'new' }],
      ['regenerate_recovery_code', { currentCredential: current }],
      ['get_settings', {}],
      ['set_auto_lock_timeout', { minutes: 15 }],
    ]);
  });

  it('wraps the item commands', async () => {
    const id = 'a'.repeat(32);
    const details = {
      id,
      kind: 'note',
      title: 'Diary',
      hasBody: true,
      createdAtMs: 1,
      updatedAtMs: 2,
    } as const;
    const calls = mockCommands({
      list_items: () => [],
      get_item: () => details,
      reveal_item_field: () => ({ value: 'dear diary' }),
      create_item: () => id,
      update_item: () => null,
      delete_item: () => null,
    });

    await expect(listItems()).resolves.toEqual([]);
    await expect(getItem(id)).resolves.toEqual(details);
    await expect(revealItemField(id, 'body')).resolves.toBe('dear diary');
    await expect(createItem({ kind: 'note', title: 'Diary', body: 'x' })).resolves.toBe(id);
    await updateItem(id, { kind: 'note', title: 'Journal', body: null });
    await deleteItem(id);

    expect(calls.map(({ command, args }) => [command, args])).toEqual([
      ['list_items', {}],
      ['get_item', { id }],
      ['reveal_item_field', { id, field: 'body' }],
      ['create_item', { item: { kind: 'note', title: 'Diary', body: 'x' } }],
      ['update_item', { id, item: { kind: 'note', title: 'Journal', body: null } }],
      ['delete_item', { id }],
    ]);
  });

  it('rejects with a typed CommandError', async () => {
    mockCommands({
      unlock: () => {
        throw shellError('invalid_credentials');
      },
    });
    const error: unknown = await unlock({ method: 'password', secret: 'x' }).catch(
      (reason: unknown) => reason,
    );
    expect(isCommandError(error, 'invalid_credentials')).toBe(true);
    expect(errorMessage(error)).toBe('That password or recovery code is not correct.');
  });
});

describe('toCommandError', () => {
  it('maps malformed and unknown rejections to internal', () => {
    for (const reason of [
      'boom',
      null,
      new Error('x'),
      { code: 'from_the_future' },
      { code: 42 },
    ]) {
      expect(toCommandError(reason).code).toBe('internal');
    }
  });

  it('keeps known codes and existing errors', () => {
    const error = new CommandError('locked', 'm');
    expect(toCommandError(error)).toBe(error);
    expect(toCommandError({ code: 'storage', message: 'm' }).code).toBe('storage');
  });
});

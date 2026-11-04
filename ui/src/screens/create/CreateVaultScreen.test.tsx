import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';

import { mockCommands, shellError, TEST_RECOVERY_CODE } from '../../test/ipc';
import { CreateVaultScreen } from './CreateVaultScreen';

const STRONG = 'violet anchor gravel summit 42 lantern';

/** Waits until zxcvbn (loaded lazily) has rated the typed password. */
async function waitForStrength(label: RegExp) {
  expect(await screen.findByText(label, {}, { timeout: 10_000 })).toBeInTheDocument();
}

async function choosePassword(password: string, confirmation = password) {
  const user = userEvent.setup();
  await user.type(screen.getByLabelText('Master password'), password);
  await user.type(screen.getByLabelText('Confirm master password'), confirmation);
  return user;
}

describe('CreateVaultScreen', () => {
  it('rejects weak passwords without creating a vault', async () => {
    const calls = mockCommands({});
    render(<CreateVaultScreen onCreated={vi.fn()} />);
    const user = await choosePassword('password1');
    await waitForStrength(/Password strength: Very weak/);
    await user.click(screen.getByRole('button', { name: 'Create vault' }));
    expect(screen.getByRole('alert')).toHaveTextContent('must rate at least “Strong”');
    expect(calls).toHaveLength(0);
  });

  it('reports mismatched confirmations on the field', async () => {
    const calls = mockCommands({});
    render(<CreateVaultScreen onCreated={vi.fn()} />);
    const user = await choosePassword(STRONG, `${STRONG}x`);
    await waitForStrength(/Password strength: Very strong/);
    await user.click(screen.getByRole('button', { name: 'Create vault' }));
    expect(screen.getByLabelText('Confirm master password')).toHaveAccessibleDescription(
      'The passwords do not match.',
    );
    expect(calls).toHaveLength(0);
  });

  it('shows shell errors', async () => {
    mockCommands({
      create_vault: () => {
        throw shellError('vault_exists');
      },
    });
    render(<CreateVaultScreen onCreated={vi.fn()} />);
    const user = await choosePassword(STRONG);
    await waitForStrength(/Very strong/);
    await user.click(screen.getByRole('button', { name: 'Create vault' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('A vault already exists');
  });

  it('creates the vault, shows the code once and requires retyping it', async () => {
    const calls = mockCommands({ create_vault: () => ({ recoveryCode: TEST_RECOVERY_CODE }) });
    const print = vi.spyOn(window, 'print').mockImplementation(() => undefined);
    const onCreated = vi.fn();
    render(<CreateVaultScreen onCreated={onCreated} />);
    const user = await choosePassword(STRONG);
    await waitForStrength(/Very strong/);
    await user.click(screen.getByRole('button', { name: 'Create vault' }));

    // Step 2: the code is shown, grouped, and can be printed.
    expect(await screen.findByRole('heading', { name: 'Save your recovery code' })).toHaveFocus();
    expect(calls).toEqual([{ command: 'create_vault', args: { password: STRONG } }]);
    const groups = screen.getAllByRole('listitem').map((item) => item.textContent);
    expect(groups.join('-')).toBe(TEST_RECOVERY_CODE);
    await user.click(screen.getByRole('button', { name: 'Print' }));
    expect(print).toHaveBeenCalledOnce();

    // Continuing requires confirming the code was saved.
    const proceed = screen.getByRole('button', { name: 'Continue' });
    expect(proceed).toBeDisabled();
    await user.click(screen.getByLabelText(/I have saved my recovery code/));
    await user.click(proceed);

    // Step 3: a wrong code is rejected, the right one in any form finishes.
    const field = screen.getByLabelText('Recovery code');
    await user.type(field, 'ABCD-EFGH-JKLM-NPQR-STUV-WXYZ-2345-6788');
    await user.click(screen.getByRole('button', { name: 'Confirm' }));
    expect(field).toHaveAttribute('aria-invalid', 'true');
    expect(onCreated).not.toHaveBeenCalled();

    await user.clear(field);
    await user.type(field, TEST_RECOVERY_CODE.toLowerCase().replaceAll('-', ' '));
    await user.click(screen.getByRole('button', { name: 'Confirm' }));
    expect(onCreated).toHaveBeenCalledOnce();
    expect(document.body).not.toHaveTextContent('ABCD');
  });

  it('can go back to the code from the confirmation step', async () => {
    mockCommands({ create_vault: () => ({ recoveryCode: TEST_RECOVERY_CODE }) });
    render(<CreateVaultScreen onCreated={vi.fn()} />);
    const user = await choosePassword(STRONG);
    await waitForStrength(/Very strong/);
    await user.click(screen.getByRole('button', { name: 'Create vault' }));
    await user.click(await screen.findByLabelText(/I have saved my recovery code/));
    await user.click(screen.getByRole('button', { name: 'Continue' }));
    await user.click(screen.getByRole('button', { name: 'Show the code again' }));
    expect(screen.getByRole('list', { name: 'Recovery code' })).toBeInTheDocument();
  });
});

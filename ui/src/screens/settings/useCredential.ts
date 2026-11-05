import { useCallback, useState } from 'react';

import type { Credential, CredentialMethod } from '../../api/types';

/** State of a re-authentication form (`CurrentCredentialFields`). */
export interface CredentialState {
  /** Which secret the user confirms with. */
  readonly method: CredentialMethod;
  /** The typed secret. */
  readonly secret: string;
  /** The credential to send to the shell. */
  readonly credential: Credential;
  /** Switches method and clears the secret, so it never carries over. */
  readonly setMethod: (method: CredentialMethod) => void;
  /** Replaces the typed secret. */
  readonly setSecret: (secret: string) => void;
  /** Clears the secret, after use or after a failure. */
  readonly clear: () => void;
}

/**
 * Holds the re-authentication credential typed in a settings form. Callers
 * must call `clear` after every attempt, so the secret does not linger in
 * state.
 */
export function useCredential(): CredentialState {
  const [method, setMethodState] = useState<CredentialMethod>('password');
  const [secret, setSecret] = useState('');
  const setMethod = useCallback((next: CredentialMethod) => {
    setMethodState(next);
    setSecret('');
  }, []);
  const clear = useCallback(() => {
    setSecret('');
  }, []);
  return { method, secret, credential: { method, secret }, setMethod, setSecret, clear };
}

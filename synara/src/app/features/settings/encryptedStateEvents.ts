import { invokeDesktopWithAvailability, isSynaraDesktop } from '../../utils/desktop';
import { getSharedSettings } from '../../state/settings';

/** Push the create/opt-in gate to Core. Decrypt in already-flagged rooms stays on. */
export async function pushEncryptedStateEventsSetting(enabled = getSharedSettings().encryptedStateEvents): Promise<void> {
  if (!isSynaraDesktop()) return;
  try {
    await invokeDesktopWithAvailability('matrix_set_encrypted_state_events_setting', {
      enabled,
    });
  } catch {
    // Setting is a backstop. Create/opt-in UI still hides the checkbox locally.
  }
}

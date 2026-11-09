import { isObject } from '../matrix-dto/parseUtil';
import type {
  CommandOutcome,
  CommandSuggestion,
  ComposerCommandError,
  ComposerRunResponse,
  MentionCandidate,
  TypedMention,
} from '../matrix-dto/generated';
import { invokeDesktopWithAvailability } from '../../utils/desktop';
import type { NativeInvoke } from '../../native/nativeWire';
import { COMPOSER_NOT_SIGNED_IN, ComposerUserError } from './composerSendError';

/**
 * Slash commands and typed mentions through Core's shared composer core
 * (`crates/synara-core/src/app/composer`). Core parses, checks power levels,
 * runs the command and says what the composer should do next.
 */

let invoke: NativeInvoke = (command, args) => invokeDesktopWithAvailability(command, args);

/** Route commands through a fake invoke (tests). */
export const setNativeComposerCommandsInvokeForTests = (next: NativeInvoke): void => {
  invoke = next;
};

const call = async (command: string, args: Record<string, unknown>): Promise<unknown> => {
  const result = await invoke(command, args);
  if (!result.available) throw new ComposerUserError(COMPOSER_NOT_SIGNED_IN);
  return result.value;
};

const isSuggestion = (value: unknown): value is CommandSuggestion =>
  isObject(value) &&
  typeof value.name === 'string' &&
  typeof value.usage === 'string' &&
  typeof value.description === 'string';

/** Commands the user may use in `roomId` that start with `prefix`. */
export const composerCommandSuggestions = async (
  roomId: string,
  prefix: string
): Promise<CommandSuggestion[]> => {
  const value = await call('matrix_composer_command_suggestions', { roomId, prefix });
  return Array.isArray(value) ? value.filter(isSuggestion) : [];
};

const OUTCOME_KINDS = new Set([
  'done',
  'needs_confirmation',
  'send_message',
  'send_poll',
  'open_gif_picker',
  'navigate',
  'open_profile',
  'help',
]);

/** A command Core refused or that failed; its message is composer copy. */
export class ComposerCommandRefusal extends ComposerUserError {
  readonly detail: ComposerCommandError;

  constructor(detail: ComposerCommandError) {
    super(detail.message);
    this.name = 'ComposerCommandRefusal';
    this.detail = detail;
  }
}

export const parseComposerRunResponse = (value: unknown): CommandOutcome => {
  if (!isObject(value)) throw new Error('Synara could not run the command. Try again.');
  const { outcome, error } = value as Partial<ComposerRunResponse>;
  if (isObject(error) && typeof error.message === 'string') {
    throw new ComposerCommandRefusal(error as ComposerCommandError);
  }
  if (isObject(outcome) && OUTCOME_KINDS.has(String(outcome.kind))) {
    return outcome as CommandOutcome;
  }
  throw new Error('Synara could not run the command. Try again.');
};

/**
 * Run what the composer holds. Throws {@link ComposerCommandRefusal} with
 * user copy when the command did not run.
 */
export const runComposerCommand = async (
  roomId: string,
  text: string,
  confirmed = false
): Promise<CommandOutcome> =>
  parseComposerRunResponse(await call('matrix_composer_run_command', { roomId, text, confirmed }));

const isTypedMention = (value: unknown): value is TypedMention =>
  isObject(value) &&
  typeof value.start === 'number' &&
  typeof value.end === 'number' &&
  typeof value.userId === 'string' &&
  typeof value.text === 'string';

/** "@Name" mentions in `text` that name exactly one of `members`. */
export const resolveTypedMentions = async (
  text: string,
  members: MentionCandidate[],
  ownUserId: string
): Promise<TypedMention[]> => {
  if (!text.includes('@') || members.length === 0) return [];
  const value = await call('matrix_composer_resolve_mentions', { text, members, ownUserId });
  return Array.isArray(value) ? value.filter(isTypedMention) : [];
};

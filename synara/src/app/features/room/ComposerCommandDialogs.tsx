import React from 'react';
import { Box, Text, config } from 'folds';
import { ConfirmDialog } from '../../components/confirm-dialog';
import type { CommandSuggestion } from '../matrix-dto/generated';

type CommandConfirmDialogProps = {
  title: string;
  message: string;
  confirmLabel: string;
  onConfirm: () => void;
  onCancel: () => void;
};

/** Asks before Core runs a destructive command (/ban, /delete, /upgraderoom). */
export function CommandConfirmDialog({
  title,
  message,
  confirmLabel,
  onConfirm,
  onCancel,
}: CommandConfirmDialogProps) {
  return (
    <ConfirmDialog
      title={title}
      description={message}
      confirmLabel={confirmLabel}
      tone="critical"
      onConfirm={onConfirm}
      onCancel={onCancel}
    />
  );
}

type CommandHelpDialogProps = {
  commands: CommandSuggestion[];
  onClose: () => void;
};

/** /help: the commands this user may use in this room, with their usage. */
export function CommandHelpDialog({ commands, onClose }: CommandHelpDialogProps) {
  return (
    <ConfirmDialog
      title="Commands"
      description="Type a command at the start of a message. Commands you can't use in this room are not listed."
      confirmLabel="Done"
      tone="primary"
      hideCancel
      onConfirm={onClose}
      onCancel={onClose}
    >
      <Box
        direction="Column"
        gap="200"
        style={{ maxHeight: '50vh', overflowY: 'auto', paddingTop: config.space.S200 }}
        data-testid="composer-command-help"
      >
        {commands.map((command) => (
          <Box key={command.name} direction="Column" gap="100">
            <Text size="T300" style={{ fontFamily: 'monospace' }}>
              {command.usage}
            </Text>
            <Text size="T200" priority="300">
              {command.aliases.length > 0
                ? `${command.description} (also ${command.aliases.map((alias) => `/${alias}`).join(', ')})`
                : command.description}
            </Text>
          </Box>
        ))}
      </Box>
    </ConfirmDialog>
  );
}

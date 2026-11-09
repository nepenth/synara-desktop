import React, { KeyboardEvent as ReactKeyboardEvent, useEffect, useState } from 'react';
import { Editor } from 'slate';
import { Box, config, MenuItem, Text } from 'folds';
import type { RoomReading } from '../../utils/room';
import {
  AutocompleteMenu,
  AutocompleteQuery,
  AutocompleteSelectFirstRef,
  useAutocompleteSelectFirst,
  createCommandElement,
  moveCursor,
  replaceWithElement,
} from '../../components/editor';
import { useKeyDown } from '../../hooks/useKeyDown';
import { onTabPress } from '../../utils/keyboard';
import type { CommandSuggestion } from '../matrix-dto/generated';
import { composerCommandSuggestions } from './nativeComposerCommands';

type CommandAutocompleteProps = {
  room: RoomReading;
  editor: Editor;
  query: AutocompleteQuery<string>;
  requestClose: () => void;
  selectFirstRef?: AutocompleteSelectFirstRef;
};

/**
 * Commands from Core for what follows "/": only the ones this user may use
 * in this room (Core filters by power level and by room kind), with usage.
 */
export function CommandAutocomplete({
  room,
  editor,
  query,
  requestClose,
  selectFirstRef,
}: CommandAutocompleteProps) {
  const [suggestions, setSuggestions] = useState<CommandSuggestion[]>([]);

  useEffect(() => {
    let current = true;
    composerCommandSuggestions(room.roomId, query.text)
      .then((next) => {
        if (current) setSuggestions(next);
      })
      .catch(() => {
        if (current) setSuggestions([]);
      });
    return () => {
      current = false;
    };
  }, [room.roomId, query.text]);

  const handleAutocomplete = (commandName: string) => {
    const cmdEl = createCommandElement(commandName);
    replaceWithElement(editor, query.range, cmdEl);
    moveCursor(editor, true);
    requestClose();
  };

  const selectFirst = (): boolean => {
    if (suggestions.length === 0) return false;
    handleAutocomplete(suggestions[0].name);
    return true;
  };
  useAutocompleteSelectFirst(selectFirstRef, selectFirst);

  useKeyDown(window, (evt: KeyboardEvent) => {
    onTabPress(evt, () => {
      selectFirst();
    });
  });

  return suggestions.length === 0 ? null : (
    <AutocompleteMenu
      headerContent={
        <Box grow="Yes" direction="Row" gap="200" justifyContent="SpaceBetween">
          <Text size="L400">Commands</Text>
        </Box>
      }
      requestClose={requestClose}
    >
      {suggestions.map((suggestion) => (
        <MenuItem
          key={suggestion.name}
          as="button"
          radii="300"
          style={{ height: 'unset' }}
          onKeyDown={(evt: ReactKeyboardEvent<HTMLButtonElement>) =>
            onTabPress(evt, () => handleAutocomplete(suggestion.name))
          }
          onClick={() => handleAutocomplete(suggestion.name)}
        >
          <Box
            style={{ padding: `${config.space.S300} 0` }}
            grow="Yes"
            direction="Column"
            gap="100"
            justifyContent="SpaceBetween"
          >
            <Text style={{ flexGrow: 1 }} size="B400" truncate>
              {suggestion.matchedAlias
                ? `/${suggestion.name} (/${suggestion.matchedAlias})`
                : `/${suggestion.name}`}
            </Text>
            <Text truncate priority="300" size="T200">
              {suggestion.description}
            </Text>
            <Text truncate priority="300" size="T200" style={{ fontFamily: 'monospace' }}>
              {suggestion.usage}
            </Text>
          </Box>
        </MenuItem>
      ))}
    </AutocompleteMenu>
  );
}
